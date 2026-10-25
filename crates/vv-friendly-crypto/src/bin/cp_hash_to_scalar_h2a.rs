use serde::{Deserialize, Serialize};
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::{
    encode_scalar, hash_to_scalar, hash_to_scalar_gadget::check_hash_to_scalar_first_valid,
    hash_to_scalar_input,
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
    transcript_bytes: usize,
    h2s_input_bytes: usize,
    sha512_blocks_per_attempt: usize,
    accepted_counter: u16,
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
struct H2AResult {
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
    counter_zero: CaseResult,
    counter_one: CaseResult,
    changed_transcript_rejected: bool,
    changed_domain_rejected: bool,
    false_counter_one_claim_rejected: bool,
    exact_wrapper_built_in_circuit: bool,
    mask_252_bits_constrained: bool,
    strict_scalar_range_constrained: bool,
    first_valid_rule_for_tested_counters: bool,
    uniform_128_attempt_circuit_implemented: bool,
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
        return Err("frozen H2-A vector has inconsistent lengths".into());
    }

    let (native_scalar, native_counter) = hash_to_scalar(domain, &transcript)?;
    let native_scalar_bytes = encode_scalar(&native_scalar).to_vec();
    let native_matches_frozen_vector =
        native_counter == frozen.rejection_counter && native_scalar_bytes == frozen_scalar;
    if !native_matches_frozen_vector {
        return Err("native H2S-v2 does not match the frozen vector".into());
    }

    let started = Instant::now();
    let circuit = check_hash_to_scalar_first_valid(
        domain,
        &transcript,
        &native_scalar_bytes,
        native_counter,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !circuit.satisfied {
        return Err("H2-A circuit rejected the native first-valid candidate".into());
    }

    let input_bytes = hash_to_scalar_input(domain, &transcript, native_counter).len();
    if input_bytes != circuit.h2s_input_bytes {
        return Err("native and circuit H2S wrapper lengths differ".into());
    }
    Ok(CaseRun {
        result: CaseResult {
            vector_file: path.display().to_string(),
            seed: frozen.seed,
            transcript_bytes: transcript.len(),
            h2s_input_bytes: circuit.h2s_input_bytes,
            sha512_blocks_per_attempt: circuit.padded_blocks_per_attempt,
            accepted_counter: native_counter,
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
    let mut output_path = PathBuf::from("experiments/g0_9_h2_raw/h2a_hash_to_scalar.json");
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
        return Err("H2-A requires one counter-zero and one counter-one vector".into());
    }

    let mut changed_transcript = transcript_zero.clone();
    changed_transcript[0] ^= 1;
    let changed_transcript_rejected =
        !check_hash_to_scalar_first_valid(b"GENERALIZED-CP", &changed_transcript, &scalar_zero, 0)?
            .satisfied;
    let changed_domain_rejected = !check_hash_to_scalar_first_valid(
        b"GENERALIZED-CP-ALTERED",
        &transcript_zero,
        &scalar_zero,
        0,
    )?
    .satisfied;
    let false_counter_one_claim_rejected =
        !check_hash_to_scalar_first_valid(b"GENERALIZED-CP", &transcript_zero, &scalar_zero, 1)?
            .satisfied;
    if !changed_transcript_rejected || !changed_domain_rejected || !false_counter_one_claim_rejected
    {
        return Err("H2-A necessary negative check failed".into());
    }

    let report = H2AResult {
        gate: "G0.9-H2-A",
        status: "PASS_H2A_COUNTER_0_1_FIRST_VALID_R1CS",
        classification: "isolated-exact-h2s-v2-counter-0-1-no-cp-no-groth16",
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
        counter_zero,
        counter_one,
        changed_transcript_rejected,
        changed_domain_rejected,
        false_counter_one_claim_rejected,
        exact_wrapper_built_in_circuit: true,
        mask_252_bits_constrained: true,
        strict_scalar_range_constrained: true,
        first_valid_rule_for_tested_counters: true,
        uniform_128_attempt_circuit_implemented: false,
        cp_equations_integrated: false,
        groth16_run: false,
        full_h2: false,
        full_p3: false,
        limitations: vec![
            "the exact first-valid relation is measured only for accepted counters 0 and 1",
            "circuit shape grows with the accepted counter; one uniform 128-attempt circuit is not implemented",
            "the CP group equations and Groth16 setup/prove/verify are outside H2-A",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H2-A: counter0={} constraints/{:.3}ms counter1={} constraints/{:.3}ms wrapper={}B blocks/attempt={} first-valid-0-1=true uniform-128=false full-h2=false full-p3=false",
        report.counter_zero.constraints,
        report.counter_zero.witness_ms,
        report.counter_one.constraints,
        report.counter_one.witness_ms,
        report.counter_zero.h2s_input_bytes,
        report.counter_zero.sha512_blocks_per_attempt,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
