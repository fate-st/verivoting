//! G0.8 native Jubjub prototype primitives.
//!
//! This crate is deliberately isolated from the official Ristretto manifest.
//! It currently implements canonical Jubjub encodings, domain-separated
//! hash-to-scalar/group helpers, native weighted ElGamal, homomorphic folding,
//! bounded discrete-log decoding, and the four-voter fixture. It does not yet
//! implement DKG, generalized CP, P3, or a production protocol suite.

use ark_ec::{AffineRepr, CurveGroup, Group};
use ark_ed_on_bls12_381::{EdwardsAffine, EdwardsProjective, Fr as JubScalar};
use ark_ff::{BigInteger, PrimeField, UniformRand, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::{collections::HashMap, time::Instant};
use thiserror::Error;
use vv_types::{winner, CanonicalEncoder};

pub const SUITE_ID: &str = "VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v1";
pub const HASH_REJECTION_MAX_ATTEMPTS: u16 = 128;
pub const POINT_BYTES: usize = 32;
pub const SCALAR_BYTES: usize = 32;

pub type Point = EdwardsProjective;
pub type Scalar = JubScalar;

#[derive(Debug, Error)]
pub enum FriendlyCryptoError {
    #[error("invalid canonical point encoding")]
    InvalidPointEncoding,
    #[error("identity point is not allowed here")]
    IdentityPoint,
    #[error("invalid canonical scalar encoding")]
    InvalidScalarEncoding,
    #[error("hash-to-scalar exhausted {HASH_REJECTION_MAX_ATTEMPTS} attempts")]
    HashToScalarExhausted,
    #[error("hash-to-group exhausted {HASH_REJECTION_MAX_ATTEMPTS} attempts")]
    HashToGroupExhausted,
    #[error("public key must be a non-identity prime-order point")]
    InvalidPublicKey,
    #[error("fresh ciphertext C0 must not be the identity")]
    InvalidCiphertext,
    #[error("candidate {choice} is outside 1..={candidate_count}")]
    InvalidChoice {
        choice: usize,
        candidate_count: usize,
    },
    #[error("weight {weight} is outside configured range 1..={w_max}")]
    InvalidWeight { weight: u64, w_max: u64 },
    #[error("ciphertext vector length mismatch")]
    LengthMismatch,
    #[error("bounded discrete log not found below {bound}")]
    DiscreteLogNotFound { bound: u64 },
    #[error("winner calculation failed: {0}")]
    Winner(#[from] vv_types::TypesError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ciphertext {
    pub c0: Point,
    pub c1: Point,
}

impl Ciphertext {
    pub fn zero() -> Self {
        Self {
            c0: Point::zero(),
            c1: Point::zero(),
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            c0: self.c0 + other.c0,
            c1: self.c1 + other.c1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Keypair {
    pub secret: Scalar,
    pub public: Point,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FourVoterReport {
    pub classification: String,
    pub suite_id: String,
    pub seed: u64,
    pub candidate_count: usize,
    pub weights: Vec<u64>,
    pub choices: Vec<usize>,
    pub registered_weight: u64,
    pub public_key_hex: String,
    pub private_tally: Vec<u64>,
    pub public_winner: usize,
    pub ballots_encrypted: usize,
    pub encryption_ms: f64,
    pub aggregate_decrypt_decode_ms: f64,
    pub all_native_relations_valid: bool,
    pub dkg_implemented: bool,
    pub generalized_cp_implemented: bool,
    pub full_g08: bool,
    pub full_p3: bool,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuiteVectors {
    pub classification: String,
    pub suite_id: String,
    pub point_encoding: String,
    pub scalar_encoding: String,
    pub generator_compressed_hex: String,
    pub identity_compressed_hex: String,
    pub secondary_generator_label_hex: String,
    pub secondary_generator_compressed_hex: String,
    pub scalar_value: u64,
    pub scalar_canonical_le_hex: String,
    pub hash_to_scalar_domain_hex: String,
    pub hash_to_scalar_transcript_hex: String,
    pub hash_to_scalar_output_le_hex: String,
    pub hash_to_scalar_rejection_counter: u16,
}

pub fn generator() -> Point {
    Point::generator()
}

pub fn encode_point(point: &Point) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(POINT_BYTES);
    point
        .into_affine()
        .serialize_compressed(&mut encoded)
        .expect("serializing a Jubjub point into a Vec cannot fail");
    encoded
}

pub fn decode_point(encoded: &[u8], allow_identity: bool) -> Result<Point, FriendlyCryptoError> {
    if encoded.len() != POINT_BYTES {
        return Err(FriendlyCryptoError::InvalidPointEncoding);
    }
    let mut input = encoded;
    let affine = EdwardsAffine::deserialize_compressed(&mut input)
        .map_err(|_| FriendlyCryptoError::InvalidPointEncoding)?;
    if !input.is_empty()
        || !affine.is_on_curve()
        || !affine.is_in_correct_subgroup_assuming_on_curve()
    {
        return Err(FriendlyCryptoError::InvalidPointEncoding);
    }
    if !allow_identity && affine.is_zero() {
        return Err(FriendlyCryptoError::IdentityPoint);
    }
    Ok(affine.into_group())
}

pub fn encode_scalar(scalar: &Scalar) -> [u8; SCALAR_BYTES] {
    let bytes = scalar.into_bigint().to_bytes_le();
    let mut encoded = [0u8; SCALAR_BYTES];
    encoded[..bytes.len()].copy_from_slice(&bytes);
    encoded
}

pub fn decode_scalar(encoded: &[u8]) -> Result<Scalar, FriendlyCryptoError> {
    if encoded.len() != SCALAR_BYTES {
        return Err(FriendlyCryptoError::InvalidScalarEncoding);
    }
    let bits: Vec<bool> = encoded
        .iter()
        .flat_map(|byte| (0..8).map(move |bit| ((byte >> bit) & 1) == 1))
        .collect();
    let integer = <Scalar as PrimeField>::BigInt::from_bits_le(&bits);
    Scalar::from_bigint(integer).ok_or(FriendlyCryptoError::InvalidScalarEncoding)
}

pub fn hash_to_scalar(
    domain: &[u8],
    transcript: &[u8],
) -> Result<(Scalar, u16), FriendlyCryptoError> {
    for counter in 0..HASH_REJECTION_MAX_ATTEMPTS {
        let mut input = CanonicalEncoder::new(b"VV/JUBJUB/H2S/v1");
        input.text(SUITE_ID);
        input.bytes(domain);
        input.bytes(transcript);
        input.u16(counter);
        let digest = Sha512::digest(input.finish());
        let candidate = &digest[..SCALAR_BYTES];
        if let Ok(scalar) = decode_scalar(candidate) {
            return Ok((scalar, counter));
        }
    }
    Err(FriendlyCryptoError::HashToScalarExhausted)
}

/// Derive a subgroup point by hashing to a curve point and clearing cofactor 8.
/// The input is public, but the implementation never constructs the result as
/// a known scalar multiple of the standard generator.
pub fn derive_secondary_generator(label: &[u8]) -> Result<Point, FriendlyCryptoError> {
    for counter in 0..HASH_REJECTION_MAX_ATTEMPTS {
        let mut input = CanonicalEncoder::new(b"VV/JUBJUB/H2G/v1");
        input.text(SUITE_ID);
        input.bytes(label);
        input.u16(counter);
        let digest = Sha512::digest(input.finish());
        if let Some(candidate) = EdwardsAffine::from_random_bytes(&digest[..POINT_BYTES]) {
            let point = candidate.mul_by_cofactor_to_group();
            if !point.is_zero() {
                return Ok(point);
            }
        }
    }
    Err(FriendlyCryptoError::HashToGroupExhausted)
}

fn random_nonzero_scalar<R: RngCore>(rng: &mut R) -> Scalar {
    loop {
        let value = Scalar::rand(rng);
        if !value.is_zero() {
            return value;
        }
    }
}

pub fn keygen<R: RngCore>(rng: &mut R) -> Keypair {
    let secret = random_nonzero_scalar(rng);
    let public = generator().mul_bigint(secret.into_bigint());
    Keypair { secret, public }
}

pub fn encrypt<R: RngCore>(
    public_key: &Point,
    message: u64,
    rng: &mut R,
) -> Result<Ciphertext, FriendlyCryptoError> {
    if public_key.is_zero() {
        return Err(FriendlyCryptoError::InvalidPublicKey);
    }
    let randomness = random_nonzero_scalar(rng);
    let c0 = generator().mul_bigint(randomness.into_bigint());
    let c1 = public_key.mul_bigint(randomness.into_bigint())
        + generator().mul_bigint(Scalar::from(message).into_bigint());
    Ok(Ciphertext { c0, c1 })
}

pub fn validate_fresh_ciphertext(ciphertext: &Ciphertext) -> Result<(), FriendlyCryptoError> {
    if ciphertext.c0.is_zero() {
        Err(FriendlyCryptoError::InvalidCiphertext)
    } else {
        Ok(())
    }
}

pub fn encrypt_weighted_vote<R: RngCore>(
    public_key: &Point,
    weight: u64,
    choice: usize,
    candidate_count: usize,
    w_max: u64,
    rng: &mut R,
) -> Result<Vec<Ciphertext>, FriendlyCryptoError> {
    if choice == 0 || choice > candidate_count {
        return Err(FriendlyCryptoError::InvalidChoice {
            choice,
            candidate_count,
        });
    }
    if weight == 0 || weight > w_max {
        return Err(FriendlyCryptoError::InvalidWeight { weight, w_max });
    }
    (1..=candidate_count)
        .map(|candidate| {
            let message = if candidate == choice { weight } else { 0 };
            encrypt(public_key, message, rng)
        })
        .collect()
}

pub fn fold_ballots(ballots: &[Vec<Ciphertext>]) -> Result<Vec<Ciphertext>, FriendlyCryptoError> {
    let candidate_count = ballots
        .first()
        .map(Vec::len)
        .ok_or(FriendlyCryptoError::LengthMismatch)?;
    let mut total = vec![Ciphertext::zero(); candidate_count];
    for ballot in ballots {
        if ballot.len() != candidate_count {
            return Err(FriendlyCryptoError::LengthMismatch);
        }
        for (accumulator, ciphertext) in total.iter_mut().zip(ballot) {
            validate_fresh_ciphertext(ciphertext)?;
            *accumulator = accumulator.add(ciphertext);
        }
    }
    Ok(total)
}

pub fn decrypt_to_group(secret: &Scalar, ciphertext: &Ciphertext) -> Point {
    ciphertext.c1 - ciphertext.c0.mul_bigint(secret.into_bigint())
}

pub fn bounded_discrete_log(target: &Point, bound: u64) -> Result<u64, FriendlyCryptoError> {
    let mut width = 1u64;
    while (width as u128) * (width as u128) <= bound as u128 {
        width += 1;
    }
    let mut table = HashMap::with_capacity(width as usize);
    let mut current = Point::zero();
    let base = generator();
    for value in 0..width {
        table.insert(encode_point(&current), value);
        current += base;
    }
    let giant_step = -base.mul_bigint(Scalar::from(width).into_bigint());
    let mut gamma = *target;
    for giant in 0..=width {
        if let Some(baby) = table.get(&encode_point(&gamma)) {
            let value = giant * width + baby;
            if value <= bound {
                return Ok(value);
            }
        }
        gamma += giant_step;
    }
    Err(FriendlyCryptoError::DiscreteLogNotFound { bound })
}

pub fn decrypt_tally(
    secret: &Scalar,
    aggregate: &[Ciphertext],
    bound: u64,
) -> Result<Vec<u64>, FriendlyCryptoError> {
    aggregate
        .iter()
        .map(|ciphertext| bounded_discrete_log(&decrypt_to_group(secret, ciphertext), bound))
        .collect()
}

pub fn run_four_voter_fixture(seed: u64) -> Result<FourVoterReport, FriendlyCryptoError> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let keypair = keygen(&mut rng);
    let weights = vec![3u64, 2, 1, 4];
    let choices = vec![1usize, 2, 1, 3];
    let registered_weight = weights.iter().sum();
    let encryption_started = Instant::now();
    let ballots = weights
        .iter()
        .zip(&choices)
        .map(|(weight, choice)| {
            encrypt_weighted_vote(
                &keypair.public,
                *weight,
                *choice,
                3,
                registered_weight,
                &mut rng,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let encryption_ms = encryption_started.elapsed().as_secs_f64() * 1000.0;

    let aggregate_started = Instant::now();
    let aggregate = fold_ballots(&ballots)?;
    let tally = decrypt_tally(&keypair.secret, &aggregate, registered_weight)?;
    let public_winner = winner(&tally, registered_weight)?;
    let aggregate_decrypt_decode_ms = aggregate_started.elapsed().as_secs_f64() * 1000.0;
    let all_native_relations_valid = tally == [4, 2, 4] && public_winner == 1;
    if !all_native_relations_valid {
        return Err(FriendlyCryptoError::DiscreteLogNotFound {
            bound: registered_weight,
        });
    }

    Ok(FourVoterReport {
        classification: "native-jubjub-weighted-elgamal-only-not-dkg-not-p3".to_owned(),
        suite_id: SUITE_ID.to_owned(),
        seed,
        candidate_count: 3,
        weights,
        choices,
        registered_weight,
        public_key_hex: hex(&encode_point(&keypair.public)),
        private_tally: tally,
        public_winner,
        ballots_encrypted: ballots.len(),
        encryption_ms,
        aggregate_decrypt_decode_ms,
        all_native_relations_valid,
        dkg_implemented: false,
        generalized_cp_implemented: false,
        full_g08: false,
        full_p3: false,
        limitations: vec![
            "single native secret key; Jubjub VSS/GJKR is not implemented".to_owned(),
            "generalized Chaum-Pedersen is not implemented for this suite".to_owned(),
            "this is native arithmetic only; no Groth16 claim is made".to_owned(),
        ],
    })
}

pub fn suite_vectors() -> Result<SuiteVectors, FriendlyCryptoError> {
    let secondary_label = b"PEDERSEN-H";
    let hash_domain = b"CP";
    let hash_transcript = b"VeriVoting-Jubjub-G0.8-vector-1";
    let secondary = derive_secondary_generator(secondary_label)?;
    let scalar = Scalar::from(123456u64);
    let (challenge, rejection_counter) = hash_to_scalar(hash_domain, hash_transcript)?;
    Ok(SuiteVectors {
        classification: "deterministic-native-jubjub-prototype-encoding-vectors".to_owned(),
        suite_id: SUITE_ID.to_owned(),
        point_encoding: "arkworks canonical compressed Jubjub point, 32 bytes".to_owned(),
        scalar_encoding: "canonical little-endian Jubjub scalar, 32 bytes, no reduction".to_owned(),
        generator_compressed_hex: hex(&encode_point(&generator())),
        identity_compressed_hex: hex(&encode_point(&Point::zero())),
        secondary_generator_label_hex: hex(secondary_label),
        secondary_generator_compressed_hex: hex(&encode_point(&secondary)),
        scalar_value: 123456,
        scalar_canonical_le_hex: hex(&encode_scalar(&scalar)),
        hash_to_scalar_domain_hex: hex(hash_domain),
        hash_to_scalar_transcript_hex: hex(hash_transcript),
        hash_to_scalar_output_le_hex: hex(&encode_scalar(&challenge)),
        hash_to_scalar_rejection_counter: rejection_counter,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_scalar_roundtrip_and_modulus_rejection() {
        let scalar = Scalar::from(123456u64);
        assert_eq!(decode_scalar(&encode_scalar(&scalar)).unwrap(), scalar);
        let mut modulus = Scalar::MODULUS.to_bytes_le();
        modulus.resize(SCALAR_BYTES, 0);
        assert!(matches!(
            decode_scalar(&modulus),
            Err(FriendlyCryptoError::InvalidScalarEncoding)
        ));
    }

    #[test]
    fn canonical_point_roundtrip_rejects_identity_and_bad_encoding() {
        let point = generator().mul_bigint(Scalar::from(17u64).into_bigint());
        assert_eq!(decode_point(&encode_point(&point), false).unwrap(), point);
        assert!(matches!(
            decode_point(&encode_point(&Point::zero()), false),
            Err(FriendlyCryptoError::IdentityPoint)
        ));
        assert!(decode_point(&[0xff; POINT_BYTES], true).is_err());
    }

    #[test]
    fn hash_to_scalar_is_canonical_deterministic_and_domain_separated() {
        let first = hash_to_scalar(b"CP", b"transcript").unwrap();
        let repeated = hash_to_scalar(b"CP", b"transcript").unwrap();
        let other_domain = hash_to_scalar(b"SIGNATURE", b"transcript").unwrap();
        assert_eq!(first, repeated);
        assert_ne!(first.0, other_domain.0);
        assert!(first.1 < HASH_REJECTION_MAX_ATTEMPTS);
        assert_eq!(decode_scalar(&encode_scalar(&first.0)).unwrap(), first.0);
    }

    #[test]
    fn secondary_generator_is_deterministic_subgroup_and_not_standard_generator() {
        let first = derive_secondary_generator(b"PEDERSEN-H").unwrap();
        let second = derive_secondary_generator(b"PEDERSEN-H").unwrap();
        assert_eq!(first, second);
        assert!(!first.is_zero());
        assert_ne!(first, generator());
        assert_eq!(decode_point(&encode_point(&first), false).unwrap(), first);
    }

    #[test]
    fn suite_vectors_are_deterministic_and_have_fixed_width_encodings() {
        let first = suite_vectors().unwrap();
        let second = suite_vectors().unwrap();
        assert_eq!(
            first.generator_compressed_hex,
            second.generator_compressed_hex
        );
        assert_eq!(
            first.secondary_generator_compressed_hex,
            second.secondary_generator_compressed_hex
        );
        assert_eq!(first.generator_compressed_hex.len(), POINT_BYTES * 2);
        assert_eq!(first.scalar_canonical_le_hex.len(), SCALAR_BYTES * 2);
        assert_eq!(first.hash_to_scalar_output_le_hex.len(), SCALAR_BYTES * 2);
    }

    #[test]
    fn four_weighted_votes_decrypt_to_expected_tally_and_winner() {
        let report = run_four_voter_fixture(20261020).unwrap();
        assert_eq!(report.private_tally, [4, 2, 4]);
        assert_eq!(report.public_winner, 1);
        assert!(report.all_native_relations_valid);
        assert!(!report.full_g08);
        assert!(!report.full_p3);
    }

    #[test]
    fn wrong_key_and_tampered_ciphertext_do_not_preserve_tally() {
        let mut rng = ChaCha20Rng::seed_from_u64(7);
        let correct = keygen(&mut rng);
        let wrong = keygen(&mut rng);
        let ballot = encrypt_weighted_vote(&correct.public, 3, 1, 3, 10, &mut rng).unwrap();
        let wrong_result = decrypt_tally(&wrong.secret, &ballot, 10);
        assert!(wrong_result.is_err() || wrong_result.unwrap() != [3, 0, 0]);

        let mut tampered = ballot;
        tampered[0].c1 += generator();
        assert_ne!(
            decrypt_tally(&correct.secret, &tampered, 10).unwrap(),
            [3, 0, 0]
        );
    }

    #[test]
    fn invalid_public_key_ciphertext_choice_and_weight_are_rejected() {
        let mut rng = ChaCha20Rng::seed_from_u64(8);
        assert!(matches!(
            encrypt(&Point::zero(), 1, &mut rng),
            Err(FriendlyCryptoError::InvalidPublicKey)
        ));
        assert!(matches!(
            validate_fresh_ciphertext(&Ciphertext::zero()),
            Err(FriendlyCryptoError::InvalidCiphertext)
        ));
        let keypair = keygen(&mut rng);
        assert!(matches!(
            encrypt_weighted_vote(&keypair.public, 1, 0, 3, 10, &mut rng),
            Err(FriendlyCryptoError::InvalidChoice { .. })
        ));
        assert!(matches!(
            encrypt_weighted_vote(&keypair.public, 11, 1, 3, 10, &mut rng),
            Err(FriendlyCryptoError::InvalidWeight { .. })
        ));
        assert!(matches!(
            encrypt_weighted_vote(&keypair.public, 0, 1, 3, 10, &mut rng),
            Err(FriendlyCryptoError::InvalidWeight { .. })
        ));
    }
}
