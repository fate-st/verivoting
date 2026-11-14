//! Converts archived, public G4-C tally-v2 material to EIP-2537 encoding.
//! Performs native verification only: no witness, setup or proof generation.

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde_json::{json, Value};
use std::{
    env, error::Error, fs, io::Write, path::Path, path::PathBuf, process::Command, time::Instant,
};
use vv_friendly_crypto::evm_export::Eip2537PublicGroth16Proof;
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const RELATION: &str = "G4-C tally-v2 fixed-123 full/partial turnout P3";
const RELATION_ID: &str = "VV/JUBJUB/P3/BOUNDED-TALLY/v2";
const PROTOCOL: &str = "VERIVOTING-JUBJUB-PROTOTYPE-G4C-3.0";
const LAYOUT: &str = "VV/JUBJUB/G4-C/INPUT-LAYOUT/v2";
const INPUT_COUNT: usize = 60;
const WINNER_INDEX: usize = 59;

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing string {key}").into())
}

fn decode(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}

fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut remaining = bytes;
    // Validated decoding includes curve/subgroup checks; scalars are never reduced.
    let value = T::deserialize_compressed(&mut remaining)?;
    let mut encoded = Vec::new();
    value.serialize_compressed(&mut encoded)?;
    if !remaining.is_empty() || encoded != bytes {
        return Err("noncanonical or trailing arkworks compressed bytes".into());
    }
    Ok(value)
}

fn inputs(value: &Value) -> Result<Vec<Fr>> {
    let values = value.as_array().ok_or("missing public-input vector")?;
    if values.len() != INPUT_COUNT {
        return Err("G4-C tally-v2 requires exactly 60 public inputs".into());
    }
    values
        .iter()
        .map(|value| {
            let bytes = decode(value.as_str().ok_or("public scalar must be hex")?)?;
            if bytes.len() != 32 {
                return Err("public scalar must be exactly 32 bytes".into());
            }
            canonical(&bytes)
        })
        .collect()
}

fn metadata(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err(format!("{program} metadata failed").into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn write_new_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn main() -> Result<()> {
    let mut fixture_path = PathBuf::from("fixtures/jubjub_g4c_partial_turnout_v2.json");
    let mut source_run_path = PathBuf::from("experiments/g4_c_raw/partial_turnout_groth16.json");
    let mut output_path = PathBuf::from("fixtures/g4c_eip2537_fixture_v2.json");
    let mut report_path = PathBuf::from("experiments/g5_d_raw/public_export.json");
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match flag.as_str() {
            "--fixture" => fixture_path = value.into(),
            "--source-run" => source_run_path = value.into(),
            "--output" => output_path = value.into(),
            "--report" => report_path = value.into(),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    if output_path.exists() || report_path.exists() || output_path == report_path {
        return Err("both export output paths must be distinct and new".into());
    }
    let fixture_bytes = fs::read(&fixture_path)?;
    let source_run_bytes = fs::read(&source_run_path)?;
    let fixture: Value = serde_json::from_slice(&fixture_bytes)?;
    let source_run: Value = serde_json::from_slice(&source_run_bytes)?;
    if source_run["gate"] != "G4-C"
        || source_run["status"] != "PASS_G4C_REAL_GROTH16_PARTIAL_TURNOUT_TALLY_V2"
        || source_run["proof_count"] != 2
        || source_run["setup_count"] != 1
        || source_run["fresh_setup_for_tally_v2"] != true
        || source_run["same_setup_parameters_used"] != true
        || source_run["old_v1_verifying_key_reused"] != false
        || source_run["negative_tests"]["changed_partial_public_winner_rejected"] != true
    {
        return Err("source run is not the passing G4-C fresh tally-v2 experiment".into());
    }
    if Path::new(string(&source_run, "fixture_file")?).canonicalize()?
        != fixture_path.canonicalize()?
    {
        return Err("source run references a different public fixture".into());
    }
    for source in [&fixture, &source_run] {
        if source["protocol_version"] != PROTOCOL
            || source["p3_tally_relation_id"] != RELATION_ID
            || source["layout_version"] != LAYOUT
            || source["selected_trustees"] != json!([1, 2, 3])
            || source["public_second_subset"] != false
            || source["production_setup"] != false
        {
            return Err("source relation, protocol or fixed-123 scope differs".into());
        }
    }
    if fixture["curve"] != "BLS12-381"
        || fixture["public_input_count"] != INPUT_COUNT
        || fixture["winner_public_input_index"] != WINNER_INDEX
        || source_run["negative_tests"]["winner_public_input_index"] != WINNER_INDEX
        || fixture["private_witness_emitted"] != false
        || fixture["proving_key_persisted"] != false
    {
        return Err("unexpected source public-input layout or fixture privacy scope".into());
    }
    let manifest_path = PathBuf::from(string(&source_run, "prototype_manifest")?);
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let manifest_sha = hex(&sha256(&manifest_bytes));
    if fixture["manifest_sha256"] != manifest_sha
        || source_run["manifest_sha256"] != manifest_sha
        || manifest["protocol_version"] != PROTOCOL
        || manifest["p3_tally_relation_id"] != RELATION_ID
        || manifest["input_layout_version"] != LAYOUT
        || manifest["measured_subset"] != json!([1, 2, 3])
    {
        return Err("source manifest binding differs".into());
    }
    let vk_bytes = decode(string(&fixture, "verifying_key_ark_compressed_hex")?)?;
    let vk_sha = hex(&sha256(&vk_bytes));
    if fixture["verifying_key_sha256"] != vk_sha
        || source_run["verifying_key_sha256"] != vk_sha
        || source_run["verifying_key_compressed_bytes"] != vk_bytes.len()
    {
        return Err("archived verifying-key digest or length differs".into());
    }
    let vk: VerifyingKey<Bls12_381> = canonical(&vk_bytes)?;
    if vk.gamma_abc_g1.len() != INPUT_COUNT + 1 {
        return Err("archived VK has the wrong public-input arity".into());
    }
    let prepared = prepare_verifying_key(&vk);
    let public_cases = fixture["cases"].as_array().ok_or("missing fixture cases")?;
    let run_cases = source_run["cases"]
        .as_array()
        .ok_or("missing source-run cases")?;
    if public_cases.len() != 2 || run_cases.len() != 2 {
        return Err("G5-D exports exactly full and partial turnout".into());
    }
    let mut cases = Vec::new();
    let mut verification_results = Vec::new();
    let mut vk_eip2537_hex = None;
    let mut encoding = None;
    for (index, name) in ["full_turnout", "partial_turnout"].iter().enumerate() {
        let public = &public_cases[index];
        let measured = &run_cases[index];
        if public["case_name"] != *name
            || measured["case_name"] != *name
            || measured["proof_verified"] != true
            || measured["verifying_key_sha256"] != vk_sha
            || measured["shape"]["public_inputs"] != INPUT_COUNT
            || public["public_winner"] != 1
            || measured["public_winner"] != 1
        {
            return Err(format!("source case {name} binding or passing status differs").into());
        }
        let proof_bytes = decode(string(public, "proof_ark_compressed_hex")?)?;
        let proof_sha = hex(&sha256(&proof_bytes));
        if public["proof_sha256"] != proof_sha || measured["proof_bytes"] != proof_bytes.len() {
            return Err(format!("source case {name} proof digest or length differs").into());
        }
        let proof: Proof<Bls12_381> = canonical(&proof_bytes)?;
        let public_inputs = inputs(&public["public_inputs_ark_compressed_le_hex"])?;
        if public_inputs[WINNER_INDEX] != Fr::from(1u64)
            || public_inputs[51] != Fr::from(0u64)
            || public_inputs[58] != Fr::from(10u64)
        {
            return Err("source fixed-123 selector, registered bound or winner differs".into());
        }
        let started = Instant::now();
        let valid = Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &public_inputs)?;
        let verify_ms = started.elapsed().as_secs_f64() * 1000.0;
        if !valid {
            return Err(format!("archived {name} proof failed native replay").into());
        }
        let material = Eip2537PublicGroth16Proof::from_proof(&vk, &proof, &public_inputs)?;
        vk_eip2537_hex = Some(material.vk_eip2537_hex);
        encoding = Some(material.encoding);
        let mut case = json!({
            "case_name": name,
            "proof_neg_a_eip2537_hex": material.proof_neg_a_eip2537_hex,
            "public_inputs_be_hex": material.public_inputs_be_hex,
            "proof_ark_sha256": proof_sha,
            "public_winner": 1
        });
        let mut replay = json!({
            "case_name": name, "proof_ark_sha256": proof_sha,
            "proof_ark_compressed_bytes": proof_bytes.len(),
            "native_proof_verified": valid, "native_verify_ms": verify_ms
        });
        if *name == "partial_turnout" {
            let changed = inputs(&public["changed_winner_public_inputs_ark_compressed_le_hex"])?;
            let changed_indices = public_inputs
                .iter()
                .zip(&changed)
                .enumerate()
                .filter_map(|(i, (a, b))| (a != b).then_some(i))
                .collect::<Vec<_>>();
            if changed_indices != [WINNER_INDEX] || changed[WINNER_INDEX] != Fr::from(2u64) {
                return Err("partial negative must change only winner 1 to 2".into());
            }
            let started = Instant::now();
            let rejected = !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed)?;
            let negative_ms = started.elapsed().as_secs_f64() * 1000.0;
            if !rejected {
                return Err("archived partial proof accepted changed winner".into());
            }
            let changed_material = Eip2537PublicGroth16Proof::from_proof(&vk, &proof, &changed)?;
            case["changed_winner_public_inputs_be_hex"] =
                json!(changed_material.public_inputs_be_hex);
            replay["changed_winner_rejected"] = json!(rejected);
            replay["changed_winner_verify_ms"] = json!(negative_ms);
            replay["changed_public_input_indices"] = json!(changed_indices);
        }
        cases.push(case);
        verification_results.push(replay);
    }
    if fs::read(&fixture_path)? != fixture_bytes
        || fs::read(&source_run_path)? != source_run_bytes
        || fs::read(&manifest_path)? != manifest_bytes
    {
        return Err("an archived source changed during public export".into());
    }
    let exported = json!({
        "schema": "VV/G5-D/P3-TALLY-V2-EIP2537-PUBLIC-FIXTURE/v1",
        "relation": RELATION, "encoding": encoding, "curve": "BLS12-381",
        "protocol_version": PROTOCOL, "p3_tally_relation_id": RELATION_ID,
        "layout_version": LAYOUT, "selected_trustees": [1, 2, 3],
        "public_second_subset": false, "public_input_count": INPUT_COUNT,
        "winner_public_input_index": WINNER_INDEX,
        "vk_eip2537_hex": vk_eip2537_hex, "verifying_key_ark_sha256": vk_sha,
        "cases": cases,
        "source_fixture": fixture_path, "source_fixture_sha256": hex(&sha256(&fixture_bytes)),
        "source_run": source_run_path, "source_run_sha256": hex(&sha256(&source_run_bytes)),
        "manifest": manifest_path, "manifest_sha256": manifest_sha,
        "setup_run": false, "proof_generation_run": false,
        "private_witness_emitted": false, "production_setup": false
    });
    let source_paths = [
        "crates/vv-friendly-dkg/src/bin/g5d_p3_v2_export.rs",
        "crates/vv-friendly-crypto/src/evm_export.rs",
        "Cargo.lock",
        "rust-toolchain.toml",
    ];
    let source_hashes = source_paths
        .iter()
        .map(|path| Ok(json!({"path": path, "sha256": hex(&sha256(&fs::read(path)?))})))
        .collect::<Result<Vec<_>>>()?;
    let report = json!({
        "gate": "G5-D-public-export", "status": "PASS_G5D_ARCHIVED_TALLY_V2_PUBLIC_EXPORT_NATIVE_REPLAY",
        "classification": "public-archive-native-verification-and-codec-conversion-only",
        "relation": RELATION, "p3_tally_relation_id": RELATION_ID,
        "git_commit": metadata("git", &["rev-parse", "HEAD"] )?,
        "git_dirty": !metadata("git", &["status", "--porcelain", "--untracked-files=normal"] )?.is_empty(),
        "rustc_version": metadata(&env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()), &["--version"] )?,
        "cargo_version": metadata("cargo", &["--version"] )?,
        "build_profile": if cfg!(debug_assertions) { "dev-unoptimized" } else { "release-opt-level-1" },
        "os": env::consts::OS, "arch": env::consts::ARCH,
        "source_fixture": fixture_path, "source_fixture_sha256": hex(&sha256(&fixture_bytes)),
        "source_run": source_run_path, "source_run_sha256": hex(&sha256(&source_run_bytes)),
        "source_run_status": source_run["status"],
        "manifest": manifest_path, "manifest_sha256": manifest_sha,
        "verifying_key_ark_sha256": vk_sha, "verifying_key_ark_compressed_bytes": vk_bytes.len(),
        "source_case_proof_digest_binding": "fixture proof_sha256 checked; source run provides lengths and VK hashes, not per-case proof hashes",
        "public_input_count": INPUT_COUNT, "winner_public_input_index": WINNER_INDEX,
        "selected_trustees": [1, 2, 3], "public_second_subset": false,
        "cases": verification_results, "native_positive_verification_count": 2,
        "native_negative_verification_count": 1, "setup_run": false,
        "proof_generation_run": false, "setup_count": 0, "proof_generation_count": 0,
        "evm_run": false, "private_witness_emitted": false, "production_setup": false,
        "source_hashes": source_hashes, "output_fixture": output_path,
        "limitations": ["fixed subset 123 only", "not an EVM result", "research setup, not production", "no full settlement or SnarkPack verification claim"]
    });
    write_new_json(&output_path, &exported)?;
    let mut report = report;
    report["output_fixture_sha256"] = json!(hex(&sha256(&fs::read(&output_path)?)));
    write_new_json(&report_path, &report)?;
    println!("G5-D public export PASS: two archived native proofs verified, changed partial winner rejected; setup/prove not run");
    println!("wrote {}", output_path.display());
    println!("wrote {}", report_path.display());
    Ok(())
}
