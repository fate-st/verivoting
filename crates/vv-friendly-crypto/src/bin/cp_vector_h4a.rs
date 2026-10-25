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
use vv_friendly_crypto::cp_h3_gadget::H3_TRANSCRIPT_BYTES;
use vv_friendly_crypto::cp_h4_gadget::{
    check_complete_trustee_cp_h4a, H4_CIPHERTEXT_C0_OFFSET, H4_COORDINATE_COUNT,
    H4_DECRYPTION_SHARE_OFFSET, H4_PROOF_COORDINATE_OFFSET,
};
use vv_friendly_crypto::{
    encode_point, encode_scalar, encrypt, generator, random_nonzero_scalar, Point, CP_H2F_SUITE_ID,
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
struct H4AResult {
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
    manifest_h4a_capability: String,
    manifest_matches_h4a: bool,
    vector_file: String,
    transcript_bytes: usize,
    transcript_sha256: String,
    adopted_suite_id: &'static str,
    coordinate_count: usize,
    ciphertext_c0_offset: usize,
    decryption_share_offset: usize,
    proof_coordinate_offset: usize,
    native_challenge_matches_vector: bool,
    native_first_equation_valid: bool,
    native_coordinate_equations_valid: usize,
    r1cs_complete_trustee_cp_satisfied: bool,
    transcript_points_bound_in_r1cs: usize,
    cp_equations_constrained_in_r1cs: usize,
    changed_transcript_rejected: bool,
    wrong_decryption_share_rejected: bool,
    wrong_proof_coordinate_rejected: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    h2f_input_bytes: usize,
    sha512_blocks: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    complete_one_trustee_cp: bool,
    multi_trustee_cp_complete: bool,
    groth16_run: bool,
    full_h4a: bool,
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
    let mut output_path = PathBuf::from("experiments/g0_9_h4a_raw/h4a_complete_trustee_cp.json");
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
        return Err("invalid frozen H4-A vector shape".into());
    }
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_h4a_capability = manifest["capabilities"]["h4a_one_trustee_cp"]
        .as_str()
        .ok_or("prototype manifest lacks H4-A capability")?
        .to_owned();
    let manifest_hash_suite = manifest["cp_hash_to_scalar_suite"]
        .as_str()
        .ok_or("prototype manifest lacks CP hash suite")?;
    let manifest_matches_h4a = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-")
        && manifest_h4a_capability == "READY_R1CS_ONE_TRUSTEE_FOUR_EQUATIONS_NO_GROTH16"
        && manifest_hash_suite.starts_with(CP_H2F_SUITE_ID);
    if !manifest_matches_h4a {
        return Err("prototype manifest does not describe the measured H4-A gate".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(vector.seed);
    let secret = random_nonzero_scalar(&mut rng);
    let verification_share = generator().mul_bigint(secret.into_bigint());
    let ciphertexts = (0u64..H4_COORDINATE_COUNT as u64)
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
        return Err("native H4-A transcript differs from the frozen CP-H2F-v3 vector".into());
    }
    let native_challenge_matches_vector = encode_scalar(&challenge).as_slice() == frozen_challenge;
    let native_first_equation_valid = generator().mul_bigint(package.proof.response.into_bigint())
        == package.proof.a0 + verification_share.mul_bigint(challenge.into_bigint());
    let ciphertext_c0 = ciphertexts
        .iter()
        .map(|ciphertext| ciphertext.c0)
        .collect::<Vec<Point>>();
    let native_coordinate_equations_valid = ciphertext_c0
        .iter()
        .zip(&package.shares)
        .zip(&package.proof.coordinates)
        .filter(|((ciphertext, share), commitment)| {
            ciphertext.mul_bigint(package.proof.response.into_bigint())
                == **commitment + share.mul_bigint(challenge.into_bigint())
        })
        .count();
    if !native_challenge_matches_vector
        || !native_first_equation_valid
        || native_coordinate_equations_valid != H4_COORDINATE_COUNT
    {
        return Err("native H4-A fixture failed".into());
    }

    let started = Instant::now();
    let valid = check_complete_trustee_cp_h4a(
        &transcript,
        verification_share,
        &ciphertext_c0,
        &package.shares,
        package.proof.a0,
        &package.proof.coordinates,
        package.proof.response,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;

    let mut changed_transcript = transcript.clone();
    changed_transcript[112] ^= 1;
    let changed_transcript_rejected = !check_complete_trustee_cp_h4a(
        &changed_transcript,
        verification_share,
        &ciphertext_c0,
        &package.shares,
        package.proof.a0,
        &package.proof.coordinates,
        package.proof.response,
    )?
    .satisfied;
    let mut wrong_shares = package.shares.clone();
    wrong_shares[0] += generator();
    let wrong_share_encoding = encode_point(&wrong_shares[0]);
    let mut wrong_share_transcript = transcript.clone();
    wrong_share_transcript
        [H4_DECRYPTION_SHARE_OFFSET..H4_DECRYPTION_SHARE_OFFSET + wrong_share_encoding.len()]
        .copy_from_slice(&wrong_share_encoding);
    let wrong_decryption_share_rejected = !check_complete_trustee_cp_h4a(
        &wrong_share_transcript,
        verification_share,
        &ciphertext_c0,
        &wrong_shares,
        package.proof.a0,
        &package.proof.coordinates,
        package.proof.response,
    )?
    .satisfied;
    let mut wrong_coordinates = package.proof.coordinates.clone();
    wrong_coordinates[1] += generator();
    let wrong_coordinate_encoding = encode_point(&wrong_coordinates[1]);
    let wrong_coordinate_offset = H4_PROOF_COORDINATE_OFFSET + wrong_coordinate_encoding.len();
    let mut wrong_coordinate_transcript = transcript.clone();
    wrong_coordinate_transcript
        [wrong_coordinate_offset..wrong_coordinate_offset + wrong_coordinate_encoding.len()]
        .copy_from_slice(&wrong_coordinate_encoding);
    let wrong_proof_coordinate_rejected = !check_complete_trustee_cp_h4a(
        &wrong_coordinate_transcript,
        verification_share,
        &ciphertext_c0,
        &package.shares,
        package.proof.a0,
        &wrong_coordinates,
        package.proof.response,
    )?
    .satisfied;
    if !valid.satisfied
        || !changed_transcript_rejected
        || !wrong_decryption_share_rejected
        || !wrong_proof_coordinate_rejected
    {
        return Err("H4-A R1CS acceptance boundary failed".into());
    }

    let report = H4AResult {
        gate: "G0.9-H4-A",
        status: "PASS_H4A_COMPLETE_ONE_TRUSTEE_CP_VECTOR_R1CS",
        classification: "one-trustee-four-equation-generalized-cp-with-single-in-circuit-cp-h2f-v3-no-groth16-no-p3",
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
        manifest_h4a_capability,
        manifest_matches_h4a,
        vector_file: vector_path.display().to_string(),
        transcript_bytes: transcript.len(),
        transcript_sha256: vector.transcript_sha256,
        adopted_suite_id: CP_H2F_SUITE_ID,
        coordinate_count: H4_COORDINATE_COUNT,
        ciphertext_c0_offset: H4_CIPHERTEXT_C0_OFFSET,
        decryption_share_offset: H4_DECRYPTION_SHARE_OFFSET,
        proof_coordinate_offset: H4_PROOF_COORDINATE_OFFSET,
        native_challenge_matches_vector,
        native_first_equation_valid,
        native_coordinate_equations_valid,
        r1cs_complete_trustee_cp_satisfied: valid.satisfied,
        transcript_points_bound_in_r1cs: valid.transcript_points_bound,
        cp_equations_constrained_in_r1cs: valid.cp_equations,
        changed_transcript_rejected,
        wrong_decryption_share_rejected,
        wrong_proof_coordinate_rejected,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        h2f_input_bytes: valid.h2f_input_bytes,
        sha512_blocks: valid.sha512_blocks,
        witness_ms,
        peak_rss_mib: peak_rss_mib(),
        complete_one_trustee_cp: true,
        multi_trustee_cp_complete: false,
        groth16_run: false,
        full_h4a: true,
        full_p3: false,
        limitations: vec![
            "H4-A covers one Trustee only; threshold selection and interpolation are not included",
            "the 684-byte transcript remains a packed public input feasibility layout, not the private P3 package layout",
            "Groth16 is deferred to H4-B now that the complete one-Trustee CP relation is available",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H4-A: equations={} bound-points={} constraints={} public-inputs={} witness={:.3}ms peak-rss={:.3}MiB changed-transcript-rejected={} wrong-share-rejected={} wrong-coordinate-rejected={} complete-one-trustee-cp=true groth16=false full-p3=false",
        report.cp_equations_constrained_in_r1cs,
        report.transcript_points_bound_in_r1cs,
        report.constraints,
        report.public_inputs,
        report.witness_ms,
        report.peak_rss_mib,
        report.changed_transcript_rejected,
        report.wrong_decryption_share_rejected,
        report.wrong_proof_coordinate_rejected,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
