//! G2-D: D5/W32 D2 prefix, regenerated native B4 aggregate and fresh state proof.
//! One omission negative; independently checked reference fold; no recursive verifier.
#![recursion_limit = "512"]

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, OptimizationGoal};
use sha2::{Digest, Sha256};
#[path = "common/g1e_fold.rs"]
mod independent_fold;
use ark_serialize::CanonicalSerialize;
use bellperson::groth16::{
    aggregate::{aggregate_proofs, setup_fake_srs, verify_aggregate_proof, AggregateVersion},
    prepare_verifying_key as prepare_bellperson_vk, verify_proof as verify_ballot,
    Proof as BellProof, VerifyingKey,
};
use blstrs::{Bls12, Scalar as BellScalar};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    env,
    error::Error,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use vv_friendly_crypto::state_gadget::{
    ballot_field_vector_bytes, ballot_public_inputs, batch_root_v2_for_batch, state_public_inputs,
    StateBallotRecord, StateTransitionCircuitForBatch as StateTransitionCircuit,
    STATE_B4_LAYOUT_VERSION, STATE_PUBLIC_INPUTS,
};
use vv_friendly_crypto::{decode_point, encode_point, Ciphertext};
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const BATCH: usize = 4;
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G2D-STATE-D5-W32-B4-v1";
const TRANSCRIPT_DOMAIN: &[u8] = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1";
const VERIFY_SEED: u64 = 0x4732_442D_5350_0004;
const SETUP_SEED: u64 = 0x4732_442D_5345_5455;
const PROOF_SEED: u64 = 0x4732_442D_5052_4F56;

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
    cs.set_optimization_goal(OptimizationGoal::Constraints);
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
    let synthesis_and_check_ms = started.elapsed().as_secs_f64() * 1000.0;
    let digest_started = Instant::now();
    let matrices = cs.to_matrices().ok_or("missing concrete R1CS matrices")?;
    let mut hasher = Sha256::new();
    hasher.update(b"VV/G2-D/ORDERED-R1CS-ABC/v1");
    for size in [
        matrices.num_instance_variables,
        matrices.num_witness_variables,
        matrices.num_constraints,
    ] {
        hasher.update((size as u64).to_be_bytes());
    }
    for (label, matrix) in [
        (b'A', &matrices.a),
        (b'B', &matrices.b),
        (b'C', &matrices.c),
    ] {
        hasher.update([label]);
        hasher.update((matrix.len() as u64).to_be_bytes());
        for row in matrix {
            hasher.update((row.len() as u64).to_be_bytes());
            for (coefficient, column) in row {
                let mut field = [0u8; 32];
                coefficient.serialize_compressed(&mut field[..])?;
                hasher.update(field);
                hasher.update((*column as u64).to_be_bytes());
            }
        }
    }
    let matrix_sha256 = hex(&hasher.finalize());
    Ok(
        json!({"matrix_sha256": matrix_sha256, "matrix_digest_encoding": "VV/G2-D/ORDERED-R1CS-ABC/v1; counts/row widths/column u64 BE; field coefficients canonical LE32; includes constant instance", "matrix_digest_ms": digest_started.elapsed().as_secs_f64()*1000.0, "satisfied": first_unsatisfied.is_none(), "first_unsatisfied_constraint": first_unsatisfied,
        "constraints": cs.num_constraints(), "public_inputs": cs.num_instance_variables() - 1,
        "witness_variables": cs.num_witness_variables(), "exact_public_input_packing": exact_packing,
        "synthesis_and_check_ms": synthesis_and_check_ms}),
    )
}

fn main() -> Result<()> {
    let git_commit = metadata("git", &["rev-parse", "HEAD"])?;
    let git_dirty = !metadata("git", &["status", "--porcelain"])?.is_empty();
    if git_dirty {
        return Err("G2-D requires a clean source commit before execution".into());
    }
    let run_started = Instant::now();
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let mut manifest_path =
        PathBuf::from("configs/protocol_manifest.JUBJUB-G2D-STATE-D5-W32-B4.json");
    let mut output = PathBuf::from("experiments/g2_d_raw/state_transition_d5w32_b4_groth16.json");
    let mut fixture_output = PathBuf::from("fixtures/jubjub_state_g2d_d5w32_b4_public_v1.json");
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
        return Err("G2-D needs two distinct NEW output paths".into());
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    if manifest["protocol_version"] != PROTOCOL
        || manifest["state_layout"] != STATE_B4_LAYOUT_VERSION
        || manifest["batch_size"] != BATCH
        || manifest["state_public_inputs"] != STATE_PUBLIC_INPUTS
        || manifest["ballot_public_inputs"] != 25
        || manifest["registration_tree_depth"] != 5
        || manifest["registered_weight_bound"] != 32
        || manifest["production_setup"] != false
    {
        return Err("wrong G2-D descriptor".into());
    }
    for (path_key, hash_key) in [
        ("source_fixture", "source_fixture_sha256"),
        ("source_raw", "source_raw_sha256"),
        ("frozen_ballot_manifest", "frozen_ballot_manifest_sha256"),
        ("state_implementation", "state_implementation_sha256"),
    ] {
        let path = manifest[path_key].as_str().ok_or("missing frozen path")?;
        if manifest[hash_key] != hex(&sha256(&fs::read(path)?)) {
            return Err(format!("frozen hash mismatch: {path_key}").into());
        }
    }
    let source_path = manifest["source_fixture"].as_str().ok_or("source path")?;
    let source_bytes = fs::read(source_path)?;
    let mut source: Value = serde_json::from_slice(&source_bytes)?;
    let source_raw_path = manifest["source_raw"].as_str().ok_or("raw path")?;
    let source_raw_bytes = fs::read(source_raw_path)?;
    let source_raw: Value = serde_json::from_slice(&source_raw_bytes)?;
    if source["schema"] != "VV/G1-D2/UNIFORM-DEPTH5-B32-PUBLIC-FIXTURE/v1"
        || source["source_proof_count"] != 32
        || source["merkle_depth"] != 5
        || source["registered_weight_bound"] != 32
        || source["production_setup"] != false
    {
        return Err("requires frozen D2 D5/W32 pool, NOT G1-C2".into());
    }
    let all_rows = source["source_public_inputs_le_hex"]
        .as_array()
        .ok_or("rows")?;
    if all_rows.len() != 32 {
        return Err("expected 32 input rows".into());
    }
    let statements = all_rows[..BATCH]
        .iter()
        .map(|row| {
            let fields = row.as_array().ok_or("row shape")?;
            if fields.len() != 25 {
                return Err("25 fields required".into());
            }
            fields
                .iter()
                .map(|v| {
                    let b: [u8; 32] = decode(v.as_str().ok_or("field hex")?)?
                        .try_into()
                        .map_err(|_| "scalar length")?;
                    Option::<BellScalar>::from(BellScalar::from_bytes_le(&b))
                        .ok_or_else(|| "noncanonical field".into())
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let all_records = source["source_public_records"]
        .as_array()
        .ok_or("records")?;
    if all_records.len() != 32 {
        return Err("expected 32 records".into());
    }
    let selected_records = all_records[..BATCH].to_vec();
    for key in ["address_hex", "nullifier_hex"] {
        if selected_records
            .iter()
            .map(|v| v[key].as_str())
            .collect::<HashSet<_>>()
            .len()
            != BATCH
        {
            return Err("selected voters/nullifiers duplicated".into());
        }
    }
    let all_proofs = source["source_proofs_ark_compressed_hex"]
        .as_array()
        .ok_or("proofs")?;
    if all_proofs.len() != 32
        || all_proofs
            .iter()
            .map(Value::as_str)
            .collect::<HashSet<_>>()
            .len()
            != 32
    {
        return Err("pool proof count/distinctness".into());
    }
    let mut pool = Vec::new();
    for value in all_proofs {
        let bytes = decode(value.as_str().ok_or("proof hex")?)?;
        let mut c = Cursor::new(&bytes);
        let p = BellProof::<Bls12>::read(&mut c)?;
        let mut canonical = Vec::new();
        p.write(&mut canonical)?;
        if c.position() as usize != bytes.len() || bytes != canonical {
            return Err("proof canonical roundtrip".into());
        }
        pool.push(p);
    }
    let selected_proofs_hex = all_proofs[..BATCH].to_vec();
    let vk_bytes = decode(source["groth16_vk_hex"].as_str().ok_or("VK")?)?;
    let mut c = Cursor::new(&vk_bytes);
    let vk = VerifyingKey::<Bls12>::read(&mut c)?;
    let mut canonical = Vec::new();
    vk.write(&mut canonical)?;
    if c.position() as usize != vk_bytes.len()
        || canonical != vk_bytes
        || vk.ic.len() != 26
        || source_raw["ballot_vk_bellperson_wire_sha256"] != hex(&sha256(&vk_bytes))
    {
        return Err("D2 ballot VK mismatch".into());
    }
    let source_prepared = prepare_bellperson_vk(&vk);
    let mut ballot_verify_ms = Vec::new();
    println!(
        "G2-D source={git_commit}; git_dirty=false; reuse FOUR D2 ballots (zero ballot Prove)"
    );
    for (p, x) in pool[..BATCH].iter().zip(&statements) {
        let t = Instant::now();
        if !verify_ballot(&source_prepared, p, x)? {
            return Err("source ballot failed".into());
        }
        ballot_verify_ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let mut transcript = TRANSCRIPT_DOMAIN.to_vec();
    transcript.extend_from_slice(&(BATCH as u32).to_be_bytes());
    for row in &statements {
        transcript.extend_from_slice(&(row.len() as u32).to_be_bytes());
        for scalar in row {
            transcript.extend_from_slice(&scalar.to_bytes_le());
        }
    }
    let t = Instant::now();
    let generic_srs =
        setup_fake_srs::<Bls12, _>(&mut ChaCha20Rng::seed_from_u64(0x4731_4430_5352_5320), 32);
    let srs_rebuild_ms = t.elapsed().as_secs_f64() * 1000.0;
    let shared_srs_sha256 = hex(&generic_srs.hash());
    if source_raw["shared_aggregation_srs_sha256"] != shared_srs_sha256
        || source["shared_aggregation_srs_sha256"] != shared_srs_sha256
    {
        return Err("D2 research SRS mismatch".into());
    }
    let t = Instant::now();
    let (prover_srs, srs) = generic_srs.specialize(BATCH);
    let specialize_ms = t.elapsed().as_secs_f64() * 1000.0;
    let t = Instant::now();
    let aggregate = aggregate_proofs::<Bls12>(
        &prover_srs,
        &transcript,
        &pool[..BATCH],
        AggregateVersion::V2,
    )?;
    let aggregation_ms = t.elapsed().as_secs_f64() * 1000.0;
    let mut aggregate_bytes = Vec::new();
    aggregate.write(&mut aggregate_bytes)?;
    let t = Instant::now();
    let source_aggregate_verified = verify_aggregate_proof(
        &srs,
        &source_prepared,
        ChaCha20Rng::seed_from_u64(VERIFY_SEED),
        &statements,
        &aggregate,
        &transcript,
        AggregateVersion::V2,
    )?;
    let source_aggregate_verify_ms = t.elapsed().as_secs_f64() * 1000.0;
    if !source_aggregate_verified {
        return Err("new B4 aggregate failed".into());
    }
    let srs_json = json!({"n":srs.n,"g":srs.g,"h":srs.h,"g_alpha":srs.g_alpha,"g_beta":srs.g_beta,"h_alpha":srs.h_alpha,"h_beta":srs.h_beta,"h_alpha_d":srs.h_alpha_d});
    source["source_public_records"] = json!(selected_records);
    println!("G2-D [1/6] FOUR source proofs and ONE freshly generated B4 aggregate PASS");
    println!(
        "G2-D [2/6] bind all four records to the exact verified matrix and ordered Merkle root"
    );
    let binding_started = Instant::now();
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
    if first.registration_root != fixed(&source_raw, "registration_root_hex")?
        || first.election_id != fixed(&source_raw, "election_id_hex")?
        || first.final_context != fixed(&source_raw, "final_context_hex")?
        || first.key_set_hash != fixed(&source_raw, "key_core_digest_hex")?
        || first.region != 4
        || encode_point(&first.public_key).as_slice()
            != decode(source_raw["public_key_hex"].as_str().ok_or("D2 key")?)?.as_slice()
    {
        return Err("records differ from frozen D2 election/key/root/region".into());
    }
    let old_state = vec![Ciphertext::zero(); 3];
    let batch_index = 1;
    let batch_root = batch_root_v2_for_batch::<BATCH>(
        &first.election_id,
        &first.final_context,
        first.region,
        batch_index,
        &records,
    )
    .ok_or("invalid B=4 tree")?;
    let common_binding_checks_ms = binding_started.elapsed().as_secs_f64() * 1000.0;
    // Prepare the submitted state outside both timed verification paths.
    let new_state = independent_fold::actual(&old_state, &records).ok_or("invalid full fold")?;
    let direct_started = Instant::now();
    let reference = independent_fold::reference(&old_state, &records).ok_or("reference fold")?;
    let direct_fold_matches = reference == new_state;
    let direct_fold_check_ms = direct_started.elapsed().as_secs_f64() * 1000.0;
    if !direct_fold_matches {
        return Err("independent reference mismatch".into());
    }
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
    println!("G2-D [3/6] check the complete B=4 R1CS; only negative omits ct3 from the NEW STATE");
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
        || negative["matrix_sha256"] != shape["matrix_sha256"]
        || negative["constraints"] != shape["constraints"]
        || negative["witness_variables"] != shape["witness_variables"]
    {
        return Err("omission R1CS was not rejected in the same relation".into());
    }
    println!(
        "  R1CS pass; omitted-third ballot rejected; constraints={}",
        shape["constraints"]
    );

    println!("G2-D [4/6] ONE fresh W32-source B4 state setup; ballot/old state VKs are NOT reused");
    let started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let setup_ms = started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_setup_mib = peak_rss_mib();
    println!("G2-D [5/6] ONE real state proof; verify valid statement and the SAME omission case");
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
    if hex(&sha256(&state_vk_bytes)) == source_raw["ballot_vk_ark_compressed_sha256"] {
        return Err("state VK must differ from ballot VK".into());
    }
    let encode_inputs = |values: &[Fr]| -> Result<Vec<String>> {
        values.iter().map(|v| Ok(hex(&compressed(v)?))).collect()
    };
    let public = json!({"schema": "VV/G2-D/STATE-D5-W32-B4-PUBLIC-FIXTURE/v1", "protocol_version": PROTOCOL,
        "state_layout": STATE_B4_LAYOUT_VERSION, "batch_size": BATCH, "public_inputs": STATE_PUBLIC_INPUTS,
        "source_ballot_proofs_hex":selected_proofs_hex,"source_ballot_vk_hex":hex(&vk_bytes),
        "source_public_inputs_le_hex":statements.iter().map(|row|row.iter().map(|f|hex(&f.to_bytes_le())).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "source_aggregate_hex":hex(&aggregate_bytes),"source_external_transcript_hex":hex(&transcript),"source_verifier_srs":srs_json,
        "matrix_sha256":shape["matrix_sha256"],"source_registration_root_hex":source_raw["registration_root_hex"],
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
        "crates/vv-friendly-dkg/src/bin/g2d_state_d5w32_b4.rs",
        "crates/vv-friendly-dkg/src/bin/common/g1e_fold.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
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
    if fs::read(source_raw_path)? != source_raw_bytes
        || metadata("git", &["rev-parse", "HEAD"])? != git_commit
        || !metadata("git", &["status", "--porcelain"])?.is_empty()
    {
        return Err("inputs/source tree changed before archival".into());
    }
    let result = json!({"gate": "G2-D", "status": "PASS_G2D_D5_W32_B4_REAL_STATE_AND_OMISSION_REJECTED",
        "classification": "D2-depth5-weight32-prefix-bound-to-fresh-B4-state-proof-single-sample-not-recursive-or-settlement",
        "git_commit":git_commit,"git_dirty":git_dirty,"git_clean_before_archival":true,"started_utc":started_utc,
        "rustc_version": metadata(&env::var("RUSTC").unwrap_or_else(|_| "rustc".into()), &["--version"])?,
        "build_profile": if cfg!(debug_assertions) {"dev-unoptimized"} else {"release-opt-level-1"},
        "cpu": metadata("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|_| "UNKNOWN".into()),
        "os": env::consts::OS, "arch": env::consts::ARCH, "source_hashes": source_hashes,
        "experiment_manifest": manifest_path, "manifest_sha256": hex(&sha256(&manifest_bytes)),
        "protocol_version": PROTOCOL, "state_layout": STATE_B4_LAYOUT_VERSION,
        "source_suite":"D5/W32/G1-D2", "merkle_depth":5,"registered_weight_bound":32,"source_voter_indices":[0,1,2,3],
        "source_raw":source_raw_path,"source_raw_sha256":hex(&sha256(&source_raw_bytes)),"frozen_ballot_manifest_sha256":manifest["frozen_ballot_manifest_sha256"],
        "ballot_vk_sha256":source_raw["ballot_vk_ark_compressed_sha256"],"state_vk_distinct_from_ballot_vk":true,
        "source_pool_proofs_decoded":32,"source_ballot_proofs_verified":4,"source_ballot_verify_ms":ballot_verify_ms,
        "new_ballot_proof_count":0,"new_source_aggregate_count":1,"new_source_aggregation_ms":aggregation_ms,
        "shared_aggregation_srs_sha256":shared_srs_sha256,"srs_rebuild_ms_offline":srs_rebuild_ms,"srs_specialize_ms_offline":specialize_ms,
        "source_aggregate_proof_bytes":aggregate_bytes.len(),"source_matrix_bytes":4*25*32,"source_transcript_bytes":transcript.len(),
        "common_binding_checks_ms":common_binding_checks_ms,"independent_direct_fold_matches":direct_fold_matches,"direct_fold_check_ms":direct_fold_check_ms,
        "comparison":{"scope":"native verification components; identical SP/matrix/root/old/new state; common costs counted once in each path; sums are NOT separately timed end-to-end transactions", "shared_snarkpack_verify_ms":source_aggregate_verify_ms,"common_matrix_root_binding_ms":common_binding_checks_ms,"baseline_shared_plus_binding_plus_direct_fold_ms":source_aggregate_verify_ms+common_binding_checks_ms+direct_fold_check_ms,"proposed_shared_plus_binding_plus_state_verify_ms":source_aggregate_verify_ms+common_binding_checks_ms+verify_ms,"state_setup_and_prove_charged_separately":true},
        "state_public_input_bytes":inputs.len()*32, "gas_total":null,"da_bytes":null,"calldata_bytes":null,
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
        "ballot_setup_rerun": false, "ballot_prove_rerun": false, "dkg_rerun": false, "aggregation_srs_rerun": true,
        "production_setup": false, "proving_key_persisted": false, "private_voter_or_trustee_secrets_emitted": false,
        "evm_run": false, "recursive_snarkpack_verification": false, "full_settlement": false,
        "limitations": ["one zero-old-state functional batch, not repeated performance measurement or a multi-batch run",
            "source fake aggregation SRS and deterministic state setup/proof seeds are research-only, not operational privacy or a production ceremony",
            "native host binds the two verifiers through identical ordered D2 matrix/root; SnarkPack is not verified inside state R1CS; W32 and registration freeze are authenticated externally, not a new public range field in the 37-input relation",
            "accepted registration root, spent nullifiers and stale-state checks remain external; no contract settlement or full Proposed Gas"]});
    println!("G2-D [6/6] save NEW public proof/VK and measurements; historical files are not overwritten");
    write_new(&fixture_output, &public)?;
    let mut result = result;
    result["public_verifier_fixture_sha256"] = json!(hex(&sha256(&fs::read(&fixture_output)?)));
    write_new(&output, &result)?;
    println!("G2-D PASS: state proof verified, omitted third ballot rejected; setup={setup_ms:.3}ms prove={prove_ms:.3}ms verify={verify_ms:.3}ms peakRSS={:.3}MiB", peak_rss_mib());
    println!("wrote {}", output.display());
    Ok(())
}
