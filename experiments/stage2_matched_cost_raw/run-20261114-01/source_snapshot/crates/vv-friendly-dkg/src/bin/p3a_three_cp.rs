use ark_bls12_381::Bls12_381;
use ark_ed_on_bls12_381::Fq;
use ark_ff::ToConstraintField;
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    verify_decryption_share, GeneralizedCpContext,
};
use vv_friendly_crypto::cp_h4_gadget::{H4ACircuit, H4_DECRYPTION_SHARE_OFFSET};
use vv_friendly_crypto::p3a_gadget::{
    check_three_trustee_cp_p3a, P3ACircuit, P3A_PRIVATE_TRANSCRIPT_BYTES_PER_TRUSTEE,
    P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE, P3A_TRUSTEE_COUNT,
};
use vv_friendly_crypto::{encode_point, encrypt_weighted_vote, fold_ballots, CP_H2F_SUITE_ID};
use vv_friendly_dkg::registration::{
    verify_close, RegistrationAuthority, RegistrationEntry, RegistrationPolicy,
};
use vv_friendly_dkg::{run_dkg, AuthenticationKeypair, DkgConfig, FaultPlan};
use vv_types::{derive_final_context, hex, sha256};

const RULE_ID: &str = "highest-weight-smallest-id-on-tie";
const DEFAULT_SEED: u64 = 20_261_023;
const LAYOUT_VERSION: &str = "VV/JUBJUB/P3-A/INPUT-LAYOUT/v1";
const SETUP_SEED: u64 = 0x5033_412D_5345_5455;
const PROOF_SEED: u64 = 0x5033_412D_5052_4F56;

#[derive(Serialize)]
struct PublicTrusteeLayout {
    trustee_id: u32,
    public_prefix_bytes: usize,
    public_prefix_hex: String,
    public_prefix_sha256: String,
    full_transcript_sha256: String,
}

#[derive(Serialize)]
struct PublicLayoutFixture {
    layout_version: &'static str,
    protocol_version: String,
    cp_suite_id: &'static str,
    seed: u64,
    selected_trustees: Vec<u32>,
    shared_election_id: String,
    shared_policy_pre: String,
    shared_final_context: String,
    shared_registration_close_digest: String,
    shared_disclosure_digest: String,
    shared_target: String,
    public_fields_per_trustee: Vec<&'static str>,
    private_witness_fields_per_trustee: Vec<&'static str>,
    public_prefix_bytes_per_trustee: usize,
    private_transcript_bytes_per_trustee: usize,
    response_scalar_bytes_per_trustee: usize,
    trustees: Vec<PublicTrusteeLayout>,
    private_values_emitted: bool,
}

#[derive(Serialize)]
struct P3AResult {
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
    manifest_p3a_capability: String,
    manifest_matches_p3a: bool,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    selected_trustees: Vec<u32>,
    selected_trustees_distinct: bool,
    shared_statement_matches: bool,
    native_cp_verified: usize,
    trustees_in_r1cs: usize,
    cp_equations_in_r1cs: usize,
    public_prefix_bytes_per_trustee: usize,
    private_transcript_bytes_per_trustee: usize,
    private_response_scalar_bytes_per_trustee: usize,
    private_cp_values_emitted: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    valid_relation_satisfied: bool,
    wrong_third_trustee_share_rejected: bool,
    groth16_run: bool,
    package_commitment_openings_in_r1cs: bool,
    lagrange_and_tally_in_r1cs: bool,
    winner_logic_in_r1cs: bool,
    p3a_r1cs_complete: bool,
    full_p3a: bool,
    full_p3: bool,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct P3AGroth16Result {
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
    manifest_p3a_groth16_capability: String,
    manifest_matches_p3a_groth16: bool,
    p3a_r1cs_file: String,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    selected_trustees: Vec<u32>,
    trustees_in_circuit: usize,
    cp_equations_in_circuit: usize,
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
    changed_public_prefix_rejected: bool,
    proving_key_persisted: bool,
    private_cp_values_emitted: bool,
    package_commitment_openings_in_circuit: bool,
    lagrange_and_tally_in_circuit: bool,
    winner_logic_in_circuit: bool,
    p3a_r1cs_complete: bool,
    p3a_groth16_complete: bool,
    full_p3a: bool,
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

fn pack_public_inputs(
    circuits: &[H4ACircuit],
    mutation: Option<(usize, usize)>,
) -> Result<Vec<Fq>, Box<dyn Error>> {
    let mut packed = Vec::new();
    for (trustee_index, circuit) in circuits.iter().enumerate() {
        let mut prefix = circuit.transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE].to_vec();
        if let Some((mutated_trustee, byte_offset)) = mutation {
            if trustee_index == mutated_trustee {
                let byte = prefix
                    .get_mut(byte_offset)
                    .ok_or("public-prefix mutation offset is out of bounds")?;
                *byte ^= 1;
            }
        }
        let fields: Vec<Fq> = prefix
            .to_field_elements()
            .ok_or("failed to pack P3-A public prefix")?;
        packed.extend(fields);
    }
    Ok(packed)
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
    let mut output_path = PathBuf::from("experiments/g0_9_p3a_raw/p3a_three_cp_r1cs.json");
    let mut groth16_output_path =
        PathBuf::from("experiments/g0_9_p3a_raw/p3a_three_cp_groth16.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_p3a_public_layout_v1.json");
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--groth16-output" => groth16_output_path = PathBuf::from(value),
            "--layout-output" => layout_path = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_p3a_capability = manifest["capabilities"]["p3a_three_trustee_cp_r1cs"]
        .as_str()
        .ok_or("prototype manifest lacks P3-A capability")?
        .to_owned();
    let manifest_p3a_groth16_capability = manifest["capabilities"]["p3a_three_trustee_cp_groth16"]
        .as_str()
        .ok_or("prototype manifest lacks P3-A Groth16 capability")?
        .to_owned();
    let manifest_matches_p3a = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-")
        && manifest_p3a_capability
            == "READY_R1CS_THREE_SELECTED_TRUSTEE_CP_NO_PACKAGE_OPENING_NO_GROTH16"
        && manifest_p3a_groth16_capability
            == "READY_REAL_GROTH16_THREE_SELECTED_TRUSTEE_CP_NO_PACKAGE_OPENING"
        && manifest["cp_hash_to_scalar_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(CP_H2F_SUITE_ID));
    if !manifest_matches_p3a {
        return Err("prototype manifest does not describe the measured P3-A gate".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let election_id = sha256(b"VV/JUBJUB/G0.9/P3A/THREE-CP");
    let registration_authority = RegistrationAuthority::generate(&mut rng);
    let registration_policy =
        RegistrationPolicy::new(registration_authority.public_key(), 10, RULE_ID)?;
    let registration_policy_digest = registration_policy.digest(election_id);
    let authentication_keys = (0..5)
        .map(|_| AuthenticationKeypair::generate(&mut rng))
        .collect::<Vec<_>>();
    let config = DkgConfig::new_with_registration_policy(
        election_id,
        sha256(&manifest_bytes),
        authentication_keys.iter().map(|key| key.public).collect(),
        registration_policy_digest,
        2,
        3,
    )?;
    let dkg = run_dkg(config, &authentication_keys, FaultPlan::default(), &mut rng)?;
    let final_context =
        derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);

    let weights = [3u64, 2, 1, 4];
    let choices = [1usize, 2, 1, 3];
    let entries = weights
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            RegistrationEntry::new(
                sha256(format!("p3a-voter-credential-{index}").as_bytes()),
                *weight,
                &mut rng,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let certificate = registration_authority.close(
        entries,
        dkg.config.election_id,
        final_context,
        dkg.config.policy_pre,
        &registration_policy,
        &mut rng,
    )?;
    let registration = verify_close(
        &certificate,
        &registration_policy,
        dkg.config.election_id,
        final_context,
        dkg.config.policy_pre,
    )?;
    let ballots = weights
        .iter()
        .zip(choices)
        .map(|(weight, choice)| {
            encrypt_weighted_vote(
                &dkg.public_key,
                *weight,
                choice,
                3,
                registration.w_registered(),
                &mut rng,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let aggregate = fold_ballots(&ballots)?;
    let registration_close_digest = registration.registration_close_digest();
    let disclosure_digest = registration.disclosure_digest();
    let target = derive_cp_target_with_registration(
        dkg.config.election_id,
        dkg.config.policy_pre,
        final_context,
        registration_close_digest,
        disclosure_digest,
        &aggregate,
    )?;
    let ciphertext_c0 = aggregate
        .iter()
        .map(|ciphertext| ciphertext.c0)
        .collect::<Vec<_>>();

    let selected_trustees = (1..=P3A_TRUSTEE_COUNT as u32).collect::<Vec<_>>();
    let selected_trustees_distinct = selected_trustees.windows(2).all(|pair| pair[0] < pair[1]);
    let mut circuits = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut public_layouts = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut native_cp_verified = 0usize;
    for trustee_id in &selected_trustees {
        let context = GeneralizedCpContext {
            election_id: dkg.config.election_id,
            policy_pre: dkg.config.policy_pre,
            final_context,
            registration_close_digest,
            disclosure_digest,
            target,
            trustee_id: *trustee_id,
        };
        let verification_share = *dkg.verification_share(*trustee_id)?;
        let package = prove_decryption_share(
            &context,
            dkg.secret_share(*trustee_id)?,
            &verification_share,
            &aggregate,
            &mut rng,
        )?;
        verify_decryption_share(&context, &verification_share, &aggregate, &package)?;
        native_cp_verified += 1;
        let (transcript, _) =
            transcript_and_challenge(&context, &verification_share, &aggregate, &package)?;
        let public_prefix = &transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE];
        public_layouts.push(PublicTrusteeLayout {
            trustee_id: *trustee_id,
            public_prefix_bytes: public_prefix.len(),
            public_prefix_hex: hex(public_prefix),
            public_prefix_sha256: hex(&sha256(public_prefix)),
            full_transcript_sha256: hex(&sha256(&transcript)),
        });
        circuits.push(H4ACircuit {
            transcript,
            verification_share,
            ciphertext_c0: ciphertext_c0.clone(),
            decryption_shares: package.shares,
            a0: package.proof.a0,
            proof_coordinates: package.proof.coordinates,
            response: package.proof.response,
        });
    }

    let shared_statement_matches = circuits.windows(2).all(|pair| {
        let left = &pair[0].transcript;
        let right = &pair[1].transcript;
        left[..304] == right[..304]
            && left[340..H4_DECRYPTION_SHARE_OFFSET] == right[340..H4_DECRYPTION_SHARE_OFFSET]
    });
    if !selected_trustees_distinct
        || !shared_statement_matches
        || native_cp_verified != P3A_TRUSTEE_COUNT
    {
        return Err("native P3-A fixture is not three distinct, shared-statement CPs".into());
    }

    let started = Instant::now();
    let valid = check_three_trustee_cp_p3a(&circuits)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let r1cs_peak_rss_mib = peak_rss_mib();

    let mut wrong_circuits = circuits.clone();
    wrong_circuits[2].decryption_shares[0] += vv_friendly_crypto::generator();
    let wrong_encoding = encode_point(&wrong_circuits[2].decryption_shares[0]);
    wrong_circuits[2].transcript
        [H4_DECRYPTION_SHARE_OFFSET..H4_DECRYPTION_SHARE_OFFSET + wrong_encoding.len()]
        .copy_from_slice(&wrong_encoding);
    let wrong_third_trustee_share_rejected =
        !check_three_trustee_cp_p3a(&wrong_circuits)?.satisfied;
    if !valid.satisfied || !wrong_third_trustee_share_rejected {
        return Err("P3-A R1CS acceptance boundary failed".into());
    }

    let layout = PublicLayoutFixture {
        layout_version: LAYOUT_VERSION,
        protocol_version: protocol_version.clone(),
        cp_suite_id: CP_H2F_SUITE_ID,
        seed,
        selected_trustees: selected_trustees.clone(),
        shared_election_id: hex(&dkg.config.election_id),
        shared_policy_pre: hex(&dkg.config.policy_pre),
        shared_final_context: hex(&final_context),
        shared_registration_close_digest: hex(&registration_close_digest),
        shared_disclosure_digest: hex(&disclosure_digest),
        shared_target: hex(&target),
        public_fields_per_trustee: vec![
            "canonical CP domain and suite",
            "election and frozen policy/context digests",
            "registration-close and disclosure digests",
            "frozen ciphertext target",
            "selected Trustee ID and verification share",
            "coordinate count and aggregate ciphertext C0 vector",
        ],
        private_witness_fields_per_trustee: vec![
            "decryption-share vector",
            "CP A0 and coordinate commitment vector",
            "CP response scalar",
        ],
        public_prefix_bytes_per_trustee: P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE,
        private_transcript_bytes_per_trustee: P3A_PRIVATE_TRANSCRIPT_BYTES_PER_TRUSTEE,
        response_scalar_bytes_per_trustee: 32,
        trustees: public_layouts,
        private_values_emitted: false,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let report = P3AResult {
        gate: "G0.9-P3-A",
        status: "PASS_P3A_THREE_SELECTED_TRUSTEE_CP_R1CS",
        classification: "three-selected-trustee-complete-cp-r1cs-private-shares-and-proofs-no-package-openings-no-groth16-not-full-p3",
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
        manifest_p3a_capability,
        manifest_matches_p3a,
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        selected_trustees: selected_trustees.clone(),
        selected_trustees_distinct,
        shared_statement_matches,
        native_cp_verified,
        trustees_in_r1cs: valid.trustees,
        cp_equations_in_r1cs: valid.cp_equations,
        public_prefix_bytes_per_trustee: valid.public_prefix_bytes_per_trustee,
        private_transcript_bytes_per_trustee: valid.private_transcript_bytes_per_trustee,
        private_response_scalar_bytes_per_trustee: 32,
        private_cp_values_emitted: false,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        witness_ms,
        peak_rss_mib: r1cs_peak_rss_mib,
        valid_relation_satisfied: valid.satisfied,
        wrong_third_trustee_share_rejected,
        groth16_run: false,
        package_commitment_openings_in_r1cs: false,
        lagrange_and_tally_in_r1cs: false,
        winner_logic_in_r1cs: false,
        p3a_r1cs_complete: true,
        full_p3a: false,
        full_p3: false,
        limitations: vec![
            "P3-A covers exactly three selected complete CP relations and stops before package commitments",
            "the verifier must bind the public prefixes to the accepted election state and selected Trustee set",
            "the real Groth16 measurement for this relation is reported in a separate P3-A-G16 artifact",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-P3-A: trustees={:?} CP-equations={} constraints={} public-inputs={} witness-vars={} witness={:.3}ms peak-rss={:.3}MiB valid={} wrong-third-share-rejected={} private-values-emitted=false groth16=false full-p3=false",
        report.selected_trustees,
        report.cp_equations_in_r1cs,
        report.constraints,
        report.public_inputs,
        report.witness_variables,
        report.witness_ms,
        report.peak_rss_mib,
        report.valid_relation_satisfied,
        report.wrong_third_trustee_share_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());

    let circuit = P3ACircuit {
        trustees: circuits.clone(),
    };
    let public_inputs = pack_public_inputs(&circuits, None)?;
    if public_inputs.len() != valid.public_inputs {
        return Err("P3-A Groth16 public input packing differs from the R1CS layout".into());
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

    let changed_public_inputs = pack_public_inputs(&circuits, Some((0, 112)))?;
    let changed_verify_started = Instant::now();
    let changed_public_prefix_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed_public_inputs)?;
    let changed_public_verify_ms = changed_verify_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_verify_mib = peak_rss_mib();
    if !valid_proof_verified || !changed_public_prefix_rejected {
        return Err("P3-A Groth16 acceptance boundary failed".into());
    }

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;
    let groth16_report = P3AGroth16Result {
        gate: "G0.9-P3-A-G16",
        status: "PASS_P3A_REAL_GROTH16_THREE_SELECTED_TRUSTEE_CP",
        classification: "real-groth16-over-three-selected-trustee-complete-cp-relations-private-shares-and-proofs-no-package-openings-not-full-p3",
        git_commit: report.git_commit.clone(),
        git_dirty: report.git_dirty.clone(),
        build_profile: report.build_profile,
        rustc_version: report.rustc_version.clone(),
        os: report.os,
        arch: report.arch,
        cpu: report.cpu.clone(),
        prototype_manifest: manifest_path.display().to_string(),
        protocol_version: report.protocol_version.clone(),
        manifest_p3a_groth16_capability,
        manifest_matches_p3a_groth16: manifest_matches_p3a,
        p3a_r1cs_file: output_path.display().to_string(),
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        selected_trustees,
        trustees_in_circuit: valid.trustees,
        cp_equations_in_circuit: valid.cp_equations,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
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
        changed_public_prefix_rejected,
        proving_key_persisted: false,
        private_cp_values_emitted: false,
        package_commitment_openings_in_circuit: false,
        lagrange_and_tally_in_circuit: false,
        winner_logic_in_circuit: false,
        p3a_r1cs_complete: true,
        p3a_groth16_complete: true,
        full_p3a: true,
        full_p3: false,
        limitations: vec![
            "the proof covers three selected CP relations but not private-package commitment openings",
            "the verifier must bind the public prefixes to the accepted election state and selected Trustee set",
            "deterministic experiment seeds are reproducibility inputs, not a production trusted setup ceremony",
        ],
    };
    if let Some(parent) = groth16_output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &groth16_output_path,
        serde_json::to_vec_pretty(&groth16_report)?,
    )?;
    println!(
        "G0.9-P3-A-G16: constraints={} public-inputs={} setup={:.3}ms prove={:.3}ms verify={:.3}ms proof={}B pk={}B vk={}B peak-rss={:.3}MiB valid={} changed-public-rejected={} full-p3a=true full-p3=false",
        groth16_report.constraints,
        groth16_report.public_inputs,
        groth16_report.setup_ms,
        groth16_report.prove_ms,
        groth16_report.verify_ms,
        groth16_report.proof_bytes,
        groth16_report.proving_key_compressed_bytes,
        groth16_report.verifying_key_compressed_bytes,
        groth16_report.peak_rss_after_verify_mib,
        groth16_report.valid_proof_verified,
        groth16_report.changed_public_prefix_rejected,
    );
    println!("wrote {}", groth16_output_path.display());
    Ok(())
}
