use ark_bls12_381::Bls12_381;
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::evm_export::Eip2537Groth16Fixture;
use vv_friendly_crypto::state_gadget::{
    batch_root_v2, check_state_transition_g2a, fold_state, state_public_inputs, StateBallotRecord,
    StateTransitionCircuit, STATE_BATCH_SIZE, STATE_LAYOUT_VERSION,
};
use vv_friendly_crypto::{decode_point, encode_point, generator, Ciphertext};
use vv_types::{hex, sha256};

const G2B_SETUP_SEED: u64 = 0x4732_422D_5345_5455;
const G2B_PROOF_SEED: u64 = 0x4732_422D_5052_4F56;

#[derive(Serialize)]
struct PublicCiphertext {
    candidate_id: usize,
    c0_hex: String,
    c1_hex: String,
}

#[derive(Serialize)]
struct StateLayout {
    layout_version: &'static str,
    public_inputs_in_order: Vec<&'static str>,
    witness: Vec<&'static str>,
    batch_size: usize,
    candidate_count: usize,
    batch_root_domain: &'static str,
    batch_node_domain: &'static str,
    ballot_public_inputs_per_record: usize,
    fixed_field_bytes: usize,
}

#[derive(Serialize)]
struct StateResult {
    gate: &'static str,
    status: &'static str,
    classification: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    prototype_manifest: String,
    protocol_version: String,
    manifest_capability: String,
    manifest_matches_g2a: bool,
    source_g1c1_artifact: String,
    source_g1c1_sha256_hex: String,
    source_g1c1_status: String,
    source_public_input_serialization_matches: bool,
    source_records_are_same_region: bool,
    source_records_have_distinct_nullifiers: bool,
    source_records_have_distinct_ciphertexts: bool,
    layout_version: &'static str,
    layout_file: String,
    batch_size: usize,
    candidate_count: usize,
    region: u64,
    batch_index: u64,
    batch_root_hex: String,
    public_inputs: usize,
    witness_variables: usize,
    constraints: usize,
    batch_hashes_in_r1cs: usize,
    accumulator_components: usize,
    group_additions_in_r1cs: usize,
    ballot_public_inputs_per_record: usize,
    ballot_field_vector_bytes_per_record: usize,
    witness_ms: f64,
    negative_witness_ms: f64,
    peak_rss_mib: f64,
    valid_transition_satisfied: bool,
    substituted_ciphertext_rejected: bool,
    old_state: Vec<PublicCiphertext>,
    new_state: Vec<PublicCiphertext>,
    groth16_run: bool,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct StateGroth16Result {
    gate: &'static str,
    status: &'static str,
    classification: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    prototype_manifest: String,
    protocol_version: String,
    manifest_capability: String,
    manifest_matches_g2b: bool,
    source_g2a_artifact: String,
    source_g1c1_artifact: String,
    source_g1c1_sha256_hex: String,
    source_public_input_serialization_matches: bool,
    layout_version: &'static str,
    layout_file: String,
    batch_size: usize,
    candidate_count: usize,
    region: u64,
    batch_index: u64,
    batch_root_hex: String,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    setup_seed: u64,
    proof_seed: u64,
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    changed_public_batch_root_verify_ms: f64,
    peak_rss_after_setup_mib: f64,
    peak_rss_after_prove_mib: f64,
    peak_rss_after_verify_mib: f64,
    proving_key_compressed_bytes: usize,
    verifying_key_compressed_bytes: usize,
    proof_bytes: usize,
    valid_proof_verified: bool,
    changed_public_batch_root_rejected: bool,
    proving_key_persisted: bool,
    private_witness_emitted: bool,
    groth16_run: bool,
    limitations: Vec<&'static str>,
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(target_os = "macos")]
fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    usage.ru_maxrss as f64 / (1024.0 * 1024.0)
}

#[cfg(not(target_os = "macos"))]
fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    usage.ru_maxrss as f64 / 1024.0
}

fn decode_hex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex string has odd length".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| Ok(u8::from_str_radix(&value[index..index + 2], 16)?))
        .collect()
}

fn decode_fixed(value: &Value, name: &str) -> Result<[u8; 32], Box<dyn Error>> {
    let encoded = value[name]
        .as_str()
        .ok_or_else(|| format!("missing {name}"))?;
    decode_hex(encoded)?
        .try_into()
        .map_err(|_| format!("{name} is not 32 bytes").into())
}

fn decode_record(value: &Value) -> Result<StateBallotRecord, Box<dyn Error>> {
    let public_key = decode_point(&decode_fixed(value, "public_key_hex")?, false)?;
    let ciphertexts = value["ciphertexts"]
        .as_array()
        .ok_or("missing ciphertexts")?
        .iter()
        .map(|ciphertext| {
            Ok(Ciphertext {
                c0: decode_point(&decode_fixed(ciphertext, "c0_hex")?, false)?,
                c1: decode_point(&decode_fixed(ciphertext, "c1_hex")?, true)?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    Ok(StateBallotRecord {
        election_id: decode_fixed(value, "election_id_hex")?,
        final_context: decode_fixed(value, "final_context_hex")?,
        region: value["region"].as_u64().ok_or("missing region")?,
        registration_root: decode_fixed(value, "registration_root_hex")?,
        nullifier: decode_fixed(value, "nullifier_hex")?,
        key_set_hash: decode_fixed(value, "key_set_hash_hex")?,
        public_key,
        ciphertexts,
    })
}

fn public_ciphertexts(state: &[Ciphertext]) -> Vec<PublicCiphertext> {
    state
        .iter()
        .enumerate()
        .map(|(index, ciphertext)| PublicCiphertext {
            candidate_id: index + 1,
            c0_hex: hex(&encode_point(&ciphertext.c0)),
            c1_hex: hex(&encode_point(&ciphertext.c1)),
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut source_path =
        PathBuf::from("experiments/g1_c1_raw/snarkpack_distinct_voters_batch_2.json");
    let mut output_path = PathBuf::from("experiments/g2_a_raw/state_transition_b2_r1cs.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_state_g2a_public_layout_v1.json");
    let mut groth16_output_path = None;
    let mut evm_fixture_output_path = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--source" => source_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--layout-output" => layout_path = PathBuf::from(value),
            "--groth16-output" => groth16_output_path = Some(PathBuf::from(value)),
            "--evm-fixture-output" => evm_fixture_output_path = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    if evm_fixture_output_path.is_some() && groth16_output_path.is_none() {
        return Err("--evm-fixture-output requires --groth16-output".into());
    }

    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_capability = manifest["capabilities"]["g2a_state_transition_r1cs"]
        .as_str()
        .ok_or("prototype manifest lacks G2-A capability")?
        .to_owned();
    let manifest_g2b_capability = manifest["capabilities"]["g2b_state_transition_groth16"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let manifest_matches_g2a = (protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2A-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2B-"))
        && manifest_capability
            == "READY_R1CS_FIXED_B2_CANONICAL_BATCH_ROOT_AND_SIX_CIPHERTEXT_COMPONENTS_NO_GROTH16";
    if !manifest_matches_g2a {
        return Err("prototype manifest does not describe G2-A".into());
    }
    let manifest_matches_g2b = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2B-")
        && manifest_g2b_capability == "READY_REAL_GROTH16_FIXED_B2_STATE_TRANSITION";
    if groth16_output_path.is_some() && !manifest_matches_g2b {
        return Err("prototype manifest does not describe G2-B".into());
    }

    let source_bytes = fs::read(&source_path)?;
    let source: Value = serde_json::from_slice(&source_bytes)?;
    let source_status = source["status"]
        .as_str()
        .ok_or("G1-C1 source lacks status")?
        .to_owned();
    let source_public_input_serialization_matches =
        source["state_record_serialization_matches_snarkpack_inputs"] == true;
    if source["gate"] != "G1-C1"
        || source_status != "PASS_G1C1_REAL_SNARKPACK_V2_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS"
        || source["protocol_version"] != protocol_version
        || source["aggregate_proof_verified"] != true
        || !source_public_input_serialization_matches
    {
        return Err("G1-C1 source artifact does not match the active G2-A suite".into());
    }
    let records = source["source_public_records"]
        .as_array()
        .ok_or("G1-C1 source lacks public records")?
        .iter()
        .map(decode_record)
        .collect::<Result<Vec<_>, _>>()?;
    if records.len() != STATE_BATCH_SIZE {
        return Err("G2-A requires exactly two source records".into());
    }
    let first = &records[0];
    let source_records_are_same_region = records.iter().all(|record| {
        record.election_id == first.election_id
            && record.final_context == first.final_context
            && record.region == first.region
            && record.registration_root == first.registration_root
            && record.key_set_hash == first.key_set_hash
            && record.public_key == first.public_key
    });
    let source_records_have_distinct_nullifiers = records[0].nullifier != records[1].nullifier;
    let source_records_have_distinct_ciphertexts = records[0].ciphertexts != records[1].ciphertexts;
    if !source_records_are_same_region
        || !source_records_have_distinct_nullifiers
        || !source_records_have_distinct_ciphertexts
    {
        return Err("G2-A source batch boundary failed".into());
    }

    let old_state = vec![Ciphertext::zero(); first.ciphertexts.len()];
    let new_state = fold_state(&old_state, &records).ok_or("failed to fold G2-A state")?;
    let batch_index = 1u64;
    let batch_root = batch_root_v2(
        &first.election_id,
        &first.final_context,
        first.region,
        batch_index,
        &records,
    )
    .ok_or("failed to compute G2-A batch root")?;
    let circuit = StateTransitionCircuit {
        election_id: first.election_id,
        final_context: first.final_context,
        region: first.region,
        batch_index,
        batch_size: STATE_BATCH_SIZE as u64,
        batch_root,
        key_set_hash: first.key_set_hash,
        public_key: first.public_key,
        old_state: old_state.clone(),
        new_state: new_state.clone(),
        records,
    };

    let started = Instant::now();
    let valid = check_state_transition_g2a(&circuit)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let mut substituted = circuit.clone();
    substituted.records[1].ciphertexts[0].c0 += generator();
    let negative_started = Instant::now();
    let substituted_ciphertext_rejected = !check_state_transition_g2a(&substituted)?.satisfied;
    let negative_witness_ms = negative_started.elapsed().as_secs_f64() * 1000.0;
    if !valid.satisfied || !substituted_ciphertext_rejected {
        return Err("G2-A R1CS acceptance boundary failed".into());
    }

    let layout = StateLayout {
        layout_version: STATE_LAYOUT_VERSION,
        public_inputs_in_order: vec![
            "election identifier",
            "final election context",
            "regional identifier",
            "regional batch index",
            "fixed batch size B=2",
            "canonical ordered batch root",
            "certified key-set hash",
            "canonical certified election public key",
            "three old-state ciphertext pairs",
            "three new-state ciphertext pairs",
        ],
        witness: vec![
            "two ordered complete public ballot records authenticated by G1-C1",
            "intermediate six-component Jubjub accumulators",
        ],
        batch_size: valid.batch_size,
        candidate_count: valid.candidate_count,
        batch_root_domain: "VV/BATCH/LEAF/v2",
        batch_node_domain: "VV/BATCH/NODE/v2",
        ballot_public_inputs_per_record: valid.ballot_public_inputs_per_record,
        fixed_field_bytes: 32,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let result = StateResult {
        gate: "G2-A",
        status: "PASS_G2A_FIXED_B2_STATE_TRANSITION_R1CS",
        classification: "fixed-b2-canonical-ordered-batch-root-and-six-component-jubjub-state-fold-r1cs-no-groth16",
        git_commit: command_output("git", &["rev-parse", "HEAD"])
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
        git_dirty: command_output(
            "git",
            &["status", "--porcelain", "--untracked-files=normal"],
        )
        .map_or_else(|| "UNKNOWN".to_owned(), |value| (!value.is_empty()).to_string()),
        build_profile: if cfg!(debug_assertions) {
            "dev-unoptimized"
        } else {
            "release-opt-level-1"
        },
        rustc_version: command_output(
            &env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()),
            &["--version"],
        )
        .unwrap_or_else(|| "UNKNOWN".to_owned()),
        os: env::consts::OS,
        arch: env::consts::ARCH,
        cpu: if cfg!(target_os = "macos") {
            command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
                .unwrap_or_else(|| "UNKNOWN".to_owned())
        } else {
            "UNKNOWN".to_owned()
        },
        prototype_manifest: manifest_path.display().to_string(),
        protocol_version: protocol_version.clone(),
        manifest_capability: manifest_capability.clone(),
        manifest_matches_g2a,
        source_g1c1_artifact: source_path.display().to_string(),
        source_g1c1_sha256_hex: hex(&sha256(&source_bytes)),
        source_g1c1_status: source_status.clone(),
        source_public_input_serialization_matches,
        source_records_are_same_region,
        source_records_have_distinct_nullifiers,
        source_records_have_distinct_ciphertexts,
        layout_version: STATE_LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        batch_size: valid.batch_size,
        candidate_count: valid.candidate_count,
        region: circuit.region,
        batch_index,
        batch_root_hex: hex(&batch_root),
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        constraints: valid.constraints,
        batch_hashes_in_r1cs: valid.batch_hashes,
        accumulator_components: valid.accumulator_components,
        group_additions_in_r1cs: valid.group_additions,
        ballot_public_inputs_per_record: valid.ballot_public_inputs_per_record,
        ballot_field_vector_bytes_per_record: valid.ballot_field_vector_bytes_per_record,
        witness_ms,
        negative_witness_ms,
        peak_rss_mib: peak_rss_mib(),
        valid_transition_satisfied: valid.satisfied,
        substituted_ciphertext_rejected,
        old_state: public_ciphertexts(&old_state),
        new_state: public_ciphertexts(&new_state),
        groth16_run: false,
        limitations: vec![
            "G2-A fixes B=2, three candidates and the G1-C1 four-leaf/four-region ballot suite",
            "the circuit measures R1CS witness synthesis only; Groth16 setup, prove and verify are not run",
            "accepted-root, spent-nullifier and stale-state checks remain external contract checks as specified by the paper",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&result)?)?;

    if let Some(groth16_path) = &groth16_output_path {
        let public_inputs = state_public_inputs(&circuit)
            .ok_or("failed to pack the frozen G2-B public input layout")?;
        if public_inputs.len() != valid.public_inputs {
            return Err("G2-B public input packing differs from G2-A".into());
        }

        let mut setup_rng = ChaCha20Rng::seed_from_u64(G2B_SETUP_SEED);
        let setup_started = Instant::now();
        let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            circuit.clone(),
            &mut setup_rng,
        )?;
        let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_setup_mib = peak_rss_mib();

        let mut proof_rng = ChaCha20Rng::seed_from_u64(G2B_PROOF_SEED);
        let prove_started = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            circuit.clone(),
            &parameters,
            &mut proof_rng,
        )?;
        let prove_ms = prove_started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_prove_mib = peak_rss_mib();

        let prepared = prepare_verifying_key(&parameters.vk);
        let verify_started = Instant::now();
        let valid_proof_verified =
            Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &public_inputs)?;
        let verify_ms = verify_started.elapsed().as_secs_f64() * 1000.0;

        let mut changed_root_circuit = circuit.clone();
        changed_root_circuit.batch_root[0] ^= 1;
        let changed_public_inputs = state_public_inputs(&changed_root_circuit)
            .ok_or("failed to pack the changed G2-B batch root")?;
        let changed_verify_started = Instant::now();
        let changed_public_batch_root_rejected =
            !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed_public_inputs)?;
        let changed_public_batch_root_verify_ms =
            changed_verify_started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_verify_mib = peak_rss_mib();
        if !valid_proof_verified || !changed_public_batch_root_rejected {
            return Err("G2-B Groth16 acceptance boundary failed".into());
        }

        if let Some(path) = &evm_fixture_output_path {
            let fixture = Eip2537Groth16Fixture::from_state_proof(
                &parameters.vk,
                &proof,
                &public_inputs,
                &changed_public_inputs,
            );
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, serde_json::to_vec_pretty(&fixture)?)?;
            println!("wrote public-only EIP-2537 fixture {}", path.display());
        }

        let mut proof_encoding = Vec::new();
        proof.serialize_compressed(&mut proof_encoding)?;
        let groth16_result = StateGroth16Result {
            gate: "G2-B",
            status: "PASS_G2B_REAL_GROTH16_FIXED_B2_STATE_TRANSITION",
            classification: "real-groth16-over-fixed-b2-canonical-batch-root-and-six-component-jubjub-state-transition",
            git_commit: command_output("git", &["rev-parse", "HEAD"])
                .unwrap_or_else(|| "UNKNOWN".to_owned()),
            git_dirty: command_output(
                "git",
                &["status", "--porcelain", "--untracked-files=normal"],
            )
            .map_or_else(|| "UNKNOWN".to_owned(), |value| (!value.is_empty()).to_string()),
            build_profile: if cfg!(debug_assertions) {
                "dev-unoptimized"
            } else {
                "release-opt-level-1"
            },
            rustc_version: command_output(
                &env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()),
                &["--version"],
            )
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
            os: env::consts::OS,
            arch: env::consts::ARCH,
            cpu: if cfg!(target_os = "macos") {
                command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
                    .unwrap_or_else(|| "UNKNOWN".to_owned())
            } else {
                "UNKNOWN".to_owned()
            },
            prototype_manifest: manifest_path.display().to_string(),
            protocol_version: protocol_version.clone(),
            manifest_capability: manifest_g2b_capability.clone(),
            manifest_matches_g2b,
            source_g2a_artifact: output_path.display().to_string(),
            source_g1c1_artifact: source_path.display().to_string(),
            source_g1c1_sha256_hex: hex(&sha256(&source_bytes)),
            source_public_input_serialization_matches,
            layout_version: STATE_LAYOUT_VERSION,
            layout_file: layout_path.display().to_string(),
            batch_size: valid.batch_size,
            candidate_count: valid.candidate_count,
            region: circuit.region,
            batch_index,
            batch_root_hex: hex(&batch_root),
            constraints: valid.constraints,
            public_inputs: valid.public_inputs,
            witness_variables: valid.witness_variables,
            setup_seed: G2B_SETUP_SEED,
            proof_seed: G2B_PROOF_SEED,
            setup_ms,
            prove_ms,
            verify_ms,
            changed_public_batch_root_verify_ms,
            peak_rss_after_setup_mib,
            peak_rss_after_prove_mib,
            peak_rss_after_verify_mib,
            proving_key_compressed_bytes: parameters.compressed_size(),
            verifying_key_compressed_bytes: parameters.vk.compressed_size(),
            proof_bytes: proof_encoding.len(),
            valid_proof_verified,
            changed_public_batch_root_rejected,
            proving_key_persisted: false,
            private_witness_emitted: false,
            groth16_run: true,
            limitations: vec![
                "G2-B proves the fixed B=2, three-candidate suite; it is not a larger-batch scale result",
                "the deterministic experiment seeds are reproducibility inputs, not a production trusted setup ceremony",
                "accepted-root, spent-nullifier and stale-state checks remain external contract checks as specified by the paper",
            ],
        };
        if let Some(parent) = groth16_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(groth16_path, serde_json::to_vec_pretty(&groth16_result)?)?;
        println!(
            "G2-B: constraints={} public-inputs={} setup={:.3}ms prove={:.3}ms verify={:.3}ms proof={}B pk={}B vk={}B peak-rss={:.3}MiB valid={} changed-batch-root-rejected={}",
            groth16_result.constraints,
            groth16_result.public_inputs,
            groth16_result.setup_ms,
            groth16_result.prove_ms,
            groth16_result.verify_ms,
            groth16_result.proof_bytes,
            groth16_result.proving_key_compressed_bytes,
            groth16_result.verifying_key_compressed_bytes,
            groth16_result.peak_rss_after_verify_mib,
            groth16_result.valid_proof_verified,
            groth16_result.changed_public_batch_root_rejected,
        );
        println!("wrote {}", groth16_path.display());
    }
    println!(
        "G2-A: batch={} candidates={} hashes={} accumulator-components={} group-additions={} constraints={} public-inputs={} witness-vars={} witness={:.3}ms peak-rss={:.3}MiB valid={} substituted-ciphertext-rejected={} groth16=false",
        result.batch_size,
        result.candidate_count,
        result.batch_hashes_in_r1cs,
        result.accumulator_components,
        result.group_additions_in_r1cs,
        result.constraints,
        result.public_inputs,
        result.witness_variables,
        result.witness_ms,
        result.peak_rss_mib,
        result.valid_transition_satisfied,
        result.substituted_ciphertext_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());
    Ok(())
}
