use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    verify_decryption_share, GeneralizedCpContext,
};
use vv_friendly_crypto::cp_h4_gadget::H4ACircuit;
use vv_friendly_crypto::p3a_gadget::{P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE, P3A_TRUSTEE_COUNT};
use vv_friendly_crypto::p3b_gadget::{check_three_package_openings_p3b1, P3B1TrusteeCircuit};
use vv_friendly_crypto::{
    encode_point, encrypt_weighted_vote, fold_ballots, generator, CP_H2F_SUITE_ID,
    PACKAGE_H2F_SUITE_ID,
};
use vv_friendly_dkg::package::{create_private_package, verify_private_package};
use vv_friendly_dkg::registration::{
    verify_close, RegistrationAuthority, RegistrationEntry, RegistrationPolicy,
};
use vv_friendly_dkg::{run_dkg, AuthenticationKeypair, DkgConfig, FaultPlan};
use vv_types::{derive_final_context, hex, sha256};

const RULE_ID: &str = "highest-weight-smallest-id-on-tie";
const DEFAULT_SEED: u64 = 20_261_025;
const LAYOUT_VERSION: &str = "VV/JUBJUB/P3-B1/INPUT-LAYOUT/v1";

#[derive(Serialize)]
struct PublicTrusteeRecord {
    trustee_id: u32,
    cp_public_prefix_hex: String,
    cp_public_prefix_sha256: String,
    private_package_sha256: String,
    package_commitment_hex: String,
    commitment_signature_hex: String,
}

#[derive(Serialize)]
struct PublicLayoutFixture {
    layout_version: &'static str,
    protocol_version: String,
    cp_suite_id: &'static str,
    package_h2f_suite_id: &'static str,
    seed: u64,
    selected_trustees: Vec<u32>,
    public_inputs_per_trustee: Vec<&'static str>,
    private_witnesses_per_trustee: Vec<&'static str>,
    records: Vec<PublicTrusteeRecord>,
    private_package_bytes_emitted: bool,
    private_openings_emitted: bool,
}

#[derive(Serialize)]
struct P3B1Result {
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
    manifest_capability: String,
    manifest_matches_p3b1: bool,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    selected_trustees: Vec<u32>,
    native_cp_verified: usize,
    native_packages_verified: usize,
    external_signatures_verified: bool,
    trustees_in_r1cs: usize,
    cp_equations_in_r1cs: usize,
    package_h2f_relations_in_r1cs: usize,
    package_opening_equations_in_r1cs: usize,
    package_bytes_per_trustee: usize,
    package_h2f_input_bytes: usize,
    package_sha512_blocks: usize,
    public_package_commitments: usize,
    private_openings_emitted: bool,
    private_package_bytes_emitted: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    valid_relation_satisfied: bool,
    changed_third_commitment_rejected: bool,
    groth16_run: bool,
    package_opening_gate_complete: bool,
    full_p3b: bool,
    lagrange_and_tally_in_r1cs: bool,
    winner_logic_in_r1cs: bool,
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
    let mut output_path =
        PathBuf::from("experiments/g0_9_p3b1_raw/p3b1_three_package_openings_r1cs.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_p3b1_public_layout_v1.json");
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
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
    let manifest_capability = manifest["capabilities"]["p3b1_package_openings_r1cs"]
        .as_str()
        .ok_or("prototype manifest lacks P3-B1 capability")?
        .to_owned();
    let manifest_matches_p3b1 = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-")
        && manifest_capability == "READY_R1CS_THREE_SELECTED_PACKAGE_OPENINGS_NO_GROTH16"
        && manifest["cp_hash_to_scalar_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(CP_H2F_SUITE_ID))
        && manifest["commitment_suite"]
            .as_str()
            .is_some_and(|suite| suite.contains(PACKAGE_H2F_SUITE_ID));
    if !manifest_matches_p3b1 {
        return Err("prototype manifest does not describe P3-B1".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let election_id = sha256(b"VV/JUBJUB/G0.9/P3B1/PACKAGE-OPENINGS");
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
                sha256(format!("p3b1-voter-credential-{index}").as_bytes()),
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
    let mut circuits = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut records = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut native_cp_verified = 0usize;
    let mut native_packages_verified = 0usize;
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
        let (private_package, public_record) = create_private_package(
            context,
            verification_share,
            package.clone(),
            &authentication_keys[usize::try_from(*trustee_id)? - 1],
            &mut rng,
        )?;
        verify_private_package(&dkg, &aggregate, &private_package, &public_record)?;
        native_packages_verified += 1;
        let encoded_package = private_package.encoded_bytes();
        let cp_public_prefix = &transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE];
        records.push(PublicTrusteeRecord {
            trustee_id: *trustee_id,
            cp_public_prefix_hex: hex(cp_public_prefix),
            cp_public_prefix_sha256: hex(&sha256(cp_public_prefix)),
            private_package_sha256: hex(&sha256(&encoded_package)),
            package_commitment_hex: hex(&encode_point(&public_record.commitment.point)),
            commitment_signature_hex: hex(&public_record.signature.canonical_bytes()),
        });
        circuits.push(P3B1TrusteeCircuit {
            cp: H4ACircuit {
                transcript,
                verification_share,
                ciphertext_c0: ciphertext_c0.clone(),
                decryption_shares: package.shares,
                a0: package.proof.a0,
                proof_coordinates: package.proof.coordinates,
                response: package.proof.response,
            },
            package_commitment: public_record.commitment.point,
            opening: private_package.opening(),
        });
    }
    if native_cp_verified != P3A_TRUSTEE_COUNT || native_packages_verified != P3A_TRUSTEE_COUNT {
        return Err("native P3-B1 fixture did not verify all selected packages".into());
    }

    let started = Instant::now();
    let valid = check_three_package_openings_p3b1(&circuits)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let r1cs_peak_rss_mib = peak_rss_mib();

    let mut changed = circuits.clone();
    changed[2].package_commitment += generator();
    let changed_third_commitment_rejected = !check_three_package_openings_p3b1(&changed)?.satisfied;
    if !valid.satisfied || !changed_third_commitment_rejected {
        return Err("P3-B1 R1CS acceptance boundary failed".into());
    }

    let layout = PublicLayoutFixture {
        layout_version: LAYOUT_VERSION,
        protocol_version: protocol_version.clone(),
        cp_suite_id: CP_H2F_SUITE_ID,
        package_h2f_suite_id: PACKAGE_H2F_SUITE_ID,
        seed,
        selected_trustees: selected_trustees.clone(),
        public_inputs_per_trustee: vec![
            "P3-A CP statement prefix",
            "canonical compressed private-package commitment",
        ],
        private_witnesses_per_trustee: vec![
            "decryption-share and CP proof suffix",
            "CP response scalar",
            "private-package commitment opening",
        ],
        records,
        private_package_bytes_emitted: false,
        private_openings_emitted: false,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let report = P3B1Result {
        gate: "G0.9-P3-B1",
        status: "PASS_P3B1_THREE_PACKAGE_OPENINGS_R1CS",
        classification: "three-selected-cp-and-package-h2f-pedersen-opening-relations-r1cs-no-groth16-no-threshold-or-winner-not-full-p3",
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
        manifest_capability,
        manifest_matches_p3b1,
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        selected_trustees,
        native_cp_verified,
        native_packages_verified,
        external_signatures_verified: true,
        trustees_in_r1cs: valid.trustees,
        cp_equations_in_r1cs: valid.cp_equations,
        package_h2f_relations_in_r1cs: valid.package_h2f_relations,
        package_opening_equations_in_r1cs: valid.package_opening_equations,
        package_bytes_per_trustee: valid.package_bytes_per_trustee,
        package_h2f_input_bytes: valid.package_h2f_input_bytes,
        package_sha512_blocks: valid.package_sha512_blocks,
        public_package_commitments: P3A_TRUSTEE_COUNT,
        private_openings_emitted: false,
        private_package_bytes_emitted: false,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        witness_ms,
        peak_rss_mib: r1cs_peak_rss_mib,
        valid_relation_satisfied: valid.satisfied,
        changed_third_commitment_rejected,
        groth16_run: false,
        package_opening_gate_complete: true,
        full_p3b: true,
        lagrange_and_tally_in_r1cs: false,
        winner_logic_in_r1cs: false,
        full_p3: false,
        limitations: vec![
            "commitment signatures are checked by the native verifier and are not duplicated inside this R1CS",
            "P3-B1 stops before threshold interpolation, secret tally binding and winner correctness",
            "Groth16 is intentionally deferred until the complete R1CS gate is accepted",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-P3-B1: trustees={:?} CP-equations={} package-H2F={} openings={} constraints={} public-inputs={} witness-vars={} witness={:.3}ms peak-rss={:.3}MiB valid={} changed-third-commitment-rejected={} groth16=false full-p3b=true full-p3=false",
        report.selected_trustees,
        report.cp_equations_in_r1cs,
        report.package_h2f_relations_in_r1cs,
        report.package_opening_equations_in_r1cs,
        report.constraints,
        report.public_inputs,
        report.witness_variables,
        report.witness_ms,
        report.peak_rss_mib,
        report.valid_relation_satisfied,
        report.changed_third_commitment_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());
    Ok(())
}
