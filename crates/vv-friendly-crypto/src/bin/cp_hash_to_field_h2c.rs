use serde::{Deserialize, Serialize};
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::{
    encode_scalar, hash_to_field_gadget::check_hash_to_field_wide_v3, hash_to_field_wide_v3,
    hash_to_field_wide_v3_input, WIDE_H2F_CANDIDATE_SUITE_ID,
};

#[derive(Deserialize)]
struct FrozenTranscript {
    seed: u64,
    hash_to_scalar_domain: String,
    transcript_bytes: usize,
    transcript_hex: String,
    rejection_counter: u16,
}

#[derive(Serialize)]
struct CaseResult {
    source_vector: String,
    source_seed: u64,
    source_v2_rejection_counter: u16,
    transcript_bytes: usize,
    h2f_input_bytes: usize,
    sha512_blocks: usize,
    scalar_le_hex: String,
    native_and_r1cs_match: bool,
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
struct H2CResult {
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
    candidate_suite_id: &'static str,
    operation_domain: String,
    digest: &'static str,
    digest_bytes: usize,
    digest_integer_endianness: &'static str,
    mapping: &'static str,
    analytical_statistical_distance_upper_bound: &'static str,
    first_case: CaseResult,
    second_case: CaseResult,
    same_constraints: bool,
    same_public_inputs: bool,
    same_witness_variables: bool,
    changed_transcript_rejected: bool,
    fixed_shape_single_hash_relation: bool,
    current_suite_v2_changed: bool,
    candidate_suite_adopted: bool,
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
    let domain = frozen.hash_to_scalar_domain.as_bytes();
    if transcript.len() != frozen.transcript_bytes {
        return Err("frozen H2-C source vector has inconsistent length".into());
    }

    let scalar = encode_scalar(&hash_to_field_wide_v3(domain, &transcript)).to_vec();
    let started = Instant::now();
    let circuit = check_hash_to_field_wide_v3(domain, &transcript, &scalar)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !circuit.satisfied {
        return Err("wide H2F circuit rejected its native output".into());
    }

    let input_bytes = hash_to_field_wide_v3_input(domain, &transcript).len();
    if input_bytes != circuit.h2f_input_bytes {
        return Err("native and circuit H2F wrapper lengths differ".into());
    }

    Ok(CaseRun {
        result: CaseResult {
            source_vector: path.display().to_string(),
            source_seed: frozen.seed,
            source_v2_rejection_counter: frozen.rejection_counter,
            transcript_bytes: transcript.len(),
            h2f_input_bytes: circuit.h2f_input_bytes,
            sha512_blocks: circuit.padded_blocks,
            scalar_le_hex: encode_hex(&scalar),
            native_and_r1cs_match: circuit.satisfied,
            constraints: circuit.constraints,
            public_inputs: circuit.public_inputs,
            witness_variables: circuit.witness_variables,
            witness_ms,
            peak_rss_mib_after_case: peak_rss_mib(),
        },
        transcript,
        scalar,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut first_path = PathBuf::from("fixtures/jubjub_cp_transcript_v3.json");
    let mut second_path = PathBuf::from("fixtures/jubjub_cp_transcript_v3.counter1.json");
    let mut output_path = PathBuf::from("experiments/g0_9_h2_raw/h2c_wide_hash_to_field.json");
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--first-vector" => first_path = PathBuf::from(value),
            "--second-vector" => second_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let CaseRun {
        result: first_case,
        transcript: first_transcript,
        scalar: first_scalar,
    } = run_case(&first_path)?;
    let second_case = run_case(&second_path)?.result;
    let same_constraints = first_case.constraints == second_case.constraints;
    let same_public_inputs = first_case.public_inputs == second_case.public_inputs;
    let same_witness_variables = first_case.witness_variables == second_case.witness_variables;
    if !same_constraints || !same_public_inputs || !same_witness_variables {
        return Err("H2-C source cases do not have the same circuit shape".into());
    }

    let mut changed_transcript = first_transcript;
    changed_transcript[0] ^= 1;
    let changed_transcript_rejected =
        !check_hash_to_field_wide_v3(b"GENERALIZED-CP", &changed_transcript, &first_scalar)?
            .satisfied;
    if !changed_transcript_rejected {
        return Err("wide H2F circuit accepted a changed transcript".into());
    }

    let report = H2CResult {
        gate: "G0.9-H2-C",
        status: "PASS_H2C_WIDE_SHA512_MOD_FR_CANDIDATE_R1CS",
        classification: "isolated-versioned-fixed-shape-hash-to-field-candidate-no-protocol-migration",
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
        candidate_suite_id: WIDE_H2F_CANDIDATE_SUITE_ID,
        operation_domain: "GENERALIZED-CP".to_owned(),
        digest: "SHA-512",
        digest_bytes: 64,
        digest_integer_endianness: "big-endian",
        mapping: "OS2IP_BE(SHA-512(canonical_input)) mod Jubjub-Fr",
        analytical_statistical_distance_upper_bound: "< 2^-259 under a uniform 512-bit digest",
        first_case,
        second_case,
        same_constraints,
        same_public_inputs,
        same_witness_variables,
        changed_transcript_rejected,
        fixed_shape_single_hash_relation: true,
        current_suite_v2_changed: false,
        candidate_suite_adopted: false,
        cp_equations_integrated: false,
        groth16_run: false,
        full_h2: false,
        full_p3: false,
        limitations: vec![
            "this is a versioned candidate mapping and does not alter the current Suite v2 rejection-128 protocol",
            "the analytical bias bound assumes SHA-512 behaves as a uniform 512-bit digest",
            "CP, signatures, manifests and test vectors have not migrated to the candidate suite",
            "the CP group equations and Groth16 setup/prove/verify are outside H2-C",
        ],
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H2-C: constraints={} public-inputs={} witness-ms={:.3}/{:.3} peak-rss={:.3}MiB fixed-shape={} changed-transcript-rejected={} suite-v2-changed=false candidate-adopted=false full-h2=false full-p3=false",
        report.first_case.constraints,
        report.first_case.public_inputs,
        report.first_case.witness_ms,
        report.second_case.witness_ms,
        report.second_case.peak_rss_mib_after_case,
        report.same_constraints && report.same_public_inputs && report.same_witness_variables,
        report.changed_transcript_rejected,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
