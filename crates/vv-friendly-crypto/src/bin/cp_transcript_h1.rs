use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::transcript_gadget::check_sha512_digest;
use vv_types::hex;

#[derive(Deserialize)]
struct FrozenTranscript {
    transcript_bytes: usize,
    transcript_hex: String,
    transcript_sha512: String,
}

#[derive(Serialize)]
struct H1Result {
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
    vector_file: String,
    transcript_bytes: usize,
    padded_sha512_blocks: usize,
    native_sha512_hex: String,
    frozen_sha512_hex: String,
    native_digest_matches_frozen_vector: bool,
    r1cs_digest_matches_native: bool,
    changed_transcript_rejected: bool,
    changed_public_digest_rejected: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    sha512_recomputed_in_circuit: bool,
    hash_to_scalar_mapping_implemented: bool,
    cp_equations_integrated: bool,
    groth16_run: bool,
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
    let mut vector_path = PathBuf::from("fixtures/jubjub_cp_transcript_v3.json");
    let mut output_path = PathBuf::from("experiments/g0_9_h1_raw/h1_sha512.json");
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--vector" => vector_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let frozen: FrozenTranscript = serde_json::from_slice(&fs::read(&vector_path)?)?;
    let transcript = decode_hex(&frozen.transcript_hex)?;
    let frozen_digest = decode_hex(&frozen.transcript_sha512)?;
    if transcript.len() != frozen.transcript_bytes || frozen_digest.len() != 64 {
        return Err("frozen H0 vector has inconsistent lengths".into());
    }
    let native_digest = Sha512::digest(&transcript).to_vec();
    let native_digest_matches_frozen_vector = native_digest == frozen_digest;
    if !native_digest_matches_frozen_vector {
        return Err("native SHA-512 does not match the frozen H0 vector".into());
    }

    let start = Instant::now();
    let valid = check_sha512_digest(&transcript, &native_digest)?;
    let witness_ms = start.elapsed().as_secs_f64() * 1000.0;

    let mut changed_transcript = transcript.clone();
    changed_transcript[0] ^= 1;
    let changed_transcript_rejected =
        !check_sha512_digest(&changed_transcript, &native_digest)?.satisfied;
    let mut changed_digest = native_digest.clone();
    changed_digest[0] ^= 1;
    let changed_public_digest_rejected =
        !check_sha512_digest(&transcript, &changed_digest)?.satisfied;

    if !valid.satisfied || !changed_transcript_rejected || !changed_public_digest_rejected {
        return Err("G0.9-H1 SHA-512 circuit acceptance failed".into());
    }

    let report = H1Result {
        gate: "G0.9-H1",
        status: "PASS_H1_SHA512_R1CS_EXACT_DIGEST",
        classification: "isolated-real-sha512-r1cs-no-hash-to-scalar-no-groth16",
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
        vector_file: vector_path.display().to_string(),
        transcript_bytes: transcript.len(),
        padded_sha512_blocks: valid.padded_blocks,
        native_sha512_hex: hex(&native_digest),
        frozen_sha512_hex: frozen.transcript_sha512,
        native_digest_matches_frozen_vector,
        r1cs_digest_matches_native: valid.satisfied,
        changed_transcript_rejected,
        changed_public_digest_rejected,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        witness_ms,
        peak_rss_mib: peak_rss_mib(),
        sha512_recomputed_in_circuit: true,
        hash_to_scalar_mapping_implemented: false,
        cp_equations_integrated: false,
        groth16_run: false,
        full_p3: false,
        limitations: vec![
            "H1 hashes the frozen CP transcript directly; it does not yet implement the protocol H2S wrapper or rejection sampling",
            "the CP group equations, package commitments, threshold reconstruction and winner relation are not integrated",
            "no Groth16 setup, proof or verification is run at H1",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H1 SHA-512: bytes={} blocks={} constraints={} public-inputs={} witness-ms={:.3} peak-rss-mib={:.1} exact=true changed-transcript-rejected=true changed-digest-rejected=true full-p3=false",
        report.transcript_bytes,
        report.padded_sha512_blocks,
        report.constraints,
        report.public_inputs,
        report.witness_ms,
        report.peak_rss_mib,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
