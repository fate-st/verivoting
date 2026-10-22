//! Native generalized Chaum--Pedersen proofs for Jubjub decryption shares.
//!
//! The verifier recomputes the Suite v2 Fiat--Shamir challenge from the exact
//! canonical transcript. This module is native protocol code, not an R1CS or
//! Groth16 implementation.

use crate::{
    encode_point, generator, hash_to_scalar, random_nonzero_scalar, validate_point, Ciphertext,
    FriendlyCryptoError, Point, Scalar, SUITE_ID,
};
use ark_ec::Group;
use ark_ff::PrimeField;
use rand_core::{CryptoRng, RngCore};
use thiserror::Error;
use vv_types::{sha256, CanonicalEncoder};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralizedCpContext {
    pub election_id: [u8; 32],
    pub policy_pre: [u8; 32],
    pub final_context: [u8; 32],
    pub registration_close_digest: [u8; 32],
    pub disclosure_digest: [u8; 32],
    pub target: [u8; 32],
    pub trustee_id: u32,
}

impl GeneralizedCpContext {
    pub fn has_registration_binding(&self) -> bool {
        self.registration_close_digest != [0; 32] && self.disclosure_digest != [0; 32]
    }
}

#[derive(Clone, Debug)]
pub struct GeneralizedCpProof {
    pub a0: Point,
    pub coordinates: Vec<Point>,
    pub response: Scalar,
    pub rejection_counter: u16,
}

#[derive(Clone, Debug)]
pub struct DecryptionSharePackage {
    pub shares: Vec<Point>,
    pub proof: GeneralizedCpProof,
}

/// A decryption-share vector that can only be constructed by this module's
/// exact native verifier.
#[derive(Clone, Debug)]
pub struct VerifiedDecryptionShare {
    election_id: [u8; 32],
    policy_pre: [u8; 32],
    final_context: [u8; 32],
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    target: [u8; 32],
    trustee_id: u32,
    shares: Vec<Point>,
}

impl VerifiedDecryptionShare {
    pub fn trustee_id(&self) -> u32 {
        self.trustee_id
    }

    pub fn shares(&self) -> &[Point] {
        &self.shares
    }

    pub fn is_bound_to(
        &self,
        election_id: [u8; 32],
        policy_pre: [u8; 32],
        final_context: [u8; 32],
        target: [u8; 32],
        coordinate_count: usize,
    ) -> bool {
        self.is_bound_to_registration(
            election_id,
            policy_pre,
            final_context,
            [0; 32],
            [0; 32],
            target,
            coordinate_count,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn is_bound_to_registration(
        &self,
        election_id: [u8; 32],
        policy_pre: [u8; 32],
        final_context: [u8; 32],
        registration_close_digest: [u8; 32],
        disclosure_digest: [u8; 32],
        target: [u8; 32],
        coordinate_count: usize,
    ) -> bool {
        self.election_id == election_id
            && self.policy_pre == policy_pre
            && self.final_context == final_context
            && self.registration_close_digest == registration_close_digest
            && self.disclosure_digest == disclosure_digest
            && self.target == target
            && self.shares.len() == coordinate_count
    }
}

#[derive(Debug, Error)]
pub enum CpError {
    #[error("invalid generalized CP context")]
    InvalidContext,
    #[error("generalized CP vector length mismatch")]
    LengthMismatch,
    #[error("verification share does not match the prover secret")]
    VerificationShareMismatch,
    #[error("target does not bind the supplied ciphertext vector")]
    TargetMismatch,
    #[error("generalized CP relation failed")]
    RelationFailed,
    #[error("friendly cryptography error: {0}")]
    Crypto(#[from] FriendlyCryptoError),
}

pub fn derive_cp_target(
    election_id: [u8; 32],
    policy_pre: [u8; 32],
    final_context: [u8; 32],
    ciphertexts: &[Ciphertext],
) -> Result<[u8; 32], CpError> {
    derive_cp_target_internal(
        election_id,
        policy_pre,
        final_context,
        [0; 32],
        [0; 32],
        ciphertexts,
    )
}

pub fn derive_cp_target_with_registration(
    election_id: [u8; 32],
    policy_pre: [u8; 32],
    final_context: [u8; 32],
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    ciphertexts: &[Ciphertext],
) -> Result<[u8; 32], CpError> {
    if registration_close_digest == [0; 32] || disclosure_digest == [0; 32] {
        return Err(CpError::InvalidContext);
    }
    derive_cp_target_internal(
        election_id,
        policy_pre,
        final_context,
        registration_close_digest,
        disclosure_digest,
        ciphertexts,
    )
}

fn derive_cp_target_internal(
    election_id: [u8; 32],
    policy_pre: [u8; 32],
    final_context: [u8; 32],
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    ciphertexts: &[Ciphertext],
) -> Result<[u8; 32], CpError> {
    if ciphertexts.is_empty() {
        return Err(CpError::LengthMismatch);
    }
    let registration_bound = registration_close_digest != [0; 32] && disclosure_digest != [0; 32];
    let domain = if registration_bound {
        b"VV/JUBJUB/TALLY/TARGET/v3".as_slice()
    } else {
        b"VV/JUBJUB/TALLY/TARGET/v2".as_slice()
    };
    let mut encoded = CanonicalEncoder::new(domain);
    encoded.text(SUITE_ID);
    encoded.fixed(&election_id);
    encoded.fixed(&policy_pre);
    encoded.fixed(&final_context);
    if registration_bound {
        encoded.fixed(&registration_close_digest);
        encoded.fixed(&disclosure_digest);
    }
    encoded.u64(ciphertexts.len() as u64);
    for ciphertext in ciphertexts {
        validate_point(&ciphertext.c0, true)?;
        validate_point(&ciphertext.c1, true)?;
        encoded.fixed(&encode_point(&ciphertext.c0));
        encoded.fixed(&encode_point(&ciphertext.c1));
    }
    Ok(sha256(&encoded.finish()))
}

pub fn prove_decryption_share<R: RngCore + CryptoRng>(
    context: &GeneralizedCpContext,
    secret_share: Scalar,
    verification_share: &Point,
    ciphertexts: &[Ciphertext],
    rng: &mut R,
) -> Result<DecryptionSharePackage, CpError> {
    validate_statement(context, verification_share, ciphertexts)?;
    if generator().mul_bigint(secret_share.into_bigint()) != *verification_share {
        return Err(CpError::VerificationShareMismatch);
    }
    let shares: Vec<_> = ciphertexts
        .iter()
        .map(|ciphertext| ciphertext.c0.mul_bigint(secret_share.into_bigint()))
        .collect();
    let nonce = random_nonzero_scalar(rng);
    let a0 = generator().mul_bigint(nonce.into_bigint());
    let coordinates: Vec<_> = ciphertexts
        .iter()
        .map(|ciphertext| ciphertext.c0.mul_bigint(nonce.into_bigint()))
        .collect();
    let transcript = cp_transcript(
        context,
        verification_share,
        ciphertexts,
        &shares,
        &a0,
        &coordinates,
    )?;
    let (challenge, rejection_counter) = hash_to_scalar(b"GENERALIZED-CP", &transcript)?;
    Ok(DecryptionSharePackage {
        shares,
        proof: GeneralizedCpProof {
            a0,
            coordinates,
            response: nonce + challenge * secret_share,
            rejection_counter,
        },
    })
}

pub fn verify_decryption_share(
    context: &GeneralizedCpContext,
    verification_share: &Point,
    ciphertexts: &[Ciphertext],
    package: &DecryptionSharePackage,
) -> Result<VerifiedDecryptionShare, CpError> {
    validate_statement(context, verification_share, ciphertexts)?;
    if package.shares.len() != ciphertexts.len()
        || package.proof.coordinates.len() != ciphertexts.len()
    {
        return Err(CpError::LengthMismatch);
    }
    for share in &package.shares {
        validate_point(share, true)?;
    }
    validate_point(&package.proof.a0, false)?;
    for coordinate in &package.proof.coordinates {
        validate_point(coordinate, true)?;
    }
    let transcript = cp_transcript(
        context,
        verification_share,
        ciphertexts,
        &package.shares,
        &package.proof.a0,
        &package.proof.coordinates,
    )?;
    let (challenge, counter) = hash_to_scalar(b"GENERALIZED-CP", &transcript)?;
    if counter != package.proof.rejection_counter
        || generator().mul_bigint(package.proof.response.into_bigint())
            != package.proof.a0 + verification_share.mul_bigint(challenge.into_bigint())
    {
        return Err(CpError::RelationFailed);
    }
    let coordinates_valid = ciphertexts
        .iter()
        .zip(&package.shares)
        .zip(&package.proof.coordinates)
        .all(|((ciphertext, share), commitment)| {
            ciphertext
                .c0
                .mul_bigint(package.proof.response.into_bigint())
                == *commitment + share.mul_bigint(challenge.into_bigint())
        });
    if !coordinates_valid {
        return Err(CpError::RelationFailed);
    }
    Ok(VerifiedDecryptionShare {
        election_id: context.election_id,
        policy_pre: context.policy_pre,
        final_context: context.final_context,
        registration_close_digest: context.registration_close_digest,
        disclosure_digest: context.disclosure_digest,
        target: context.target,
        trustee_id: context.trustee_id,
        shares: package.shares.clone(),
    })
}

pub fn transcript_and_challenge(
    context: &GeneralizedCpContext,
    verification_share: &Point,
    ciphertexts: &[Ciphertext],
    package: &DecryptionSharePackage,
) -> Result<(Vec<u8>, Scalar, u16), CpError> {
    let transcript = cp_transcript(
        context,
        verification_share,
        ciphertexts,
        &package.shares,
        &package.proof.a0,
        &package.proof.coordinates,
    )?;
    let (challenge, counter) = hash_to_scalar(b"GENERALIZED-CP", &transcript)?;
    Ok((transcript, challenge, counter))
}

fn validate_statement(
    context: &GeneralizedCpContext,
    verification_share: &Point,
    ciphertexts: &[Ciphertext],
) -> Result<(), CpError> {
    let one_registration_digest_missing =
        (context.registration_close_digest == [0; 32]) != (context.disclosure_digest == [0; 32]);
    if context.trustee_id == 0 || ciphertexts.is_empty() || one_registration_digest_missing {
        return Err(CpError::InvalidContext);
    }
    validate_point(verification_share, false)?;
    let expected_target = derive_cp_target_internal(
        context.election_id,
        context.policy_pre,
        context.final_context,
        context.registration_close_digest,
        context.disclosure_digest,
        ciphertexts,
    )?;
    if context.target != expected_target {
        return Err(CpError::TargetMismatch);
    }
    Ok(())
}

fn cp_transcript(
    context: &GeneralizedCpContext,
    verification_share: &Point,
    ciphertexts: &[Ciphertext],
    shares: &[Point],
    a0: &Point,
    coordinates: &[Point],
) -> Result<Vec<u8>, CpError> {
    if ciphertexts.is_empty()
        || ciphertexts.len() != shares.len()
        || ciphertexts.len() != coordinates.len()
    {
        return Err(CpError::LengthMismatch);
    }
    let domain = if context.has_registration_binding() {
        b"VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3".as_slice()
    } else {
        b"VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v2".as_slice()
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
    encoded.u64(ciphertexts.len() as u64);
    for ciphertext in ciphertexts {
        encoded.fixed(&encode_point(&ciphertext.c0));
    }
    encoded.u64(shares.len() as u64);
    for share in shares {
        encoded.fixed(&encode_point(share));
    }
    encoded.fixed(&encode_point(a0));
    encoded.u64(coordinates.len() as u64);
    for coordinate in coordinates {
        encoded.fixed(&encode_point(coordinate));
    }
    Ok(encoded.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_scalar, encrypt};
    use ark_ff::Field;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
    use vv_types::hex;

    fn fixture() -> (
        GeneralizedCpContext,
        Scalar,
        Point,
        Vec<Ciphertext>,
        DecryptionSharePackage,
    ) {
        let mut rng = ChaCha20Rng::seed_from_u64(20261020);
        let secret = random_nonzero_scalar(&mut rng);
        let verification_share = generator().mul_bigint(secret.into_bigint());
        let ciphertexts = (0u64..3)
            .map(|message| encrypt(&verification_share, message, &mut rng).unwrap())
            .collect::<Vec<_>>();
        let election_id = sha256(b"CP-TEST-ELECTION");
        let policy_pre = sha256(b"CP-TEST-POLICY");
        let final_context = sha256(b"CP-TEST-CONTEXT");
        let target =
            derive_cp_target(election_id, policy_pre, final_context, &ciphertexts).unwrap();
        let context = GeneralizedCpContext {
            election_id,
            policy_pre,
            final_context,
            registration_close_digest: [0; 32],
            disclosure_digest: [0; 32],
            target,
            trustee_id: 1,
        };
        let package = prove_decryption_share(
            &context,
            secret,
            &verification_share,
            &ciphertexts,
            &mut rng,
        )
        .unwrap();
        (context, secret, verification_share, ciphertexts, package)
    }

    #[test]
    fn three_coordinate_generalized_cp_verifies() {
        let (context, _, verification_share, ciphertexts, package) = fixture();
        let verified =
            verify_decryption_share(&context, &verification_share, &ciphertexts, &package).unwrap();
        assert_eq!(verified.trustee_id(), 1);
        assert_eq!(verified.shares().len(), 3);
        let (_, _, counter) =
            transcript_and_challenge(&context, &verification_share, &ciphertexts, &package)
                .unwrap();
        assert_eq!(counter, package.proof.rejection_counter);
    }

    #[test]
    fn altered_share_commitment_response_and_counter_are_rejected() {
        let (context, _, verification_share, ciphertexts, package) = fixture();

        let mut bad_share = package.clone();
        bad_share.shares[0] += generator();
        assert!(
            verify_decryption_share(&context, &verification_share, &ciphertexts, &bad_share)
                .is_err()
        );

        let mut bad_commitment = package.clone();
        bad_commitment.proof.coordinates[1] += generator();
        assert!(verify_decryption_share(
            &context,
            &verification_share,
            &ciphertexts,
            &bad_commitment
        )
        .is_err());

        let mut bad_response = package.clone();
        bad_response.proof.response += Scalar::ONE;
        assert!(verify_decryption_share(
            &context,
            &verification_share,
            &ciphertexts,
            &bad_response
        )
        .is_err());

        let mut bad_counter = package.clone();
        bad_counter.proof.rejection_counter ^= 1;
        assert!(
            verify_decryption_share(&context, &verification_share, &ciphertexts, &bad_counter)
                .is_err()
        );
    }

    #[test]
    fn altered_context_trustee_order_and_length_are_rejected() {
        let (context, _, verification_share, ciphertexts, package) = fixture();

        let mut old_context = context.clone();
        old_context.final_context[0] ^= 1;
        assert!(
            verify_decryption_share(&old_context, &verification_share, &ciphertexts, &package)
                .is_err()
        );

        let mut wrong_trustee = context.clone();
        wrong_trustee.trustee_id = 2;
        assert!(verify_decryption_share(
            &wrong_trustee,
            &verification_share,
            &ciphertexts,
            &package
        )
        .is_err());

        let mut reordered = ciphertexts.clone();
        reordered.swap(0, 1);
        let reordered_context = GeneralizedCpContext {
            target: derive_cp_target(
                context.election_id,
                context.policy_pre,
                context.final_context,
                &reordered,
            )
            .unwrap(),
            ..context.clone()
        };
        assert!(verify_decryption_share(
            &reordered_context,
            &verification_share,
            &reordered,
            &package
        )
        .is_err());

        let mut shortened = package.clone();
        shortened.shares.pop();
        assert!(matches!(
            verify_decryption_share(&context, &verification_share, &ciphertexts, &shortened),
            Err(CpError::LengthMismatch)
        ));
    }

    #[test]
    fn prover_rejects_secret_not_matching_verification_share() {
        let (context, secret, verification_share, ciphertexts, _) = fixture();
        let mut rng = ChaCha20Rng::seed_from_u64(7);
        assert!(matches!(
            prove_decryption_share(
                &context,
                secret + Scalar::ONE,
                &verification_share,
                &ciphertexts,
                &mut rng
            ),
            Err(CpError::VerificationShareMismatch)
        ));
    }

    #[test]
    fn registration_close_digest_is_bound_to_target_and_proof() {
        let (mut context, secret, verification_share, ciphertexts, _) = fixture();
        context.registration_close_digest = sha256(b"REGISTRATION-CLOSE");
        context.disclosure_digest = sha256(b"DISCLOSURE-CLOSE");
        context.target = derive_cp_target_with_registration(
            context.election_id,
            context.policy_pre,
            context.final_context,
            context.registration_close_digest,
            context.disclosure_digest,
            &ciphertexts,
        )
        .unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(20261021);
        let package = prove_decryption_share(
            &context,
            secret,
            &verification_share,
            &ciphertexts,
            &mut rng,
        )
        .unwrap();
        verify_decryption_share(&context, &verification_share, &ciphertexts, &package).unwrap();

        let mut substituted = context.clone();
        substituted.registration_close_digest[0] ^= 1;
        assert!(matches!(
            verify_decryption_share(&substituted, &verification_share, &ciphertexts, &package),
            Err(CpError::TargetMismatch)
        ));
    }

    #[test]
    fn registration_bound_transcript_v3_golden_vector_is_stable() {
        let mut rng = ChaCha20Rng::seed_from_u64(20261021);
        let secret = random_nonzero_scalar(&mut rng);
        let verification_share = generator().mul_bigint(secret.into_bigint());
        let ciphertexts = (0u64..3)
            .map(|message| encrypt(&verification_share, message, &mut rng).unwrap())
            .collect::<Vec<_>>();
        let election_id = sha256(b"VV/JUBJUB/G0.9/H0/ELECTION");
        let policy_pre = sha256(b"VV/JUBJUB/G0.9/H0/POLICY-PRE");
        let final_context = sha256(b"VV/JUBJUB/G0.9/H0/FINAL-CONTEXT");
        let registration_close_digest = sha256(b"VV/JUBJUB/G0.9/H0/REGISTRATION-CLOSE");
        let disclosure_digest = sha256(b"VV/JUBJUB/G0.9/H0/DISCLOSURE-CLOSE");
        let target = derive_cp_target_with_registration(
            election_id,
            policy_pre,
            final_context,
            registration_close_digest,
            disclosure_digest,
            &ciphertexts,
        )
        .unwrap();
        let context = GeneralizedCpContext {
            election_id,
            policy_pre,
            final_context,
            registration_close_digest,
            disclosure_digest,
            target,
            trustee_id: 3,
        };
        let package = prove_decryption_share(
            &context,
            secret,
            &verification_share,
            &ciphertexts,
            &mut rng,
        )
        .unwrap();
        let (transcript, challenge, counter) =
            transcript_and_challenge(&context, &verification_share, &ciphertexts, &package)
                .unwrap();

        assert_eq!(transcript.len(), 684);
        assert_eq!(
            hex(&sha256(&transcript)),
            "b902f98b94fd9a5b04a73782a8ab6e73baa7e1525a20d49ee6d2dec8c8bd1d96"
        );
        assert_eq!(
            hex(&encode_scalar(&challenge)),
            "552c8f3d6c4bcdebf1042342cccd93d7a589c1d27f257434d2d33051b0799d02"
        );
        assert_eq!(counter, 0);
    }
}
