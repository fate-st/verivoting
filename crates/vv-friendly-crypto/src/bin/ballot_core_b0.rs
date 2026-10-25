use ark_ec::Group;
use ark_ff::PrimeField;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::ballot_gadget::{
    check_ballot_core_b0, BallotCoreCircuit, BALLOT_CORE_CANDIDATE_COUNT,
};
use vv_friendly_crypto::{
    derive_secondary_generator, encode_point, generator, random_nonzero_scalar, Ciphertext, Scalar,
    CP_H2F_SUITE_ID, SUITE_ID,
};
use vv_types::hex;

const DEFAULT_SEED: u64 = 20_261_027;
const LAYOUT_VERSION: &str = "VV/JUBJUB/G1-B0/BALLOT-CORE-LAYOUT/v1";

#[derive(Serialize)]
struct PublicCiphertext {
    candidate_id: usize,
    c0_hex: String,
    c1_hex: String,
}

#[derive(Serialize)]
struct PublicLayoutFixture {
    layout_version: &'static str,
    protocol_version: String,
    suite_id: &'static str,
    seed: u64,
    candidate_count: usize,
    public_inputs: Vec<&'static str>,
    private_witnesses: Vec<&'static str>,
    omitted_full_c_vote_fields: Vec<&'static str>,
    registered_weight_bound: u64,
    public_key_hex: String,
    weight_commitment_hex: String,
    ciphertexts: Vec<PublicCiphertext>,
    private_weight_emitted: bool,
    private_choice_emitted: bool,
    private_opening_emitted: bool,
    private_randomness_emitted: bool,
}

#[derive(Serialize)]
struct BallotCoreResult {
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
    manifest_matches_g1b0: bool,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    candidate_count: usize,
    registered_weight_bound: u64,
    native_relations_match: bool,
    one_hot_relations_in_r1cs: usize,
    weight_range_relations_in_r1cs: usize,
    weight_commitment_opening_equations_in_r1cs: usize,
    elgamal_group_equations_in_r1cs: usize,
    public_group_points: usize,
    private_scalar_values: usize,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    valid_relation_satisfied: bool,
    two_hot_witness_rejected: bool,
    groth16_run: bool,
    full_ballot_core_b0: bool,
    full_c_vote: bool,
    snarkpack_run: bool,
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut output_path = PathBuf::from("experiments/g1_b0_raw/ballot_core_r1cs.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_ballot_core_b0_public_layout_v1.json");
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--layout-output" => layout_path = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_capability = manifest["capabilities"]["g1b0_ballot_core_r1cs"]
        .as_str()
        .ok_or("prototype manifest lacks G1-B0 capability")?
        .to_owned();
    let manifest_matches_g1b0 = (protocol_version
        .starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1B0-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1B1-"))
        && manifest_capability
            == "READY_R1CS_ONE_HOT_WEIGHT_COMMITMENT_AND_THREE_ELGAMAL_NO_CONTEXT_PRF_MERKLE_OR_GROTH16"
        && manifest["cp_hash_to_scalar_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(CP_H2F_SUITE_ID));
    if !manifest_matches_g1b0 {
        return Err("prototype manifest does not describe G1-B0".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let secret = random_nonzero_scalar(&mut rng);
    let public_key = generator().mul_bigint(secret.into_bigint());
    let registered_weight = 10u64;
    let weight = 3u64;
    let choices = vec![true, false, false];
    let weight_opening = random_nonzero_scalar(&mut rng);
    let registration_h = derive_secondary_generator(b"REGISTRATION-PEDERSEN-H")?;
    let weight_commitment = generator().mul_bigint(Scalar::from(weight).into_bigint())
        + registration_h.mul_bigint(weight_opening.into_bigint());
    let randomnesses = (0..BALLOT_CORE_CANDIDATE_COUNT)
        .map(|_| random_nonzero_scalar(&mut rng))
        .collect::<Vec<_>>();
    let ciphertexts = randomnesses
        .iter()
        .zip(&choices)
        .map(|(randomness, choice)| {
            let message = if *choice { weight } else { 0 };
            Ciphertext {
                c0: generator().mul_bigint(randomness.into_bigint()),
                c1: public_key.mul_bigint(randomness.into_bigint())
                    + generator().mul_bigint(Scalar::from(message).into_bigint()),
            }
        })
        .collect::<Vec<_>>();
    let native_relations_match = choices.iter().filter(|choice| **choice).count() == 1
        && weight > 0
        && weight <= registered_weight
        && weight_commitment
            == generator().mul_bigint(Scalar::from(weight).into_bigint())
                + registration_h.mul_bigint(weight_opening.into_bigint());
    if !native_relations_match {
        return Err("native G1-B0 fixture is inconsistent".into());
    }

    let circuit = BallotCoreCircuit {
        public_key,
        weight_commitment,
        ciphertexts: ciphertexts.clone(),
        registered_weight,
        weight,
        weight_opening,
        choices,
        randomnesses: randomnesses.clone(),
    };
    let started = Instant::now();
    let valid = check_ballot_core_b0(&circuit)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let r1cs_peak_rss_mib = peak_rss_mib();

    let mut two_hot = circuit.clone();
    two_hot.choices = vec![true, true, false];
    let two_hot_witness_rejected = !check_ballot_core_b0(&two_hot)?.satisfied;
    if !valid.satisfied || !two_hot_witness_rejected {
        return Err("G1-B0 R1CS acceptance boundary failed".into());
    }

    let layout = PublicLayoutFixture {
        layout_version: LAYOUT_VERSION,
        protocol_version: protocol_version.clone(),
        suite_id: SUITE_ID,
        seed,
        candidate_count: BALLOT_CORE_CANDIDATE_COUNT,
        public_inputs: vec![
            "canonical certified election public key",
            "canonical registered-weight commitment",
            "registered-weight upper bound",
            "three canonical Weighted ElGamal ciphertext pairs",
        ],
        private_witnesses: vec![
            "registered weight",
            "registered-weight commitment opening",
            "three one-hot choice bits",
            "three encryption randomness scalars",
        ],
        omitted_full_c_vote_fields: vec![
            "election and final-context identifiers",
            "credential secret and PRF-derived address",
            "deterministic region assignment",
            "election-scoped voting nullifier",
            "registration-root membership path",
        ],
        registered_weight_bound: registered_weight,
        public_key_hex: hex(&encode_point(&public_key)),
        weight_commitment_hex: hex(&encode_point(&weight_commitment)),
        ciphertexts: ciphertexts
            .iter()
            .enumerate()
            .map(|(index, ciphertext)| PublicCiphertext {
                candidate_id: index + 1,
                c0_hex: hex(&encode_point(&ciphertext.c0)),
                c1_hex: hex(&encode_point(&ciphertext.c1)),
            })
            .collect(),
        private_weight_emitted: false,
        private_choice_emitted: false,
        private_opening_emitted: false,
        private_randomness_emitted: false,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let report = BallotCoreResult {
        gate: "G1-B0",
        status: "PASS_G1B0_WEIGHTED_BALLOT_CORE_R1CS",
        classification: "one-ballot-one-hot-weight-commitment-and-three-coordinate-jubjub-elgamal-r1cs-no-context-prf-merkle-groth16-or-snarkpack",
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
        protocol_version,
        manifest_capability,
        manifest_matches_g1b0,
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        candidate_count: valid.candidate_count,
        registered_weight_bound: registered_weight,
        native_relations_match,
        one_hot_relations_in_r1cs: valid.one_hot_relations,
        weight_range_relations_in_r1cs: valid.weight_range_relations,
        weight_commitment_opening_equations_in_r1cs: valid
            .weight_commitment_opening_equations,
        elgamal_group_equations_in_r1cs: valid.elgamal_group_equations,
        public_group_points: valid.public_group_points,
        private_scalar_values: valid.private_scalar_values,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        witness_ms,
        peak_rss_mib: r1cs_peak_rss_mib,
        valid_relation_satisfied: valid.satisfied,
        two_hot_witness_rejected,
        groth16_run: false,
        full_ballot_core_b0: true,
        full_c_vote: false,
        snarkpack_run: false,
        limitations: vec![
            "G1-B0 is the weighted-ballot cryptographic core, not the paper's five-condition C_vote",
            "election context, address and region PRFs, voting nullifier and registration-root membership remain absent",
            "Groth16 and SnarkPack are not run in G1-B0",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G1-B0: candidates={} one-hot={} weight-range={} commitment-opening={} elgamal-equations={} constraints={} public-inputs={} witness-vars={} witness={:.3}ms peak-rss={:.3}MiB valid={} two-hot-rejected={} groth16=false full-c-vote=false snarkpack=false",
        report.candidate_count,
        report.one_hot_relations_in_r1cs,
        report.weight_range_relations_in_r1cs,
        report.weight_commitment_opening_equations_in_r1cs,
        report.elgamal_group_equations_in_r1cs,
        report.constraints,
        report.public_inputs,
        report.witness_variables,
        report.witness_ms,
        report.peak_rss_mib,
        report.valid_relation_satisfied,
        report.two_hot_witness_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());
    Ok(())
}
