//! G2-E: two consecutive B4 batches with a native atomic checkpoint.
//! Reuse the exact G2-D first proof after VK/statement checks; prove only batch two.
#![recursion_limit = "512"]

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, OptimizationGoal};
use sha2::{Digest, Sha256};
#[path = "common/g1e_fold.rs"]
mod independent_fold;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::{
    aggregate::{
        aggregate_proofs, setup_fake_srs, verify_aggregate_proof, AggregateProof, AggregateVersion,
        VerifierSRS,
    },
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
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G2E-MULTIBATCH-B4-v1";
const TRANSCRIPT_DOMAIN: &[u8] = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1";
const VERIFY_SEED: u64 = 0x4732_442D_5350_0004;
const SETUP_SEED: u64 = 0x4732_442D_5345_5455;
const PROOF_SEED: u64 = 0x4732_452D_5052_0002;
#[path = "common/g2e_receiver.rs"]
mod receiver;
use receiver::Receiver;

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

fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut rest = bytes;
    let value = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || compressed(&value)? != bytes {
        return Err("noncanonical Ark bytes".into());
    }
    Ok(value)
}

fn rows(values: &[Value]) -> Result<Vec<Vec<BellScalar>>> {
    values
        .iter()
        .map(|row| {
            let fields = row.as_array().ok_or("matrix row")?;
            if fields.len() != 25 {
                return Err("25 fields required".into());
            }
            fields
                .iter()
                .map(|v| {
                    let b: [u8; 32] = decode(v.as_str().ok_or("field hex")?)?
                        .try_into()
                        .map_err(|_| "field width")?;
                    Option::<BellScalar>::from(BellScalar::from_bytes_le(&b))
                        .ok_or_else(|| "noncanonical field".into())
                })
                .collect()
        })
        .collect()
}

fn transcript(rows: &[Vec<BellScalar>]) -> Vec<u8> {
    let mut bytes = TRANSCRIPT_DOMAIN.to_vec();
    bytes.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for row in rows {
        bytes.extend_from_slice(&(row.len() as u32).to_be_bytes());
        for f in row {
            bytes.extend_from_slice(&f.to_bytes_le());
        }
    }
    bytes
}

fn input_hex(inputs: &[Fr]) -> Result<Vec<String>> {
    inputs.iter().map(|v| Ok(hex(&compressed(v)?))).collect()
}
fn states(values: &Value) -> Result<Vec<Ciphertext>> {
    let values = values.as_array().ok_or("state vector")?;
    if values.len() != 3 {
        return Err("state width".into());
    }
    values
        .iter()
        .enumerate()
        .map(|(j, c)| {
            if c["candidate_id"] != j + 1 {
                return Err("state candidate order".into());
            }
            Ok(Ciphertext {
                c0: decode_point(&fixed(c, "c0_hex")?, true)?,
                c1: decode_point(&fixed(c, "c1_hex")?, true)?,
            })
        })
        .collect()
}

fn srs_from_json(s: &Value) -> Result<VerifierSRS<Bls12>> {
    let srs = VerifierSRS {
        n: s["n"].as_u64().ok_or("SRS n")? as usize,
        g: serde_json::from_value(s["g"].clone())?,
        h: serde_json::from_value(s["h"].clone())?,
        g_alpha: serde_json::from_value(s["g_alpha"].clone())?,
        g_beta: serde_json::from_value(s["g_beta"].clone())?,
        h_alpha: serde_json::from_value(s["h_alpha"].clone())?,
        h_beta: serde_json::from_value(s["h_beta"].clone())?,
        h_alpha_d: serde_json::from_value(s["h_alpha_d"].clone())?,
    };
    if srs.n != BATCH {
        return Err("not a B4 SRS".into());
    }
    Ok(srs)
}

fn aggregate_from_hex(value: &Value) -> Result<AggregateProof<Bls12>> {
    let bytes = decode(value.as_str().ok_or("aggregate hex")?)?;
    let mut c = Cursor::new(&bytes);
    let p = AggregateProof::<Bls12>::read(&mut c)?;
    p.parsing_check()?;
    let mut roundtrip = Vec::new();
    p.write(&mut roundtrip)?;
    if c.position() as usize != bytes.len()
        || roundtrip != bytes
        || p.tmipp.gipa.nproofs != BATCH as u32
    {
        return Err("aggregate wire boundary".into());
    }
    Ok(p)
}

struct Source {
    descriptor: Value,
    baseline: Value,
    first: Value,
    first_raw: Value,
    d2: Value,
    d2_raw: Value,
    pins: Vec<Value>,
}

fn load_source() -> Result<Source> {
    let path = "configs/protocol_manifest.JUBJUB-G2E-MULTIBATCH-B4.json";
    let bytes = fs::read(path)?;
    let descriptor: Value = serde_json::from_slice(&bytes)?;
    if descriptor["protocol_version"] != PROTOCOL
        || descriptor["batch_count"] != 2
        || descriptor["batch_size"] != BATCH
        || descriptor["source_voter_indices"] != json!([[0, 1, 2, 3], [4, 5, 6, 7]])
        || descriptor["state_public_inputs"] != STATE_PUBLIC_INPUTS
        || descriptor["state_layout"] != STATE_B4_LAYOUT_VERSION
        || descriptor["production_setup"] != false
    {
        return Err("wrong multi-batch descriptor".into());
    }
    let mut pins = vec![json!({"path":path,"sha256":hex(&sha256(&bytes))})];
    let mut pinned_json = |d: &Value, key: &str| -> Result<Value> {
        let p = d[key].as_str().ok_or("pinned path")?;
        let b = fs::read(p)?;
        let h = hex(&sha256(&b));
        if d[format!("{key}_sha256")] != h {
            return Err(format!("pinned artifact mismatch: {p}").into());
        }
        pins.push(json!({"path":p,"sha256":h}));
        Ok(serde_json::from_slice(&b)?)
    };
    let baseline = pinned_json(&descriptor, "baseline_descriptor")?;
    let first = pinned_json(&descriptor, "first_fixture")?;
    let first_raw = pinned_json(&descriptor, "first_raw")?;
    let d2 = pinned_json(&baseline, "source_fixture")?;
    let d2_raw = pinned_json(&baseline, "source_raw")?;
    for (key, hkey) in [
        ("frozen_ballot_manifest", "frozen_ballot_manifest_sha256"),
        ("state_implementation", "state_implementation_sha256"),
    ] {
        let p = baseline[key].as_str().ok_or("implementation path")?;
        let h = hex(&sha256(&fs::read(p)?));
        if baseline[hkey] != h {
            return Err("frozen implementation changed".into());
        }
        pins.push(json!({"path":p,"sha256":h}));
    }
    if d2["schema"] != "VV/G1-D2/UNIFORM-DEPTH5-B32-PUBLIC-FIXTURE/v1"
        || d2["source_proof_count"] != 32
        || d2["merkle_depth"] != 5
        || d2["registered_weight_bound"] != 32
        || d2["production_setup"] != false
        || first_raw["state_proof_verified"] != true
        || first["public_inputs"] != STATE_PUBLIC_INPUTS
        || first["matrix_sha256"] != first_raw["shape"]["matrix_sha256"]
    {
        return Err("wrong source scope".into());
    }
    Ok(Source {
        descriptor,
        baseline,
        first,
        first_raw,
        d2,
        d2_raw,
        pins,
    })
}

fn circuits(source: &Source) -> Result<Vec<StateTransitionCircuit<BATCH>>> {
    let values = source.d2["source_public_records"]
        .as_array()
        .ok_or("source records")?;
    if values.len() != 32 {
        return Err("D2 records count".into());
    }
    let mut selected = Vec::new();
    let mut addresses = HashSet::new();
    let mut nfs = HashSet::new();
    for (i, value) in values[..8].iter().enumerate() {
        if value["voter_index"] != i
            || !addresses.insert(value["address_hex"].as_str().ok_or("address")?)
        {
            return Err("voter order/distinctness".into());
        }
        let r = record(value)?;
        if !nfs.insert(r.nullifier) {
            return Err("duplicate source nullifier".into());
        }
        selected.push(r);
    }
    let first = &selected[0];
    if first.election_id != fixed(&source.d2_raw, "election_id_hex")?
        || first.final_context != fixed(&source.d2_raw, "final_context_hex")?
        || first.registration_root != fixed(&source.d2_raw, "registration_root_hex")?
        || first.key_set_hash != fixed(&source.d2_raw, "key_core_digest_hex")?
        || first.region != 4
        || encode_point(&first.public_key) != fixed(&source.d2_raw, "public_key_hex")?
    {
        return Err("frozen election/key/root mismatch".into());
    }
    let all_rows = source.d2["source_public_inputs_le_hex"]
        .as_array()
        .ok_or("source rows")?;
    if all_rows.len() != 32 {
        return Err("D2 rows count".into());
    }
    let matrix = rows(&all_rows[..8])?;
    for (r, row) in selected.iter().zip(&matrix) {
        if r.election_id != first.election_id
            || r.final_context != first.final_context
            || r.registration_root != first.registration_root
            || r.key_set_hash != first.key_set_hash
            || r.public_key != first.public_key
            || r.region != first.region
        {
            return Err("source binding mismatch".into());
        }
        let packed = ballot_public_inputs(r).ok_or("pack record")?;
        let mut be = Vec::new();
        for (actual, expected) in packed.iter().zip(row) {
            let b = compressed(actual)?;
            if b != expected.to_bytes_le() {
                return Err("record/matrix mismatch".into());
            }
            be.extend(b.into_iter().rev());
        }
        if packed.len() != 25 || be != ballot_field_vector_bytes(r).ok_or("leaf encoding")? {
            return Err("leaf packing mismatch".into());
        }
    }
    let mut old = vec![Ciphertext::zero(); 3];
    let mut out = Vec::new();
    for (i, chunk) in selected.as_chunks::<BATCH>().0.iter().enumerate() {
        let new = independent_fold::actual(&old, chunk).ok_or("fold")?;
        if independent_fold::reference(&old, chunk).ok_or("reference")? != new {
            return Err("independent fold mismatch".into());
        }
        let c = StateTransitionCircuit {
            election_id: first.election_id,
            final_context: first.final_context,
            region: first.region,
            batch_index: (i + 1) as u64,
            batch_size: BATCH as u64,
            batch_root: batch_root_v2_for_batch::<BATCH>(
                &first.election_id,
                &first.final_context,
                first.region,
                (i + 1) as u64,
                chunk,
            )
            .ok_or("batch root")?,
            key_set_hash: first.key_set_hash,
            public_key: first.public_key,
            old_state: old,
            new_state: new.clone(),
            records: chunk.to_vec(),
        };
        old = new;
        out.push(c);
    }
    if out[1].old_state == vec![Ciphertext::zero(); 3]
        || out[1].old_state != out[0].new_state
        || independent_fold::reference(&vec![Ciphertext::zero(); 3], &selected)
            .ok_or("flat reference")?
            != out[1].new_state
    {
        return Err("continuous/flat sum mismatch".into());
    }
    if input_hex(&state_public_inputs(&out[0]).ok_or("first inputs")?)?
        != serde_json::from_value::<Vec<String>>(source.first["public_inputs_le_hex"].clone())?
        || source.first["source_public_records"] != json!(&values[..4])
        || source.first["source_public_inputs_le_hex"] != json!(&all_rows[..4])
    {
        return Err("archived first statement differs".into());
    }
    Ok(out)
}

fn source_hashes() -> Result<Vec<Value>> {
    [
        "crates/vv-friendly-dkg/src/bin/g2e_multibatch_state.rs",
        "crates/vv-friendly-dkg/src/bin/common/g2e_receiver.rs",
        "crates/vv-friendly-dkg/src/bin/common/g1e_fold.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "crates/vv-friendly-crypto/src/state_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h3_gadget.rs",
        "Cargo.toml",
        "Cargo.lock",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect()
}

fn replay(path: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        return Err("replay output must be new".into());
    }
    let source = load_source()?;
    let expected = circuits(&source)?;
    let bytes = fs::read(path)?;
    let public: Value = serde_json::from_slice(&bytes)?;
    if public["schema"] != "VV/G2-E/MULTIBATCH-B4-PUBLIC-FIXTURE/v1"
        || public["protocol_version"] != PROTOCOL
        || public["state_layout"] != STATE_B4_LAYOUT_VERSION
        || public["state_vk_hex"] != source.first["groth16_vk_hex"]
        || public["source_ballot_vk_hex"] != source.first["source_ballot_vk_hex"]
        || public["source_verifier_srs"] != source.first["source_verifier_srs"]
    {
        return Err("replay trust anchor mismatch".into());
    }
    let vk: ark_groth16::VerifyingKey<Bls12_381> =
        canonical(&decode(public["state_vk_hex"].as_str().ok_or("VK")?)?)?;
    let prepared = prepare_verifying_key(&vk);
    let ballot_bytes = decode(public["source_ballot_vk_hex"].as_str().ok_or("ballot VK")?)?;
    let mut cursor = Cursor::new(&ballot_bytes);
    let ballot_vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    let ballot_prepared = prepare_bellperson_vk(&ballot_vk);
    let srs = srs_from_json(&public["source_verifier_srs"])?;
    let bs = public["batches"].as_array().ok_or("batches")?;
    if bs.len() != 2 {
        return Err("two batches required".into());
    }
    let mut receiver = Receiver::new(
        expected[0].records[0].clone(),
        expected.iter().map(|c| c.batch_root).collect(),
    );
    let started = Instant::now();
    let mut results = Vec::new();
    for (i, (b, c)) in bs.iter().zip(&expected).enumerate() {
        let offset = i * BATCH;
        if b["source_public_records"]
            != json!(
                &source.d2["source_public_records"]
                    .as_array()
                    .ok_or("records")?[offset..offset + BATCH]
            )
            || b["source_public_inputs_le_hex"]
                != json!(
                    &source.d2["source_public_inputs_le_hex"]
                        .as_array()
                        .ok_or("rows")?[offset..offset + BATCH]
                )
            || b["source_ballot_proofs_hex"]
                != json!(
                    &source.d2["source_proofs_ark_compressed_hex"]
                        .as_array()
                        .ok_or("proofs")?[offset..offset + BATCH]
                )
            || b["batch_root_hex"] != hex(&c.batch_root)
            || states(&b["old_state"])? != c.old_state
            || states(&b["new_state"])? != c.new_state
            || b["state_public_inputs_le_hex"]
                != json!(input_hex(&state_public_inputs(c).ok_or("packing")?)?)
        {
            return Err("fixture transition differs from frozen source".into());
        }
        let rows = rows(
            &source.d2["source_public_inputs_le_hex"]
                .as_array()
                .ok_or("rows")?[offset..offset + BATCH],
        )?;
        let t = transcript(&rows);
        if b["source_external_transcript_hex"] != hex(&t) {
            return Err("transcript mismatch".into());
        }
        let agg = aggregate_from_hex(&b["source_aggregate_hex"])?;
        let proof: ark_groth16::Proof<Bls12_381> =
            canonical(&decode(b["state_proof_hex"].as_str().ok_or("proof")?)?)?;
        let inputs = state_public_inputs(c).ok_or("state inputs")?;
        receiver
            .apply(c, || {
                Ok(verify_aggregate_proof(
                    &srs,
                    &ballot_prepared,
                    ChaCha20Rng::seed_from_u64(VERIFY_SEED + i as u64),
                    &rows,
                    &agg,
                    &t,
                    AggregateVersion::V2,
                )? && Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &inputs)?)
            })
            .map_err(|e| format!("replay rejected: {e:?}"))?;
        results.push(json!({"batch_index":c.batch_index,"verified_and_applied":true}));
    }
    if states(&public["final_state"])? != receiver.state()
        || receiver.spent_count() != 8
        || receiver.next() != 3
    {
        return Err("final checkpoint mismatch".into());
    }
    write_new(
        output,
        &json!({"gate":"G2-E-public-replay","status":"PASS_TWO_PUBLIC_BATCHES_REPLAYED",
        "fixture":path,"fixture_sha256":hex(&sha256(&bytes)),"git_commit":metadata("git",&["rev-parse","HEAD"])?,
        "batches":results,"next_batch_index":receiver.next(),"spent_nullifier_count":receiver.spent_count(),
        "final_state":ciphertexts(receiver.state()),"elapsed_ms":started.elapsed().as_secs_f64()*1000.0,
        "setup_count":0,"proof_generation_count":0,"production_setup":false,"evm_run":false}),
    )?;
    println!("G2-E PUBLIC REPLAY PASS: both SnarkPack/state proofs verify; next=3 spent=8; zero setup/prove");
    Ok(())
}

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--verify-fixture") {
        if args.len() != 4 || args[2] != "--output" {
            return Err("use --verify-fixture PATH --output NEW_PATH".into());
        }
        return replay(Path::new(&args[1]), Path::new(&args[3]));
    }
    let mut output = PathBuf::from("experiments/g2_e_raw/multibatch_state_b4.json");
    let mut fixture_output =
        PathBuf::from("fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json");
    for pair in args.chunks(2) {
        if pair.len() != 2 {
            return Err("missing option value".into());
        }
        match pair[0].as_str() {
            "--output" => output = pair[1].clone().into(),
            "--fixture-output" => fixture_output = pair[1].clone().into(),
            _ => return Err("unknown option".into()),
        }
    }
    if output.exists() || fixture_output.exists() || output == fixture_output {
        return Err("outputs must be distinct and new".into());
    }
    let run_started = Instant::now();
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let git_commit = metadata("git", &["rev-parse", "HEAD"])?;
    let git_status = metadata("git", &["status", "--porcelain", "--untracked-files=all"])?;
    let source_hashes_before = source_hashes()?;
    let source = load_source()?;
    let circuits = circuits(&source)?;
    let accepted_roots = circuits.iter().map(|c| c.batch_root).collect::<Vec<_>>();
    let schedule = json!({"election_id_hex":hex(&circuits[0].election_id),"final_context_hex":hex(&circuits[0].final_context),
        "region":4,"source_voter_indices":[[0,1,2,3],[4,5,6,7]],"ordered_roots_hex":accepted_roots.iter().map(|r|hex(r)).collect::<Vec<_>>()});
    let schedule_hash = hex(&sha256(&serde_json::to_vec(&schedule)?));
    println!(
        "G2-E [1/5] reuse eight D2 ballot proofs; freeze TWO B4 roots before state setup/proving"
    );
    let ballot_vk_bytes = decode(source.d2["groth16_vk_hex"].as_str().ok_or("ballot VK")?)?;
    let mut cursor = Cursor::new(&ballot_vk_bytes);
    let vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    let mut roundtrip = Vec::new();
    vk.write(&mut roundtrip)?;
    if cursor.position() as usize != ballot_vk_bytes.len()
        || roundtrip != ballot_vk_bytes
        || vk.ic.len() != 26
        || source.d2_raw["ballot_vk_bellperson_wire_sha256"] != hex(&sha256(&ballot_vk_bytes))
        || source.first["source_ballot_vk_hex"] != hex(&ballot_vk_bytes)
    {
        return Err("ballot VK mismatch".into());
    }
    let ballot_prepared = prepare_bellperson_vk(&vk);
    let proof_values = source.d2["source_proofs_ark_compressed_hex"]
        .as_array()
        .ok_or("proof pool")?;
    if proof_values.len() != 32 {
        return Err("expected 32 proofs".into());
    }
    let all_rows = source.d2["source_public_inputs_le_hex"]
        .as_array()
        .ok_or("rows")?;
    let mut pool = Vec::new();
    let mut individual_ms = Vec::new();
    for (i, value) in proof_values[..8].iter().enumerate() {
        let bytes = decode(value.as_str().ok_or("proof")?)?;
        let mut cursor = Cursor::new(&bytes);
        let p = BellProof::<Bls12>::read(&mut cursor)?;
        let mut b = Vec::new();
        p.write(&mut b)?;
        if b != bytes || cursor.position() as usize != bytes.len() {
            return Err("ballot canonical roundtrip".into());
        }
        let row = rows(&all_rows[i..i + 1])?;
        let t = Instant::now();
        if !verify_ballot(&ballot_prepared, &p, &row[0])? {
            return Err("ballot verification failed".into());
        }
        individual_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        pool.push(p);
    }
    let t = Instant::now();
    let generic =
        setup_fake_srs::<Bls12, _>(&mut ChaCha20Rng::seed_from_u64(0x4731_4430_5352_5320), 32);
    let srs_rebuild_ms = t.elapsed().as_secs_f64() * 1000.0;
    if source.d2_raw["shared_aggregation_srs_sha256"] != hex(&generic.hash()) {
        return Err("research SRS mismatch".into());
    }
    let (prover_srs, srs) = generic.specialize(BATCH);
    let srs_json = json!({"n":srs.n,"g":srs.g,"h":srs.h,"g_alpha":srs.g_alpha,"g_beta":srs.g_beta,"h_alpha":srs.h_alpha,"h_beta":srs.h_beta,"h_alpha_d":srs.h_alpha_d});
    if srs_json != source.first["source_verifier_srs"] {
        return Err("archived verifier SRS differs".into());
    }
    let first_agg = aggregate_from_hex(&source.first["source_aggregate_hex"])?;
    let first_rows = rows(&all_rows[..4])?;
    if source.first["source_external_transcript_hex"] != hex(&transcript(&first_rows)) {
        return Err("first transcript differs".into());
    }
    let second_rows = rows(&all_rows[4..8])?;
    let second_transcript = transcript(&second_rows);
    let t = Instant::now();
    let second_agg = aggregate_proofs::<Bls12>(
        &prover_srs,
        &second_transcript,
        &pool[4..8],
        AggregateVersion::V2,
    )?;
    let new_aggregation_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!("G2-E [2/5] fingerprint nonzero-old-state batch two; reconstruct G2-D setup and require byte-identical VK");
    let shape = measure(&circuits[1])?;
    if shape["satisfied"] != true
        || shape["matrix_sha256"] != source.first_raw["shape"]["matrix_sha256"]
        || shape["public_inputs"] != STATE_PUBLIC_INPUTS
    {
        return Err("nonzero-old-state relation mismatch".into());
    }
    let t = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuits[0].clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let setup_rebuild_ms = t.elapsed().as_secs_f64() * 1000.0;
    let vk_bytes = compressed(&parameters.vk)?;
    if source.first["groth16_vk_hex"] != hex(&vk_bytes)
        || source.first_raw["verifying_key_sha256"] != hex(&sha256(&vk_bytes))
    {
        return Err("G2-D setup/VK reconstruction differs; cannot reuse first proof".into());
    }
    let first_proof: ark_groth16::Proof<Bls12_381> = canonical(&decode(
        source.first["proof_hex"]
            .as_str()
            .ok_or("first state proof")?,
    )?)?;
    println!("G2-E [3/5] reuse ONE archived first state proof; generate ONE new proof with old=S1, NOT zero");
    let t = Instant::now();
    let second_proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuits[1].clone(),
        &parameters,
        &mut ChaCha20Rng::seed_from_u64(PROOF_SEED),
    )?;
    let second_prove_ms = t.elapsed().as_secs_f64() * 1000.0;
    let prepared = prepare_verifying_key(&parameters.vk);
    let mut receiver = Receiver::new(circuits[0].records[0].clone(), accepted_roots.clone());
    let snapshot = |r: &Receiver| json!({"next":r.next(),"spent":r.spent_count(),"state":ciphertexts(r.state())});
    let initial = snapshot(&receiver);
    // The second statement/proof is real, but cannot be received before batch one.
    let skipped = receiver.apply(&circuits[1], || {
        panic!("out-of-order must reject before cryptography")
    });
    if skipped != Err(receiver::Rejection::NextIndex) || snapshot(&receiver) != initial {
        return Err("skipping first was not atomic".into());
    }
    let mut public_batches = Vec::new();
    let mut metrics = Vec::new();
    for (i, ((c, agg), proof)) in circuits
        .iter()
        .zip([&first_agg, &second_agg])
        .zip([&first_proof, &second_proof])
        .enumerate()
    {
        let matrix = if i == 0 { &first_rows } else { &second_rows };
        let external = transcript(matrix);
        let inputs = state_public_inputs(c).ok_or("packing")?;
        let mut agg_ms = 0.0;
        let mut state_ms = 0.0;
        let t = Instant::now();
        receiver
            .apply(c, || {
                let t = Instant::now();
                let valid = verify_aggregate_proof(
                    &srs,
                    &ballot_prepared,
                    ChaCha20Rng::seed_from_u64(VERIFY_SEED + i as u64),
                    matrix,
                    agg,
                    &external,
                    AggregateVersion::V2,
                )?;
                agg_ms = t.elapsed().as_secs_f64() * 1000.0;
                if !valid {
                    return Ok(false);
                }
                let t = Instant::now();
                let valid = Groth16::<Bls12_381>::verify_proof(&prepared, proof, &inputs)?;
                state_ms = t.elapsed().as_secs_f64() * 1000.0;
                Ok(valid)
            })
            .map_err(|e| format!("valid transition rejected: {e:?}"))?;
        let receiver_total_ms = t.elapsed().as_secs_f64() * 1000.0;
        let mut aggregate_bytes = Vec::new();
        agg.write(&mut aggregate_bytes)?;
        let proof_bytes = compressed(proof)?;
        let direct = Instant::now();
        let reference = independent_fold::reference(&c.old_state, &c.records).ok_or("reference")?;
        if reference != c.new_state {
            return Err("direct reference differs".into());
        }
        let direct_fold_ms = direct.elapsed().as_secs_f64() * 1000.0;
        let offset = i * BATCH;
        public_batches.push(json!({"batch_index":c.batch_index,"batch_root_hex":hex(&c.batch_root),
            "source_public_records":&source.d2["source_public_records"].as_array().ok_or("records")?[offset..offset+BATCH],
            "source_public_inputs_le_hex":&all_rows[offset..offset+BATCH],"source_ballot_proofs_hex":&proof_values[offset..offset+BATCH],
            "source_aggregate_hex":hex(&aggregate_bytes),"source_aggregate_sha256":hex(&sha256(&aggregate_bytes)),
            "source_external_transcript_hex":hex(&external),"state_proof_hex":hex(&proof_bytes),"state_proof_sha256":hex(&sha256(&proof_bytes)),
            "state_public_inputs_le_hex":input_hex(&inputs)?,"old_state":ciphertexts(&c.old_state),"new_state":ciphertexts(&c.new_state),
            "state_proof_reused":i==0,"aggregate_reused":i==0}));
        metrics.push(json!({"batch_index":c.batch_index,"source_voter_indices":(offset..offset+BATCH).collect::<Vec<_>>(),
            "snarkpack_verify_ms":agg_ms,"state_verify_ms":state_ms,"receiver_check_verify_apply_elapsed_ms":receiver_total_ms,
            "independent_direct_fold_check_ms":direct_fold_ms,"state_proof_bytes":proof_bytes.len(),"aggregate_bytes":aggregate_bytes.len(),
            "state_public_input_bytes":inputs.len()*32,"ballot_matrix_bytes":BATCH*25*32,"verified_and_applied":true,
            "checkpoint":snapshot(&receiver)}));
        println!(
            "  batch {} PASS: next={} spent={} stateVerify={state_ms:.6}ms",
            i + 1,
            receiver.next(),
            receiver.spent_count()
        );
        if i == 0 {
            let after_first = snapshot(&receiver);
            let repeat = receiver.apply(c, || panic!("replay must reject before cryptography"));
            let mut reset = circuits[1].clone();
            reset.old_state = vec![Ciphertext::zero(); 3];
            let reset_result = receiver.apply(&reset, || {
                panic!("reset old state must reject before cryptography")
            });
            if repeat != Err(receiver::Rejection::NextIndex)
                || reset_result != Err(receiver::Rejection::OldState)
                || snapshot(&receiver) != after_first
            {
                return Err("replay/reset changed current state".into());
            }
        }
    }
    let mut reset = circuits[1].clone();
    reset.old_state = vec![Ciphertext::zero(); 3];
    let reset_inputs = state_public_inputs(&reset).ok_or("reset inputs")?;
    let t = Instant::now();
    let reset_proof_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &second_proof, &reset_inputs)?;
    let reset_verify_ms = t.elapsed().as_secs_f64() * 1000.0;
    if !reset_proof_rejected
        || receiver.next() != 3
        || receiver.spent_count() != 8
        || receiver.state() != circuits[1].new_state
    {
        return Err("final continuity acceptance failed".into());
    }
    println!("G2-E [4/5] skipped first / replay / reset-old-state REJECT; flat independent eight-ballot sum MATCH");
    let public = json!({"schema":"VV/G2-E/MULTIBATCH-B4-PUBLIC-FIXTURE/v1","protocol_version":PROTOCOL,
        "state_layout":STATE_B4_LAYOUT_VERSION,"state_vk_hex":hex(&vk_bytes),"state_vk_sha256":hex(&sha256(&vk_bytes)),
        "source_ballot_vk_hex":hex(&ballot_vk_bytes),"source_verifier_srs":srs_json,"accepted_schedule":schedule,
        "accepted_schedule_json_sha256":schedule_hash,"batches":public_batches,"final_state":ciphertexts(receiver.state()),
        "next_batch_index":receiver.next(),"spent_nullifier_count":receiver.spent_count(),"production_setup":false,
        "proving_key_persisted":false,"private_voter_or_trustee_secrets_emitted":false});
    if source_hashes_before != source_hashes()?
        || git_commit != metadata("git", &["rev-parse", "HEAD"])?
    {
        return Err("source changed during run".into());
    }
    for pin in &source.pins {
        let path = pin["path"].as_str().ok_or("pin")?;
        if pin["sha256"] != hex(&sha256(&fs::read(path)?)) {
            return Err("source artifact changed during run".into());
        }
    }
    let mut result = json!({"gate":"G2-E","status":"PASS_TWO_CONSECUTIVE_B4_TRANSITIONS_AND_NATIVE_CHECKPOINT",
        "started_utc":started_utc,"git_commit":git_commit,"git_dirty":!git_status.is_empty(),"git_status_at_start":git_status,
        "source_hashes":source_hashes_before,"source_artifact_pins":source.pins,"experiment_descriptor":source.descriptor,"sources_unchanged_before_archival":true,
        "rustc_version":metadata(&env::var("RUSTC").unwrap_or_else(|_|"rustc".into()),&["--version"])?,
        "cpu":metadata("sysctl",&["-n","machdep.cpu.brand_string"]).unwrap_or_else(|_|"UNKNOWN".into()),
        "os":env::consts::OS,"arch":env::consts::ARCH,"build_profile":if cfg!(debug_assertions){"dev"}else{"release-opt-level-1"},
        "batch_size":BATCH,"batch_count":2,"total_distinct_ballots":8,"state_layout":STATE_B4_LAYOUT_VERSION,
        "source_suite":"D5/W32/G1-D2","frozen_ballot_manifest_sha256":source.baseline["frozen_ballot_manifest_sha256"],
        "source_individual_verify_ms":individual_ms,"source_ballot_proofs_reused":8,"new_ballot_proof_count":0,
        "source_aggregate_reused":1,"new_source_aggregate_count":1,"new_source_aggregation_ms":new_aggregation_ms,
        "srs_rebuild_ms_offline":srs_rebuild_ms,"shared_aggregation_srs_sha256":source.d2_raw["shared_aggregation_srs_sha256"],
        "setup_rebuild_count":1,"setup_rebuild_ms":setup_rebuild_ms,"setup_seed":SETUP_SEED,"new_second_proof_seed":PROOF_SEED,"state_vk_matches_g2d_byte_for_byte":true,
        "state_vk_sha256":hex(&sha256(&vk_bytes)),"proving_key_compressed_bytes":parameters.compressed_size(),
        "reused_state_proof_count":1,"new_state_proof_count":1,"new_second_prove_ms":second_prove_ms,
        "nonzero_old_state_shape":shape,"all_both_verifiers_pass":true,"batches":metrics,
        "accepted_schedule":schedule,"accepted_schedule_json_sha256":schedule_hash,"schedule_frozen_before_setup_and_prove":true,
        "second_old_equals_first_new":true,"second_old_state_is_nonzero":true,"final_equals_independent_eight_ballot_fold":true,
        "negative_tests":{"skipped_first_batch_rejected":"NextIndex","replayed_first_batch_rejected":"NextIndex",
            "reset_second_old_state_rejected":"OldState","all_preflight_rejections_leave_state_and_spent_unchanged":true,
            "original_second_proof_with_zero_old_state_rejected":reset_proof_rejected,"reset_proof_verify_ms":reset_verify_ms},
        "final_state":ciphertexts(receiver.state()),"next_batch_index":receiver.next(),"spent_nullifier_count":receiver.spent_count(),
        "peak_rss_mib":peak_rss_mib(),"total_run_ms_excluding_compilation":run_started.elapsed().as_secs_f64()*1000.0,
        "production_setup":false,"proving_key_persisted":false,"dkg_rerun":false,"ballot_setup_rerun":false,"evm_run":false,
        "gas_total":null,"calldata_bytes":null,"full_settlement":false,"recursive_snarkpack_verification":false,
        "private_voter_or_trustee_secrets_emitted":false,"public_verifier_fixture":fixture_output,
        "limitations":["two functional B4 transitions, not a B8 circuit or repeated performance statistics",
            "deterministic research setup and fake aggregation SRS, no production security ceremony",
            "native serial in-memory checkpoint only; no crash persistence, concurrent writes or EVM execution",
            "accepted roots locally frozen from selected D2 records; no independent intake/censorship-resistance authority",
            "P3 ten-subset v2, W32 state-to-disclosure end-to-end and complete on-chain verifier remain future steps"]});
    println!("G2-E [5/5] archive new public material and real measurements, preserve ALL historical results");
    write_new(&fixture_output, &public)?;
    result["public_verifier_fixture_sha256"] = json!(hex(&sha256(&fs::read(&fixture_output)?)));
    write_new(&output, &result)?;
    println!("G2-E PASS: setup rebuild={setup_rebuild_ms:.3}ms new second prove={second_prove_ms:.3}ms peakRSS={:.3}MiB",peak_rss_mib());
    Ok(())
}
