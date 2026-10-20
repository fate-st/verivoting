use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use rand_core::{CryptoRng, RngCore};
use std::collections::HashSet;
use thiserror::Error;
use vv_crypto::{
    commit_scalar, point_bytes, verify_commitment, verify_signature, PedersenCommitment,
    SchnorrKeypair, SchnorrSignature,
};
use vv_types::{sha256, CanonicalEncoder};

pub const REGISTRATION_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum RegistryError {
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
    #[error("cryptographic operation failed: {0}")]
    Crypto(#[from] vv_crypto::CryptoError),
}

#[derive(Clone, Debug)]
pub struct RegistrationEntry {
    pub credential_id: [u8; 32],
    pub weight: u64,
    pub commitment: PedersenCommitment,
    pub opening: Scalar,
}

impl RegistrationEntry {
    pub fn new(credential_id: [u8; 32], weight: u64, opening: Scalar) -> Self {
        Self {
            credential_id,
            weight,
            commitment: commit_scalar(Scalar::from(weight), opening),
            opening,
        }
    }

    pub fn public_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/REGISTER/ENTRY/v1");
        encoded.fixed(&self.credential_id);
        encoded.fixed(&point_bytes(&self.commitment.point));
        encoded.finish()
    }

    fn authority_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/REGISTER/AUTHORITY-ENTRY/v1");
        encoded.fixed(&self.credential_id);
        encoded.u64(self.weight);
        encoded.fixed(&point_bytes(&self.commitment.point));
        encoded.finish()
    }
}

#[derive(Clone, Debug)]
pub struct RegClose {
    pub election_id: [u8; 32],
    pub context: [u8; 32],
    pub root_close: [u8; 32],
    pub n_registered: u64,
    pub w_registered: u64,
    pub w_max: u64,
    pub policy_pre: [u8; 32],
    pub version: u32,
    pub canonical_registry_digest: [u8; 32],
}

impl RegClose {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/REGISTER/CLOSE/RECORD/v1");
        encoded.fixed(&self.election_id);
        encoded.fixed(&self.context);
        encoded.fixed(&self.root_close);
        encoded.u64(self.n_registered);
        encoded.u64(self.w_registered);
        encoded.u64(self.w_max);
        encoded.fixed(&self.policy_pre);
        encoded.u32(self.version);
        encoded.fixed(&self.canonical_registry_digest);
        encoded.finish()
    }

    pub fn signing_message(&self) -> [u8; 32] {
        let mut encoded = CanonicalEncoder::new(b"VV/REGISTER/CLOSE/v1");
        encoded.bytes(&self.canonical_bytes());
        sha256(&encoded.finish())
    }
}

#[derive(Clone, Debug)]
pub struct RegistrationCertificate {
    pub close: RegClose,
    pub signature: SchnorrSignature,
}

impl RegistrationCertificate {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoded = CanonicalEncoder::new(b"VV/REGISTER/CERTIFICATE/v1");
        encoded.bytes(&self.close.canonical_bytes());
        encoded.fixed(&self.signature.canonical_bytes());
        encoded.finish()
    }
}

#[derive(Clone, Debug)]
pub struct RegistrationAuthority {
    pub keypair: SchnorrKeypair,
}

impl RegistrationAuthority {
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            keypair: SchnorrKeypair::generate(rng),
        }
    }

    pub fn close<R: RngCore + CryptoRng>(
        &self,
        mut entries: Vec<RegistrationEntry>,
        election_id: [u8; 32],
        context: [u8; 32],
        policy_pre: [u8; 32],
        w_max: u64,
        rng: &mut R,
    ) -> Result<RegistrationCertificate, RegistryError> {
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
            if !verify_commitment(&entry.commitment, Scalar::from(entry.weight), entry.opening) {
                return Err(RegistryError::BadCommitment);
            }
            total = total
                .checked_add(entry.weight)
                .ok_or(RegistryError::WeightOverflow)?;
        }
        if total > w_max {
            return Err(RegistryError::WeightOverflow);
        }
        let public_leaves: Vec<_> = entries
            .iter()
            .map(|entry| sha256(&entry.public_bytes()))
            .collect();
        let root_close = merkle_root(&public_leaves);
        let mut registry = CanonicalEncoder::new(b"VV/REGISTER/CANONICAL/v1");
        registry.u64(entries.len() as u64);
        for entry in &entries {
            registry.bytes(&entry.authority_bytes());
        }
        let close = RegClose {
            election_id,
            context,
            root_close,
            n_registered: entries.len() as u64,
            w_registered: total,
            w_max,
            policy_pre,
            version: REGISTRATION_VERSION,
            canonical_registry_digest: sha256(&registry.finish()),
        };
        let signature = self.keypair.sign(
            b"VV/REGISTER/CLOSE/SIGNATURE/v1",
            &close.signing_message(),
            rng,
        )?;
        Ok(RegistrationCertificate { close, signature })
    }
}

pub fn policy_pre_digest(
    election_id: [u8; 32],
    context: [u8; 32],
    w_max: u64,
    rule_id: &str,
    manifest_digest: [u8; 32],
    ra_public: &RistrettoPoint,
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/DISCLOSURE/POLICY/PRE/v1");
    encoded.fixed(&election_id);
    encoded.fixed(&context);
    encoded.u64(w_max);
    encoded.text(rule_id);
    encoded.u32(REGISTRATION_VERSION);
    encoded.fixed(&manifest_digest);
    encoded.fixed(&point_bytes(ra_public));
    sha256(&encoded.finish())
}

pub fn disclosure_digest(certificate: &RegistrationCertificate, rule_id: &str) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/DISCLOSURE/CLOSE/v1");
    encoded.bytes(&certificate.canonical_bytes());
    encoded.text(rule_id);
    sha256(&encoded.finish())
}

pub fn verify_close(
    certificate: &RegistrationCertificate,
    ra_public: &RistrettoPoint,
    expected_election_id: [u8; 32],
    expected_context: [u8; 32],
    expected_policy_pre: [u8; 32],
    expected_w_max: u64,
) -> Result<(), RegistryError> {
    let close = &certificate.close;
    let fields_match = close.election_id == expected_election_id
        && close.context == expected_context
        && close.policy_pre == expected_policy_pre
        && close.w_max == expected_w_max
        && close.w_registered <= close.w_max
        && close.version == REGISTRATION_VERSION
        && close.n_registered > 0;
    let authentic = verify_signature(
        ra_public,
        b"VV/REGISTER/CLOSE/SIGNATURE/v1",
        &close.signing_message(),
        &certificate.signature,
    );
    if fields_match && authentic {
        Ok(())
    } else {
        Err(RegistryError::InvalidCertificate)
    }
}

pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return sha256(b"VV/MERKLE/EMPTY/v1");
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().expect("nonempty"));
        }
        level = level
            .chunks(2)
            .map(|pair| {
                let mut encoded = CanonicalEncoder::new(b"VV/MERKLE/NODE/v1");
                encoded.fixed(&pair[0]);
                encoded.fixed(&pair[1]);
                sha256(&encoded.finish())
            })
            .collect();
    }
    level[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

    fn fixture(rng: &mut ChaCha20Rng) -> Vec<RegistrationEntry> {
        [3u64, 2, 1, 4]
            .into_iter()
            .enumerate()
            .map(|(index, weight)| {
                RegistrationEntry::new(
                    sha256(format!("credential-{index}").as_bytes()),
                    weight,
                    Scalar::random(rng),
                )
            })
            .collect()
    }

    #[test]
    fn ra_close_authenticates_exact_total_and_rejects_mutation() {
        let mut rng = ChaCha20Rng::seed_from_u64(17);
        let authority = RegistrationAuthority::generate(&mut rng);
        let eid = [1; 32];
        let context = [2; 32];
        let policy = [3; 32];
        let mut certificate = authority
            .close(fixture(&mut rng), eid, context, policy, 10, &mut rng)
            .unwrap();
        assert_eq!(certificate.close.w_registered, 10);
        verify_close(
            &certificate,
            &authority.keypair.public,
            eid,
            context,
            policy,
            10,
        )
        .unwrap();
        certificate.close.w_registered = 9;
        assert!(verify_close(
            &certificate,
            &authority.keypair.public,
            eid,
            context,
            policy,
            10
        )
        .is_err());
    }

    #[test]
    fn duplicate_credentials_and_bad_openings_are_rejected() {
        let mut rng = ChaCha20Rng::seed_from_u64(19);
        let authority = RegistrationAuthority::generate(&mut rng);
        let mut entries = fixture(&mut rng);
        entries[1].credential_id = entries[0].credential_id;
        assert!(matches!(
            authority.close(entries, [1; 32], [2; 32], [3; 32], 10, &mut rng),
            Err(RegistryError::DuplicateCredential)
        ));

        let mut entries = fixture(&mut rng);
        entries[0].opening += Scalar::ONE;
        assert!(matches!(
            authority.close(entries, [1; 32], [2; 32], [3; 32], 10, &mut rng),
            Err(RegistryError::BadCommitment)
        ));
    }
}
