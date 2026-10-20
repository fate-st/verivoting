use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use thiserror::Error;

pub const CODEC_MAGIC: &[u8] = b"VV-CANONICAL-V1";

#[derive(Debug, Error)]
pub enum TypesError {
    #[error("manifest I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid winner input: {0}")]
    InvalidWinner(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityStatus {
    pub native_core: String,
    pub p1_groth16: String,
    pub snarkpack: String,
    pub state_proof: String,
    pub full_p3: String,
    pub target_chain: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolManifest {
    pub protocol_version: String,
    pub release_policy: String,
    pub n: usize,
    pub t: usize,
    pub tau: usize,
    pub q_cert: usize,
    pub candidate_count: usize,
    pub candidate_order: Vec<u32>,
    pub winning_rule: String,
    pub encryption_group: String,
    pub ballot_groth16_curve: String,
    pub snarkpack_revision: String,
    pub state_groth16_curve: String,
    pub disclosure_groth16_curve: String,
    pub cp_hash_to_scalar_suite: String,
    pub commitment_suite: String,
    pub signature_suite: String,
    pub canonical_codec_version: String,
    pub hash_rejection_max_attempts: u16,
    pub supported_batch_sizes: Vec<usize>,
    pub capabilities: CapabilityStatus,
}

impl ProtocolManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, TypesError> {
        let bytes = fs::read(path)?;
        let value: Self = serde_json::from_slice(&bytes)?;
        value.validate_core()?;
        Ok(value)
    }

    pub fn validate_core(&self) -> Result<(), TypesError> {
        let reject = |message: &str| Err(TypesError::InvalidManifest(message.to_owned()));
        if self.n < 2 * self.t + 1 {
            return reject("n must satisfy n >= 2t + 1");
        }
        if self.tau != self.t + 1 {
            return reject("tau must equal t + 1");
        }
        if self.q_cert != self.n - self.t {
            return reject("q_cert must equal n - t");
        }
        if self.candidate_count == 0
            || self.candidate_order.len() != self.candidate_count
            || self
                .candidate_order
                .iter()
                .copied()
                .ne(1..=self.candidate_count as u32)
        {
            return reject("candidate_order must be [1, ..., candidate_count]");
        }
        if self.release_policy != "WINNER_ONLY_V1" {
            return reject("only WINNER_ONLY_V1 is accepted by the official runner");
        }
        if self.winning_rule != "highest-weight-smallest-id-on-tie" {
            return reject("unsupported winner rule");
        }
        if self.hash_rejection_max_attempts != 128 {
            return reject("the implemented hash-to-scalar suite requires 128 attempts");
        }
        if self.supported_batch_sizes.is_empty()
            || self
                .supported_batch_sizes
                .iter()
                .any(|value| !value.is_power_of_two())
        {
            return reject("supported batch sizes must be nonempty powers of two");
        }
        let fields = [
            self.protocol_version.as_str(),
            self.encryption_group.as_str(),
            self.ballot_groth16_curve.as_str(),
            self.snarkpack_revision.as_str(),
            self.state_groth16_curve.as_str(),
            self.disclosure_groth16_curve.as_str(),
            self.cp_hash_to_scalar_suite.as_str(),
            self.commitment_suite.as_str(),
            self.signature_suite.as_str(),
            self.canonical_codec_version.as_str(),
        ];
        if fields
            .iter()
            .any(|value| value.trim().is_empty() || value.contains("UNDECIDED"))
        {
            return reject("empty or UNDECIDED cryptographic field");
        }
        if self.capabilities.native_core != "READY" || self.capabilities.p1_groth16 != "READY" {
            return reject("this build requires READY native_core and p1_groth16");
        }
        Ok(())
    }

    pub fn validate_full_submission(&self) -> Result<(), TypesError> {
        self.validate_core()?;
        for (name, status) in [
            ("snarkpack", &self.capabilities.snarkpack),
            ("state_proof", &self.capabilities.state_proof),
            ("full_p3", &self.capabilities.full_p3),
            ("target_chain", &self.capabilities.target_chain),
        ] {
            if status != "READY" {
                return Err(TypesError::InvalidManifest(format!(
                    "full submission blocked: capability {name} is {status}"
                )));
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = CanonicalEncoder::new(b"VV/PROTOCOL/MANIFEST/v1");
        out.text(&self.protocol_version);
        out.text(&self.release_policy);
        out.u64(self.n as u64);
        out.u64(self.t as u64);
        out.u64(self.tau as u64);
        out.u64(self.q_cert as u64);
        out.u64(self.candidate_count as u64);
        out.u64(self.candidate_order.len() as u64);
        for id in &self.candidate_order {
            out.u32(*id);
        }
        out.text(&self.winning_rule);
        out.text(&self.encryption_group);
        out.text(&self.ballot_groth16_curve);
        out.text(&self.snarkpack_revision);
        out.text(&self.state_groth16_curve);
        out.text(&self.disclosure_groth16_curve);
        out.text(&self.cp_hash_to_scalar_suite);
        out.text(&self.commitment_suite);
        out.text(&self.signature_suite);
        out.text(&self.canonical_codec_version);
        out.u16(self.hash_rejection_max_attempts);
        out.u64(self.supported_batch_sizes.len() as u64);
        for size in &self.supported_batch_sizes {
            out.u64(*size as u64);
        }
        out.finish()
    }

    pub fn digest_hex(&self) -> String {
        sha256_hex(&self.canonical_bytes())
    }
}

#[derive(Clone, Debug, Default)]
pub struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    pub fn new(domain: &[u8]) -> Self {
        let mut value = Self { bytes: Vec::new() };
        value.bytes(CODEC_MAGIC);
        value.bytes(domain);
        value
    }

    pub fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn fixed(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.fixed(value);
    }

    pub fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Bind the immutable pre-DKG election policy to the protocol manifest.
pub fn derive_prekey_context(
    election_id: [u8; 32],
    policy_pre: [u8; 32],
    manifest_digest: [u8; 32],
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/PREKEY/v3");
    encoded.fixed(&election_id);
    encoded.fixed(&policy_pre);
    encoded.fixed(&manifest_digest);
    sha256(&encoded.finish())
}

/// Derive the only DKG session identifier authorized by a frozen pre-key
/// context.
pub fn derive_dkg_session_id(prekey_context: [u8; 32]) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/DKG/SESSION/v1");
    encoded.fixed(&prekey_context);
    sha256(&encoded.finish())
}

/// Complete the acyclic binding chain after DKG finalization.
pub fn derive_final_context(prekey_context: [u8; 32], key_core_digest: [u8; 32]) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/FINALCONTEXT/v3");
    encoded.fixed(&prekey_context);
    encoded.fixed(&key_core_digest);
    sha256(&encoded.finish())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&sha256(bytes))
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn winner(scores: &[u64], w_registered: u64) -> Result<usize, TypesError> {
    if scores.is_empty() {
        return Err(TypesError::InvalidWinner(
            "candidate list cannot be empty".to_owned(),
        ));
    }
    if scores.iter().any(|score| *score > w_registered) {
        return Err(TypesError::InvalidWinner(
            "score exceeds W_registered".to_owned(),
        ));
    }
    let best = scores.iter().copied().max().expect("nonempty");
    Ok(scores
        .iter()
        .position(|score| *score == best)
        .expect("maximum exists")
        + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ProtocolManifest {
        serde_json::from_str(include_str!("../../../configs/protocol_manifest.json")).unwrap()
    }

    #[test]
    fn winner_uses_smallest_id_on_tie() {
        assert_eq!(winner(&[4, 2, 4], 10).unwrap(), 1);
        assert_eq!(winner(&[3, 5, 1], 10).unwrap(), 2);
        assert_eq!(winner(&[2, 2, 2], 10).unwrap(), 1);
    }

    #[test]
    fn winner_rejects_range_failure() {
        assert!(winner(&[], 10).is_err());
        assert!(winner(&[11, 0, 0], 10).is_err());
    }

    #[test]
    fn manifest_has_two_validation_levels() {
        let value = manifest();
        value.validate_core().unwrap();
        assert!(value.validate_full_submission().is_err());
        assert_eq!(value.digest_hex().len(), 64);
    }

    #[test]
    fn undecided_manifest_is_rejected() {
        let mut value = manifest();
        value.encryption_group = "UNDECIDED".to_owned();
        assert!(value.validate_core().is_err());
    }

    #[test]
    fn prekey_session_and_final_context_form_an_acyclic_binding_chain() {
        let manifest_digest = sha256(&manifest().canonical_bytes());
        let election = [1; 32];
        let policy = [2; 32];
        let prekey = derive_prekey_context(election, policy, manifest_digest);
        let session = derive_dkg_session_id(prekey);
        let context = derive_final_context(prekey, [3; 32]);

        assert_ne!(
            prekey,
            derive_prekey_context([9; 32], policy, manifest_digest)
        );
        assert_ne!(
            prekey,
            derive_prekey_context(election, [9; 32], manifest_digest)
        );
        assert_ne!(session, derive_dkg_session_id([9; 32]));
        assert_ne!(context, derive_final_context(prekey, [9; 32]));
    }
}
