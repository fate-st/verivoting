use ark_ec::Group;
use ark_ff::PrimeField;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use sha2::{Digest, Sha512};
use std::{env, error::Error, fs, path::PathBuf, process::Command};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    GeneralizedCpContext,
};
use vv_friendly_crypto::transcript_gadget::check_transcript_byte_binding;
use vv_friendly_crypto::{
    encode_point, encode_scalar, encrypt, generator, random_nonzero_scalar, SUITE_ID,
};
use vv_types::{hex, sha256};

#[derive(Serialize)]
struct TranscriptVector {
    classification: &'static str,
    suite_id: &'static str,
    codec_magic: &'static str,
    transcript_domain: &'static str,
    hash_to_scalar_domain: &'static str,
    field_order: Vec<&'static str>,
    seed: u64,
    election_id: String,
    policy_pre: String,
    final_context: String,
    registration_close_digest: String,
    disclosure_digest: String,
    target: String,
    trustee_id: u32,
    verification_share: String,
    ciphertext_c0: Vec<String>,
    decryption_shares: Vec<String>,
    proof_a0: String,
    proof_coordinates: Vec<String>,
    transcript_bytes: usize,
    transcript_hex: String,
    transcript_sha256: String,
    transcript_sha512: String,
    challenge_scalar_le_hex: String,
    rejection_counter: u16,
}

#[derive(Serialize)]
struct H0Result {
    gate: &'static str,
    status: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    vector_file: String,
    vector: TranscriptVector,
    r1cs_byte_binding_satisfied: bool,
    r1cs_constraints: usize,
    r1cs_public_inputs: usize,
    r1cs_witness_variables: usize,
    swapped_registration_fields_rejected: bool,
    changed_registration_digest_rejected: bool,
    exact_transcript_layout_frozen: bool,
    sha512_circuit_implemented: bool,
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut report_output = PathBuf::from("experiments/g0_9_h0_raw/h0_transcript.json");
    let mut vector_output = PathBuf::from("fixtures/jubjub_cp_transcript_v3.json");
    let mut seed = 20261021u64;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--output" => report_output = PathBuf::from(value),
            "--vector-output" => vector_output = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let git_commit =
        command_output("git", &["rev-parse", "HEAD"]).unwrap_or_else(|| "UNKNOWN".to_owned());
    let git_dirty = command_output(
        "git",
        &["status", "--porcelain", "--untracked-files=normal"],
    )
    .map_or_else(
        || "UNKNOWN".to_owned(),
        |value| (!value.is_empty()).to_string(),
    );
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
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
    let (transcript, challenge, rejection_counter) =
        transcript_and_challenge(&context, &verification_share, &ciphertexts, &package)?;

    let valid_binding = check_transcript_byte_binding(&transcript, &transcript)?;
    let mut swapped_context = context.clone();
    std::mem::swap(
        &mut swapped_context.registration_close_digest,
        &mut swapped_context.disclosure_digest,
    );
    let (swapped_transcript, _, _) = transcript_and_challenge(
        &swapped_context,
        &verification_share,
        &ciphertexts,
        &package,
    )?;
    let swapped_registration_fields_rejected =
        !check_transcript_byte_binding(&transcript, &swapped_transcript)?.satisfied;
    let mut changed_context = context.clone();
    changed_context.registration_close_digest[0] ^= 1;
    let (changed_transcript, _, _) = transcript_and_challenge(
        &changed_context,
        &verification_share,
        &ciphertexts,
        &package,
    )?;
    let changed_registration_digest_rejected =
        !check_transcript_byte_binding(&transcript, &changed_transcript)?.satisfied;
    if !valid_binding.satisfied
        || !swapped_registration_fields_rejected
        || !changed_registration_digest_rejected
    {
        return Err("G0.9-H0 transcript binding acceptance failed".into());
    }

    let transcript_sha512 = Sha512::digest(&transcript);
    let vector = TranscriptVector {
        classification: "canonical-registration-bound-generalized-cp-transcript-v3",
        suite_id: SUITE_ID,
        codec_magic: "VV-CANONICAL-V1",
        transcript_domain: "VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3",
        hash_to_scalar_domain: "GENERALIZED-CP",
        field_order: vec![
            "codec_magic:length+bytes",
            "transcript_domain:length+bytes",
            "suite_id:length+bytes",
            "election_id:32",
            "policy_pre:32",
            "final_context:32",
            "registration_close_digest:32",
            "disclosure_digest:32",
            "target:32",
            "trustee_id:u32be",
            "verification_share:32",
            "ciphertext_c0_count:u64be",
            "ciphertext_c0[count]:32",
            "decryption_share_count:u64be",
            "decryption_shares[count]:32",
            "proof_a0:32",
            "proof_coordinate_count:u64be",
            "proof_coordinates[count]:32",
        ],
        seed,
        election_id: hex(&election_id),
        policy_pre: hex(&policy_pre),
        final_context: hex(&final_context),
        registration_close_digest: hex(&registration_close_digest),
        disclosure_digest: hex(&disclosure_digest),
        target: hex(&target),
        trustee_id: context.trustee_id,
        verification_share: hex(&encode_point(&verification_share)),
        ciphertext_c0: ciphertexts
            .iter()
            .map(|ciphertext| hex(&encode_point(&ciphertext.c0)))
            .collect(),
        decryption_shares: package
            .shares
            .iter()
            .map(|share| hex(&encode_point(share)))
            .collect(),
        proof_a0: hex(&encode_point(&package.proof.a0)),
        proof_coordinates: package
            .proof
            .coordinates
            .iter()
            .map(|point| hex(&encode_point(point)))
            .collect(),
        transcript_bytes: transcript.len(),
        transcript_hex: hex(&transcript),
        transcript_sha256: hex(&sha256(&transcript)),
        transcript_sha512: hex(&transcript_sha512),
        challenge_scalar_le_hex: hex(&encode_scalar(&challenge)),
        rejection_counter,
    };

    if let Some(parent) = vector_output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&vector_output, serde_json::to_vec_pretty(&vector)?)?;
    let report = H0Result {
        gate: "G0.9-H0",
        status: "PASS_H0_CANONICAL_TRANSCRIPT_R1CS_BOUND_NO_HASH",
        git_commit,
        git_dirty,
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
        vector_file: vector_output.display().to_string(),
        vector,
        r1cs_byte_binding_satisfied: valid_binding.satisfied,
        r1cs_constraints: valid_binding.constraints,
        r1cs_public_inputs: valid_binding.public_inputs,
        r1cs_witness_variables: valid_binding.witness_variables,
        swapped_registration_fields_rejected,
        changed_registration_digest_rejected,
        exact_transcript_layout_frozen: true,
        sha512_circuit_implemented: false,
        groth16_run: false,
        full_p3: false,
        limitations: vec![
            "H0 binds transcript bytes only; it does not compute SHA-512 in R1CS",
            "the R1CS public-byte layout is a feasibility boundary, not the final packed P3 public input layout",
            "no Groth16 proof is generated for this byte-binding relation",
        ],
    };
    if let Some(parent) = report_output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&report_output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-H0 transcript: bytes={} constraints={} public-inputs={} exact=true swapped-fields-rejected=true changed-digest-rejected=true sha512-circuit=false full-p3=false",
        report.vector.transcript_bytes,
        report.r1cs_constraints,
        report.r1cs_public_inputs,
    );
    println!("wrote {}", vector_output.display());
    println!("wrote {}", report_output.display());
    Ok(())
}
