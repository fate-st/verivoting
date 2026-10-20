use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar, traits::Identity};
use rand_core::{CryptoRng, RngCore};
use thiserror::Error;
use vv_crypto::{
    generator, lagrange_at_zero, point_bytes, secondary_generator, verify_signature,
    SchnorrKeypair, SchnorrSignature,
};
use vv_types::{derive_dkg_session_id, derive_prekey_context, sha256, CanonicalEncoder};

#[derive(Clone, Debug)]
pub struct DkgConfig {
    pub election_id: [u8; 32],
    pub policy_pre: [u8; 32],
    pub manifest_digest: [u8; 32],
    pub prekey_context: [u8; 32],
    pub session_id: [u8; 32],
    pub n: usize,
    pub t: usize,
    pub q_cert: usize,
}

impl DkgConfig {
    pub fn validate(&self) -> Result<(), DkgError> {
        if self.n < 2 * self.t + 1
            || self.q_cert != self.n - self.t
            || self.prekey_context
                != derive_prekey_context(self.election_id, self.policy_pre, self.manifest_digest)
            || self.session_id != derive_dkg_session_id(self.prekey_context)
        {
            return Err(DkgError::InvalidConfig);
        }
        Ok(())
    }
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

#[derive(Clone, Debug)]
pub struct DkgOutput {
    pub public_key: RistrettoPoint,
    pub public_coefficients: Vec<RistrettoPoint>,
    pub secret_shares: Vec<Scalar>,
    pub verification_shares: Vec<RistrettoPoint>,
    pub authentication_keys: Vec<SchnorrKeypair>,
    pub qual: Vec<u32>,
    pub certificate: DkgCertificate,
    pub transcript: DkgTranscript,
    pub config: DkgConfig,
}

#[derive(Debug, Error)]
pub enum DkgError {
    #[error("invalid DKG configuration")]
    InvalidConfig,
    #[error("too few qualified dealers")]
    TooFewQualified,
    #[error("identity public key")]
    IdentityPublicKey,
    #[error("certificate generation failed")]
    CertificateFailure,
    #[error("cryptographic operation failed: {0}")]
    Crypto(#[from] vv_crypto::CryptoError),
}

#[derive(Clone)]
struct Dealer {
    coefficients: Vec<Scalar>,
    blind_coefficients: Vec<Scalar>,
    pedersen: Vec<RistrettoPoint>,
}

impl Dealer {
    fn generate<R: RngCore + CryptoRng>(degree: usize, rng: &mut R) -> Self {
        let coefficients: Vec<_> = (0..=degree).map(|_| Scalar::random(rng)).collect();
        let blind_coefficients: Vec<_> = (0..=degree).map(|_| Scalar::random(rng)).collect();
        let pedersen = coefficients
            .iter()
            .zip(&blind_coefficients)
            .map(|(value, blind)| value * generator() + blind * secondary_generator())
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
    faults: FaultPlan,
    rng: &mut R,
) -> Result<DkgOutput, DkgError> {
    config.validate()?;
    let dealers: Vec<_> = (0..config.n)
        .map(|_| Dealer::generate(config.t, rng))
        .collect();
    let authentication_keys: Vec<_> = (0..config.n)
        .map(|_| SchnorrKeypair::generate(rng))
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
            if !verify_pedersen_share(&dealers[dealer_index].pedersen, receiver, share, blind) {
                transcript.complaints += 1;
                let response = if faults.invalid_complaint_response_dealer == Some(dealer_index) {
                    (share, blind)
                } else {
                    dealers[dealer_index].deal(receiver)
                };
                if verify_pedersen_share(
                    &dealers[dealer_index].pedersen,
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

    let mut feldman_records: Vec<Option<Vec<RistrettoPoint>>> = Vec::with_capacity(config.n);
    for (dealer_index, dealer) in dealers.iter().enumerate() {
        if !qualified[dealer_index] {
            feldman_records.push(None);
            continue;
        }
        let mut record: Vec<_> = dealer
            .coefficients
            .iter()
            .map(|coefficient| coefficient * generator())
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
            let recovered = interpolate_coefficients(&points);
            record = recovered
                .iter()
                .map(|coefficient| coefficient * generator())
                .collect();
            transcript
                .extraction_recoveries
                .push((dealer_index + 1) as u32);
        }
        feldman_records.push(Some(record));
    }

    let mut public_coefficients = vec![RistrettoPoint::identity(); config.t + 1];
    for record in feldman_records.iter().flatten() {
        for (aggregate, coefficient) in public_coefficients.iter_mut().zip(record) {
            *aggregate += coefficient;
        }
    }
    let public_key = public_coefficients[0];
    if public_key == RistrettoPoint::identity() {
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
        .any(|(secret, public)| secret * generator() != *public)
    {
        return Err(DkgError::CertificateFailure);
    }

    transcript.estimated_bytes =
        config.n * config.n * 64 + config.n * (config.t + 1) * 64 + transcript.complaints * 96;
    transcript.public_digest = transcript_digest(&config, &qual, &public_coefficients, &transcript);
    let key_core_digest = key_core_digest(
        &config,
        &qual,
        &public_coefficients,
        transcript.public_digest,
    );
    let mut signatures = Vec::with_capacity(config.n);
    for (index, key) in authentication_keys.iter().enumerate() {
        signatures.push((
            (index + 1) as u32,
            key.sign(b"VV/DKG/FINALIZE/v1", &key_core_digest, rng)?,
        ));
    }
    let certificate = DkgCertificate {
        key_core_digest,
        signatures,
    };
    let output = DkgOutput {
        public_key,
        public_coefficients,
        secret_shares,
        verification_shares,
        authentication_keys,
        qual,
        certificate,
        transcript,
        config,
    };
    if !output.verify_certificate() {
        return Err(DkgError::CertificateFailure);
    }
    Ok(output)
}

impl DkgOutput {
    pub fn verify_certificate(&self) -> bool {
        let expected = key_core_digest(
            &self.config,
            &self.qual,
            &self.public_coefficients,
            self.transcript.public_digest,
        );
        if expected != self.certificate.key_core_digest {
            return false;
        }
        let mut seen = std::collections::HashSet::new();
        let valid = self
            .certificate
            .signatures
            .iter()
            .filter(|(id, signature)| {
                let index = *id as usize;
                index > 0
                    && index <= self.authentication_keys.len()
                    && seen.insert(*id)
                    && verify_signature(
                        &self.authentication_keys[index - 1].public,
                        b"VV/DKG/FINALIZE/v1",
                        &expected,
                        signature,
                    )
            })
            .count();
        valid >= self.config.q_cert
    }
}

fn verify_pedersen_share(
    commitments: &[RistrettoPoint],
    receiver: usize,
    share: Scalar,
    blind: Scalar,
) -> bool {
    share * generator() + blind * secondary_generator()
        == evaluate_public_polynomial(commitments, receiver)
}

fn verify_feldman_share(commitments: &[RistrettoPoint], receiver: usize, share: Scalar) -> bool {
    share * generator() == evaluate_public_polynomial(commitments, receiver)
}

fn evaluate_polynomial(coefficients: &[Scalar], x: Scalar) -> Scalar {
    coefficients
        .iter()
        .rev()
        .fold(Scalar::ZERO, |value, coefficient| value * x + coefficient)
}

fn evaluate_public_polynomial(coefficients: &[RistrettoPoint], receiver: usize) -> RistrettoPoint {
    let x = Scalar::from(receiver as u64);
    coefficients
        .iter()
        .rev()
        .fold(RistrettoPoint::identity(), |value, coefficient| {
            x * value + coefficient
        })
}

fn interpolate_coefficients(points: &[(Scalar, Scalar)]) -> Vec<Scalar> {
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
        let scale = *y_i * denominator.invert();
        for (target, coefficient) in result.iter_mut().zip(basis) {
            *target += scale * coefficient;
        }
    }
    result
}

fn multiply_linear(polynomial: &[Scalar], constant: Scalar, linear: Scalar) -> Vec<Scalar> {
    let mut output = vec![Scalar::ZERO; polynomial.len() + 1];
    for (index, coefficient) in polynomial.iter().enumerate() {
        output[index] += coefficient * constant;
        output[index + 1] += coefficient * linear;
    }
    output
}

fn transcript_digest(
    config: &DkgConfig,
    qual: &[u32],
    public_coefficients: &[RistrettoPoint],
    transcript: &DkgTranscript,
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/DKG/TRANSCRIPT/v1");
    encoded.fixed(&config.election_id);
    encoded.fixed(&config.policy_pre);
    encoded.fixed(&config.manifest_digest);
    encoded.fixed(&config.prekey_context);
    encoded.fixed(&config.session_id);
    encoded.u64(config.n as u64);
    encoded.u64(config.t as u64);
    encoded.u64(transcript.complaints as u64);
    encoded.u64(transcript.corrected_complaints as u64);
    encoded.u64(qual.len() as u64);
    for trustee in qual {
        encoded.u32(*trustee);
    }
    for coefficient in public_coefficients {
        encoded.fixed(&point_bytes(coefficient));
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
    public_coefficients: &[RistrettoPoint],
    transcript_digest: [u8; 32],
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/KEYCORE/v3");
    encoded.fixed(&config.election_id);
    encoded.fixed(&config.policy_pre);
    encoded.fixed(&config.manifest_digest);
    encoded.fixed(&config.prekey_context);
    encoded.fixed(&config.session_id);
    encoded.u64(config.n as u64);
    encoded.u64(config.t as u64);
    encoded.u64((config.t + 1) as u64);
    encoded.u64(config.q_cert as u64);
    encoded.u64(qual.len() as u64);
    for trustee in qual {
        encoded.u32(*trustee);
    }
    for coefficient in public_coefficients {
        encoded.fixed(&point_bytes(coefficient));
    }
    encoded.fixed(&transcript_digest);
    sha256(&encoded.finish())
}

pub fn reconstruct_secret(indices: &[u32], shares: &[Scalar]) -> Result<Scalar, DkgError> {
    if indices.len() != shares.len() {
        return Err(DkgError::InvalidConfig);
    }
    let coefficients = lagrange_at_zero(indices)?;
    Ok(shares
        .iter()
        .zip(coefficients)
        .fold(Scalar::ZERO, |sum, (share, coefficient)| {
            sum + share * coefficient
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

    fn config() -> DkgConfig {
        let election_id = [1; 32];
        let policy_pre = [2; 32];
        let manifest_digest = [3; 32];
        let prekey_context = derive_prekey_context(election_id, policy_pre, manifest_digest);
        DkgConfig {
            election_id,
            policy_pre,
            manifest_digest,
            prekey_context,
            session_id: derive_dkg_session_id(prekey_context),
            n: 5,
            t: 2,
            q_cert: 3,
        }
    }

    fn combinations(values: &[u32], choose: usize) -> Vec<Vec<u32>> {
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

    #[test]
    fn every_three_of_five_subset_reconstructs_same_key() {
        let mut rng = ChaCha20Rng::seed_from_u64(23);
        let output = run_dkg(config(), FaultPlan::default(), &mut rng).unwrap();
        let ids = [1, 2, 3, 4, 5];
        for subset in combinations(&ids, 3) {
            let shares: Vec<_> = subset
                .iter()
                .map(|id| output.secret_shares[*id as usize - 1])
                .collect();
            let secret = reconstruct_secret(&subset, &shares).unwrap();
            assert_eq!(secret * generator(), output.public_key);
        }
        assert_eq!(output.certificate.signatures.len(), 5);
        assert!(output.verify_certificate());
    }

    #[test]
    fn complaints_and_extraction_recovery_are_exercised() {
        let mut rng = ChaCha20Rng::seed_from_u64(29);
        let output = run_dkg(
            config(),
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
        assert!(output.verify_certificate());
    }

    #[test]
    fn invalid_response_disqualifies_dealer_and_short_certificate_rejects() {
        let mut rng = ChaCha20Rng::seed_from_u64(31);
        let mut output = run_dkg(
            config(),
            FaultPlan {
                bad_private_share: Some((0, 2)),
                invalid_complaint_response_dealer: Some(0),
                ..FaultPlan::default()
            },
            &mut rng,
        )
        .unwrap();
        assert_eq!(output.transcript.disqualified_dealers, vec![1]);
        output.certificate.signatures.truncate(2);
        assert!(!output.verify_certificate());
    }

    #[test]
    fn stale_session_and_policy_replacement_are_rejected() {
        let mut stale = config();
        stale.policy_pre[0] ^= 1;
        assert!(matches!(stale.validate(), Err(DkgError::InvalidConfig)));

        let mut replay = config();
        replay.election_id[0] ^= 1;
        assert!(matches!(replay.validate(), Err(DkgError::InvalidConfig)));

        let mut rng = ChaCha20Rng::seed_from_u64(37);
        let mut output = run_dkg(config(), FaultPlan::default(), &mut rng).unwrap();
        output.config.policy_pre[0] ^= 1;
        assert!(!output.verify_certificate());
    }
}
