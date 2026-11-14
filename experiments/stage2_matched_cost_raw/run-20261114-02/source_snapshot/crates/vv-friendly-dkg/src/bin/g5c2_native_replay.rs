//! Replays an existing public SnarkPack fixture; never sets up or proves.
//! The G5-C2 driver also runs this source against a temporary, logging-only
//! copy of the pinned bellperson dependency to capture actual verifier calls.
use bellperson::groth16::{
    aggregate::{verify_aggregate_proof, AggregateProof, AggregateVersion, VerifierSRS},
    prepare_verifying_key, VerifyingKey,
};
use blstrs::{Bls12, Scalar};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{env, error::Error, fs, io::Cursor, path::PathBuf, process::Command, time::Instant};
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const DOMAIN: &[u8] = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1";
const VERIFY_SEED: u64 = 0x4731_4330_5645_5202;

fn decode(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| Ok(u8::from_str_radix(std::str::from_utf8(bytes)?, 16)?))
        .collect()
}

fn statements(value: &Value) -> Result<Vec<Vec<Scalar>>> {
    let rows = value.as_array().ok_or("missing public statements")?;
    if rows.len() != 2 {
        return Err("G5-C2 requires exactly two statements".into());
    }
    rows.iter()
        .map(|row| {
            let values = row.as_array().ok_or("invalid statement row")?;
            if values.len() != 25 {
                return Err("G5-C2 requires 25 inputs per statement".into());
            }
            values
                .iter()
                .map(|value| {
                    let bytes: [u8; 32] = decode(value.as_str().ok_or("invalid scalar hex")?)?
                        .try_into()
                        .map_err(|_| "scalar must be 32 bytes")?;
                    Option::<Scalar>::from(Scalar::from_bytes_le(&bytes))
                        .ok_or_else(|| "noncanonical public scalar".into())
                })
                .collect()
        })
        .collect()
}

fn transcript(rows: &[Vec<Scalar>]) -> Vec<u8> {
    let mut bytes = DOMAIN.to_vec();
    bytes.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for row in rows {
        bytes.extend_from_slice(&(row.len() as u32).to_be_bytes());
        for scalar in row {
            bytes.extend_from_slice(&scalar.to_bytes_le());
        }
    }
    bytes
}

fn metadata(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err(format!("{program} metadata failed").into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn main() -> Result<()> {
    let mut fixture_path = PathBuf::from("fixtures/snarkpack_g5c1_public_v1.json");
    let mut output_path = PathBuf::from("experiments/g5_c2_raw/native_replay.json");
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match flag.as_str() {
            "--fixture" => fixture_path = value.into(),
            "--output" => output_path = value.into(),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    let fixture_bytes = fs::read(&fixture_path)?;
    let fixture: Value = serde_json::from_slice(&fixture_bytes)?;
    if fixture["schema"] != "VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1" {
        return Err("unexpected fixture schema".into());
    }
    let proof_bytes = decode(
        fixture["aggregate_proof_hex"]
            .as_str()
            .ok_or("missing aggregate")?,
    )?;
    if fixture["aggregate_proof_sha256"] != hex(&sha256(&proof_bytes)) {
        return Err("aggregate digest mismatch".into());
    }
    let mut cursor = Cursor::new(&proof_bytes);
    let proof = AggregateProof::<Bls12>::read(&mut cursor)?;
    if cursor.position() as usize != proof_bytes.len() || proof.tmipp.gipa.nproofs != 2 {
        return Err("invalid B=2 wire boundary".into());
    }
    proof.parsing_check()?;
    let vk_bytes = decode(fixture["groth16_vk_hex"].as_str().ok_or("missing VK")?)?;
    let mut cursor = Cursor::new(&vk_bytes);
    let vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    if cursor.position() as usize != vk_bytes.len() {
        return Err("VK trailing bytes".into());
    }
    let s = &fixture["verifier_srs"];
    let srs = VerifierSRS::<Bls12> {
        n: s["n"].as_u64().ok_or("missing verifier SRS n")? as usize,
        g: serde_json::from_value(s["g"].clone())?,
        h: serde_json::from_value(s["h"].clone())?,
        g_alpha: serde_json::from_value(s["g_alpha"].clone())?,
        g_beta: serde_json::from_value(s["g_beta"].clone())?,
        h_alpha: serde_json::from_value(s["h_alpha"].clone())?,
        h_beta: serde_json::from_value(s["h_beta"].clone())?,
        h_alpha_d: serde_json::from_value(s["h_alpha_d"].clone())?,
    };
    if srs.n != 2 {
        return Err("unexpected verifier SRS size".into());
    }
    let valid_inputs = statements(&fixture["public_inputs_le_hex"])?;
    let wrong_inputs = statements(&fixture["changed_second_nullifier_public_inputs_le_hex"])?;
    let changed: Vec<_> = valid_inputs
        .iter()
        .flatten()
        .zip(wrong_inputs.iter().flatten())
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    if changed != [25 + 7] {
        return Err("negative must change only the second nullifier".into());
    }
    let valid_transcript = transcript(&valid_inputs);
    let wrong_transcript = transcript(&wrong_inputs);
    if fixture["external_transcript_hex"] != hex(&valid_transcript)
        || fixture["changed_external_transcript_hex"] != hex(&wrong_transcript)
    {
        return Err("external statement transcript mismatch".into());
    }
    let prepared = prepare_verifying_key(&vk);
    let started = Instant::now();
    let valid = verify_aggregate_proof(
        &srs,
        &prepared,
        ChaCha20Rng::seed_from_u64(VERIFY_SEED),
        &valid_inputs,
        &proof,
        &valid_transcript,
        AggregateVersion::V2,
    )?;
    let valid_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let wrong_rejected = !verify_aggregate_proof(
        &srs,
        &prepared,
        ChaCha20Rng::seed_from_u64(VERIFY_SEED.wrapping_add(2)),
        &wrong_inputs,
        &proof,
        &wrong_transcript,
        AggregateVersion::V2,
    )?;
    let negative_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !valid || !wrong_rejected {
        return Err("native fixture replay failed".into());
    }
    let result = json!({
        "gate": "G5-C2-native-replay", "status": "PASS_EXISTING_SNARKPACK_NATIVE_REPLAY",
        "git_commit": metadata("git", &["rev-parse", "HEAD"] )?,
        "git_dirty": !metadata("git", &["status", "--porcelain"] )?.is_empty(),
        "rustc_version": metadata("rustc", &["--version"] )?,
        "build_profile": if cfg!(debug_assertions) { "dev" } else { "release-opt-level-1" },
        "fixture": fixture_path.display().to_string(),
        "fixture_sha256": hex(&sha256(&fixture_bytes)),
        "aggregate_proof_sha256": hex(&sha256(&proof_bytes)),
        "aggregate_proof_bytes": proof_bytes.len(), "batch_size": 2,
        "valid_aggregate_verified": valid,
        "changed_second_nullifier_rejected": wrong_rejected,
        "changed_flat_input_indices": changed,
        "valid_verify_ms": valid_ms, "negative_verify_ms": negative_ms,
        "instrumented_reference": env::var_os("VV_G5C2_TRACE_OUT").is_some(),
        "counter_bytes": std::mem::size_of::<usize>(),
        "setup_run": false, "prove_run": false,
        "private_witness_emitted": false, "exact_snarkpack_evm_verified": false,
        "native_verifier_rng": "deterministic ChaCha20 functional harness, not production contract randomness",
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&result)?)?;
    println!("G5-C2 native replay PASS: valid=true changed-second-nullifier-rejected=true; setup/prove not run");
    Ok(())
}
