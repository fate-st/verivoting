use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_POINT,
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
    traits::Identity,
};
use rand_core::{CryptoRng, RngCore};
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use thiserror::Error;
use vv_types::{sha256, CanonicalEncoder};

pub const HASH_TO_SCALAR_MAX_ATTEMPTS: u16 = 128;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CryptoError {
    #[error("hash-to-scalar rejection bound exhausted")]
    HashToScalarExhausted,
    #[error("invalid canonical point encoding")]
    InvalidPoint,
    #[error("identity point is prohibited in this position")]
    ProhibitedIdentity,
    #[error("invalid threshold set")]
    InvalidThresholdSet,
    #[error("bounded discrete logarithm not found")]
    DecodeFailure,
    #[error("ciphertext vector length mismatch")]
    LengthMismatch,
}

pub fn generator() -> RistrettoPoint {
    RISTRETTO_BASEPOINT_POINT
}

pub fn secondary_generator() -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(b"VV/PEDERSEN/H/v1")
}

pub fn point_bytes(point: &RistrettoPoint) -> [u8; 32] {
    point.compress().to_bytes()
}

pub fn scalar_bytes(scalar: &Scalar) -> [u8; 32] {
    scalar.to_bytes()
}

pub fn decode_point(bytes: [u8; 32], allow_identity: bool) -> Result<RistrettoPoint, CryptoError> {
    let point = CompressedRistretto(bytes)
        .decompress()
        .ok_or(CryptoError::InvalidPoint)?;
    if !allow_identity && point == RistrettoPoint::identity() {
        return Err(CryptoError::ProhibitedIdentity);
    }
    Ok(point)
}

/// Bounded rejection sampling from a 253-bit candidate. Since the Ristretto
/// scalar modulus is greater than 2^252, each attempt succeeds with probability
/// greater than 1/2, so 128 failures have probability below 2^-128 in the ROM.
pub fn hash_to_scalar(domain: &[u8], message: &[u8]) -> Result<(Scalar, u16), CryptoError> {
    for counter in 0..HASH_TO_SCALAR_MAX_ATTEMPTS {
        let mut hash = Sha512::new();
        hash.update(b"VV/H2S/REJECTION/v1");
        hash.update((domain.len() as u64).to_be_bytes());
        hash.update(domain);
        hash.update((message.len() as u64).to_be_bytes());
        hash.update(message);
        hash.update(counter.to_be_bytes());
        let digest = hash.finalize();
        let mut candidate = [0u8; 32];
        candidate.copy_from_slice(&digest[..32]);
        candidate[31] &= 0x1f;
        if let Some(value) = Option::<Scalar>::from(Scalar::from_canonical_bytes(candidate)) {
            return Ok((value, counter));
        }
    }
    Err(CryptoError::HashToScalarExhausted)
}

#[derive(Clone, Debug)]
pub struct SchnorrKeypair {
    secret: Scalar,
    pub public: RistrettoPoint,
}

impl SchnorrKeypair {
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut secret = Scalar::random(rng);
        while secret == Scalar::ZERO {
            secret = Scalar::random(rng);
        }
        Self {
            secret,
            public: secret * generator(),
        }
    }

    pub fn sign<R: RngCore + CryptoRng>(
        &self,
        domain: &[u8],
        message: &[u8],
        rng: &mut R,
    ) -> Result<SchnorrSignature, CryptoError> {
        let nonce = Scalar::random(rng);
        let commitment = nonce * generator();
        let challenge = signature_challenge(domain, &self.public, &commitment, message)?;
        Ok(SchnorrSignature {
            commitment,
            response: nonce + challenge * self.secret,
        })
    }

    pub fn secret(&self) -> Scalar {
        self.secret
    }
}

#[derive(Clone, Debug)]
pub struct SchnorrSignature {
    pub commitment: RistrettoPoint,
    pub response: Scalar,
}

impl SchnorrSignature {
    pub fn canonical_bytes(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[..32].copy_from_slice(&point_bytes(&self.commitment));
        out[32..].copy_from_slice(&scalar_bytes(&self.response));
        out
    }
}

fn signature_challenge(
    domain: &[u8],
    public: &RistrettoPoint,
    commitment: &RistrettoPoint,
    message: &[u8],
) -> Result<Scalar, CryptoError> {
    let mut encoded = CanonicalEncoder::new(b"VV/SCHNORR/SIGNATURE/CHALLENGE/v1");
    encoded.bytes(domain);
    encoded.fixed(&point_bytes(public));
    encoded.fixed(&point_bytes(commitment));
    encoded.bytes(message);
    Ok(hash_to_scalar(b"VV/SCHNORR/H2S/v1", &encoded.finish())?.0)
}

pub fn verify_signature(
    public: &RistrettoPoint,
    domain: &[u8],
    message: &[u8],
    signature: &SchnorrSignature,
) -> bool {
    let Ok(challenge) = signature_challenge(domain, public, &signature.commitment, message) else {
        return false;
    };
    signature.response * generator() == signature.commitment + challenge * public
}

#[derive(Clone, Debug)]
pub struct PedersenCommitment {
    pub point: RistrettoPoint,
}

pub fn commit_scalar(message: Scalar, randomness: Scalar) -> PedersenCommitment {
    PedersenCommitment {
        point: message * generator() + randomness * secondary_generator(),
    }
}

pub fn commit_bytes(
    domain: &[u8],
    message: &[u8],
    randomness: Scalar,
) -> Result<PedersenCommitment, CryptoError> {
    let value = hash_to_scalar(domain, message)?.0;
    Ok(commit_scalar(value, randomness))
}

pub fn verify_commitment(
    commitment: &PedersenCommitment,
    message: Scalar,
    randomness: Scalar,
) -> bool {
    commitment.point == commit_scalar(message, randomness).point
}

#[derive(Clone, Debug)]
pub struct Ciphertext {
    pub c0: RistrettoPoint,
    pub c1: RistrettoPoint,
}

impl Ciphertext {
    pub fn identity() -> Self {
        Self {
            c0: RistrettoPoint::identity(),
            c1: RistrettoPoint::identity(),
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            c0: self.c0 + other.c0,
            c1: self.c1 + other.c1,
        }
    }

    pub fn canonical_bytes(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[..32].copy_from_slice(&point_bytes(&self.c0));
        out[32..].copy_from_slice(&point_bytes(&self.c1));
        out
    }
}

pub fn encrypt<R: RngCore + CryptoRng>(
    public_key: &RistrettoPoint,
    message: u64,
    rng: &mut R,
) -> (Ciphertext, Scalar) {
    let randomness = Scalar::random(rng);
    (
        Ciphertext {
            c0: randomness * generator(),
            c1: randomness * public_key + Scalar::from(message) * generator(),
        },
        randomness,
    )
}

pub fn aggregate_vectors(vectors: &[Vec<Ciphertext>]) -> Result<Vec<Ciphertext>, CryptoError> {
    let Some(first) = vectors.first() else {
        return Ok(Vec::new());
    };
    let width = first.len();
    if vectors.iter().any(|vector| vector.len() != width) {
        return Err(CryptoError::LengthMismatch);
    }
    let mut output = vec![Ciphertext::identity(); width];
    for vector in vectors {
        for (target, value) in output.iter_mut().zip(vector) {
            *target = target.add(value);
        }
    }
    Ok(output)
}

#[derive(Clone, Debug)]
pub struct ThresholdContext {
    pub election_id: [u8; 32],
    pub context: [u8; 32],
    pub policy_pre: [u8; 32],
    pub disclose: [u8; 32],
    pub target: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct DecryptionProof {
    pub a0: RistrettoPoint,
    pub coordinates: Vec<RistrettoPoint>,
    pub response: Scalar,
    pub rejection_counter: u16,
}

fn cp_transcript(
    context: &ThresholdContext,
    trustee: u32,
    verification_share: &RistrettoPoint,
    ciphertexts: &[Ciphertext],
    shares: &[RistrettoPoint],
    a0: &RistrettoPoint,
    coordinates: &[RistrettoPoint],
) -> Result<Vec<u8>, CryptoError> {
    if ciphertexts.len() != shares.len() || shares.len() != coordinates.len() {
        return Err(CryptoError::LengthMismatch);
    }
    let mut encoded = CanonicalEncoder::new(b"VV/THRESHOLD/DKEY/v1");
    encoded.fixed(&context.election_id);
    encoded.fixed(&context.context);
    encoded.fixed(&context.policy_pre);
    encoded.fixed(&context.disclose);
    encoded.fixed(&context.target);
    encoded.u32(trustee);
    encoded.fixed(&point_bytes(verification_share));
    encoded.u64(ciphertexts.len() as u64);
    for ciphertext in ciphertexts {
        encoded.fixed(&point_bytes(&ciphertext.c0));
    }
    for share in shares {
        encoded.fixed(&point_bytes(share));
    }
    encoded.fixed(&point_bytes(a0));
    for coordinate in coordinates {
        encoded.fixed(&point_bytes(coordinate));
    }
    Ok(encoded.finish())
}

/// Return the exact canonical CP transcript and its bounded hash-to-scalar
/// challenge. This is public for circuit compatibility vectors; production
/// verification still recomputes the value rather than accepting it.
pub fn decryption_transcript_and_challenge(
    context: &ThresholdContext,
    trustee: u32,
    verification_share: &RistrettoPoint,
    ciphertexts: &[Ciphertext],
    shares: &[RistrettoPoint],
    proof: &DecryptionProof,
) -> Result<(Vec<u8>, Scalar, u16), CryptoError> {
    let transcript = cp_transcript(
        context,
        trustee,
        verification_share,
        ciphertexts,
        shares,
        &proof.a0,
        &proof.coordinates,
    )?;
    let (challenge, counter) = hash_to_scalar(b"VV/THRESHOLD/DKEY/H2S/v1", &transcript)?;
    Ok((transcript, challenge, counter))
}

pub fn prove_decryption<R: RngCore + CryptoRng>(
    context: &ThresholdContext,
    trustee: u32,
    secret_share: Scalar,
    verification_share: &RistrettoPoint,
    ciphertexts: &[Ciphertext],
    rng: &mut R,
) -> Result<(Vec<RistrettoPoint>, DecryptionProof), CryptoError> {
    let shares: Vec<_> = ciphertexts
        .iter()
        .map(|ciphertext| secret_share * ciphertext.c0)
        .collect();
    let nonce = Scalar::random(rng);
    let a0 = nonce * generator();
    let coordinates: Vec<_> = ciphertexts
        .iter()
        .map(|ciphertext| nonce * ciphertext.c0)
        .collect();
    let transcript = cp_transcript(
        context,
        trustee,
        verification_share,
        ciphertexts,
        &shares,
        &a0,
        &coordinates,
    )?;
    let (challenge, rejection_counter) = hash_to_scalar(b"VV/THRESHOLD/DKEY/H2S/v1", &transcript)?;
    Ok((
        shares,
        DecryptionProof {
            a0,
            coordinates,
            response: nonce + challenge * secret_share,
            rejection_counter,
        },
    ))
}

pub fn verify_decryption(
    context: &ThresholdContext,
    trustee: u32,
    verification_share: &RistrettoPoint,
    ciphertexts: &[Ciphertext],
    shares: &[RistrettoPoint],
    proof: &DecryptionProof,
) -> bool {
    let Ok((_, challenge, counter)) = decryption_transcript_and_challenge(
        context,
        trustee,
        verification_share,
        ciphertexts,
        shares,
        proof,
    ) else {
        return false;
    };
    if counter != proof.rejection_counter
        || proof.response * generator() != proof.a0 + challenge * verification_share
    {
        return false;
    }
    ciphertexts.iter().zip(shares).zip(&proof.coordinates).all(
        |((ciphertext, share), coordinate)| {
            proof.response * ciphertext.c0 == coordinate + challenge * share
        },
    )
}

pub fn lagrange_at_zero(indices: &[u32]) -> Result<Vec<Scalar>, CryptoError> {
    if indices.is_empty()
        || indices.contains(&0)
        || indices.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(CryptoError::InvalidThresholdSet);
    }
    let field_indices: Vec<_> = indices
        .iter()
        .map(|index| Scalar::from(*index as u64))
        .collect();
    Ok(field_indices
        .iter()
        .enumerate()
        .map(|(i, current)| {
            field_indices
                .iter()
                .enumerate()
                .filter(|(j, _)| i != *j)
                .fold(Scalar::ONE, |value, (_, other)| {
                    value * (-other) * (*current - other).invert()
                })
        })
        .collect())
}

pub fn combine_decryption_shares(
    indices: &[u32],
    shares: &[Vec<RistrettoPoint>],
) -> Result<Vec<RistrettoPoint>, CryptoError> {
    if shares.len() != indices.len() || shares.is_empty() {
        return Err(CryptoError::InvalidThresholdSet);
    }
    let width = shares[0].len();
    if shares.iter().any(|share| share.len() != width) {
        return Err(CryptoError::LengthMismatch);
    }
    let coefficients = lagrange_at_zero(indices)?;
    let mut output = vec![RistrettoPoint::identity(); width];
    for (coordinate, target) in output.iter_mut().enumerate() {
        for (share, coefficient) in shares.iter().zip(&coefficients) {
            *target += coefficient * share[coordinate];
        }
    }
    Ok(output)
}

pub fn bounded_discrete_log(target: &RistrettoPoint, maximum: u64) -> Result<u64, CryptoError> {
    let mut step = 1u64;
    while step.saturating_mul(step) <= maximum {
        step += 1;
    }
    let mut babies = HashMap::with_capacity(step as usize);
    let mut point = RistrettoPoint::identity();
    for value in 0..step {
        babies.insert(point_bytes(&point), value);
        point += generator();
    }
    let giant = -(Scalar::from(step) * generator());
    let mut current = *target;
    for i in 0..=step {
        if let Some(j) = babies.get(&point_bytes(&current)) {
            let value = i.saturating_mul(step).saturating_add(*j);
            if value <= maximum {
                return Ok(value);
            }
        }
        current += giant;
    }
    Err(CryptoError::DecodeFailure)
}

pub fn ciphertext_vector_digest(ciphertexts: &[Ciphertext]) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/CIPHERTEXT/VECTOR/v1");
    encoded.u64(ciphertexts.len() as u64);
    for ciphertext in ciphertexts {
        encoded.fixed(&ciphertext.canonical_bytes());
    }
    sha256(&encoded.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

    #[test]
    fn encryption_cp_combination_and_decode_are_real_group_operations() {
        let mut rng = ChaCha20Rng::seed_from_u64(7);
        let secret = Scalar::from(19u64);
        let public = secret * generator();
        let (ciphertext, _) = encrypt(&public, 42, &mut rng);
        let context = ThresholdContext {
            election_id: [1; 32],
            context: [2; 32],
            policy_pre: [3; 32],
            disclose: [4; 32],
            target: [5; 32],
        };
        let (shares, proof) = prove_decryption(
            &context,
            1,
            secret,
            &public,
            std::slice::from_ref(&ciphertext),
            &mut rng,
        )
        .unwrap();
        assert!(verify_decryption(
            &context,
            1,
            &public,
            std::slice::from_ref(&ciphertext),
            &shares,
            &proof
        ));
        let recovered = ciphertext.c1 - shares[0];
        assert_eq!(bounded_discrete_log(&recovered, 100).unwrap(), 42);
    }

    #[test]
    fn cp_is_bound_to_target_and_transcript() {
        let mut rng = ChaCha20Rng::seed_from_u64(11);
        let secret = Scalar::from(23u64);
        let public = secret * generator();
        let (ciphertext, _) = encrypt(&public, 3, &mut rng);
        let mut context = ThresholdContext {
            election_id: [1; 32],
            context: [2; 32],
            policy_pre: [3; 32],
            disclose: [4; 32],
            target: [5; 32],
        };
        let (shares, proof) = prove_decryption(
            &context,
            1,
            secret,
            &public,
            std::slice::from_ref(&ciphertext),
            &mut rng,
        )
        .unwrap();
        context.target[0] ^= 1;
        assert!(!verify_decryption(
            &context,
            1,
            &public,
            std::slice::from_ref(&ciphertext),
            &shares,
            &proof
        ));
    }

    #[test]
    fn signatures_and_commitments_reject_mutation() {
        let mut rng = ChaCha20Rng::seed_from_u64(13);
        let key = SchnorrKeypair::generate(&mut rng);
        let signature = key.sign(b"test", b"message", &mut rng).unwrap();
        assert!(verify_signature(
            &key.public,
            b"test",
            b"message",
            &signature
        ));
        assert!(!verify_signature(
            &key.public,
            b"test",
            b"changed",
            &signature
        ));

        let opening = Scalar::random(&mut rng);
        let value = Scalar::from(10u64);
        let commitment = commit_scalar(value, opening);
        assert!(verify_commitment(&commitment, value, opening));
        assert!(!verify_commitment(
            &commitment,
            value + Scalar::ONE,
            opening
        ));
    }
}
