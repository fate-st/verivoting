//! G2-C: reverify archived B=4 ballots, then prove a separately keyed B=4 state fold.
//! One omitted-third-ballot case; no new ballot proofs, SRS, EVM or recursive verification.
#![recursion_limit = "512"]

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_serialize::CanonicalSerialize;
use bellperson::groth16::{
    aggregate::{verify_aggregate_proof, AggregateProof, AggregateVersion, VerifierSRS},
    prepare_verifying_key as prepare_bellperson_vk, VerifyingKey,
};
use blstrs::{Bls12, Scalar as BellScalar};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{
    env,
    error::Error,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use vv_friendly_crypto::state_gadget::{
    ballot_field_vector_bytes, ballot_public_inputs, batch_root_v2_for_batch, fold_state_for_batch,
    state_public_inputs, StateBallotRecord,
    StateTransitionCircuitForBatch as StateTransitionCircuit, STATE_B4_LAYOUT_VERSION,
    STATE_PUBLIC_INPUTS,
};
use vv_friendly_crypto::{decode_point, encode_point, Ciphertext};
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const BATCH: usize = 4;
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G2C-STATE-B4-v1";
const TRANSCRIPT_DOMAIN: &[u8] = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1";
const VERIFY_SEED: u64 = 0x4732_432D_5350_0004;
const SETUP_SEED: u64 = 0x4732_432D_5345_5455;
const PROOF_SEED: u64 = 0x4732_432D_5052_4F56;

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

fn fixed(value: &Value, name: &str) -> Result<[u8; 32]> {
    decode(
        value[name]
            .as_str()
            .ok_or_else(|| format!("missing {name}"))?,
    )?
    .try_into()
    .map_err(|_| format!("invalid {name} length").into())
}

fn record(value: &Value) -> Result<StateBallotRecord> {
    let ciphertexts = value["ciphertexts"]
        .as_array()
        .ok_or("missing ciphertexts")?
        .iter()
        .enumerate()
        .map(|(j, ct)| {
            if ct["candidate_id"] != j + 1 {
                return Err("candidate order mismatch".into());
            }
            Ok(Ciphertext {
                c0: decode_point(&fixed(ct, "c0_hex")?, false)?,
                c1: decode_point(&fixed(ct, "c1_hex")?, true)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if ciphertexts.len() != 3 {
        return Err("requires three ciphertext coordinates".into());
    }
    Ok(StateBallotRecord {
        election_id: fixed(value, "election_id_hex")?,
        final_context: fixed(value, "final_context_hex")?,
        region: value["region"].as_u64().ok_or("missing region")?,
        registration_root: fixed(value, "registration_root_hex")?,
        nullifier: fixed(value, "nullifier_hex")?,
        key_set_hash: fixed(value, "key_set_hash_hex")?,
        public_key: decode_point(&fixed(value, "public_key_hex")?, false)?,
        ciphertexts,
    })
}

fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    #[cfg(target_os = "macos")]
    let divisor = 1024.0 * 1024.0;
    #[cfg(not(target_os = "macos"))]
    let divisor = 1024.0;
    usage.ru_maxrss as f64 / divisor
}

fn metadata(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err(format!("{program} metadata failed").into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn write_new(path: &Path, value: &Value) -> Result<()> {
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

fn ciphertexts(values: &[Ciphertext]) -> Vec<Value> {
    values
        .iter()
        .enumerate()
        .map(|(j, ct)| {
            json!({"candidate_id": j + 1,
        "c0_hex": hex(&encode_point(&ct.c0)), "c1_hex": hex(&encode_point(&ct.c1))})
        })
        .collect()
}

fn measure(circuit: &StateTransitionCircuit<BATCH>) -> Result<Value> {
    let started = Instant::now();
    let inputs = state_public_inputs(circuit).ok_or("invalid state public packing")?;
    let cs = ConstraintSystem::<Fr>::new_ref();
    circuit.clone().generate_constraints(cs.clone())?;
    cs.finalize();
    let first_unsatisfied = cs.which_is_unsatisfied()?;
    let exact_packing = cs
        .borrow()
        .ok_or("missing constraint system")?
        .instance_assignment[1..]
        == inputs;
    if !exact_packing {
        return Err("actual state instance packing differs".into());
    }
    Ok(
        json!({"satisfied": first_unsatisfied.is_none(), "first_unsatisfied_constraint": first_unsatisfied,
        "constraints": cs.num_constraints(), "public_inputs": cs.num_instance_variables() - 1,
        "witness_variables": cs.num_witness_variables(), "exact_public_input_packing": exact_packing,
        "synthesis_and_check_ms": started.elapsed().as_secs_f64() * 1000.0}),
    )
}

fn main() -> Result<()> {
    let run_started = Instant::now();
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-G2C-STATE-B4.json");
    let mut output = PathBuf::from("experiments/g2_c_raw/state_transition_b4_groth16.json");
    let mut fixture_output = PathBuf::from("fixtures/jubjub_state_g2c_b4_public_v1.json");
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match flag.as_str() {
            "--manifest" => manifest_path = value.into(),
            "--output" => output = value.into(),
            "--fixture-output" => fixture_output = value.into(),
            _ => return Err(format!("unknown {flag}").into()),
        }
    }
    if output.exists() || fixture_output.exists() || output == fixture_output {
        return Err("G2-C requires two distinct new output paths".into());
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    if manifest["protocol_version"] != PROTOCOL
        || manifest["state_layout"] != STATE_B4_LAYOUT_VERSION
        || manifest["batch_size"] != BATCH
        || manifest["state_public_inputs"] != STATE_PUBLIC_INPUTS
        || manifest["ballot_public_inputs"] != 25
        || manifest["candidate_count"] != 3
        || manifest["production_setup"] != false
    {
        return Err("descriptor does not describe this G2-C B=4 relation".into());
    }
    let source_path = manifest["source_fixture"]
        .as_str()
        .ok_or("missing source fixture")?;
    let source_bytes = fs::read(source_path)?;
    if manifest["source_fixture_sha256"] != hex(&sha256(&source_bytes)) {
        return Err("source fixture digest mismatch".into());
    }
    let source: Value = serde_json::from_slice(&source_bytes)?;
    if source["schema"] != "VV/G1-C2/SNARKPACK-B4-PUBLIC-FIXTURE/v1"
        || source["batch_size"] != BATCH
        || source["aggregate_version"] != "V2"
        || source["production_setup"] != false
    {
        return Err("requires the archived G1-C2 B=4 experimental fixture".into());
    }
    println!(
        "G2-C [1/6] decode and reverify the saved genuine B=4 aggregate (no setup/prove rerun)"
    );
    let aggregate_bytes = decode(
        source["aggregate_proof_hex"]
            .as_str()
            .ok_or("missing aggregate")?,
    )?;
    if source["aggregate_proof_sha256"] != hex(&sha256(&aggregate_bytes)) {
        return Err("aggregate hash mismatch".into());
    }
    let mut cursor = Cursor::new(&aggregate_bytes);
    let aggregate = AggregateProof::<Bls12>::read(&mut cursor)?;
    if cursor.position() as usize != aggregate_bytes.len()
        || aggregate.tmipp.gipa.nproofs != BATCH as u32
    {
        return Err("invalid B=4 aggregate wire boundary".into());
    }
    aggregate.parsing_check()?;
    let vk_bytes = decode(
        source["groth16_vk_hex"]
            .as_str()
            .ok_or("missing source VK")?,
    )?;
    let mut cursor = Cursor::new(&vk_bytes);
    let vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    if cursor.position() as usize != vk_bytes.len() || vk.ic.len() != 26 {
        return Err("invalid source VK".into());
    }
    let s = &source["verifier_srs"];
    let srs = VerifierSRS::<Bls12> {
        n: s["n"].as_u64().ok_or("missing SRS size")? as usize,
        g: serde_json::from_value(s["g"].clone())?,
        h: serde_json::from_value(s["h"].clone())?,
        g_alpha: serde_json::from_value(s["g_alpha"].clone())?,
        g_beta: serde_json::from_value(s["g_beta"].clone())?,
        h_alpha: serde_json::from_value(s["h_alpha"].clone())?,
        h_beta: serde_json::from_value(s["h_beta"].clone())?,
        h_alpha_d: serde_json::from_value(s["h_alpha_d"].clone())?,
    };
    if srs.n != BATCH {
        return Err("wrong source SRS size".into());
    }
    let rows = source["public_inputs_le_hex"]
        .as_array()
        .ok_or("missing source matrix")?;
    if rows.len() != BATCH {
        return Err("requires four ordered statements".into());
    }
    let statements = rows
        .iter()
        .map(|row| {
            let fields = row.as_array().ok_or("invalid row")?;
            if fields.len() != 25 {
                return Err("requires 25 fields per ballot".into());
            }
            fields
                .iter()
                .map(|field| {
                    let bytes: [u8; 32] = decode(field.as_str().ok_or("invalid field")?)?
                        .try_into()
                        .map_err(|_| "invalid field size")?;
                    Option::<BellScalar>::from(BellScalar::from_bytes_le(&bytes))
                        .ok_or_else(|| "noncanonical field".into())
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let mut transcript = TRANSCRIPT_DOMAIN.to_vec();
    transcript.extend_from_slice(&(BATCH as u32).to_be_bytes());
    for row in &statements {
        transcript.extend_from_slice(&(row.len() as u32).to_be_bytes());
        for field in row {
            transcript.extend_from_slice(&field.to_bytes_le());
        }
    }
    if source["external_transcript_hex"] != hex(&transcript) {
        return Err("source transcript mismatch".into());
    }
    let source_prepared = prepare_bellperson_vk(&vk);
    let started = Instant::now();
    let source_aggregate_verified = verify_aggregate_proof(
        &srs,
        &source_prepared,
        ChaCha20Rng::seed_from_u64(VERIFY_SEED),
        &statements,
        &aggregate,
        &transcript,
        AggregateVersion::V2,
    )?;
    let source_aggregate_verify_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !source_aggregate_verified {
        return Err("saved B=4 aggregate failed".into());
    }

    println!(
        "G2-C [2/6] bind all four records to the exact verified matrix and ordered Merkle root"
    );
    let record_values = source["source_public_records"]
        .as_array()
        .ok_or("missing source records")?;
    if record_values.len() != BATCH {
        return Err("requires four source records".into());
    }
    let records = record_values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            if value["voter_index"] != i {
                return Err("source voter order mismatch".into());
            }
            record(value)
        })
        .collect::<Result<Vec<_>>>()?;
    let first = &records[0];
    let mut field_vectors = Vec::new();
    for (record, row) in records.iter().zip(&statements) {
        if record.election_id != first.election_id
            || record.final_context != first.final_context
            || record.region != first.region
            || record.registration_root != first.registration_root
            || record.key_set_hash != first.key_set_hash
            || record.public_key != first.public_key
        {
            return Err("source context/key/region differs".into());
        }
        let packed = ballot_public_inputs(record).ok_or("cannot pack ballot record")?;
        if packed.len() != row.len() {
            return Err("record field width differs".into());
        }
        let mut expected_be = Vec::new();
        for (field, expected) in packed.iter().zip(row) {
            let bytes = compressed(field)?;
            if bytes != expected.to_bytes_le() {
                return Err("record differs from verified source matrix".into());
            }
            expected_be.extend(bytes.into_iter().rev());
        }
        let encoded = ballot_field_vector_bytes(record).ok_or("invalid leaf field encoding")?;
        if encoded != expected_be {
            return Err("LE-to-BE leaf field conversion differs".into());
        }
        field_vectors.push(hex(&encoded));
    }
    let old_state = vec![Ciphertext::zero(); 3];
    let new_state =
        fold_state_for_batch::<BATCH>(&old_state, &records).ok_or("invalid full fold")?;
    let batch_index = 1;
    let batch_root = batch_root_v2_for_batch::<BATCH>(
        &first.election_id,
        &first.final_context,
        first.region,
        batch_index,
        &records,
    )
    .ok_or("invalid B=4 tree")?;
    let circuit = StateTransitionCircuit::<BATCH> {
        election_id: first.election_id,
        final_context: first.final_context,
        region: first.region,
        batch_index,
        batch_size: BATCH as u64,
        batch_root,
        key_set_hash: first.key_set_hash,
        public_key: first.public_key,
        old_state,
        new_state,
        records,
    };
    println!("G2-C [3/6] check the complete B=4 R1CS; only negative omits ct3 from the NEW STATE");
    let shape = measure(&circuit)?;
    if shape["satisfied"] != true || shape["public_inputs"] != STATE_PUBLIC_INPUTS {
        return Err("valid B=4 relation failed".into());
    }
    let mut omitted = circuit.clone();
    omitted.new_state = omitted.old_state.clone();
    for (i, record) in omitted.records.iter().enumerate() {
        if i != 2 {
            for (sum, ct) in omitted.new_state.iter_mut().zip(&record.ciphertexts) {
                *sum = sum.add(ct);
            }
        }
    }
    let inputs = state_public_inputs(&circuit).ok_or("invalid state packing")?;
    let omitted_inputs = state_public_inputs(&omitted).ok_or("invalid omitted-state packing")?;
    let changed_fields = inputs
        .iter()
        .zip(&omitted_inputs)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect::<Vec<_>>();
    if changed_fields.is_empty() || inputs[..25] != omitted_inputs[..25] {
        return Err("omission must change only the new state".into());
    }
    let omitted_root = batch_root_v2_for_batch::<BATCH>(
        &omitted.election_id,
        &omitted.final_context,
        omitted.region,
        omitted.batch_index,
        &omitted.records,
    )
    .ok_or("invalid unchanged records")?;
    if omitted_root != batch_root {
        return Err("negative changed the committed source records".into());
    }
    let negative = measure(&omitted)?;
    if negative["satisfied"] != false
        || negative["constraints"] != shape["constraints"]
        || negative["witness_variables"] != shape["witness_variables"]
    {
        return Err("omission R1CS was not rejected in the same relation".into());
    }
    println!(
        "  R1CS pass; omitted-third ballot rejected; constraints={}",
        shape["constraints"]
    );

    println!("G2-C [4/6] ONE fresh B=4 state setup (old B=2 VK is not reused)");
    let started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let setup_ms = started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_setup_mib = peak_rss_mib();
    println!("G2-C [5/6] ONE real state proof; verify valid statement and the SAME omission case");
    let started = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit.clone(),
        &parameters,
        &mut ChaCha20Rng::seed_from_u64(PROOF_SEED),
    )?;
    let prove_ms = started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_prove_mib = peak_rss_mib();
    let started = Instant::now();
    let prepared = prepare_verifying_key(&parameters.vk);
    let vk_preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let state_proof_verified = Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &inputs)?;
    let verify_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let omitted_state_proof_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &omitted_inputs)?;
    let omitted_verify_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !state_proof_verified || !omitted_state_proof_rejected {
        return Err("state Groth16 acceptance failed".into());
    }
    let proof_bytes = compressed(&proof)?;
    let state_vk_bytes = compressed(&parameters.vk)?;
    let encode_inputs = |values: &[Fr]| -> Result<Vec<String>> {
        values.iter().map(|v| Ok(hex(&compressed(v)?))).collect()
    };
    let public = json!({"schema": "VV/G2-C/STATE-B4-PUBLIC-FIXTURE/v1", "protocol_version": PROTOCOL,
        "state_layout": STATE_B4_LAYOUT_VERSION, "batch_size": BATCH, "public_inputs": STATE_PUBLIC_INPUTS,
        "proof_hex": hex(&proof_bytes), "proof_encoding": "arkworks canonical compressed, BLS12-381",
        "proof_sha256": hex(&sha256(&proof_bytes)), "groth16_vk_hex": hex(&state_vk_bytes),
        "groth16_vk_encoding": "arkworks canonical compressed, BLS12-381",
        "public_inputs_le_hex": encode_inputs(&inputs)?, "omitted_third_public_inputs_le_hex": encode_inputs(&omitted_inputs)?,
        "source_fixture": source_path, "source_fixture_sha256": hex(&sha256(&source_bytes)),
        "source_aggregate_sha256": hex(&sha256(&aggregate_bytes)), "batch_root_hex": hex(&batch_root),
        "ordered_leaf_field_vectors_be_hex": field_vectors, "source_public_records": source["source_public_records"],
        "old_state": ciphertexts(&circuit.old_state), "new_state": ciphertexts(&circuit.new_state),
        "omitted_third_new_state": ciphertexts(&omitted.new_state),
        "production_setup": false, "proving_key_persisted": false, "private_voter_or_trustee_secrets_emitted": false});
    let source_paths = [
        "crates/vv-friendly-dkg/src/bin/g2c_state_batch4.rs",
        "crates/vv-friendly-crypto/src/state_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h3_gadget.rs",
        "crates/vv-friendly-crypto/src/lib.rs",
        "Cargo.lock",
    ];
    let source_hashes = source_paths
        .iter()
        .map(|path| Ok(json!({"path": path, "sha256": hex(&sha256(&fs::read(path)?))})))
        .collect::<Result<Vec<_>>>()?;
    if fs::read(&manifest_path)? != manifest_bytes || fs::read(source_path)? != source_bytes {
        return Err("inputs changed during run".into());
    }
    let result = json!({"gate": "G2-C", "status": "PASS_G2C_REAL_GROTH16_B4_STATE_AND_OMITTED_THIRD_BALLOT_REJECTED",
        "classification": "archived-native-b4-snarkpack-bound-to-separately-verified-state-proof-not-recursive-or-settlement",
        "git_commit": metadata("git", &["rev-parse", "HEAD"])?, "git_dirty": !metadata("git", &["status", "--porcelain"])?.is_empty(),
        "rustc_version": metadata(&env::var("RUSTC").unwrap_or_else(|_| "rustc".into()), &["--version"])?,
        "build_profile": if cfg!(debug_assertions) {"dev-unoptimized"} else {"release-opt-level-1"},
        "cpu": metadata("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|_| "UNKNOWN".into()),
        "os": env::consts::OS, "arch": env::consts::ARCH, "source_hashes": source_hashes,
        "experiment_manifest": manifest_path, "manifest_sha256": hex(&sha256(&manifest_bytes)),
        "protocol_version": PROTOCOL, "state_layout": STATE_B4_LAYOUT_VERSION,
        "source_fixture": source_path, "source_fixture_sha256": hex(&sha256(&source_bytes)),
        "source_aggregate_sha256": hex(&sha256(&aggregate_bytes)), "source_aggregate_verified": source_aggregate_verified,
        "source_aggregate_verify_ms": source_aggregate_verify_ms, "source_aggregate_verification_count": 1,
        "source_matrix_matches_state_records": true, "leaf_be_values_match_verified_le_matrix": true,
        "ordered_leaf_field_vectors_be_hex": field_vectors, "state_and_source_context_key_region_equal": true,
        "batch_size": BATCH, "batch_index": batch_index, "candidate_count": 3, "region": circuit.region,
        "batch_root_hex": hex(&batch_root), "batch_leaf_hashes": BATCH, "batch_internal_hashes": BATCH - 1,
        "group_additions": BATCH * 3 * 2, "shape": shape, "valid_state_relation_satisfied": true,
        "omitted_third_ballot": {"voter_index": 2, "records_unchanged": true, "batch_root_unchanged": true,
            "source_aggregate_unchanged": true, "only_new_state_changed": true,
            "changed_state_public_field_indices": changed_fields, "shape": negative,
            "r1cs_rejected": true, "original_proof_with_omitted_new_state_rejected": omitted_state_proof_rejected,
            "verify_ms": omitted_verify_ms, "invalid_witness_proving_attempted": false},
        "state_setup_count": 1, "state_proof_count": 1, "negative_case_count": 1,
        "setup_seed": SETUP_SEED, "proof_seed": PROOF_SEED, "setup_ms": setup_ms, "prove_ms": prove_ms,
        "vk_preparation_ms": vk_preparation_ms, "verify_ms": verify_ms, "state_proof_verified": state_proof_verified,
        "proof_bytes": proof_bytes.len(), "proof_sha256": hex(&sha256(&proof_bytes)),
        "verifying_key_compressed_bytes": state_vk_bytes.len(), "verifying_key_sha256": hex(&sha256(&state_vk_bytes)),
        "proving_key_compressed_bytes": parameters.compressed_size(),
        "peak_rss_after_setup_mib": peak_rss_after_setup_mib, "peak_rss_after_prove_mib": peak_rss_after_prove_mib,
        "peak_rss_mib": peak_rss_mib(), "total_run_ms_excluding_compilation": run_started.elapsed().as_secs_f64() * 1000.0,
        "old_state": ciphertexts(&circuit.old_state), "new_state": ciphertexts(&circuit.new_state),
        "omitted_third_new_state": ciphertexts(&omitted.new_state), "public_verifier_fixture": fixture_output,
        "ballot_setup_rerun": false, "ballot_prove_rerun": false, "dkg_rerun": false, "aggregation_srs_rerun": false,
        "production_setup": false, "proving_key_persisted": false, "private_voter_or_trustee_secrets_emitted": false,
        "evm_run": false, "recursive_snarkpack_verification": false, "full_settlement": false,
        "limitations": ["one zero-old-state functional batch, not repeated performance measurement or a multi-batch run",
            "source fake aggregation SRS and deterministic state setup/proof seeds are research-only, not operational privacy or a production ceremony",
            "native host binds the two verifiers through identical ordered matrix/root; SnarkPack is not verified inside state R1CS",
            "accepted registration root, spent nullifiers and stale-state checks remain external; no contract settlement or full Proposed Gas"]});
    println!("G2-C [6/6] save NEW public proof/VK and measurements; historical files are not overwritten");
    write_new(&fixture_output, &public)?;
    let mut result = result;
    result["public_verifier_fixture_sha256"] = json!(hex(&sha256(&fs::read(&fixture_output)?)));
    write_new(&output, &result)?;
    println!("G2-C PASS: state proof verified, omitted third ballot rejected; setup={setup_ms:.3}ms prove={prove_ms:.3}ms verify={verify_ms:.3}ms peakRSS={:.3}MiB", peak_rss_mib());
    println!("wrote {}", output.display());
    Ok(())
}
