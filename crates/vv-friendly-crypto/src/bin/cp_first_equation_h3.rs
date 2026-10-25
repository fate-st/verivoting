use ark_ec::Group;
use ark_ff::PrimeField;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    GeneralizedCpContext,
};
use vv_friendly_crypto::cp_h3_gadget::{
    check_cp_first_equation_h3, H3_A0_OFFSET, H3_TRANSCRIPT_BYTES, H3_VERIFICATION_SHARE_OFFSET,
};
use vv_friendly_crypto::{
    encode_scalar, encrypt, generator, random_nonzero_scalar, CP_H2F_SUITE_ID,
};
use vv_types::sha256;

#[derive(Deserialize)]
struct FrozenVector {
    seed: u64,
    transcript_bytes: usize,
    transcript_hex: String,
    transcript_sha256: String,
    challenge_scalar_le_hex: String,
}

#[derive(Serialize)]
struct H3Result {
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
    manifest_h3_capability: String,
    manifest_matches_h3: bool,
    vector_file: String,
    transcript_bytes: usize,
    transcript_sha256: String,
    verification_share_offset: usize,
    a0_offset: usize,
    adopted_suite_id: &'static str,
    native_challenge_matches_vector: bool,
    native_first_equation_valid: bool,
    r1cs_first_equation_satisfied: bool,
    transcript_points_bound_in_r1cs: bool,
    group_arithmetic_constrained_in_r1cs: bool,
    changed_transcript_rejected: bool,
    wrong_verification_share_rejected: bool,
    wrong_response_rejected: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    h2f_input_bytes: usize,
    sha512_blocks: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    first_cp_equation_complete: bool,
    vector_cp_equations_complete: bool,
    groth16_run: bool,
    full_h3: bool,
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
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
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
    let mut vector_path = PathBuf::from("fixtures/jubjub_cp_transcript_h2f_v3.json");
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut output_path = PathBuf::from("experiments/g0_9_h3_raw/h3_first_cp_equation.json");
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
    let frozen_transcript = decode_hex(&vector.transcript_hex)?;
    let frozen_challenge = decode_hex(&vector.challenge_scalar_le_hex)?;
    if vector.transcript_bytes != H3_TRANSCRIPT_BYTES
        || frozen_transcript.len() != H3_TRANSCRIPT_BYTES
        || frozen_challenge.len() != 32
    {
        return Err("invalid frozen H3 vector shape".into());
    }
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_h3_capability = manifest["capabilities"]["h3_first_cp_equation"]
        .as_str()
        .ok_or("prototype manifest lacks H3 capability")?
        .to_owned();
    let manifest_hash_suite = manifest["cp_hash_to_scalar_suite"]
        .as_str()
        .ok_or("prototype manifest lacks CP hash suite")?;
    let manifest_matches_h3 = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-H")
        && manifest_h3_capability == "READY_R1CS_ONE_TRUSTEE_FIRST_EQUATION_NO_GROTH16"
        && manifest_hash_suite.starts_with(CP_H2F_SUITE_ID);
    if !manifest_matches_h3 {
        return Err("prototype manifest does not describe the measured H3 gate".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(vector.seed);
    let secret = random_nonzero_scalar(&mut rng);
    let verification_share = generator().mul_bigint(secret.into_bigint());
    let ciphertexts = (0u64..3)
        .map(|message| encrypt(&verification_share, message, &mut rng))
        .collect::<Result<Vec<_>, _>>()?;
    let election_id = sha256(b"VV/JUBJUB/G0.9/H0/ELECTION");
    let policy_pre = sha256(b"VV/JUBJUB/G0.9/H0/POLICY-PRE");
    let final_context = sha256(b"VV/JUBJUB/G0.9/H0/FINAL-CONTEXT");
    let registration_close_digest = sha256(b"VV/JUBJUB/G0.9/H0/REGISTRATION-CLOSE");
    let disclosure_digest = sha256(b"VV/JUBJUB/G0.9/H0/DISCLOSURE-CLOSE");
    let target = derive_cp_target_with_registration(
        election_id,
        policy_pre,
        final_context,
        registration_close_digest,
        disclosure_digest,
        &ciphertexts,
    )?;
    let context = GeneralizedCpContext {
        election_id,
        policy_pre,
        final_context,
        registration_close_digest,
        disclosure_digest,
        target,
        trustee_id: 3,
    };
    let package = prove_decryption_share(
        &context,
        secret,
        &verification_share,
        &ciphertexts,
        &mut rng,
    )?;
    let (transcript, challenge) =
        transcript_and_challenge(&context, &verification_share, &ciphertexts, &package)?;
    if transcript != frozen_transcript {
        return Err("native H3 transcript differs from the frozen CP-H2F-v3 vector".into());
    }
    let native_challenge_matches_vector = encode_scalar(&challenge).as_slice() == frozen_challenge;
    let native_first_equation_valid = generator().mul_bigint(package.proof.response.into_bigint())
        == package.proof.a0 + verification_share.mul_bigint(challenge.into_bigint());
    if !native_challenge_matches_vector || !native_first_equation_valid {
        return Err("native H3 fixture failed".into());
    }

    let started = Instant::now();
    let valid = check_cp_first_equation_h3(
        &transcript,
        verification_share,
        package.proof.a0,
        package.proof.response,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;

    let mut changed_transcript = transcript.clone();
    changed_transcript[112] ^= 1;
    let changed_transcript_rejected = !check_cp_first_equation_h3(
        &changed_transcript,
        verification_share,
        package.proof.a0,
        package.proof.response,
    )?
    .satisfied;
    let wrong_verification_share_rejected = !check_cp_first_equation_h3(
        &transcript,
        verification_share + generator(),
        package.proof.a0,
        package.proof.response,
    )?
    .satisfied;
    let wrong_response_rejected = !check_cp_first_equation_h3(
        &transcript,
        verification_share,
        package.proof.a0,
        package.proof.response + vv_friendly_crypto::Scalar::from(1u64),
    )?
    .satisfied;
    if !valid.satisfied
        || !changed_transcript_rejected
        || !wrong_verification_share_rejected
        || !wrong_response_rejected
    {
        return Err("H3 R1CS acceptance boundary failed".into());
    }

    let report = H3Result {
        gate: "G0.9-H3",
        status: "PASS_H3_H2F_BOUND_FIRST_CP_EQUATION_R1CS",
        classification: "one-trustee-first-cp-equation-with-in-circuit-cp-h2f-v3-no-groth16-no-vector-cp-no-p3",
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
        manifest_h3_capability,
        manifest_matches_h3,
        vector_file: vector_path.display().to_string(),
        transcript_bytes: transcript.len(),
        transcript_sha256: vector.transcript_sha256,
        verification_share_offset: H3_VERIFICATION_SHARE_OFFSET,
        a0_offset: H3_A0_OFFSET,
        adopted_suite_id: CP_H2F_SUITE_ID,
        native_challenge_matches_vector,
        native_first_equation_valid,
        r1cs_first_equation_satisfied: valid.satisfied,
        transcript_points_bound_in_r1cs: true,
        group_arithmetic_constrained_in_r1cs: true,
        changed_transcript_rejected,
        wrong_verification_share_rejected,
        wrong_response_rejected,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        h2f_input_bytes: valid.h2f_input_bytes,
        sha512_blocks: valid.sha512_blocks,
        witness_ms,
        peak_rss_mib: peak_rss_mib(),
        first_cp_equation_complete: true,
        vector_cp_equations_complete: false,
        groth16_run: false,
        full_h3: true,
        full_p3: false,
        limitations: vec![
            "H3 covers one Trustee and only zG = A0 + dVK; coordinate equations are not included",
            "the 684-byte transcript is exposed as packed public input for this H3 feasibility gate; full P3 requires a private-package input layout",
            "no Groth16 setup/prove/verify is run before the complete one-Trustee CP relation is assembled",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H3: constraints={} public-inputs={} witness={:.3}ms peak-rss={:.3}MiB transcript-bound={} changed-transcript-rejected={} wrong-vk-rejected={} wrong-response-rejected={} full-h3=true full-p3=false",
        report.constraints,
        report.public_inputs,
        report.witness_ms,
        report.peak_rss_mib,
        report.transcript_points_bound_in_r1cs,
        report.changed_transcript_rejected,
        report.wrong_verification_share_rejected,
        report.wrong_response_rejected,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
