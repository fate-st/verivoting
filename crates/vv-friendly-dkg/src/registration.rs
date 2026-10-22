//! Minimal Jubjub registration policy and authenticated closing record.

use super::{verify_signature_with_domain, AuthenticationKeypair, DkgError, SchnorrSignature};
use ark_ec::Group;
use ark_ff::PrimeField;
use rand_core::{CryptoRng, RngCore};
use std::collections::HashSet;
use thiserror::Error;
use vv_friendly_crypto::{
    derive_secondary_generator, encode_point, generator, random_nonzero_scalar, validate_point,
    FriendlyCryptoError, Point, Scalar, SUITE_ID,
};
use vv_types::{sha256, CanonicalEncoder};

pub const REGISTRATION_VERSION: u32 = 1;
pub const CLOSE_SIGNATURE_DOMAIN: &[u8] = b"REGISTRATION-CLOSE-SIGNATURE";

#[derive(Clone, Debug)]
pub struct RegistrationPolicy {
    pub authority_public: Point,
    pub w_max: u64,
    pub rule_id: String,
    pub version: u32,
}

impl RegistrationPolicy {
    pub fn new(
        authority_public: Point,
        w_max: u64,
        rule_id: impl Into<String>,
    ) -> Result<Self, RegistryError> {
        validate_point(&authority_public, false)?;
        let rule_id = rule_id.into();
        if w_max == 0 || rule_id.is_empty() {
            return Err(RegistryError::InvalidPolicy);
        }
        Ok(Self {
            authority_public,
            w_max,
            rule_id,
            version: REGISTRATION_VERSION,
        })
    }

    pub fn digest(&self, election_id: [u8; 32]) -> [u8; 32] {
        let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/REGISTRATION/POLICY/v1");
        encoded.text(SUITE_ID);
        encoded.fixed(&election_id);
        encoded.fixed(&encode_point(&self.authority_public));
        encoded.u64(self.w_max);
        encoded.text(&self.rule_id);
        encoded.u32(self.version);
        sha256(&encoded.finish())
    }
}

#[derive(Clone)]
pub struct RegistrationAuthority {
    keypair: AuthenticationKeypair,
}

impl RegistrationAuthority {
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            keypair: AuthenticationKeypair::generate(rng),
        }
    }

    pub fn public_key(&self) -> Point {
        self.keypair.public
    }

    pub fn close<R: RngCore + CryptoRng>(
        &self,
        mut entries: Vec<RegistrationEntry>,
        election_id: [u8; 32],
        final_context: [u8; 32],
        policy_pre: [u8; 32],
        policy: &RegistrationPolicy,
        rng: &mut R,
    ) -> Result<RegistrationCertificate, RegistryError> {
        if self.keypair.public != policy.authority_public {
            return Err(RegistryError::InvalidPolicy);
        }
        if entries.is_empty() {
            return Err(RegistryError::Empty);
        }
        entries.sort_by_key(|entry| entry.credential_id);
        let mut seen = HashSet::new();
        let mut total = 0u64;
        for entry in &entries {
            if !seen.insert(entry.credential_id) {
                return Err(RegistryError::DuplicateCredential);
            }
            if entry.weight == 0 || entry.weight > policy.w_max {
                return Err(RegistryError::WeightOverflow);
            }
            if !entry.verify()? {
                return Err(RegistryError::BadCommitment);
            }
            total = total
                .checked_add(entry.weight)
                .ok_or(RegistryError::WeightOverflow)?;
        }
        if total > policy.w_max {
            return Err(RegistryError::WeightOverflow);
        }
        let public_leaves = entries
            .iter()
            .map(|entry| sha256(&entry.public_bytes()))
            .collect::<Vec<_>>();
        let mut registry = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/CANONICAL/v1");
        registry.text(SUITE_ID);
        registry.u64(entries.len() as u64);
        for entry in &entries {
            registry.bytes(&entry.authority_bytes());
        }
        let close = RegistrationClose {
            election_id,
            final_context,
            policy_pre,
            root_close: merkle_root(&public_leaves),
            n_registered: entries.len() as u64,
            w_registered: total,
            w_max: policy.w_max,
            version: policy.version,
            canonical_registry_digest: sha256(&registry.finish()),
        };
        let signature = self
            .keypair
            .sign(CLOSE_SIGNATURE_DOMAIN, &close.signing_message(), rng)?;
        Ok(RegistrationCertificate { close, signature })
    }
}

#[derive(Clone)]
pub struct RegistrationEntry {
    pub credential_id: [u8; 32],
    pub weight: u64,
    commitment: Point,
    opening: Scalar,
}

impl RegistrationEntry {
    pub fn new<R: RngCore + CryptoRng>(
        credential_id: [u8; 32],
        weight: u64,
        rng: &mut R,
    ) -> Result<Self, RegistryError> {
        let opening = random_nonzero_scalar(rng);
        let secondary = derive_secondary_generator(b"REGISTRATION-PEDERSEN-H")?;
        let commitment = generator().mul_bigint(Scalar::from(weight).into_bigint())
            + secondary.mul_bigint(opening.into_bigint());
        Ok(Self {
            credential_id,
            weight,
            commitment,
            opening,
        })
    }

    pub fn commitment(&self) -> Point {
        self.commitment
    }

    fn verify(&self) -> Result<bool, RegistryError> {
        validate_point(&self.commitment, false)?;
        let secondary = derive_secondary_generator(b"REGISTRATION-PEDERSEN-H")?;
        Ok(self.commitment
            == generator().mul_bigint(Scalar::from(self.weight).into_bigint())
                + secondary.mul_bigint(self.opening.into_bigint()))
    }

    fn public_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/ENTRY/v1");
        encoded.fixed(&self.credential_id);
        encoded.fixed(&encode_point(&self.commitment));
        encoded.finish()
    }

    fn authority_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/AUTHORITY-ENTRY/v1");
        encoded.fixed(&self.credential_id);
        encoded.u64(self.weight);
        encoded.fixed(&encode_point(&self.commitment));
        encoded.finish()
    }
}

#[derive(Clone, Debug)]
pub struct RegistrationClose {
    pub election_id: [u8; 32],
    pub final_context: [u8; 32],
    pub policy_pre: [u8; 32],
    pub root_close: [u8; 32],
    pub n_registered: u64,
    pub w_registered: u64,
    pub w_max: u64,
    pub version: u32,
    pub canonical_registry_digest: [u8; 32],
}

impl RegistrationClose {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/CLOSE/RECORD/v1");
        encoded.text(SUITE_ID);
        encoded.fixed(&self.election_id);
        encoded.fixed(&self.final_context);
        encoded.fixed(&self.policy_pre);
        encoded.fixed(&self.root_close);
        encoded.u64(self.n_registered);
        encoded.u64(self.w_registered);
        encoded.u64(self.w_max);
        encoded.u32(self.version);
        encoded.fixed(&self.canonical_registry_digest);
        encoded.finish()
    }

    fn signing_message(&self) -> [u8; 32] {
        let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/CLOSE/v1");
        encoded.bytes(&self.canonical_bytes());
        sha256(&encoded.finish())
    }
}

#[derive(Clone, Debug)]
pub struct RegistrationCertificate {
    pub close: RegistrationClose,
    pub signature: SchnorrSignature,
}

impl RegistrationCertificate {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/CERTIFICATE/v1");
        encoded.bytes(&self.close.canonical_bytes());
        encoded.bytes(&self.signature.canonical_bytes());
        encoded.finish()
    }
}

#[derive(Clone)]
pub struct VerifiedRegistrationClose {
    certificate: RegistrationCertificate,
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
}

impl VerifiedRegistrationClose {
    pub fn certificate(&self) -> &RegistrationCertificate {
        &self.certificate
    }

    pub fn w_registered(&self) -> u64 {
        self.certificate.close.w_registered
    }

    pub fn registration_close_digest(&self) -> [u8; 32] {
        self.registration_close_digest
    }

    pub fn disclosure_digest(&self) -> [u8; 32] {
        self.disclosure_digest
    }
}

pub fn verify_close(
    certificate: &RegistrationCertificate,
    policy: &RegistrationPolicy,
    expected_election_id: [u8; 32],
    expected_final_context: [u8; 32],
    expected_policy_pre: [u8; 32],
) -> Result<VerifiedRegistrationClose, RegistryError> {
    validate_point(&policy.authority_public, false)?;
    let close = &certificate.close;
    let fields_match = close.election_id == expected_election_id
        && close.final_context == expected_final_context
        && close.policy_pre == expected_policy_pre
        && close.w_max == policy.w_max
        && close.w_registered <= close.w_max
        && close.n_registered > 0
        && close.version == policy.version;
    let authentic = verify_signature_with_domain(
        &policy.authority_public,
        CLOSE_SIGNATURE_DOMAIN,
        &close.signing_message(),
        &certificate.signature,
    );
    if !fields_match || !authentic {
        return Err(RegistryError::InvalidCertificate);
    }
    let mut record = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/CLOSE-DIGEST/v1");
    record.bytes(&close.canonical_bytes());
    let registration_close_digest = sha256(&record.finish());
    let mut disclosure = CanonicalEncoder::new(b"VV/JUBJUB/DISCLOSURE/CLOSE/v1");
    disclosure.bytes(&certificate.canonical_bytes());
    disclosure.text(&policy.rule_id);
    let disclosure_digest = sha256(&disclosure.finish());
    Ok(VerifiedRegistrationClose {
        certificate: certificate.clone(),
        registration_close_digest,
        disclosure_digest,
    })
}

fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return sha256(b"VV/JUBJUB/MERKLE/EMPTY/v1");
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().expect("nonempty"));
        }
        level = level
            .chunks(2)
            .map(|pair| {
                let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/MERKLE/NODE/v1");
                encoded.fixed(&pair[0]);
                encoded.fixed(&pair[1]);
                sha256(&encoded.finish())
            })
            .collect();
    }
    level[0]
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("invalid registration policy")]
    InvalidPolicy,
    #[error("registry is empty")]
    Empty,
    #[error("duplicate credential")]
    DuplicateCredential,
    #[error("weight commitment does not open to the authorized weight")]
    BadCommitment,
    #[error("registered weight exceeds W_max")]
    WeightOverflow,
    #[error("registration certificate is invalid")]
    InvalidCertificate,
    #[error("friendly cryptography error: {0}")]
    Crypto(#[from] FriendlyCryptoError),
    #[error("DKG/signature error: {0}")]
    Dkg(#[from] DkgError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::Field;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

    fn fixture(
        rng: &mut ChaCha20Rng,
    ) -> (
        RegistrationAuthority,
        RegistrationPolicy,
        Vec<RegistrationEntry>,
    ) {
        let authority = RegistrationAuthority::generate(rng);
        let policy = RegistrationPolicy::new(
            authority.public_key(),
            10,
            "highest-weight-smallest-id-on-tie",
        )
        .unwrap();
        let entries = [3u64, 2, 1, 4]
            .into_iter()
            .enumerate()
            .map(|(index, weight)| {
                RegistrationEntry::new(
                    sha256(format!("credential-{index}").as_bytes()),
                    weight,
                    rng,
                )
                .unwrap()
            })
            .collect();
        (authority, policy, entries)
    }

    #[test]
    fn exact_total_close_verifies_and_derives_binding_digests() {
        let mut rng = ChaCha20Rng::seed_from_u64(20261021);
        let (authority, policy, entries) = fixture(&mut rng);
        let certificate = authority
            .close(entries, [1; 32], [2; 32], [3; 32], &policy, &mut rng)
            .unwrap();
        let verified = verify_close(&certificate, &policy, [1; 32], [2; 32], [3; 32]).unwrap();
        assert_eq!(verified.w_registered(), 10);
        assert_ne!(verified.registration_close_digest(), [0; 32]);
        assert_ne!(verified.disclosure_digest(), [0; 32]);
    }

    #[test]
    fn duplicate_credential_and_bad_opening_are_rejected() {
        let mut rng = ChaCha20Rng::seed_from_u64(20261022);
        let (authority, policy, mut entries) = fixture(&mut rng);
        entries[1].credential_id = entries[0].credential_id;
        assert!(matches!(
            authority.close(entries, [1; 32], [2; 32], [3; 32], &policy, &mut rng),
            Err(RegistryError::DuplicateCredential)
        ));

        let (authority, policy, mut entries) = fixture(&mut rng);
        entries[0].opening += Scalar::ONE;
        assert!(matches!(
            authority.close(entries, [1; 32], [2; 32], [3; 32], &policy, &mut rng),
            Err(RegistryError::BadCommitment)
        ));
    }

    #[test]
    fn altered_close_and_cross_context_replay_are_rejected() {
        let mut rng = ChaCha20Rng::seed_from_u64(20261023);
        let (authority, policy, entries) = fixture(&mut rng);
        let certificate = authority
            .close(entries, [1; 32], [2; 32], [3; 32], &policy, &mut rng)
            .unwrap();
        let mut altered = certificate.clone();
        altered.close.w_registered = 9;
        assert!(verify_close(&altered, &policy, [1; 32], [2; 32], [3; 32]).is_err());
        assert!(verify_close(&certificate, &policy, [9; 32], [2; 32], [3; 32]).is_err());
        assert!(verify_close(&certificate, &policy, [1; 32], [9; 32], [3; 32]).is_err());
    }
}
