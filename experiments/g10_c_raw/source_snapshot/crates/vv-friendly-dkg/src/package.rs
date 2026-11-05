//! Authenticated private-package workflow for verified Jubjub CP shares.

use super::{
    verify_signature_with_domain, AuthenticationKeypair, DkgError, DkgOutput, SchnorrSignature,
};
use ark_ec::Group;
use ark_ff::PrimeField;
use rand_core::{CryptoRng, RngCore};
use thiserror::Error;
use vv_friendly_crypto::cp::{
    verify_decryption_share, DecryptionSharePackage, GeneralizedCpContext, VerifiedDecryptionShare,
};
use vv_friendly_crypto::{
    derive_secondary_generator, encode_point, encode_scalar, generator, package_hash_to_field_v1,
    random_nonzero_scalar, validate_point, Ciphertext, FriendlyCryptoError, Point, Scalar,
    SUITE_ID,
};
use vv_types::{derive_final_context, sha256, CanonicalEncoder};

pub const PACKAGE_SIGNATURE_DOMAIN: &[u8] = b"PRIVATE-PACKAGE-COMMITMENT-SIGNATURE";

#[derive(Clone, Debug)]
pub struct PackageCommitment {
    pub point: Point,
}

#[derive(Clone, Debug)]
pub struct PublicPackageRecord {
    pub trustee_id: u32,
    pub commitment: PackageCommitment,
    pub signature: SchnorrSignature,
}

#[derive(Clone)]
pub struct PrivatePackage {
    context: GeneralizedCpContext,
    verification_share: Point,
    cp_package: DecryptionSharePackage,
    opening: Scalar,
}

impl PrivatePackage {
    pub fn encoded_len(&self) -> usize {
        package_bytes(&self.context, &self.verification_share, &self.cp_package).len()
    }

    pub fn encoded_bytes(&self) -> Vec<u8> {
        package_bytes(&self.context, &self.verification_share, &self.cp_package)
    }

    pub fn opening(&self) -> Scalar {
        self.opening
    }
}

#[derive(Clone)]
pub struct VerifiedPrivatePackage {
    public_record: PublicPackageRecord,
    verified_share: VerifiedDecryptionShare,
}

impl VerifiedPrivatePackage {
    pub fn trustee_id(&self) -> u32 {
        self.public_record.trustee_id
    }

    pub fn public_record(&self) -> &PublicPackageRecord {
        &self.public_record
    }

    pub fn verified_share(&self) -> &VerifiedDecryptionShare {
        &self.verified_share
    }
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("private package is not bound to the expected election statement")]
    InvalidBinding,
    #[error("private package commitment opening failed")]
    BadCommitment,
    #[error("private package signature failed")]
    BadSignature,
    #[error("invalid set of verified private packages")]
    InvalidSelection,
    #[error("DKG error: {0}")]
    Dkg(#[from] DkgError),
    #[error("friendly cryptography error: {0}")]
    Crypto(#[from] FriendlyCryptoError),
    #[error("generalized CP error: {0}")]
    Cp(#[from] vv_friendly_crypto::cp::CpError),
}

pub fn create_private_package<R: RngCore + CryptoRng>(
    context: GeneralizedCpContext,
    verification_share: Point,
    cp_package: DecryptionSharePackage,
    authentication_key: &AuthenticationKeypair,
    rng: &mut R,
) -> Result<(PrivatePackage, PublicPackageRecord), PackageError> {
    validate_point(&authentication_key.public, false)?;
    validate_point(&verification_share, false)?;
    let opening = random_nonzero_scalar(rng);
    let encoded = package_bytes(&context, &verification_share, &cp_package);
    let package_scalar = package_hash_to_field_v1(&encoded);
    let secondary = derive_secondary_generator(b"PEDERSEN-H")?;
    let commitment = PackageCommitment {
        point: generator().mul_bigint(package_scalar.into_bigint())
            + secondary.mul_bigint(opening.into_bigint()),
    };
    let message = commitment_message(&context, &commitment);
    let signature = authentication_key.sign(PACKAGE_SIGNATURE_DOMAIN, &message, rng)?;
    let record = PublicPackageRecord {
        trustee_id: context.trustee_id,
        commitment,
        signature,
    };
    Ok((
        PrivatePackage {
            context,
            verification_share,
            cp_package,
            opening,
        },
        record,
    ))
}

pub fn verify_private_package(
    output: &DkgOutput,
    ciphertexts: &[Ciphertext],
    private: &PrivatePackage,
    public: &PublicPackageRecord,
) -> Result<VerifiedPrivatePackage, PackageError> {
    let trustee_index = usize::try_from(public.trustee_id)
        .ok()
        .and_then(|id| id.checked_sub(1))
        .filter(|index| *index < output.config.n)
        .ok_or(PackageError::InvalidBinding)?;
    let final_context = derive_final_context(
        output.config.prekey_context,
        output.certificate.key_core_digest,
    );
    let binding_matches = private.context.election_id == output.config.election_id
        && private.context.policy_pre == output.config.policy_pre
        && private.context.final_context == final_context
        && private.context.trustee_id == public.trustee_id
        && private.verification_share == output.verification_shares[trustee_index];
    if !binding_matches {
        return Err(PackageError::InvalidBinding);
    }
    validate_point(&public.commitment.point, true)?;
    let message = commitment_message(&private.context, &public.commitment);
    if !verify_signature_with_domain(
        &output.config.authentication_public_keys[trustee_index],
        PACKAGE_SIGNATURE_DOMAIN,
        &message,
        &public.signature,
    ) {
        return Err(PackageError::BadSignature);
    }
    let encoded = package_bytes(
        &private.context,
        &private.verification_share,
        &private.cp_package,
    );
    let package_scalar = package_hash_to_field_v1(&encoded);
    let secondary = derive_secondary_generator(b"PEDERSEN-H")?;
    let expected_commitment = generator().mul_bigint(package_scalar.into_bigint())
        + secondary.mul_bigint(private.opening.into_bigint());
    if expected_commitment != public.commitment.point {
        return Err(PackageError::BadCommitment);
    }
    let verified_share = verify_decryption_share(
        &private.context,
        &private.verification_share,
        ciphertexts,
        &private.cp_package,
    )?;
    Ok(VerifiedPrivatePackage {
        public_record: public.clone(),
        verified_share,
    })
}

pub fn select_verified_packages(
    packages: &[VerifiedPrivatePackage],
    threshold: usize,
) -> Result<Vec<VerifiedPrivatePackage>, PackageError> {
    if threshold == 0 || packages.len() < threshold {
        return Err(PackageError::InvalidSelection);
    }
    let mut ordered = packages.to_vec();
    ordered.sort_by_key(VerifiedPrivatePackage::trustee_id);
    if ordered
        .windows(2)
        .any(|pair| pair[0].trustee_id() == pair[1].trustee_id())
    {
        return Err(PackageError::InvalidSelection);
    }
    ordered.truncate(threshold);
    Ok(ordered)
}

pub fn package_bytes(
    context: &GeneralizedCpContext,
    verification_share: &Point,
    package: &DecryptionSharePackage,
) -> Vec<u8> {
    let domain = if context.has_registration_binding() {
        b"VV/JUBJUB/PRIVATE-PACKAGE/v5".as_slice()
    } else {
        b"VV/JUBJUB/PRIVATE-PACKAGE/v4".as_slice()
    };
    let mut encoded = CanonicalEncoder::new(domain);
    encoded.text(SUITE_ID);
    encoded.fixed(&context.election_id);
    encoded.fixed(&context.policy_pre);
    encoded.fixed(&context.final_context);
    if context.has_registration_binding() {
        encoded.fixed(&context.registration_close_digest);
        encoded.fixed(&context.disclosure_digest);
    }
    encoded.fixed(&context.target);
    encoded.u32(context.trustee_id);
    encoded.fixed(&encode_point(verification_share));
    encoded.u64(package.shares.len() as u64);
    for share in &package.shares {
        encoded.fixed(&encode_point(share));
    }
    encoded.fixed(&encode_point(&package.proof.a0));
    encoded.u64(package.proof.coordinates.len() as u64);
    for coordinate in &package.proof.coordinates {
        encoded.fixed(&encode_point(coordinate));
    }
    encoded.fixed(&encode_scalar(&package.proof.response));
    encoded.finish()
}

pub fn commitment_message(
    context: &GeneralizedCpContext,
    commitment: &PackageCommitment,
) -> [u8; 32] {
    let domain = if context.has_registration_binding() {
        b"VV/JUBJUB/PRIVATE-PACKAGE/COMMIT/v5".as_slice()
    } else {
        b"VV/JUBJUB/PRIVATE-PACKAGE/COMMIT/v4".as_slice()
    };
    let mut encoded = CanonicalEncoder::new(domain);
    encoded.text(SUITE_ID);
    encoded.fixed(&context.election_id);
    encoded.fixed(&context.policy_pre);
    encoded.fixed(&context.final_context);
    if context.has_registration_binding() {
        encoded.fixed(&context.registration_close_digest);
        encoded.fixed(&context.disclosure_digest);
    }
    encoded.fixed(&context.target);
    encoded.u32(context.trustee_id);
    encoded.fixed(&encode_point(&commitment.point));
    sha256(&encoded.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{combinations, run_dkg, threshold_decrypt_verified_shares, DkgConfig, FaultPlan};
    use ark_ff::Field;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
    use vv_friendly_crypto::cp::{derive_cp_target, prove_decryption_share};
    use vv_friendly_crypto::{encrypt_weighted_vote, fold_ballots};
    use vv_types::{sha256, winner};

    struct Fixture {
        dkg: DkgOutput,
        ciphertexts: Vec<Ciphertext>,
        private: Vec<PrivatePackage>,
        public: Vec<PublicPackageRecord>,
    }

    fn make_fixture() -> Fixture {
        let mut rng = ChaCha20Rng::seed_from_u64(20261022);
        let keys: Vec<_> = (0..5)
            .map(|_| AuthenticationKeypair::generate(&mut rng))
            .collect();
        let config = DkgConfig::new(
            sha256(b"PACKAGE-TEST-ELECTION"),
            sha256(b"PACKAGE-TEST-MANIFEST"),
            keys.iter().map(|key| key.public).collect(),
            2,
            3,
        )
        .unwrap();
        let dkg = run_dkg(config, &keys, FaultPlan::default(), &mut rng).unwrap();
        let ballots = [3u64, 2, 1, 4]
            .iter()
            .zip([1usize, 2, 1, 3])
            .map(|(weight, choice)| {
                encrypt_weighted_vote(&dkg.public_key, *weight, choice, 3, 10, &mut rng)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let ciphertexts = fold_ballots(&ballots).unwrap();
        let final_context =
            derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);
        let target = derive_cp_target(
            dkg.config.election_id,
            dkg.config.policy_pre,
            final_context,
            &ciphertexts,
        )
        .unwrap();
        let (private, public): (Vec<_>, Vec<_>) = (1..=5u32)
            .map(|trustee_id| {
                let context = GeneralizedCpContext {
                    election_id: dkg.config.election_id,
                    policy_pre: dkg.config.policy_pre,
                    final_context,
                    registration_close_digest: [0; 32],
                    disclosure_digest: [0; 32],
                    target,
                    trustee_id,
                };
                let cp = prove_decryption_share(
                    &context,
                    dkg.secret_share(trustee_id).unwrap(),
                    dkg.verification_share(trustee_id).unwrap(),
                    &ciphertexts,
                    &mut rng,
                )
                .unwrap();
                create_private_package(
                    context,
                    *dkg.verification_share(trustee_id).unwrap(),
                    cp,
                    &keys[trustee_id as usize - 1],
                    &mut rng,
                )
                .unwrap()
            })
            .unzip();
        Fixture {
            dkg,
            ciphertexts,
            private,
            public,
        }
    }

    #[test]
    fn verified_packages_are_selected_before_threshold_combination() {
        let fixture = make_fixture();
        let verified = fixture
            .private
            .iter()
            .zip(&fixture.public)
            .map(|(private, public)| {
                verify_private_package(&fixture.dkg, &fixture.ciphertexts, private, public).unwrap()
            })
            .collect::<Vec<_>>();
        let selected = select_verified_packages(&verified, 3).unwrap();
        assert_eq!(
            selected
                .iter()
                .map(VerifiedPrivatePackage::trustee_id)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        let ids = selected
            .iter()
            .map(VerifiedPrivatePackage::trustee_id)
            .collect::<Vec<_>>();
        let shares = selected
            .iter()
            .map(|package| package.verified_share().clone())
            .collect::<Vec<_>>();
        let tally = threshold_decrypt_verified_shares(
            &fixture.dkg,
            &ids,
            &fixture.ciphertexts,
            &shares,
            10,
        )
        .unwrap();
        assert_eq!(tally, [4, 2, 4]);
        assert_eq!(winner(&tally, 10).unwrap(), 1);

        let all_subsets = combinations(&[1, 2, 3, 4, 5], 3);
        assert_eq!(all_subsets.len(), 10);
    }

    #[test]
    fn bad_opening_signature_and_context_are_rejected() {
        let mut fixture = make_fixture();
        fixture.private[0].opening += Scalar::ONE;
        assert!(matches!(
            verify_private_package(
                &fixture.dkg,
                &fixture.ciphertexts,
                &fixture.private[0],
                &fixture.public[0]
            ),
            Err(PackageError::BadCommitment)
        ));

        let mut fixture = make_fixture();
        fixture.public[0].signature.response += Scalar::ONE;
        assert!(matches!(
            verify_private_package(
                &fixture.dkg,
                &fixture.ciphertexts,
                &fixture.private[0],
                &fixture.public[0]
            ),
            Err(PackageError::BadSignature)
        ));

        let mut fixture = make_fixture();
        fixture.private[0].context.election_id[0] ^= 1;
        assert!(matches!(
            verify_private_package(
                &fixture.dkg,
                &fixture.ciphertexts,
                &fixture.private[0],
                &fixture.public[0]
            ),
            Err(PackageError::InvalidBinding)
        ));
    }

    #[test]
    fn fewer_than_threshold_and_duplicate_verified_packages_are_rejected() {
        let fixture = make_fixture();
        let verified = fixture
            .private
            .iter()
            .zip(&fixture.public)
            .take(2)
            .map(|(private, public)| {
                verify_private_package(&fixture.dkg, &fixture.ciphertexts, private, public).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            select_verified_packages(&verified, 3),
            Err(PackageError::InvalidSelection)
        ));

        let first = verify_private_package(
            &fixture.dkg,
            &fixture.ciphertexts,
            &fixture.private[0],
            &fixture.public[0],
        )
        .unwrap();
        let duplicates = vec![first.clone(), first.clone(), first];
        assert!(matches!(
            select_verified_packages(&duplicates, 3),
            Err(PackageError::InvalidSelection)
        ));
    }
}
