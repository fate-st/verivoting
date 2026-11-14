use ark_bls12_381::Bls12_381;
use ark_ec::Group;
use ark_ed_on_bls12_381::Fq;
use ark_ff::{PrimeField, ToConstraintField};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    GeneralizedCpContext,
};
use vv_friendly_crypto::cp_h3_gadget::H3_TRANSCRIPT_BYTES;
use vv_friendly_crypto::cp_h4_gadget::{H4ACircuit, H4_COORDINATE_COUNT};
use vv_friendly_crypto::{encrypt, generator, random_nonzero_scalar, Point, CP_H2F_SUITE_ID};
use vv_types::sha256;

const SETUP_SEED: u64 = 0x4834_422D_5345_5455;
const PROOF_SEED: u64 = 0x4834_422D_5052_4F56;

#[derive(Deserialize)]
struct FrozenVector {
    seed: u64,
    transcript_bytes: usize,
    transcript_hex: String,
    transcript_sha256: String,
}

#[derive(Deserialize)]
struct H4ABaseline {
    status: String,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
}

#[derive(Serialize)]
struct H4BResult {
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
    manifest_h4b_capability: String,
    manifest_matches_h4b: bool,
    h4a_baseline_file: String,
    vector_file: String,
    transcript_bytes: usize,
    transcript_sha256: String,
    adopted_suite_id: &'static str,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    setup_seed: u64,
    proof_seed: u64,
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    changed_public_verify_ms: f64,
    peak_rss_after_setup_mib: f64,
    peak_rss_after_prove_mib: f64,
    peak_rss_after_verify_mib: f64,
    proving_key_compressed_bytes: usize,
    verifying_key_compressed_bytes: usize,
    proof_bytes: usize,
    valid_proof_verified: bool,
    changed_public_transcript_rejected: bool,
    proving_key_persisted: bool,
    complete_one_trustee_cp: bool,
    groth16_run: bool,
    full_h4b: bool,
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
    let mut baseline_path = PathBuf::from("experiments/g0_9_h4a_raw/h4a_complete_trustee_cp.json");
    let mut output_path = PathBuf::from("experiments/g0_9_h4b_raw/h4b_groth16.json");
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--vector" => vector_path = PathBuf::from(value),
            "--manifest" => manifest_path = PathBuf::from(value),
            "--baseline" => baseline_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let vector: FrozenVector = serde_json::from_slice(&fs::read(&vector_path)?)?;
    let transcript = decode_hex(&vector.transcript_hex)?;
    if vector.transcript_bytes != H3_TRANSCRIPT_BYTES || transcript.len() != H3_TRANSCRIPT_BYTES {
        return Err("invalid frozen H4-B vector shape".into());
    }
    let baseline: H4ABaseline = serde_json::from_slice(&fs::read(&baseline_path)?)?;
    if baseline.status != "PASS_H4A_COMPLETE_ONE_TRUSTEE_CP_VECTOR_R1CS" {
        return Err("H4-A baseline is not a passing measured artifact".into());
    }
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_h4b_capability = manifest["capabilities"]["h4b_one_trustee_cp_groth16"]
        .as_str()
        .ok_or("prototype manifest lacks H4-B capability")?
        .to_owned();
    let manifest_hash_suite = manifest["cp_hash_to_scalar_suite"]
        .as_str()
        .ok_or("prototype manifest lacks CP hash suite")?;
    let manifest_matches_h4b = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-")
        && manifest_h4b_capability == "READY_REAL_GROTH16_ONE_TRUSTEE_CP"
        && manifest_hash_suite.starts_with(CP_H2F_SUITE_ID);
    if !manifest_matches_h4b {
        return Err("prototype manifest does not describe the measured H4-B gate".into());
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
    let (native_transcript, _) =
        transcript_and_challenge(&context, &verification_share, &ciphertexts, &package)?;
    if native_transcript != transcript {
        return Err("native H4-B transcript differs from the frozen vector".into());
    }
    let ciphertext_c0 = ciphertexts
        .iter()
        .map(|ciphertext| ciphertext.c0)
        .collect::<Vec<Point>>();
    let circuit = H4ACircuit {
        transcript: transcript.clone(),
        verification_share,
        ciphertext_c0,
        decryption_shares: package.shares,
        a0: package.proof.a0,
        proof_coordinates: package.proof.coordinates,
        response: package.proof.response,
    };
    if !circuit.has_expected_shape() {
        return Err("H4-B circuit does not have the H4-A shape".into());
    }
    let public_inputs: Vec<Fq> = transcript
        .to_field_elements()
        .ok_or("failed to pack H4-B public transcript")?;
    if public_inputs.len() != baseline.public_inputs {
        return Err("H4-B public input packing differs from H4-A".into());
    }

    let mut setup_rng = ChaCha20Rng::seed_from_u64(SETUP_SEED);
    let setup_started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut setup_rng,
    )?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_setup_mib = peak_rss_mib();

    let mut proof_rng = ChaCha20Rng::seed_from_u64(PROOF_SEED);
    let prove_started = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit,
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

    let mut changed_transcript = transcript;
    changed_transcript[112] ^= 1;
    let changed_public_inputs: Vec<Fq> = changed_transcript
        .to_field_elements()
        .ok_or("failed to pack changed H4-B transcript")?;
    let changed_verify_started = Instant::now();
    let changed_public_transcript_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed_public_inputs)?;
    let changed_public_verify_ms = changed_verify_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_verify_mib = peak_rss_mib();
    if !valid_proof_verified || !changed_public_transcript_rejected {
        return Err("H4-B Groth16 acceptance boundary failed".into());
    }

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;
    let report = H4BResult {
        gate: "G0.9-H4-B",
        status: "PASS_H4B_REAL_GROTH16_ONE_TRUSTEE_CP",
        classification: "real-groth16-over-complete-one-trustee-cp-h2f-v3-relation-not-multi-trustee-not-p3",
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
        manifest_h4b_capability,
        manifest_matches_h4b,
        h4a_baseline_file: baseline_path.display().to_string(),
        vector_file: vector_path.display().to_string(),
        transcript_bytes: vector.transcript_bytes,
        transcript_sha256: vector.transcript_sha256,
        adopted_suite_id: CP_H2F_SUITE_ID,
        constraints: baseline.constraints,
        public_inputs: baseline.public_inputs,
        witness_variables: baseline.witness_variables,
        setup_seed: SETUP_SEED,
        proof_seed: PROOF_SEED,
        setup_ms,
        prove_ms,
        verify_ms,
        changed_public_verify_ms,
        peak_rss_after_setup_mib,
        peak_rss_after_prove_mib,
        peak_rss_after_verify_mib,
        proving_key_compressed_bytes: parameters.compressed_size(),
        verifying_key_compressed_bytes: parameters.vk.compressed_size(),
        proof_bytes: proof_encoding.len(),
        valid_proof_verified,
        changed_public_transcript_rejected,
        proving_key_persisted: false,
        complete_one_trustee_cp: true,
        groth16_run: true,
        full_h4b: true,
        full_p3: false,
        limitations: vec![
            "the Groth16 proof covers one Trustee package, not the threshold set",
            "the H4-A public transcript feasibility layout is retained; private-package P3 inputs are not implemented",
            "the deterministic experiment seeds are reproducibility inputs, not a production trusted setup ceremony",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H4-B: constraints={} public-inputs={} setup={:.3}ms prove={:.3}ms verify={:.3}ms proof={}B pk={}B vk={}B peak-rss={:.3}MiB valid={} changed-public-rejected={} full-p3=false",
        report.constraints,
        report.public_inputs,
        report.setup_ms,
        report.prove_ms,
        report.verify_ms,
        report.proof_bytes,
        report.proving_key_compressed_bytes,
        report.verifying_key_compressed_bytes,
        report.peak_rss_after_verify_mib,
        report.valid_proof_verified,
        report.changed_public_transcript_rejected,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
