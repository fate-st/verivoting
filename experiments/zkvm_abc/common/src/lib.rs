//! Shared, deterministic ORIGINAL BLS12-381 ballot verification. No setup/prove.
//! No native FFI, parallel pairing, RNG, file/network access, or host-success input.
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ORIGINAL_VK_SHA256: &str =
    "6d00cab6cd02e9926bd8ef05f8e782fbf1f32b5993c134699155accf7b37aa98";

#[derive(Clone, Serialize, Deserialize)]
pub struct BatchInput {
    pub vk_ark_hex: String,
    pub proofs_ark_hex: Vec<String>,
    pub inputs_le_hex: Vec<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationOutput {
    pub schema: String,
    pub phase: String,
    pub curve: String,
    pub batch_size: u32,
    pub inputs_per_ballot: u32,
    pub vk_sha256: String,
    pub ordered_proofs_sha256: String,
    pub ordered_inputs_sha256: String,
    pub state_binding_implemented: bool,
}

pub fn unhex(text: &str) -> Result<Vec<u8>, String> {
    hex::decode(text.strip_prefix("0x").unwrap_or(text)).map_err(|e| e.to_string())
}

pub fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    value
        .serialize_compressed(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

pub fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T, String> {
    let mut remaining = bytes;
    // Ark Validate::Yes: field/curve/subgroup checks remain active.
    let value = T::deserialize_compressed(&mut remaining).map_err(|e| e.to_string())?;
    if !remaining.is_empty() || compressed(&value)? != bytes {
        return Err("G17_NONCANONICAL_OR_TRAILING_BYTES".into());
    }
    Ok(value)
}

pub fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

pub fn verify_batch(input: &BatchInput) -> Result<VerificationOutput, String> {
    let count = input.proofs_ark_hex.len();
    if !(1..=2).contains(&count) || input.inputs_le_hex.len() != count {
        return Err("G17_Z1_REQUIRES_ONE_OR_TWO_BALLOTS".into());
    }
    let vk_bytes = unhex(&input.vk_ark_hex)?;
    if sha(&vk_bytes) != ORIGINAL_VK_SHA256 {
        return Err("G17_ORIGINAL_VK_REQUIRED".into());
    }
    let vk: VerifyingKey<Bls12_381> = canonical(&vk_bytes)?;
    if vk.gamma_abc_g1.len() != 26 {
        return Err("G17_ORIGINAL_25_INPUT_LAYOUT_REQUIRED".into());
    }
    let prepared = prepare_verifying_key(&vk);
    let mut proof_hash = Sha256::new();
    proof_hash.update(b"VV/G17/Z1/ORDERED-PROOFS/v1");
    proof_hash.update((count as u64).to_le_bytes());
    let mut matrix_hash = Sha256::new();
    matrix_hash.update(b"VV/G17/Z1/ORDERED-INPUTS/v1");
    matrix_hash.update((count as u64).to_le_bytes());
    for (i, (proof_hex, row)) in input
        .proofs_ark_hex
        .iter()
        .zip(&input.inputs_le_hex)
        .enumerate()
    {
        let proof_bytes = unhex(proof_hex)?;
        if proof_bytes.len() != 192 || row.len() != 25 {
            return Err("G17_WRONG_PROOF_OR_ROW_LENGTH".into());
        }
        let proof: Proof<Bls12_381> = canonical(&proof_bytes)?;
        let mut fields = Vec::with_capacity(25);
        matrix_hash.update(25u64.to_le_bytes());
        for encoded in row {
            let bytes = unhex(encoded)?;
            if bytes.len() != 32 {
                return Err("G17_WRONG_SCALAR_LENGTH".into());
            }
            fields.push(canonical::<Fr>(&bytes)?);
            frame(&mut matrix_hash, &bytes);
        }
        let verified = Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &fields)
            .map_err(|e| format!("G17_VERIFY_ERROR_{i}:{e}"))?;
        if !verified {
            return Err(format!("G17_INVALID_GROTH16_{i}"));
        }
        frame(&mut proof_hash, &proof_bytes);
    }
    Ok(VerificationOutput {
        schema: "VV/G17/Z1/VERIFIED-BALLOTS/v1".into(),
        phase: "BALLOT_VALIDITY_ONLY_NOT_STATE_TRANSITION".into(),
        curve: "BLS12-381".into(),
        batch_size: count as u32,
        inputs_per_ballot: 25,
        vk_sha256: sha(&vk_bytes),
        ordered_proofs_sha256: hex::encode(proof_hash.finalize()),
        ordered_inputs_sha256: hex::encode(matrix_hash.finalize()),
        state_binding_implemented: false,
    })
}
