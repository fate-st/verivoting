//! Minimal G3-A five-process happy path and G3-B single restart path.
//!
//! Each Trustee owns one private state file, signs its point-to-point VSS
//! messages, independently verifies all received shares, and signs the common
//! KeyCore. The first three Trustees later sign decryption-share vectors for a
//! single fixed threshold subset. G3-B additionally authenticates one resumed
//! process against its persisted-state hash. Transport is the local filesystem;
//! atomic rename prevents readers from observing partial messages.

use super::{
    evaluate_public_polynomial, key_core_digest, transcript_digest, verify_feldman_share,
    verify_pedersen_share, verify_signature_with_domain, AuthenticationKeypair, Dealer, DkgConfig,
    DkgError, DkgTranscript, SchnorrSignature,
};
use ark_ec::Group;
use ark_ff::{PrimeField, Zero};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use vv_friendly_crypto::{
    decode_point, decode_scalar, derive_secondary_generator, encode_point, encode_scalar,
    generator, random_nonzero_scalar, Ciphertext, FriendlyCryptoError, Point, Scalar,
};
use vv_types::{hex, sha256, CanonicalEncoder};

pub const G3A_N: usize = 5;
pub const G3A_T: usize = 2;
pub const G3A_TAU: usize = 3;
pub const G3A_Q_CERT: usize = 3;
pub const G3A_SUBSET: [u32; 3] = [1, 2, 3];
pub const G3A_DEAL_DOMAIN: &[u8] = b"VV/JUBJUB/G3A/DEAL/v1";
pub const G3A_DECRYPT_DOMAIN: &[u8] = b"VV/JUBJUB/G3A/DECRYPT/v1";
pub const G3B_RESUME_DOMAIN: &[u8] = b"VV/JUBJUB/G3B/RESUME/v1";

#[derive(Debug, Error)]
pub enum MultiprocessError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("DKG error: {0}")]
    Dkg(#[from] DkgError),
    #[error("cryptography error: {0}")]
    Crypto(#[from] FriendlyCryptoError),
    #[error("invalid G3-A artifact: {0}")]
    Invalid(&'static str),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignatureRecord {
    pub nonce_hex: String,
    pub response_hex: String,
    pub rejection_counter: u16,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthRecord {
    pub trustee_id: u32,
    pub process_id: u32,
    pub authentication_public_key_hex: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResumeRecord {
    pub protocol_version: String,
    pub session_id_hex: String,
    pub trustee_id: u32,
    pub initial_process_id: u32,
    pub resumed_process_id: u32,
    pub private_state_sha256_hex: String,
    pub signature: SignatureRecord,
}

#[derive(Clone, Debug)]
pub struct RestartEvidence {
    pub trustee_id: u32,
    pub initial_process_id: u32,
    pub resumed_process_id: u32,
    pub private_state_sha256_hex: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct TrusteePrivateState {
    trustee_id: u32,
    protocol_version: String,
    manifest_sha256_hex: String,
    authentication_secret_hex: String,
    coefficients_hex: Vec<String>,
    blind_coefficients_hex: Vec<String>,
    secret_share_hex: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DealRecord {
    pub protocol_version: String,
    pub session_id_hex: String,
    pub dealer_id: u32,
    pub receiver_id: u32,
    pub process_id: u32,
    pub pedersen_commitments_hex: Vec<String>,
    pub feldman_commitments_hex: Vec<String>,
    pub share_hex: String,
    pub blind_hex: String,
    pub signature: SignatureRecord,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FinalRecord {
    pub protocol_version: String,
    pub session_id_hex: String,
    pub trustee_id: u32,
    pub process_id: u32,
    pub qualified_trustees: Vec<u32>,
    pub complaints: usize,
    pub corrected_complaints: usize,
    pub disqualified_dealers: Vec<u32>,
    pub extraction_recoveries: Vec<u32>,
    pub estimated_dkg_bytes: usize,
    pub transcript_digest_hex: String,
    pub key_core_digest_hex: String,
    pub public_key_hex: String,
    pub public_coefficients_hex: Vec<String>,
    pub verification_shares_hex: Vec<String>,
    pub certificate_signature: SignatureRecord,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PublicCiphertextRecord {
    pub c0_hex: String,
    pub c1_hex: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AggregateRecord {
    pub protocol_version: String,
    pub session_id_hex: String,
    pub aggregate_digest_hex: String,
    pub ciphertexts: Vec<PublicCiphertextRecord>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DecryptionRecord {
    pub protocol_version: String,
    pub session_id_hex: String,
    pub aggregate_digest_hex: String,
    pub trustee_id: u32,
    pub process_id: u32,
    pub decryption_shares_hex: Vec<String>,
    pub signature: SignatureRecord,
}

#[derive(Clone)]
pub struct PublicDkgResult {
    pub config: DkgConfig,
    pub public_key: Point,
    pub public_coefficients: Vec<Point>,
    pub verification_shares: Vec<Point>,
    pub transcript_digest: [u8; 32],
    pub key_core_digest: [u8; 32],
    pub process_ids: Vec<u32>,
    pub initial_process_ids: Vec<u32>,
    pub certificate_signers: Vec<u32>,
    pub estimated_dkg_bytes: usize,
}

fn decode_hex(value: &str) -> Result<Vec<u8>, MultiprocessError> {
    if !value.len().is_multiple_of(2) {
        return Err(MultiprocessError::Invalid("odd-length hex"));
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|_| MultiprocessError::Invalid("invalid hex"))
        })
        .collect()
}

fn decode_fixed(value: &str) -> Result<[u8; 32], MultiprocessError> {
    decode_hex(value)?
        .try_into()
        .map_err(|_| MultiprocessError::Invalid("expected 32-byte encoding"))
}

fn decode_scalar_hex(value: &str) -> Result<Scalar, MultiprocessError> {
    Ok(decode_scalar(&decode_fixed(value)?)?)
}

fn decode_point_hex(value: &str, allow_identity: bool) -> Result<Point, MultiprocessError> {
    Ok(decode_point(&decode_fixed(value)?, allow_identity)?)
}

fn signature_record(signature: &SchnorrSignature) -> SignatureRecord {
    SignatureRecord {
        nonce_hex: hex(&encode_point(&signature.nonce)),
        response_hex: hex(&encode_scalar(&signature.response)),
        rejection_counter: signature.rejection_counter,
    }
}

fn decode_signature(record: &SignatureRecord) -> Result<SchnorrSignature, MultiprocessError> {
    Ok(SchnorrSignature {
        nonce: decode_point_hex(&record.nonce_hex, false)?,
        response: decode_scalar_hex(&record.response_hex)?,
        rejection_counter: record.rejection_counter,
    })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, MultiprocessError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn atomic_write_json<T: Serialize>(
    path: &Path,
    value: &T,
    private: bool,
) -> Result<(), MultiprocessError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

pub fn auth_path(work_dir: &Path, trustee_id: u32) -> PathBuf {
    work_dir
        .join("auth")
        .join(format!("trustee-{trustee_id}.json"))
}

pub fn private_path(work_dir: &Path, trustee_id: u32) -> PathBuf {
    work_dir
        .join("private")
        .join(format!("trustee-{trustee_id}.json"))
}

pub fn resume_path(work_dir: &Path, trustee_id: u32) -> PathBuf {
    work_dir
        .join("resume")
        .join(format!("trustee-{trustee_id}.json"))
}

pub fn deal_path(work_dir: &Path, dealer_id: u32, receiver_id: u32) -> PathBuf {
    work_dir
        .join("deals")
        .join(format!("dealer-{dealer_id}-to-{receiver_id}.json"))
}

pub fn final_path(work_dir: &Path, trustee_id: u32) -> PathBuf {
    work_dir
        .join("final")
        .join(format!("trustee-{trustee_id}.json"))
}

pub fn aggregate_path(work_dir: &Path) -> PathBuf {
    work_dir.join("aggregate.json")
}

pub fn decryption_path(work_dir: &Path, trustee_id: u32) -> PathBuf {
    work_dir
        .join("decryption")
        .join(format!("trustee-{trustee_id}.json"))
}

fn manifest_data(manifest_path: &Path) -> Result<(Vec<u8>, String), MultiprocessError> {
    let bytes = fs::read(manifest_path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let version = value["protocol_version"]
        .as_str()
        .ok_or(MultiprocessError::Invalid(
            "manifest lacks protocol_version",
        ))?
        .to_owned();
    Ok((bytes, version))
}

pub fn init_trustee(
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<(), MultiprocessError> {
    if trustee_id == 0 || trustee_id as usize > G3A_N {
        return Err(MultiprocessError::Invalid("invalid Trustee identifier"));
    }
    let (manifest_bytes, protocol_version) = manifest_data(manifest_path)?;
    let trustee_seed = seed ^ 0x4733_412D_5452_5553u64.wrapping_mul(u64::from(trustee_id));
    let mut rng = ChaCha20Rng::seed_from_u64(trustee_seed);
    let authentication_secret = random_nonzero_scalar(&mut rng);
    let authentication_public = generator().mul_bigint(authentication_secret.into_bigint());
    let secondary = derive_secondary_generator(b"PEDERSEN-H")?;
    let dealer = Dealer::generate(G3A_T, &secondary, &mut rng);
    let state = TrusteePrivateState {
        trustee_id,
        protocol_version,
        manifest_sha256_hex: hex(&sha256(&manifest_bytes)),
        authentication_secret_hex: hex(&encode_scalar(&authentication_secret)),
        coefficients_hex: dealer
            .coefficients
            .iter()
            .map(|value| hex(&encode_scalar(value)))
            .collect(),
        blind_coefficients_hex: dealer
            .blind_coefficients
            .iter()
            .map(|value| hex(&encode_scalar(value)))
            .collect(),
        secret_share_hex: None,
    };
    atomic_write_json(&private_path(work_dir, trustee_id), &state, true)?;
    atomic_write_json(
        &auth_path(work_dir, trustee_id),
        &AuthRecord {
            trustee_id,
            process_id: std::process::id(),
            authentication_public_key_hex: hex(&encode_point(&authentication_public)),
        },
        false,
    )
}

pub fn load_auth_records(work_dir: &Path) -> Result<Vec<AuthRecord>, MultiprocessError> {
    (1..=G3A_N)
        .map(|id| read_json(&auth_path(work_dir, id as u32)))
        .collect()
}

pub fn derive_config(
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<(DkgConfig, String), MultiprocessError> {
    let (manifest_bytes, protocol_version) = manifest_data(manifest_path)?;
    let auth_records = load_auth_records(work_dir)?;
    if auth_records
        .iter()
        .enumerate()
        .any(|(index, record)| record.trustee_id != (index + 1) as u32)
    {
        return Err(MultiprocessError::Invalid(
            "authentication record order mismatch",
        ));
    }
    let authentication_public_keys = auth_records
        .iter()
        .map(|record| decode_point_hex(&record.authentication_public_key_hex, false))
        .collect::<Result<Vec<_>, _>>()?;
    let election_id = sha256(b"VV/JUBJUB/G3-A/FIVE-PROCESS-DKG");
    let config = DkgConfig::new(
        election_id,
        sha256(&manifest_bytes),
        authentication_public_keys,
        G3A_T,
        G3A_Q_CERT,
    )?;
    Ok((config, protocol_version))
}

fn load_private_state(
    work_dir: &Path,
    trustee_id: u32,
) -> Result<TrusteePrivateState, MultiprocessError> {
    read_json(&private_path(work_dir, trustee_id))
}

fn authentication_keypair(
    state: &TrusteePrivateState,
) -> Result<AuthenticationKeypair, MultiprocessError> {
    let secret = decode_scalar_hex(&state.authentication_secret_hex)?;
    if secret.is_zero() {
        return Err(MultiprocessError::Invalid("zero authentication secret"));
    }
    Ok(AuthenticationKeypair {
        secret,
        public: generator().mul_bigint(secret.into_bigint()),
    })
}

fn deal_signing_bytes(record: &DealRecord) -> Result<Vec<u8>, MultiprocessError> {
    let mut encoded = CanonicalEncoder::new(G3A_DEAL_DOMAIN);
    encoded.text(&record.protocol_version);
    encoded.fixed(&decode_fixed(&record.session_id_hex)?);
    encoded.u32(record.dealer_id);
    encoded.u32(record.receiver_id);
    encoded.u32(record.process_id);
    encoded.u64(record.pedersen_commitments_hex.len() as u64);
    for commitment in &record.pedersen_commitments_hex {
        encoded.fixed(&decode_fixed(commitment)?);
    }
    encoded.u64(record.feldman_commitments_hex.len() as u64);
    for commitment in &record.feldman_commitments_hex {
        encoded.fixed(&decode_fixed(commitment)?);
    }
    encoded.fixed(&decode_fixed(&record.share_hex)?);
    encoded.fixed(&decode_fixed(&record.blind_hex)?);
    Ok(encoded.finish())
}

fn resume_signing_bytes(record: &ResumeRecord) -> Result<Vec<u8>, MultiprocessError> {
    let mut encoded = CanonicalEncoder::new(G3B_RESUME_DOMAIN);
    encoded.text(&record.protocol_version);
    encoded.fixed(&decode_fixed(&record.session_id_hex)?);
    encoded.u32(record.trustee_id);
    encoded.u32(record.initial_process_id);
    encoded.u32(record.resumed_process_id);
    encoded.fixed(&decode_fixed(&record.private_state_sha256_hex)?);
    Ok(encoded.finish())
}

pub fn record_resume(
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<(), MultiprocessError> {
    let state_path = private_path(work_dir, trustee_id);
    let state_bytes = fs::read(&state_path)?;
    let state: TrusteePrivateState = serde_json::from_slice(&state_bytes)?;
    let (config, protocol_version) = derive_config(work_dir, manifest_path)?;
    let auth: AuthRecord = read_json(&auth_path(work_dir, trustee_id))?;
    let keypair = authentication_keypair(&state)?;
    if auth.authentication_public_key_hex != hex(&encode_point(&keypair.public)) {
        return Err(MultiprocessError::Invalid(
            "resumed authentication key mismatch",
        ));
    }
    let mut record = ResumeRecord {
        protocol_version,
        session_id_hex: hex(&config.session_id),
        trustee_id,
        initial_process_id: auth.process_id,
        resumed_process_id: std::process::id(),
        private_state_sha256_hex: hex(&sha256(&state_bytes)),
        signature: SignatureRecord {
            nonce_hex: String::new(),
            response_hex: String::new(),
            rejection_counter: 0,
        },
    };
    let mut rng = ChaCha20Rng::seed_from_u64(
        seed ^ 0x4733_422D_5245_5355u64.wrapping_mul(u64::from(trustee_id)),
    );
    let signature = keypair.sign(G3B_RESUME_DOMAIN, &resume_signing_bytes(&record)?, &mut rng)?;
    record.signature = signature_record(&signature);
    atomic_write_json(&resume_path(work_dir, trustee_id), &record, false)
}

pub fn verify_resume_record(
    trustee_id: u32,
    expected_private_state_sha256: [u8; 32],
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<RestartEvidence, MultiprocessError> {
    let record: ResumeRecord = read_json(&resume_path(work_dir, trustee_id))?;
    let (config, protocol_version) = derive_config(work_dir, manifest_path)?;
    let auth: AuthRecord = read_json(&auth_path(work_dir, trustee_id))?;
    if record.protocol_version != protocol_version
        || record.session_id_hex != hex(&config.session_id)
        || record.trustee_id != trustee_id
        || record.initial_process_id != auth.process_id
        || record.resumed_process_id == record.initial_process_id
        || record.private_state_sha256_hex != hex(&expected_private_state_sha256)
    {
        return Err(MultiprocessError::Invalid("restart evidence mismatch"));
    }
    let signature = decode_signature(&record.signature)?;
    if !verify_signature_with_domain(
        &config.authentication_public_keys[trustee_id as usize - 1],
        G3B_RESUME_DOMAIN,
        &resume_signing_bytes(&record)?,
        &signature,
    ) {
        return Err(MultiprocessError::Invalid(
            "restart evidence signature mismatch",
        ));
    }
    Ok(RestartEvidence {
        trustee_id,
        initial_process_id: record.initial_process_id,
        resumed_process_id: record.resumed_process_id,
        private_state_sha256_hex: record.private_state_sha256_hex,
    })
}

pub fn deal_trustee(
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<(), MultiprocessError> {
    let state = load_private_state(work_dir, trustee_id)?;
    let (config, protocol_version) = derive_config(work_dir, manifest_path)?;
    if state.protocol_version != protocol_version
        || state.manifest_sha256_hex != hex(&config.manifest_digest)
    {
        return Err(MultiprocessError::Invalid(
            "private state manifest mismatch",
        ));
    }
    let coefficients = state
        .coefficients_hex
        .iter()
        .map(|value| decode_scalar_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    let blind_coefficients = state
        .blind_coefficients_hex
        .iter()
        .map(|value| decode_scalar_hex(value))
        .collect::<Result<Vec<_>, _>>()?;
    if coefficients.len() != G3A_T + 1 || blind_coefficients.len() != G3A_T + 1 {
        return Err(MultiprocessError::Invalid(
            "dealer polynomial width mismatch",
        ));
    }
    let secondary = derive_secondary_generator(b"PEDERSEN-H")?;
    let pedersen = coefficients
        .iter()
        .zip(&blind_coefficients)
        .map(|(value, blind)| {
            generator().mul_bigint(value.into_bigint()) + secondary.mul_bigint(blind.into_bigint())
        })
        .collect::<Vec<_>>();
    let dealer = Dealer {
        coefficients: coefficients.clone(),
        blind_coefficients,
        pedersen: pedersen.clone(),
    };
    let feldman = coefficients
        .iter()
        .map(|value| generator().mul_bigint(value.into_bigint()))
        .collect::<Vec<_>>();
    let keypair = authentication_keypair(&state)?;
    let mut rng = ChaCha20Rng::seed_from_u64(
        seed ^ 0x4733_412D_4445_414Cu64.wrapping_mul(u64::from(trustee_id)),
    );
    for receiver in 1..=G3A_N {
        let (share, blind) = dealer.deal(receiver);
        let mut record = DealRecord {
            protocol_version: protocol_version.clone(),
            session_id_hex: hex(&config.session_id),
            dealer_id: trustee_id,
            receiver_id: receiver as u32,
            process_id: std::process::id(),
            pedersen_commitments_hex: pedersen
                .iter()
                .map(|point| hex(&encode_point(point)))
                .collect(),
            feldman_commitments_hex: feldman
                .iter()
                .map(|point| hex(&encode_point(point)))
                .collect(),
            share_hex: hex(&encode_scalar(&share)),
            blind_hex: hex(&encode_scalar(&blind)),
            signature: SignatureRecord {
                nonce_hex: String::new(),
                response_hex: String::new(),
                rejection_counter: 0,
            },
        };
        let signature = keypair.sign(G3A_DEAL_DOMAIN, &deal_signing_bytes(&record)?, &mut rng)?;
        record.signature = signature_record(&signature);
        atomic_write_json(
            &deal_path(work_dir, trustee_id, receiver as u32),
            &record,
            true,
        )?;
    }
    Ok(())
}

pub fn finalize_trustee(
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<(), MultiprocessError> {
    let mut state = load_private_state(work_dir, trustee_id)?;
    let (config, protocol_version) = derive_config(work_dir, manifest_path)?;
    let secondary = derive_secondary_generator(b"PEDERSEN-H")?;
    let mut secret_share = Scalar::zero();
    let mut public_coefficients = vec![Point::zero(); G3A_T + 1];
    for dealer_id in 1..=G3A_N {
        let record: DealRecord = read_json(&deal_path(work_dir, dealer_id as u32, trustee_id))?;
        if record.protocol_version != protocol_version
            || record.session_id_hex != hex(&config.session_id)
            || record.dealer_id != dealer_id as u32
            || record.receiver_id != trustee_id
        {
            return Err(MultiprocessError::Invalid("deal routing mismatch"));
        }
        let signature = decode_signature(&record.signature)?;
        if !verify_signature_with_domain(
            &config.authentication_public_keys[dealer_id - 1],
            G3A_DEAL_DOMAIN,
            &deal_signing_bytes(&record)?,
            &signature,
        ) {
            return Err(MultiprocessError::Invalid("unauthenticated deal"));
        }
        let pedersen = record
            .pedersen_commitments_hex
            .iter()
            .map(|value| decode_point_hex(value, true))
            .collect::<Result<Vec<_>, _>>()?;
        let feldman = record
            .feldman_commitments_hex
            .iter()
            .map(|value| decode_point_hex(value, true))
            .collect::<Result<Vec<_>, _>>()?;
        let share = decode_scalar_hex(&record.share_hex)?;
        let blind = decode_scalar_hex(&record.blind_hex)?;
        if pedersen.len() != G3A_T + 1
            || feldman.len() != G3A_T + 1
            || !verify_pedersen_share(&pedersen, &secondary, trustee_id as usize, share, blind)
            || !verify_feldman_share(&feldman, trustee_id as usize, share)
        {
            return Err(MultiprocessError::Invalid("VSS share verification failed"));
        }
        secret_share += share;
        for (aggregate, coefficient) in public_coefficients.iter_mut().zip(feldman) {
            *aggregate += coefficient;
        }
    }
    let public_key = public_coefficients[0];
    let verification_shares = (1..=G3A_N)
        .map(|receiver| evaluate_public_polynomial(&public_coefficients, receiver))
        .collect::<Vec<_>>();
    if generator().mul_bigint(secret_share.into_bigint())
        != verification_shares[trustee_id as usize - 1]
    {
        return Err(MultiprocessError::Invalid("final secret share mismatch"));
    }
    let qualified_trustees = (1..=G3A_N).map(|id| id as u32).collect::<Vec<_>>();
    let mut transcript = DkgTranscript {
        estimated_bytes: G3A_N * G3A_N * 64 + G3A_N * (G3A_T + 1) * 32,
        ..DkgTranscript::default()
    };
    transcript.public_digest = transcript_digest(
        &config,
        &qualified_trustees,
        &public_coefficients,
        &transcript,
    );
    let key_core = key_core_digest(
        &config,
        &qualified_trustees,
        &public_coefficients,
        transcript.public_digest,
    );
    let keypair = authentication_keypair(&state)?;
    let mut rng = ChaCha20Rng::seed_from_u64(
        seed ^ 0x4733_412D_4345_5254u64.wrapping_mul(u64::from(trustee_id)),
    );
    let certificate_signature = keypair.sign(b"DKG-CERT-SIGNATURE", &key_core, &mut rng)?;
    state.secret_share_hex = Some(hex(&encode_scalar(&secret_share)));
    atomic_write_json(&private_path(work_dir, trustee_id), &state, true)?;
    atomic_write_json(
        &final_path(work_dir, trustee_id),
        &FinalRecord {
            protocol_version,
            session_id_hex: hex(&config.session_id),
            trustee_id,
            process_id: std::process::id(),
            qualified_trustees,
            complaints: 0,
            corrected_complaints: 0,
            disqualified_dealers: Vec::new(),
            extraction_recoveries: Vec::new(),
            estimated_dkg_bytes: transcript.estimated_bytes,
            transcript_digest_hex: hex(&transcript.public_digest),
            key_core_digest_hex: hex(&key_core),
            public_key_hex: hex(&encode_point(&public_key)),
            public_coefficients_hex: public_coefficients
                .iter()
                .map(|point| hex(&encode_point(point)))
                .collect(),
            verification_shares_hex: verification_shares
                .iter()
                .map(|point| hex(&encode_point(point)))
                .collect(),
            certificate_signature: signature_record(&certificate_signature),
        },
        false,
    )
}

fn aggregate_digest(record: &AggregateRecord) -> Result<[u8; 32], MultiprocessError> {
    let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/G3A/AGGREGATE/v1");
    encoded.text(&record.protocol_version);
    encoded.fixed(&decode_fixed(&record.session_id_hex)?);
    encoded.u64(record.ciphertexts.len() as u64);
    for ciphertext in &record.ciphertexts {
        encoded.fixed(&decode_fixed(&ciphertext.c0_hex)?);
        encoded.fixed(&decode_fixed(&ciphertext.c1_hex)?);
    }
    Ok(sha256(&encoded.finish()))
}

pub fn write_aggregate(
    work_dir: &Path,
    protocol_version: &str,
    session_id: [u8; 32],
    ciphertexts: &[Ciphertext],
) -> Result<[u8; 32], MultiprocessError> {
    let mut record = AggregateRecord {
        protocol_version: protocol_version.to_owned(),
        session_id_hex: hex(&session_id),
        aggregate_digest_hex: String::new(),
        ciphertexts: ciphertexts
            .iter()
            .map(|ciphertext| PublicCiphertextRecord {
                c0_hex: hex(&encode_point(&ciphertext.c0)),
                c1_hex: hex(&encode_point(&ciphertext.c1)),
            })
            .collect(),
    };
    let digest = aggregate_digest(&record)?;
    record.aggregate_digest_hex = hex(&digest);
    atomic_write_json(&aggregate_path(work_dir), &record, false)?;
    Ok(digest)
}

fn decryption_signing_bytes(record: &DecryptionRecord) -> Result<Vec<u8>, MultiprocessError> {
    let mut encoded = CanonicalEncoder::new(G3A_DECRYPT_DOMAIN);
    encoded.text(&record.protocol_version);
    encoded.fixed(&decode_fixed(&record.session_id_hex)?);
    encoded.fixed(&decode_fixed(&record.aggregate_digest_hex)?);
    encoded.u32(record.trustee_id);
    encoded.u64(record.decryption_shares_hex.len() as u64);
    for share in &record.decryption_shares_hex {
        encoded.fixed(&decode_fixed(share)?);
    }
    Ok(encoded.finish())
}

pub fn decrypt_trustee(
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<(), MultiprocessError> {
    if !G3A_SUBSET.contains(&trustee_id) {
        return Ok(());
    }
    let state = load_private_state(work_dir, trustee_id)?;
    let (config, protocol_version) = derive_config(work_dir, manifest_path)?;
    let aggregate: AggregateRecord = read_json(&aggregate_path(work_dir))?;
    let digest = aggregate_digest(&aggregate)?;
    if aggregate.protocol_version != protocol_version
        || aggregate.session_id_hex != hex(&config.session_id)
        || aggregate.aggregate_digest_hex != hex(&digest)
    {
        return Err(MultiprocessError::Invalid("aggregate binding mismatch"));
    }
    let secret_share = decode_scalar_hex(
        state
            .secret_share_hex
            .as_deref()
            .ok_or(MultiprocessError::Invalid("missing persisted secret share"))?,
    )?;
    let shares = aggregate
        .ciphertexts
        .iter()
        .map(|ciphertext| {
            Ok(decode_point_hex(&ciphertext.c0_hex, false)?.mul_bigint(secret_share.into_bigint()))
        })
        .collect::<Result<Vec<Point>, MultiprocessError>>()?;
    let mut record = DecryptionRecord {
        protocol_version,
        session_id_hex: hex(&config.session_id),
        aggregate_digest_hex: hex(&digest),
        trustee_id,
        process_id: std::process::id(),
        decryption_shares_hex: shares
            .iter()
            .map(|point| hex(&encode_point(point)))
            .collect(),
        signature: SignatureRecord {
            nonce_hex: String::new(),
            response_hex: String::new(),
            rejection_counter: 0,
        },
    };
    let keypair = authentication_keypair(&state)?;
    let mut rng = ChaCha20Rng::seed_from_u64(
        seed ^ 0x4733_412D_4445_4352u64.wrapping_mul(u64::from(trustee_id)),
    );
    let signature = keypair.sign(
        G3A_DECRYPT_DOMAIN,
        &decryption_signing_bytes(&record)?,
        &mut rng,
    )?;
    record.signature = signature_record(&signature);
    atomic_write_json(&decryption_path(work_dir, trustee_id), &record, false)
}

pub fn load_public_dkg(
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<PublicDkgResult, MultiprocessError> {
    let (config, protocol_version) = derive_config(work_dir, manifest_path)?;
    let finals = (1..=G3A_N)
        .map(|id| read_json::<FinalRecord>(&final_path(work_dir, id as u32)))
        .collect::<Result<Vec<_>, _>>()?;
    let first = &finals[0];
    let process_ids = finals
        .iter()
        .map(|record| record.process_id)
        .collect::<Vec<_>>();
    let auth = load_auth_records(work_dir)?;
    if finals.iter().enumerate().any(|(index, record)| {
        record.protocol_version != protocol_version
            || record.session_id_hex != hex(&config.session_id)
            || record.trustee_id != (index + 1) as u32
            || record.qualified_trustees != first.qualified_trustees
            || record.transcript_digest_hex != first.transcript_digest_hex
            || record.key_core_digest_hex != first.key_core_digest_hex
            || record.public_key_hex != first.public_key_hex
            || record.public_coefficients_hex != first.public_coefficients_hex
            || record.verification_shares_hex != first.verification_shares_hex
    }) {
        return Err(MultiprocessError::Invalid(
            "Trustee final-state disagreement",
        ));
    }
    let public_coefficients = first
        .public_coefficients_hex
        .iter()
        .map(|value| decode_point_hex(value, true))
        .collect::<Result<Vec<_>, _>>()?;
    let verification_shares = first
        .verification_shares_hex
        .iter()
        .map(|value| decode_point_hex(value, false))
        .collect::<Result<Vec<_>, _>>()?;
    if public_coefficients.len() != G3A_T + 1
        || verification_shares.len() != G3A_N
        || verification_shares
            .iter()
            .enumerate()
            .any(|(index, share)| {
                *share != evaluate_public_polynomial(&public_coefficients, index + 1)
            })
    {
        return Err(MultiprocessError::Invalid("public DKG polynomial mismatch"));
    }
    let transcript_hash = decode_fixed(&first.transcript_digest_hex)?;
    let key_core = decode_fixed(&first.key_core_digest_hex)?;
    let transcript = DkgTranscript {
        complaints: first.complaints,
        corrected_complaints: first.corrected_complaints,
        disqualified_dealers: first.disqualified_dealers.clone(),
        extraction_recoveries: first.extraction_recoveries.clone(),
        public_digest: transcript_hash,
        estimated_bytes: first.estimated_dkg_bytes,
    };
    let expected_transcript = transcript_digest(
        &config,
        &first.qualified_trustees,
        &public_coefficients,
        &transcript,
    );
    let expected_key_core = key_core_digest(
        &config,
        &first.qualified_trustees,
        &public_coefficients,
        expected_transcript,
    );
    if transcript_hash != expected_transcript
        || key_core != expected_key_core
        || decode_point_hex(&first.public_key_hex, false)? != public_coefficients[0]
    {
        return Err(MultiprocessError::Invalid(
            "transcript or KeyCore digest mismatch",
        ));
    }
    let mut certificate_signers = Vec::new();
    for (index, record) in finals.iter().enumerate() {
        let signature = decode_signature(&record.certificate_signature)?;
        if verify_signature_with_domain(
            &config.authentication_public_keys[index],
            b"DKG-CERT-SIGNATURE",
            &key_core,
            &signature,
        ) {
            certificate_signers.push((index + 1) as u32);
        }
    }
    if certificate_signers.len() < config.q_cert {
        return Err(MultiprocessError::Invalid(
            "insufficient KeyCore signatures",
        ));
    }
    Ok(PublicDkgResult {
        config,
        public_key: decode_point_hex(&first.public_key_hex, false)?,
        public_coefficients,
        verification_shares,
        transcript_digest: transcript_hash,
        key_core_digest: key_core,
        process_ids,
        initial_process_ids: auth.iter().map(|record| record.process_id).collect(),
        certificate_signers,
        estimated_dkg_bytes: first.estimated_dkg_bytes,
    })
}

pub fn load_and_verify_decryption_shares(
    work_dir: &Path,
    public_dkg: &PublicDkgResult,
    aggregate: &[Ciphertext],
) -> Result<Vec<Vec<Point>>, MultiprocessError> {
    let aggregate_record: AggregateRecord = read_json(&aggregate_path(work_dir))?;
    let digest = aggregate_digest(&aggregate_record)?;
    if aggregate_record.ciphertexts.len() != aggregate.len()
        || aggregate_record.aggregate_digest_hex != hex(&digest)
    {
        return Err(MultiprocessError::Invalid("aggregate artifact mismatch"));
    }
    G3A_SUBSET
        .iter()
        .map(|trustee_id| {
            let record: DecryptionRecord = read_json(&decryption_path(work_dir, *trustee_id))?;
            if record.protocol_version != aggregate_record.protocol_version
                || record.session_id_hex != hex(&public_dkg.config.session_id)
                || record.aggregate_digest_hex != hex(&digest)
                || record.trustee_id != *trustee_id
                || record.process_id != public_dkg.process_ids[*trustee_id as usize - 1]
                || record.decryption_shares_hex.len() != aggregate.len()
            {
                return Err(MultiprocessError::Invalid(
                    "decryption-share routing mismatch",
                ));
            }
            let signature = decode_signature(&record.signature)?;
            if !verify_signature_with_domain(
                &public_dkg.config.authentication_public_keys[*trustee_id as usize - 1],
                G3A_DECRYPT_DOMAIN,
                &decryption_signing_bytes(&record)?,
                &signature,
            ) {
                return Err(MultiprocessError::Invalid(
                    "unauthenticated decryption-share message",
                ));
            }
            record
                .decryption_shares_hex
                .iter()
                .map(|value| decode_point_hex(value, true))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect()
}

pub fn public_transport_bytes(work_dir: &Path) -> Result<u64, MultiprocessError> {
    let mut paths = Vec::new();
    for id in 1..=G3A_N as u32 {
        paths.push(auth_path(work_dir, id));
        paths.push(final_path(work_dir, id));
        for receiver in 1..=G3A_N as u32 {
            paths.push(deal_path(work_dir, id, receiver));
        }
    }
    paths.push(aggregate_path(work_dir));
    for id in G3A_SUBSET {
        paths.push(decryption_path(work_dir, id));
    }
    for id in 1..=G3A_N as u32 {
        let path = resume_path(work_dir, id);
        if path.is_file() {
            paths.push(path);
        }
    }
    paths
        .iter()
        .try_fold(0u64, |total, path| Ok(total + fs::metadata(path)?.len()))
}
