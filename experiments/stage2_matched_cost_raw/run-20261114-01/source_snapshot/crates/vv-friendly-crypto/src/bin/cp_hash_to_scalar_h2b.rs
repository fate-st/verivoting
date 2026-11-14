use serde::{Deserialize, Serialize};
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::{
    encode_scalar, hash_to_scalar,
    hash_to_scalar_gadget::{check_hash_to_scalar_fixed_two, H2B_FIXED_ATTEMPTS},
    hash_to_scalar_input, HASH_REJECTION_MAX_ATTEMPTS,
};

#[derive(Deserialize)]
struct FrozenTranscript {
    seed: u64,
    hash_to_scalar_domain: String,
    transcript_bytes: usize,
    transcript_hex: String,
    challenge_scalar_le_hex: String,
    rejection_counter: u16,
}

#[derive(Serialize)]
struct CaseResult {
    vector_file: String,
    seed: u64,
    accepted_counter: u16,
    transcript_bytes: usize,
    h2s_input_bytes: usize,
    sha512_blocks_per_attempt: usize,
    attempts_hashed: usize,
    scalar_le_hex: String,
    native_matches_frozen_vector: bool,
    r1cs_matches_native: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib_after_case: f64,
}

struct CaseRun {
    result: CaseResult,
    transcript: Vec<u8>,
    scalar: Vec<u8>,
}

#[derive(Serialize)]
struct H2BResult {
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
    operation_domain: String,
    fixed_attempts: u16,
    native_suite_max_attempts: u16,
    counter_zero: CaseResult,
    counter_one: CaseResult,
    same_constraints: bool,
    same_public_inputs: bool,
    same_witness_variables: bool,
    false_counter_one_claim_rejected: bool,
    fixed_shape_two_attempt_relation: bool,
    suite_v2_128_attempt_relation: bool,
    cp_equations_integrated: bool,
    groth16_run: bool,
    full_h2: bool,
    full_p3: bool,
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

fn decode_hex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex string has odd length".into());
    }
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(text, 16)?)
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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

fn run_case(path: &PathBuf) -> Result<CaseRun, Box<dyn Error>> {
    let frozen: FrozenTranscript = serde_json::from_slice(&fs::read(path)?)?;
    let transcript = decode_hex(&frozen.transcript_hex)?;
    let frozen_scalar = decode_hex(&frozen.challenge_scalar_le_hex)?;
    let domain = frozen.hash_to_scalar_domain.as_bytes();
    if transcript.len() != frozen.transcript_bytes || frozen_scalar.len() != 32 {
        return Err("frozen H2-B vector has inconsistent lengths".into());
    }

    let (native_scalar, native_counter) = hash_to_scalar(domain, &transcript)?;
    let native_scalar_bytes = encode_scalar(&native_scalar).to_vec();
    let native_matches_frozen_vector =
        native_counter == frozen.rejection_counter && native_scalar_bytes == frozen_scalar;
    if !native_matches_frozen_vector || native_counter >= H2B_FIXED_ATTEMPTS {
        return Err("frozen vector is not a native counter-0/1 H2S-v2 case".into());
    }

    let started = Instant::now();
    let circuit =
        check_hash_to_scalar_fixed_two(domain, &transcript, &native_scalar_bytes, native_counter)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !circuit.satisfied {
        return Err("fixed two-attempt circuit rejected the native result".into());
    }

    let input_bytes = hash_to_scalar_input(domain, &transcript, native_counter).len();
    if input_bytes != circuit.h2s_input_bytes {
        return Err("native and circuit H2S wrapper lengths differ".into());
    }

    Ok(CaseRun {
        result: CaseResult {
            vector_file: path.display().to_string(),
            seed: frozen.seed,
            accepted_counter: native_counter,
            transcript_bytes: transcript.len(),
            h2s_input_bytes: circuit.h2s_input_bytes,
            sha512_blocks_per_attempt: circuit.padded_blocks_per_attempt,
            attempts_hashed: circuit.attempts_hashed,
            scalar_le_hex: encode_hex(&native_scalar_bytes),
            native_matches_frozen_vector,
            r1cs_matches_native: circuit.satisfied,
            constraints: circuit.constraints,
            public_inputs: circuit.public_inputs,
            witness_variables: circuit.witness_variables,
            witness_ms,
            peak_rss_mib_after_case: peak_rss_mib(),
        },
        transcript,
        scalar: native_scalar_bytes,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut counter_zero_path = PathBuf::from("fixtures/jubjub_cp_transcript_v3.json");
    let mut counter_one_path = PathBuf::from("fixtures/jubjub_cp_transcript_v3.counter1.json");
    let mut output_path = PathBuf::from("experiments/g0_9_h2_raw/h2b_fixed_two.json");
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--counter-zero-vector" => counter_zero_path = PathBuf::from(value),
            "--counter-one-vector" => counter_one_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let CaseRun {
        result: counter_zero,
        transcript: transcript_zero,
        scalar: scalar_zero,
    } = run_case(&counter_zero_path)?;
    let counter_one = run_case(&counter_one_path)?.result;
    if counter_zero.accepted_counter != 0 || counter_one.accepted_counter != 1 {
        return Err("H2-B requires one counter-zero and one counter-one vector".into());
    }

    let same_constraints = counter_zero.constraints == counter_one.constraints;
    let same_public_inputs = counter_zero.public_inputs == counter_one.public_inputs;
    let same_witness_variables = counter_zero.witness_variables == counter_one.witness_variables;
    if !same_constraints || !same_public_inputs || !same_witness_variables {
        return Err("H2-B counter paths do not have the same circuit shape".into());
    }

    let false_counter_one_claim_rejected =
        !check_hash_to_scalar_fixed_two(b"GENERALIZED-CP", &transcript_zero, &scalar_zero, 1)?
            .satisfied;
    if !false_counter_one_claim_rejected {
        return Err("fixed circuit accepted a false counter-one claim".into());
    }

    let report = H2BResult {
        gate: "G0.9-H2-B",
        status: "PASS_H2B_FIXED_TWO_ATTEMPT_R1CS_FEASIBILITY",
        classification: "fixed-shape-two-attempt-h2s-v2-no-cp-no-groth16-not-full-h2",
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
        operation_domain: "GENERALIZED-CP".to_owned(),
        fixed_attempts: H2B_FIXED_ATTEMPTS,
        native_suite_max_attempts: HASH_REJECTION_MAX_ATTEMPTS,
        counter_zero,
        counter_one,
        same_constraints,
        same_public_inputs,
        same_witness_variables,
        false_counter_one_claim_rejected,
        fixed_shape_two_attempt_relation: true,
        suite_v2_128_attempt_relation: false,
        cp_equations_integrated: false,
        groth16_run: false,
        full_h2: false,
        full_p3: false,
        limitations: vec![
            "both counter paths use one fixed two-attempt shape, not the Suite v2 128-attempt bound",
            "two attempts are a circuit-shape feasibility result and are not an acceptable replacement security parameter",
            "the CP group equations and Groth16 setup/prove/verify are outside H2-B",
        ],
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H2-B: fixed-attempts={} counter0={} constraints/{:.3}ms counter1={} constraints/{:.3}ms same-shape={} false-counter-rejected={} suite-v2-128=false full-h2=false full-p3=false",
        report.fixed_attempts,
        report.counter_zero.constraints,
        report.counter_zero.witness_ms,
        report.counter_one.constraints,
        report.counter_one.witness_ms,
        report.same_constraints && report.same_public_inputs && report.same_witness_variables,
        report.false_counter_one_claim_rejected,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
