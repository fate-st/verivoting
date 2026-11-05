//! Real-group Jubjub Pedersen VSS/GJKR experiments.
//!
//! This crate models five authenticated trustees and the protocol's complaint,
//! Feldman extraction, certification, generalized CP package binding, and
//! threshold-decryption relations. The `multiprocess` module adds the minimal
//! G3-A five-process honest-path experiment; the original `run_dkg` API remains
//! the deterministic single-process reference implementation.

pub mod multiprocess;
pub mod package;
pub mod registration;

use ark_ec::Group;
use ark_ff::{Field, PrimeField, UniformRand, Zero};
use rand_core::{CryptoRng, RngCore};
use std::collections::HashSet;
use thiserror::Error;
use vv_friendly_crypto::cp::{derive_cp_target, VerifiedDecryptionShare};
use vv_friendly_crypto::{
    bounded_discrete_log, derive_secondary_generator, encode_point, generator, hash_to_scalar,
    random_nonzero_scalar, validate_point, Ciphertext, FriendlyCryptoError, Point, Scalar,
    SUITE_ID,
};
use vv_types::{
    derive_dkg_session_id, derive_final_context, derive_prekey_context, sha256, CanonicalEncoder,
};

#[derive(Clone)]
pub struct AuthenticationKeypair {
    secret: Scalar,
    pub public: Point,
}

impl AuthenticationKeypair {
    pub fn generate<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let secret = random_nonzero_scalar(rng);
        let public = generator().mul_bigint(secret.into_bigint());
        Self { secret, public }
    }

    fn sign<R: RngCore + CryptoRng>(
        &self,
        domain: &[u8],
        message: &[u8],
        rng: &mut R,
    ) -> Result<SchnorrSignature, DkgError> {
        let nonce_scalar = random_nonzero_scalar(rng);
        let nonce = generator().mul_bigint(nonce_scalar.into_bigint());
        let (challenge, rejection_counter) =
            signature_challenge(&self.public, &nonce, domain, message)?;
        Ok(SchnorrSignature {
            nonce,
            response: nonce_scalar + challenge * self.secret,
            rejection_counter,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SchnorrSignature {
    pub nonce: Point,
    pub response: Scalar,
    pub rejection_counter: u16,
}

impl SchnorrSignature {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(66);
        encoded.extend_from_slice(&encode_point(&self.nonce));
        encoded.extend_from_slice(&vv_friendly_crypto::encode_scalar(&self.response));
        encoded.extend_from_slice(&self.rejection_counter.to_be_bytes());
        encoded
    }
}

pub fn verify_signature(public: &Point, message: &[u8], signature: &SchnorrSignature) -> bool {
    verify_signature_with_domain(public, b"DKG-CERT-SIGNATURE", message, signature)
}

pub fn verify_signature_with_domain(
    public: &Point,
    domain: &[u8],
    message: &[u8],
    signature: &SchnorrSignature,
) -> bool {
    if validate_point(public, false).is_err() || validate_point(&signature.nonce, false).is_err() {
        return false;
    }
    let Ok((challenge, counter)) = signature_challenge(public, &signature.nonce, domain, message)
    else {
        return false;
    };
    counter == signature.rejection_counter
        && generator().mul_bigint(signature.response.into_bigint())
            == signature.nonce + public.mul_bigint(challenge.into_bigint())
}

fn signature_challenge(
    public: &Point,
    nonce: &Point,
    domain: &[u8],
    message: &[u8],
) -> Result<(Scalar, u16), DkgError> {
    let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/SCHNORR/TRANSCRIPT/v2");
    encoded.text(SUITE_ID);
    encoded.bytes(domain);
    encoded.fixed(&encode_point(public));
    encoded.fixed(&encode_point(nonce));
    encoded.bytes(message);
    Ok(hash_to_scalar(b"SCHNORR-SIGNATURE", &encoded.finish())?)
}

#[derive(Clone, Debug)]
pub struct DkgConfig {
    pub election_id: [u8; 32],
    pub policy_pre: [u8; 32],
    pub manifest_digest: [u8; 32],
    pub prekey_context: [u8; 32],
    pub session_id: [u8; 32],
    pub n: usize,
    pub t: usize,
    pub tau: usize,
    pub q_cert: usize,
    pub authentication_public_keys: Vec<Point>,
    pub registration_policy_digest: Option<[u8; 32]>,
}

impl DkgConfig {
    pub fn new(
        election_id: [u8; 32],
        manifest_digest: [u8; 32],
        authentication_public_keys: Vec<Point>,
        t: usize,
        q_cert: usize,
    ) -> Result<Self, DkgError> {
        Self::new_internal(
            election_id,
            manifest_digest,
            authentication_public_keys,
            None,
            t,
            q_cert,
        )
    }

    pub fn new_with_registration_policy(
        election_id: [u8; 32],
        manifest_digest: [u8; 32],
        authentication_public_keys: Vec<Point>,
        registration_policy_digest: [u8; 32],
        t: usize,
        q_cert: usize,
    ) -> Result<Self, DkgError> {
        Self::new_internal(
            election_id,
            manifest_digest,
            authentication_public_keys,
            Some(registration_policy_digest),
            t,
            q_cert,
        )
    }

    fn new_internal(
        election_id: [u8; 32],
        manifest_digest: [u8; 32],
        authentication_public_keys: Vec<Point>,
        registration_policy_digest: Option<[u8; 32]>,
        t: usize,
        q_cert: usize,
    ) -> Result<Self, DkgError> {
        let n = authentication_public_keys.len();
        let tau = t.checked_add(1).ok_or(DkgError::InvalidConfig)?;
        let policy_pre = derive_policy_pre_internal(
            election_id,
            manifest_digest,
            n,
            t,
            tau,
            q_cert,
            &authentication_public_keys,
            registration_policy_digest,
        )?;
        let prekey_context = derive_prekey_context(election_id, policy_pre, manifest_digest);
        let session_id = derive_dkg_session_id(prekey_context);
        let config = Self {
            election_id,
            policy_pre,
            manifest_digest,
            prekey_context,
            session_id,
            n,
            t,
            tau,
            q_cert,
            authentication_public_keys,
            registration_policy_digest,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), DkgError> {
        if self.n != self.authentication_public_keys.len()
            || self.n < 2 * self.t + 1
            || self.tau != self.t + 1
            || self.q_cert != self.n - self.t
            || self.q_cert < self.tau
            || self
                .authentication_public_keys
                .iter()
                .any(|key| validate_point(key, false).is_err())
        {
            return Err(DkgError::InvalidConfig);
        }
        let expected_policy = derive_policy_pre_internal(
            self.election_id,
            self.manifest_digest,
            self.n,
            self.t,
            self.tau,
            self.q_cert,
            &self.authentication_public_keys,
            self.registration_policy_digest,
        )?;
        if self.policy_pre != expected_policy
            || self.prekey_context
                != derive_prekey_context(self.election_id, self.policy_pre, self.manifest_digest)
            || self.session_id != derive_dkg_session_id(self.prekey_context)
        {
            return Err(DkgError::InvalidConfig);
        }
        Ok(())
    }
}

pub fn derive_policy_pre(
    election_id: [u8; 32],
    manifest_digest: [u8; 32],
    n: usize,
    t: usize,
    tau: usize,
    q_cert: usize,
    authentication_public_keys: &[Point],
) -> Result<[u8; 32], DkgError> {
    derive_policy_pre_internal(
        election_id,
        manifest_digest,
        n,
        t,
        tau,
        q_cert,
        authentication_public_keys,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn derive_policy_pre_internal(
    election_id: [u8; 32],
    manifest_digest: [u8; 32],
    n: usize,
    t: usize,
    tau: usize,
    q_cert: usize,
    authentication_public_keys: &[Point],
    registration_policy_digest: Option<[u8; 32]>,
) -> Result<[u8; 32], DkgError> {
    if authentication_public_keys.len() != n
        || authentication_public_keys
            .iter()
            .any(|key| validate_point(key, false).is_err())
    {
        return Err(DkgError::InvalidConfig);
    }
    let domain = if registration_policy_digest.is_some() {
        b"VV/JUBJUB/POLICY-PRE/v3".as_slice()
    } else {
        b"VV/JUBJUB/POLICY-PRE/v2".as_slice()
    };
    let mut encoded = CanonicalEncoder::new(domain);
    encoded.text(SUITE_ID);
    encoded.fixed(&election_id);
    encoded.fixed(&manifest_digest);
    encoded.u64(n as u64);
    encoded.u64(t as u64);
    encoded.u64(tau as u64);
    encoded.u64(q_cert as u64);
    encoded.u64(authentication_public_keys.len() as u64);
    for (index, key) in authentication_public_keys.iter().enumerate() {
        encoded.u32((index + 1) as u32);
        encoded.fixed(&encode_point(key));
    }
    if let Some(digest) = registration_policy_digest {
        encoded.fixed(&digest);
    }
    Ok(sha256(&encoded.finish()))
}

#[derive(Clone, Debug, Default)]
pub struct FaultPlan {
    pub bad_private_share: Option<(usize, usize)>,
    pub invalid_complaint_response_dealer: Option<usize>,
    pub bad_feldman_broadcast_dealer: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct DkgTranscript {
    pub complaints: usize,
    pub corrected_complaints: usize,
    pub disqualified_dealers: Vec<u32>,
    pub extraction_recoveries: Vec<u32>,
    pub public_digest: [u8; 32],
    pub estimated_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct DkgCertificate {
    pub key_core_digest: [u8; 32],
    pub signatures: Vec<(u32, SchnorrSignature)>,
}

#[derive(Clone)]
pub struct DkgOutput {
    pub public_key: Point,
    pub public_coefficients: Vec<Point>,
    secret_shares: Vec<Scalar>,
    pub verification_shares: Vec<Point>,
    pub qual: Vec<u32>,
    pub certificate: DkgCertificate,
    pub transcript: DkgTranscript,
    pub config: DkgConfig,
}

impl DkgOutput {
    pub fn secret_share(&self, trustee_id: u32) -> Result<Scalar, DkgError> {
        let index = trustee_index(trustee_id, self.config.n)?;
        Ok(self.secret_shares[index])
    }

    pub fn verification_share(&self, trustee_id: u32) -> Result<&Point, DkgError> {
        let index = trustee_index(trustee_id, self.config.n)?;
        Ok(&self.verification_shares[index])
    }

    pub fn verify_certificate(&self) -> bool {
        if self.config.validate().is_err() {
            return false;
        }
        let expected = key_core_digest(
            &self.config,
            &self.qual,
            &self.public_coefficients,
            self.transcript.public_digest,
        );
        if expected != self.certificate.key_core_digest {
            return false;
        }
        let mut seen = HashSet::new();
        let valid = self
            .certificate
            .signatures
            .iter()
            .filter(|(id, signature)| {
                let Ok(index) = trustee_index(*id, self.config.n) else {
                    return false;
                };
                seen.insert(*id)
                    && verify_signature(
                        &self.config.authentication_public_keys[index],
                        &expected,
                        signature,
                    )
            })
            .count();
        valid >= self.config.q_cert
    }
}

#[derive(Debug, Error)]
pub enum DkgError {
    #[error("invalid DKG configuration")]
    InvalidConfig,
    #[error("authentication keys do not match the frozen pre-policy")]
    AuthenticationMismatch,
    #[error("too few qualified dealers")]
    TooFewQualified,
    #[error("identity DKG public key")]
    IdentityPublicKey,
    #[error("certificate generation or verification failed")]
    CertificateFailure,
    #[error("invalid threshold subset")]
    InvalidThresholdSubset,
    #[error("interpolation denominator is zero")]
    InterpolationFailure,
    #[error("friendly cryptography error: {0}")]
    Crypto(#[from] FriendlyCryptoError),
    #[error("generalized CP error: {0}")]
    Cp(#[from] vv_friendly_crypto::cp::CpError),
}

#[derive(Clone)]
struct Dealer {
    coefficients: Vec<Scalar>,
    blind_coefficients: Vec<Scalar>,
    pedersen: Vec<Point>,
}

impl Dealer {
    fn generate<R: RngCore + CryptoRng>(degree: usize, secondary: &Point, rng: &mut R) -> Self {
        let coefficients: Vec<_> = (0..=degree).map(|_| Scalar::rand(rng)).collect();
        let blind_coefficients: Vec<_> = (0..=degree).map(|_| Scalar::rand(rng)).collect();
        let pedersen = coefficients
            .iter()
            .zip(&blind_coefficients)
            .map(|(value, blind)| {
                generator().mul_bigint(value.into_bigint())
                    + secondary.mul_bigint(blind.into_bigint())
            })
            .collect();
        Self {
            coefficients,
            blind_coefficients,
            pedersen,
        }
    }

    fn deal(&self, receiver: usize) -> (Scalar, Scalar) {
        let x = Scalar::from(receiver as u64);
        (
            evaluate_polynomial(&self.coefficients, x),
            evaluate_polynomial(&self.blind_coefficients, x),
        )
    }
}

pub fn run_dkg<R: RngCore + CryptoRng>(
    config: DkgConfig,
    authentication_keys: &[AuthenticationKeypair],
    faults: FaultPlan,
    rng: &mut R,
) -> Result<DkgOutput, DkgError> {
    config.validate()?;
    if authentication_keys.len() != config.n
        || authentication_keys
            .iter()
            .zip(&config.authentication_public_keys)
            .any(|(secret, public)| secret.public != *public)
    {
        return Err(DkgError::AuthenticationMismatch);
    }
    let secondary = derive_secondary_generator(b"PEDERSEN-H")?;
    let dealers: Vec<_> = (0..config.n)
        .map(|_| Dealer::generate(config.t, &secondary, rng))
        .collect();

    let mut delivered = vec![vec![(Scalar::ZERO, Scalar::ZERO); config.n]; config.n];
    for (dealer_index, dealer) in dealers.iter().enumerate() {
        for receiver in 1..=config.n {
            delivered[dealer_index][receiver - 1] = dealer.deal(receiver);
        }
    }
    if let Some((dealer, receiver)) = faults.bad_private_share {
        if dealer < config.n && receiver > 0 && receiver <= config.n {
            delivered[dealer][receiver - 1].0 += Scalar::ONE;
        }
    }

    let mut transcript = DkgTranscript::default();
    let mut qualified = vec![true; config.n];
    for dealer_index in 0..config.n {
        for receiver in 1..=config.n {
            let (share, blind) = delivered[dealer_index][receiver - 1];
            if !verify_pedersen_share(
                &dealers[dealer_index].pedersen,
                &secondary,
                receiver,
                share,
                blind,
            ) {
                transcript.complaints += 1;
                let response = if faults.invalid_complaint_response_dealer == Some(dealer_index) {
                    (share, blind)
                } else {
                    dealers[dealer_index].deal(receiver)
                };
                if verify_pedersen_share(
                    &dealers[dealer_index].pedersen,
                    &secondary,
                    receiver,
                    response.0,
                    response.1,
                ) {
                    delivered[dealer_index][receiver - 1] = response;
                    transcript.corrected_complaints += 1;
                } else {
                    qualified[dealer_index] = false;
                }
            }
        }
    }
    transcript.disqualified_dealers = qualified
        .iter()
        .enumerate()
        .filter_map(|(index, accepted)| (!accepted).then_some((index + 1) as u32))
        .collect();
    let qual: Vec<u32> = qualified
        .iter()
        .enumerate()
        .filter_map(|(index, accepted)| accepted.then_some((index + 1) as u32))
        .collect();
    if qual.len() <= config.t {
        return Err(DkgError::TooFewQualified);
    }

    let mut feldman_records: Vec<Option<Vec<Point>>> = Vec::with_capacity(config.n);
    for (dealer_index, dealer) in dealers.iter().enumerate() {
        if !qualified[dealer_index] {
            feldman_records.push(None);
            continue;
        }
        let mut record: Vec<_> = dealer
            .coefficients
            .iter()
            .map(|coefficient| generator().mul_bigint(coefficient.into_bigint()))
            .collect();
        if faults.bad_feldman_broadcast_dealer == Some(dealer_index) {
            record[0] += generator();
        }
        let record_valid = (1..=config.n).all(|receiver| {
            verify_feldman_share(&record, receiver, delivered[dealer_index][receiver - 1].0)
        });
        if !record_valid {
            let points: Vec<_> = (1..=config.t + 1)
                .map(|receiver| {
                    (
                        Scalar::from(receiver as u64),
                        delivered[dealer_index][receiver - 1].0,
                    )
                })
                .collect();
            let recovered = interpolate_coefficients(&points)?;
            record = recovered
                .iter()
                .map(|coefficient| generator().mul_bigint(coefficient.into_bigint()))
                .collect();
            transcript
                .extraction_recoveries
                .push((dealer_index + 1) as u32);
        }
        feldman_records.push(Some(record));
    }

    let mut public_coefficients = vec![Point::zero(); config.t + 1];
    for record in feldman_records.iter().flatten() {
        for (aggregate, coefficient) in public_coefficients.iter_mut().zip(record) {
            *aggregate += coefficient;
        }
    }
    let public_key = public_coefficients[0];
    if validate_point(&public_key, false).is_err() {
        return Err(DkgError::IdentityPublicKey);
    }
    let secret_shares: Vec<_> = (0..config.n)
        .map(|receiver| {
            qualified
                .iter()
                .enumerate()
                .filter(|(_, accepted)| **accepted)
                .fold(Scalar::ZERO, |sum, (dealer, _)| {
                    sum + delivered[dealer][receiver].0
                })
        })
        .collect();
    let verification_shares: Vec<_> = (1..=config.n)
        .map(|receiver| evaluate_public_polynomial(&public_coefficients, receiver))
        .collect();
    if secret_shares
        .iter()
        .zip(&verification_shares)
        .any(|(secret, public)| generator().mul_bigint(secret.into_bigint()) != *public)
    {
        return Err(DkgError::CertificateFailure);
    }

    transcript.estimated_bytes =
        config.n * config.n * 64 + config.n * (config.t + 1) * 32 + transcript.complaints * 96;
    transcript.public_digest = transcript_digest(&config, &qual, &public_coefficients, &transcript);
    let key_core_digest = key_core_digest(
        &config,
        &qual,
        &public_coefficients,
        transcript.public_digest,
    );
    let signatures = authentication_keys
        .iter()
        .enumerate()
        .map(|(index, key)| {
            Ok((
                (index + 1) as u32,
                key.sign(b"DKG-CERT-SIGNATURE", &key_core_digest, rng)?,
            ))
        })
        .collect::<Result<Vec<_>, DkgError>>()?;
    let output = DkgOutput {
        public_key,
        public_coefficients,
        secret_shares,
        verification_shares,
        qual,
        certificate: DkgCertificate {
            key_core_digest,
            signatures,
        },
        transcript,
        config,
    };
    if !output.verify_certificate() {
        return Err(DkgError::CertificateFailure);
    }
    Ok(output)
}

fn trustee_index(id: u32, n: usize) -> Result<usize, DkgError> {
    let index = usize::try_from(id).map_err(|_| DkgError::InvalidThresholdSubset)?;
    if index == 0 || index > n {
        return Err(DkgError::InvalidThresholdSubset);
    }
    Ok(index - 1)
}

fn verify_pedersen_share(
    commitments: &[Point],
    secondary: &Point,
    receiver: usize,
    share: Scalar,
    blind: Scalar,
) -> bool {
    generator().mul_bigint(share.into_bigint()) + secondary.mul_bigint(blind.into_bigint())
        == evaluate_public_polynomial(commitments, receiver)
}

fn verify_feldman_share(commitments: &[Point], receiver: usize, share: Scalar) -> bool {
    generator().mul_bigint(share.into_bigint()) == evaluate_public_polynomial(commitments, receiver)
}

fn evaluate_polynomial(coefficients: &[Scalar], x: Scalar) -> Scalar {
    coefficients
        .iter()
        .rev()
        .fold(Scalar::ZERO, |value, coefficient| value * x + coefficient)
}

fn evaluate_public_polynomial(coefficients: &[Point], receiver: usize) -> Point {
    let x = Scalar::from(receiver as u64);
    coefficients
        .iter()
        .rev()
        .fold(Point::zero(), |value, coefficient| {
            value.mul_bigint(x.into_bigint()) + coefficient
        })
}

fn interpolate_coefficients(points: &[(Scalar, Scalar)]) -> Result<Vec<Scalar>, DkgError> {
    let mut result = vec![Scalar::ZERO; points.len()];
    for (i, (x_i, y_i)) in points.iter().enumerate() {
        let mut basis = vec![Scalar::ONE];
        let mut denominator = Scalar::ONE;
        for (j, (x_j, _)) in points.iter().enumerate() {
            if i == j {
                continue;
            }
            basis = multiply_linear(&basis, -*x_j, Scalar::ONE);
            denominator *= x_i - x_j;
        }
        let inverse = denominator
            .inverse()
            .ok_or(DkgError::InterpolationFailure)?;
        let scale = *y_i * inverse;
        for (target, coefficient) in result.iter_mut().zip(basis) {
            *target += scale * coefficient;
        }
    }
    Ok(result)
}

fn multiply_linear(polynomial: &[Scalar], constant: Scalar, linear: Scalar) -> Vec<Scalar> {
    let mut output = vec![Scalar::ZERO; polynomial.len() + 1];
    for (index, coefficient) in polynomial.iter().enumerate() {
        output[index] += *coefficient * constant;
        output[index + 1] += *coefficient * linear;
    }
    output
}

fn transcript_digest(
    config: &DkgConfig,
    qual: &[u32],
    public_coefficients: &[Point],
    transcript: &DkgTranscript,
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/DKG/TRANSCRIPT/v2");
    encode_config_prefix(&mut encoded, config);
    encoded.u64(transcript.complaints as u64);
    encoded.u64(transcript.corrected_complaints as u64);
    encoded.u64(qual.len() as u64);
    for trustee in qual {
        encoded.u32(*trustee);
    }
    for coefficient in public_coefficients {
        encoded.fixed(&encode_point(coefficient));
    }
    for dealer in &transcript.disqualified_dealers {
        encoded.u32(*dealer);
    }
    for dealer in &transcript.extraction_recoveries {
        encoded.u32(*dealer);
    }
    sha256(&encoded.finish())
}

fn key_core_digest(
    config: &DkgConfig,
    qual: &[u32],
    public_coefficients: &[Point],
    transcript_digest: [u8; 32],
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/KEYCORE/v2");
    encode_config_prefix(&mut encoded, config);
    encoded.u64(qual.len() as u64);
    for trustee in qual {
        encoded.u32(*trustee);
    }
    for coefficient in public_coefficients {
        encoded.fixed(&encode_point(coefficient));
    }
    encoded.fixed(&transcript_digest);
    sha256(&encoded.finish())
}

fn encode_config_prefix(encoded: &mut CanonicalEncoder, config: &DkgConfig) {
    encoded.text(SUITE_ID);
    encoded.fixed(&config.election_id);
    encoded.fixed(&config.policy_pre);
    encoded.fixed(&config.manifest_digest);
    encoded.fixed(&config.prekey_context);
    encoded.fixed(&config.session_id);
    encoded.u64(config.n as u64);
    encoded.u64(config.t as u64);
    encoded.u64(config.tau as u64);
    encoded.u64(config.q_cert as u64);
    encoded.u64(config.authentication_public_keys.len() as u64);
    for (index, key) in config.authentication_public_keys.iter().enumerate() {
        encoded.u32((index + 1) as u32);
        encoded.fixed(&encode_point(key));
    }
}

pub fn lagrange_at_zero(
    indices: &[u32],
    n: usize,
    threshold: usize,
) -> Result<Vec<Scalar>, DkgError> {
    if indices.len() != threshold || threshold == 0 {
        return Err(DkgError::InvalidThresholdSubset);
    }
    let mut previous = 0u32;
    for id in indices {
        trustee_index(*id, n)?;
        if *id <= previous {
            return Err(DkgError::InvalidThresholdSubset);
        }
        previous = *id;
    }
    let xs: Vec<_> = indices.iter().map(|id| Scalar::from(*id as u64)).collect();
    xs.iter()
        .enumerate()
        .map(|(i, x_i)| {
            let mut numerator = Scalar::ONE;
            let mut denominator = Scalar::ONE;
            for (j, x_j) in xs.iter().enumerate() {
                if i == j {
                    continue;
                }
                numerator *= -*x_j;
                denominator *= *x_i - x_j;
            }
            denominator
                .inverse()
                .map(|inverse| numerator * inverse)
                .ok_or(DkgError::InterpolationFailure)
        })
        .collect()
}

pub fn interpolate_verification_key(output: &DkgOutput, subset: &[u32]) -> Result<Point, DkgError> {
    let coefficients = lagrange_at_zero(subset, output.config.n, output.config.tau)?;
    subset
        .iter()
        .zip(coefficients)
        .try_fold(Point::zero(), |sum, (id, coefficient)| {
            Ok(sum
                + output
                    .verification_share(*id)?
                    .mul_bigint(coefficient.into_bigint()))
        })
}

pub fn threshold_decrypt_tally(
    output: &DkgOutput,
    subset: &[u32],
    aggregate: &[Ciphertext],
    bound: u64,
) -> Result<Vec<u64>, DkgError> {
    let coefficients = lagrange_at_zero(subset, output.config.n, output.config.tau)?;
    aggregate
        .iter()
        .map(|ciphertext| {
            validate_point(&ciphertext.c0, true)?;
            validate_point(&ciphertext.c1, true)?;
            let mut combined_share = Point::zero();
            for (id, coefficient) in subset.iter().zip(&coefficients) {
                let decryption_share = ciphertext
                    .c0
                    .mul_bigint(output.secret_share(*id)?.into_bigint());
                combined_share += decryption_share.mul_bigint(coefficient.into_bigint());
            }
            Ok(bounded_discrete_log(
                &(ciphertext.c1 - combined_share),
                bound,
            )?)
        })
        .collect()
}

pub fn threshold_decrypt_verified_shares(
    output: &DkgOutput,
    subset: &[u32],
    aggregate: &[Ciphertext],
    verified_shares: &[VerifiedDecryptionShare],
    bound: u64,
) -> Result<Vec<u64>, DkgError> {
    threshold_decrypt_verified_shares_internal(
        output,
        subset,
        aggregate,
        verified_shares,
        [0; 32],
        [0; 32],
        bound,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn threshold_decrypt_verified_shares_with_registration(
    output: &DkgOutput,
    subset: &[u32],
    aggregate: &[Ciphertext],
    verified_shares: &[VerifiedDecryptionShare],
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    bound: u64,
) -> Result<Vec<u64>, DkgError> {
    if registration_close_digest == [0; 32] || disclosure_digest == [0; 32] {
        return Err(DkgError::InvalidThresholdSubset);
    }
    threshold_decrypt_verified_shares_internal(
        output,
        subset,
        aggregate,
        verified_shares,
        registration_close_digest,
        disclosure_digest,
        bound,
    )
}

#[allow(clippy::too_many_arguments)]
fn threshold_decrypt_verified_shares_internal(
    output: &DkgOutput,
    subset: &[u32],
    aggregate: &[Ciphertext],
    verified_shares: &[VerifiedDecryptionShare],
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    bound: u64,
) -> Result<Vec<u64>, DkgError> {
    let coefficients = lagrange_at_zero(subset, output.config.n, output.config.tau)?;
    let final_context = derive_final_context(
        output.config.prekey_context,
        output.certificate.key_core_digest,
    );
    let target = if registration_close_digest == [0; 32] && disclosure_digest == [0; 32] {
        derive_cp_target(
            output.config.election_id,
            output.config.policy_pre,
            final_context,
            aggregate,
        )?
    } else {
        vv_friendly_crypto::cp::derive_cp_target_with_registration(
            output.config.election_id,
            output.config.policy_pre,
            final_context,
            registration_close_digest,
            disclosure_digest,
            aggregate,
        )?
    };
    if aggregate.is_empty()
        || verified_shares.len() != subset.len()
        || verified_shares
            .iter()
            .zip(subset)
            .any(|(package, trustee)| {
                package.trustee_id() != *trustee
                    || !package.is_bound_to_registration(
                        output.config.election_id,
                        output.config.policy_pre,
                        final_context,
                        registration_close_digest,
                        disclosure_digest,
                        target,
                        aggregate.len(),
                    )
            })
    {
        return Err(DkgError::InvalidThresholdSubset);
    }
    aggregate
        .iter()
        .enumerate()
        .map(|(coordinate, ciphertext)| {
            validate_point(&ciphertext.c0, true)?;
            validate_point(&ciphertext.c1, true)?;
            let combined_share = verified_shares.iter().zip(&coefficients).fold(
                Point::zero(),
                |sum, (package, coefficient)| {
                    sum + package.shares()[coordinate].mul_bigint(coefficient.into_bigint())
                },
            );
            Ok(bounded_discrete_log(
                &(ciphertext.c1 - combined_share),
                bound,
            )?)
        })
        .collect()
}

pub fn combinations(values: &[u32], choose: usize) -> Vec<Vec<u32>> {
    fn visit(
        values: &[u32],
        choose: usize,
        start: usize,
        current: &mut Vec<u32>,
        output: &mut Vec<Vec<u32>>,
    ) {
        if current.len() == choose {
            output.push(current.clone());
            return;
        }
        for index in start..values.len() {
            current.push(values[index]);
            visit(values, choose, index + 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    visit(values, choose, 0, &mut Vec::new(), &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
    use vv_friendly_crypto::cp::{
        prove_decryption_share, verify_decryption_share, GeneralizedCpContext,
    };
    use vv_friendly_crypto::{encrypt_weighted_vote, fold_ballots};
    use vv_types::{derive_final_context, winner};

    fn authentication_keys<R: RngCore + CryptoRng>(rng: &mut R) -> Vec<AuthenticationKeypair> {
        (0..5)
            .map(|_| AuthenticationKeypair::generate(rng))
            .collect()
    }

    fn config(keys: &[AuthenticationKeypair]) -> DkgConfig {
        DkgConfig::new(
            sha256(b"VV/JUBJUB/TEST/ELECTION"),
            sha256(b"VV/JUBJUB/TEST/MANIFEST"),
            keys.iter().map(|key| key.public).collect(),
            2,
            3,
        )
        .unwrap()
    }

    #[test]
    fn all_ten_subsets_interpolate_key_and_threshold_decrypt() {
        let mut rng = ChaCha20Rng::seed_from_u64(20261020);
        let keys = authentication_keys(&mut rng);
        let output = run_dkg(config(&keys), &keys, FaultPlan::default(), &mut rng).unwrap();
        let weights = [3u64, 2, 1, 4];
        let choices = [1usize, 2, 1, 3];
        let ballots = weights
            .iter()
            .zip(choices)
            .map(|(weight, choice)| {
                encrypt_weighted_vote(&output.public_key, *weight, choice, 3, 10, &mut rng)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let aggregate = fold_ballots(&ballots).unwrap();
        let subsets = combinations(&[1, 2, 3, 4, 5], 3);
        assert_eq!(subsets.len(), 10);
        for subset in subsets {
            assert_eq!(
                interpolate_verification_key(&output, &subset).unwrap(),
                output.public_key
            );
            let tally = threshold_decrypt_tally(&output, &subset, &aggregate, 10).unwrap();
            assert_eq!(tally, [4, 2, 4]);
            assert_eq!(winner(&tally, 10).unwrap(), 1);
        }
        assert!(output.verify_certificate());
    }

    #[test]
    fn complaints_recovery_and_disqualification_are_explicit() {
        let mut rng = ChaCha20Rng::seed_from_u64(29);
        let keys = authentication_keys(&mut rng);
        let output = run_dkg(
            config(&keys),
            &keys,
            FaultPlan {
                bad_private_share: Some((1, 3)),
                bad_feldman_broadcast_dealer: Some(2),
                ..FaultPlan::default()
            },
            &mut rng,
        )
        .unwrap();
        assert_eq!(output.transcript.complaints, 1);
        assert_eq!(output.transcript.corrected_complaints, 1);
        assert_eq!(output.transcript.extraction_recoveries, vec![3]);

        let disqualified = run_dkg(
            config(&keys),
            &keys,
            FaultPlan {
                bad_private_share: Some((0, 2)),
                invalid_complaint_response_dealer: Some(0),
                ..FaultPlan::default()
            },
            &mut rng,
        )
        .unwrap();
        assert_eq!(disqualified.transcript.disqualified_dealers, vec![1]);
    }

    #[test]
    fn frozen_context_certificate_and_threshold_subset_reject_mutation() {
        let mut rng = ChaCha20Rng::seed_from_u64(37);
        let keys = authentication_keys(&mut rng);
        let mut output = run_dkg(config(&keys), &keys, FaultPlan::default(), &mut rng).unwrap();

        assert!(matches!(
            lagrange_at_zero(&[1, 1, 3], 5, 3),
            Err(DkgError::InvalidThresholdSubset)
        ));
        assert!(matches!(
            lagrange_at_zero(&[2, 1, 3], 5, 3),
            Err(DkgError::InvalidThresholdSubset)
        ));
        assert!(matches!(
            lagrange_at_zero(&[1, 2], 5, 3),
            Err(DkgError::InvalidThresholdSubset)
        ));

        output.certificate.signatures.truncate(2);
        assert!(!output.verify_certificate());

        let mut replay = config(&keys);
        replay.election_id[0] ^= 1;
        assert!(matches!(replay.validate(), Err(DkgError::InvalidConfig)));
    }

    #[test]
    fn verified_cp_packages_drive_all_ten_threshold_combinations() {
        let mut rng = ChaCha20Rng::seed_from_u64(20261021);
        let keys = authentication_keys(&mut rng);
        let output = run_dkg(config(&keys), &keys, FaultPlan::default(), &mut rng).unwrap();
        let ballots = [3u64, 2, 1, 4]
            .iter()
            .zip([1usize, 2, 1, 3])
            .map(|(weight, choice)| {
                encrypt_weighted_vote(&output.public_key, *weight, choice, 3, 10, &mut rng)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let aggregate = fold_ballots(&ballots).unwrap();
        let final_context = derive_final_context(
            output.config.prekey_context,
            output.certificate.key_core_digest,
        );
        let target = derive_cp_target(
            output.config.election_id,
            output.config.policy_pre,
            final_context,
            &aggregate,
        )
        .unwrap();
        let verified = (1..=5u32)
            .map(|trustee_id| {
                let context = GeneralizedCpContext {
                    election_id: output.config.election_id,
                    policy_pre: output.config.policy_pre,
                    final_context,
                    registration_close_digest: [0; 32],
                    disclosure_digest: [0; 32],
                    target,
                    trustee_id,
                };
                let package = prove_decryption_share(
                    &context,
                    output.secret_share(trustee_id).unwrap(),
                    output.verification_share(trustee_id).unwrap(),
                    &aggregate,
                    &mut rng,
                )
                .unwrap();
                verify_decryption_share(
                    &context,
                    output.verification_share(trustee_id).unwrap(),
                    &aggregate,
                    &package,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        for subset in combinations(&[1, 2, 3, 4, 5], 3) {
            let selected = subset
                .iter()
                .map(|id| verified[*id as usize - 1].clone())
                .collect::<Vec<_>>();
            let tally =
                threshold_decrypt_verified_shares(&output, &subset, &aggregate, &selected, 10)
                    .unwrap();
            assert_eq!(tally, [4, 2, 4]);
            assert_eq!(winner(&tally, 10).unwrap(), 1);
        }

        let mut other_election = output.config.election_id;
        other_election[0] ^= 1;
        let other_target = derive_cp_target(
            other_election,
            output.config.policy_pre,
            final_context,
            &aggregate,
        )
        .unwrap();
        let other_context = GeneralizedCpContext {
            election_id: other_election,
            policy_pre: output.config.policy_pre,
            final_context,
            registration_close_digest: [0; 32],
            disclosure_digest: [0; 32],
            target: other_target,
            trustee_id: 1,
        };
        let other_package = prove_decryption_share(
            &other_context,
            output.secret_share(1).unwrap(),
            output.verification_share(1).unwrap(),
            &aggregate,
            &mut rng,
        )
        .unwrap();
        let other_verified = verify_decryption_share(
            &other_context,
            output.verification_share(1).unwrap(),
            &aggregate,
            &other_package,
        )
        .unwrap();
        let mixed = vec![other_verified, verified[1].clone(), verified[2].clone()];
        assert!(matches!(
            threshold_decrypt_verified_shares(&output, &[1, 2, 3], &aggregate, &mixed, 10),
            Err(DkgError::InvalidThresholdSubset)
        ));
    }
}
