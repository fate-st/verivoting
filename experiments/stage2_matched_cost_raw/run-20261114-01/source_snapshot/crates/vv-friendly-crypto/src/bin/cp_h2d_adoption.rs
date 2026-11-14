use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::{
    cp_hash_to_field_v3, cp_hash_to_field_v3_input, encode_scalar,
    hash_to_field_gadget::check_cp_hash_to_field_v3, CP_H2F_SUITE_ID,
};

#[derive(Deserialize)]
struct FrozenVector {
    hash_to_field_domain: String,
    hash_to_field_suite_id: String,
    transcript_bytes: usize,
    transcript_hex: String,
    transcript_sha256: String,
    challenge_scalar_le_hex: String,
}

#[derive(Serialize)]
struct H2DResult {
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
    cp_hash_to_scalar_suite: String,
    manifest_suite_matches: bool,
    adopted_suite_id: &'static str,
    operation_domain: String,
    vector_file: String,
    transcript_bytes: usize,
    transcript_sha256: String,
    h2f_input_bytes: usize,
    sha512_blocks: usize,
    scalar_le_hex: String,
    native_matches_frozen_vector: bool,
    r1cs_matches_native: bool,
    changed_transcript_rejected: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    proof_rejection_counter_serialized: bool,
    native_cp_migrated: bool,
    fixed_shape_cp_hash_suite_adopted: bool,
    cp_equations_integrated_in_r1cs: bool,
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut vector_path = PathBuf::from("fixtures/jubjub_cp_transcript_h2f_v3.json");
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut output_path = PathBuf::from("experiments/g0_9_h2d_raw/h2d_adoption.json");
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--vector" => vector_path = PathBuf::from(value),
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let vector: FrozenVector = serde_json::from_slice(&fs::read(&vector_path)?)?;
    let transcript = decode_hex(&vector.transcript_hex)?;
    let frozen_scalar = decode_hex(&vector.challenge_scalar_le_hex)?;
    if transcript.len() != vector.transcript_bytes
        || frozen_scalar.len() != 32
        || vector.hash_to_field_suite_id != CP_H2F_SUITE_ID
    {
        return Err("invalid adopted CP-H2F-v3 vector".into());
    }

    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let cp_hash_to_scalar_suite = manifest["cp_hash_to_scalar_suite"]
        .as_str()
        .ok_or("prototype manifest lacks cp_hash_to_scalar_suite")?
        .to_owned();
    let manifest_suite_matches = cp_hash_to_scalar_suite.starts_with(CP_H2F_SUITE_ID);
    if !manifest_suite_matches {
        return Err("prototype manifest does not pin adopted CP-H2F-v3".into());
    }

    let native_scalar = encode_scalar(&cp_hash_to_field_v3(
        vector.hash_to_field_domain.as_bytes(),
        &transcript,
    ));
    let native_matches_frozen_vector = native_scalar.as_slice() == frozen_scalar;
    if !native_matches_frozen_vector {
        return Err("native adopted suite does not match the frozen vector".into());
    }

    let started = Instant::now();
    let circuit = check_cp_hash_to_field_v3(
        vector.hash_to_field_domain.as_bytes(),
        &transcript,
        &native_scalar,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !circuit.satisfied {
        return Err("adopted CP-H2F-v3 circuit rejected the native result".into());
    }
    let input_bytes =
        cp_hash_to_field_v3_input(vector.hash_to_field_domain.as_bytes(), &transcript).len();
    if input_bytes != circuit.h2f_input_bytes {
        return Err("native and circuit adopted H2F wrapper lengths differ".into());
    }

    let mut changed_transcript = transcript;
    changed_transcript[0] ^= 1;
    let changed_transcript_rejected = !check_cp_hash_to_field_v3(
        vector.hash_to_field_domain.as_bytes(),
        &changed_transcript,
        &native_scalar,
    )?
    .satisfied;
    if !changed_transcript_rejected {
        return Err("adopted CP-H2F-v3 accepted a changed transcript".into());
    }

    let report = H2DResult {
        gate: "G0.9-H2-D",
        status: "PASS_H2D_CP_H2F_V3_ADOPTED",
        classification: "prototype-manifest-pinned-fixed-shape-cp-hash-suite-no-cp-r1cs-no-groth16",
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
        cp_hash_to_scalar_suite,
        manifest_suite_matches,
        adopted_suite_id: CP_H2F_SUITE_ID,
        operation_domain: vector.hash_to_field_domain,
        vector_file: vector_path.display().to_string(),
        transcript_bytes: vector.transcript_bytes,
        transcript_sha256: vector.transcript_sha256,
        h2f_input_bytes: circuit.h2f_input_bytes,
        sha512_blocks: circuit.padded_blocks,
        scalar_le_hex: encode_hex(&native_scalar),
        native_matches_frozen_vector,
        r1cs_matches_native: circuit.satisfied,
        changed_transcript_rejected,
        constraints: circuit.constraints,
        public_inputs: circuit.public_inputs,
        witness_variables: circuit.witness_variables,
        witness_ms,
        peak_rss_mib: peak_rss_mib(),
        proof_rejection_counter_serialized: false,
        native_cp_migrated: true,
        fixed_shape_cp_hash_suite_adopted: true,
        cp_equations_integrated_in_r1cs: false,
        groth16_run: false,
        full_h2: true,
        full_p3: false,
        limitations: vec![
            "H2-D completes the standalone CP hash-to-field gate, not the CP group-equation circuit",
            "Schnorr signatures and non-CP hash users remain on their existing domain-separated H2S-v2 suites",
            "the official Ristretto manifest is unchanged",
            "no Groth16 setup/prove/verify is run in H2-D",
        ],
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H2-D: suite={} constraints={} public-inputs={} witness={:.3}ms peak-rss={:.3}MiB manifest-match={} native-r1cs-match={} changed-transcript-rejected={} full-h2=true full-p3=false",
        report.adopted_suite_id,
        report.constraints,
        report.public_inputs,
        report.witness_ms,
        report.peak_rss_mib,
        report.manifest_suite_matches,
        report.r1cs_matches_native,
        report.changed_transcript_rejected,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
