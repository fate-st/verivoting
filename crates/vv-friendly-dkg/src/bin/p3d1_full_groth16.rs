use ark_bls12_381::Bls12_381;
use ark_ec::Group;
use ark_ff::{PrimeField, Zero};
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
use vv_friendly_crypto::cp_h4_gadget::H4ACircuit;
use vv_friendly_crypto::p3a_gadget::{P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE, P3A_TRUSTEE_COUNT};
use vv_friendly_crypto::p3b_gadget::P3B1TrusteeCircuit;
use vv_friendly_crypto::p3c_gadget::{P3C0_LAGRANGE_COEFFICIENTS, P3C0_LAGRANGE_SUBSET};
use vv_friendly_crypto::p3d_gadget::{check_winner_linkage_p3d0, p3d0_public_inputs, P3D0Circuit};
use vv_friendly_crypto::{
    encode_point, encrypt_weighted_vote, fold_ballots, generator, Point, CP_H2F_SUITE_ID,
    PACKAGE_H2F_SUITE_ID,
};
use vv_friendly_dkg::package::{create_private_package, verify_private_package};
use vv_friendly_dkg::registration::{
    verify_close, RegistrationAuthority, RegistrationEntry, RegistrationPolicy,
};
use vv_friendly_dkg::{
    lagrange_at_zero, run_dkg, threshold_decrypt_verified_shares_with_registration,
    AuthenticationKeypair, DkgConfig, FaultPlan,
};
use vv_types::{derive_final_context, hex, sha256};

const RULE_ID: &str = "highest-weight-smallest-id-on-tie";
const DEFAULT_SEED: u64 = 20_261_026;
const LAYOUT_VERSION: &str = "VV/JUBJUB/P3-D1/INPUT-LAYOUT/v1";
const SETUP_SEED: u64 = 0x5033_4431_5345_5455;
const PROOF_SEED: u64 = 0x5033_4431_5052_4F56;

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
    selected_trustees: [u32; 3],
    lagrange_coefficients: [&'static str; 3],
    public_inputs_per_trustee: Vec<&'static str>,
    shared_public_inputs: Vec<&'static str>,
    private_witnesses_per_trustee: Vec<&'static str>,
    shared_private_witnesses: Vec<&'static str>,
    registered_weight: u64,
    public_winner: u64,
    aggregate_ciphertext_c1_hex: Vec<String>,
    records: Vec<PublicTrusteeRecord>,
    private_package_bytes_emitted: bool,
    private_openings_emitted: bool,
    private_plaintext_groups_emitted: bool,
    private_tallies_emitted: bool,
}

#[derive(Serialize)]
struct P3D1Result {
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
    manifest_matches_p3d1: bool,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    selected_trustees: [u32; 3],
    lagrange_coefficients: [&'static str; 3],
    native_coefficients_match: bool,
    native_cp_verified: usize,
    native_packages_verified: usize,
    native_threshold_tally_matches_fixture: bool,
    native_winner_matches_fixture: bool,
    trustees_in_r1cs: usize,
    cp_equations_in_r1cs: usize,
    package_h2f_relations_in_r1cs: usize,
    package_opening_equations_in_r1cs: usize,
    lagrange_combination_equations_in_r1cs: usize,
    tally_group_binding_equations_in_r1cs: usize,
    tally_range_relations_in_r1cs: usize,
    tally_sum_relations_in_r1cs: usize,
    public_registered_weight: u64,
    public_winner: u64,
    winner_candidate_flags_in_r1cs: usize,
    winner_pairwise_relations_in_r1cs: usize,
    private_plaintext_groups_emitted: bool,
    private_tallies_emitted: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    valid_relation_satisfied: bool,
    wrong_public_winner_rejected: bool,
    setup_seed: u64,
    proof_seed: u64,
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    wrong_winner_verify_ms: f64,
    peak_rss_after_setup_mib: f64,
    peak_rss_after_prove_mib: f64,
    peak_rss_after_verify_mib: f64,
    proving_key_compressed_bytes: usize,
    verifying_key_compressed_bytes: usize,
    proof_bytes: usize,
    valid_proof_verified: bool,
    proving_key_persisted: bool,
    groth16_run: bool,
    threshold_combination_gate_complete: bool,
    full_p3d1: bool,
    bounded_tally_binding_in_r1cs: bool,
    tally_values_in_r1cs: bool,
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
    let mut output_path = PathBuf::from("experiments/g0_9_p3d1_raw/p3d1_full_groth16.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_p3d1_public_layout_v1.json");
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
    let manifest_capability = manifest["capabilities"]["p3d1_full_p3_groth16"]
        .as_str()
        .ok_or("prototype manifest lacks P3-D1 capability")?
        .to_owned();
    let manifest_matches_p3d1 = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-")
        && manifest_capability == "READY_REAL_GROTH16_FIXED_123_COMPLETE_P3"
        && manifest["cp_hash_to_scalar_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(CP_H2F_SUITE_ID))
        && manifest["commitment_suite"]
            .as_str()
            .is_some_and(|suite| suite.contains(PACKAGE_H2F_SUITE_ID));
    if !manifest_matches_p3d1 {
        return Err("prototype manifest does not describe P3-D1".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let election_id = sha256(b"VV/JUBJUB/G0.9/P3D1/FULL-GROTH16");
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
                sha256(format!("p3d1-voter-credential-{index}").as_bytes()),
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
    let ciphertext_c1 = aggregate
        .iter()
        .map(|ciphertext| ciphertext.c1)
        .collect::<Vec<_>>();

    let mut circuits = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut records = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut share_vectors = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut verified_shares = Vec::with_capacity(P3A_TRUSTEE_COUNT);
    let mut native_cp_verified = 0usize;
    for trustee_id in P3C0_LAGRANGE_SUBSET {
        let context = GeneralizedCpContext {
            election_id: dkg.config.election_id,
            policy_pre: dkg.config.policy_pre,
            final_context,
            registration_close_digest,
            disclosure_digest,
            target,
            trustee_id,
        };
        let verification_share = *dkg.verification_share(trustee_id)?;
        let package = prove_decryption_share(
            &context,
            dkg.secret_share(trustee_id)?,
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
            &authentication_keys[usize::try_from(trustee_id)? - 1],
            &mut rng,
        )?;
        let verified = verify_private_package(&dkg, &aggregate, &private_package, &public_record)?;
        verified_shares.push(verified.verified_share().clone());
        share_vectors.push(package.shares.clone());
        let encoded_package = private_package.encoded_bytes();
        let cp_public_prefix = &transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE];
        records.push(PublicTrusteeRecord {
            trustee_id,
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

    let coefficients = lagrange_at_zero(&P3C0_LAGRANGE_SUBSET, dkg.config.n, dkg.config.tau)?;
    let expected_coefficients = [
        vv_friendly_crypto::Scalar::from(3u64),
        -vv_friendly_crypto::Scalar::from(3u64),
        vv_friendly_crypto::Scalar::from(1u64),
    ];
    let native_coefficients_match = coefficients == expected_coefficients;
    if !native_coefficients_match {
        return Err("native Lagrange coefficients are not (3,-3,1)".into());
    }
    let plaintext_groups = aggregate
        .iter()
        .enumerate()
        .map(|(coordinate, ciphertext)| {
            let combined_share = share_vectors.iter().zip(&coefficients).fold(
                Point::zero(),
                |sum, (shares, coefficient)| {
                    sum + shares[coordinate].mul_bigint(coefficient.into_bigint())
                },
            );
            ciphertext.c1 - combined_share
        })
        .collect::<Vec<_>>();
    let native_tally = threshold_decrypt_verified_shares_with_registration(
        &dkg,
        &P3C0_LAGRANGE_SUBSET,
        &aggregate,
        &verified_shares,
        registration_close_digest,
        disclosure_digest,
        registration.w_registered(),
    )?;
    let expected_tally = [4u64, 2, 4];
    let native_threshold_tally_matches_fixture = native_tally == expected_tally
        && plaintext_groups
            .iter()
            .zip(expected_tally)
            .all(|(point, tally)| {
                *point
                    == generator().mul_bigint(vv_friendly_crypto::Scalar::from(tally).into_bigint())
            });
    if native_cp_verified != P3A_TRUSTEE_COUNT
        || verified_shares.len() != P3A_TRUSTEE_COUNT
        || !native_threshold_tally_matches_fixture
    {
        return Err("native P3-D1 fixture failed threshold consistency".into());
    }

    let registered_weight = registration.w_registered();
    let public_winner = 1u64;
    let native_winner_matches_fixture =
        u64::try_from(vv_types::winner(&native_tally, registered_weight)?)? == public_winner;
    if !native_winner_matches_fixture {
        return Err("native P3-D1 winner does not match the fixture".into());
    }
    let started = Instant::now();
    let valid = check_winner_linkage_p3d0(
        &circuits,
        &ciphertext_c1,
        &plaintext_groups,
        &expected_tally,
        registered_weight,
        public_winner,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let r1cs_peak_rss_mib = peak_rss_mib();
    if !valid.satisfied {
        return Err("P3-D1 R1CS preflight failed".into());
    }

    let circuit = P3D0Circuit {
        trustees: circuits.clone(),
        ciphertext_c1: ciphertext_c1.clone(),
        plaintext_groups: plaintext_groups.clone(),
        tallies: expected_tally.to_vec(),
        registered_weight,
        public_winner,
    };
    let public_inputs =
        p3d0_public_inputs(&circuits, &ciphertext_c1, registered_weight, public_winner)?;
    if public_inputs.len() != valid.public_inputs {
        return Err("P3-D1 public input packing differs from the R1CS layout".into());
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

    let wrong_winner_inputs = p3d0_public_inputs(&circuits, &ciphertext_c1, registered_weight, 2)?;
    let wrong_winner_verify_started = Instant::now();
    let wrong_public_winner_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &wrong_winner_inputs)?;
    let wrong_winner_verify_ms = wrong_winner_verify_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_verify_mib = peak_rss_mib();
    if !valid_proof_verified || !wrong_public_winner_rejected {
        return Err("P3-D1 Groth16 acceptance boundary failed".into());
    }

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;

    let layout = PublicLayoutFixture {
        layout_version: LAYOUT_VERSION,
        protocol_version: protocol_version.clone(),
        cp_suite_id: CP_H2F_SUITE_ID,
        package_h2f_suite_id: PACKAGE_H2F_SUITE_ID,
        seed,
        selected_trustees: P3C0_LAGRANGE_SUBSET,
        lagrange_coefficients: P3C0_LAGRANGE_COEFFICIENTS,
        public_inputs_per_trustee: vec![
            "P3-A CP statement prefix",
            "canonical compressed private-package commitment",
        ],
        shared_public_inputs: vec![
            "three canonical aggregate ciphertext C1 points",
            "registered weight bound",
            "winner candidate identifier",
        ],
        private_witnesses_per_trustee: vec![
            "decryption-share and CP proof suffix",
            "CP response scalar",
            "private-package commitment opening",
        ],
        shared_private_witnesses: vec![
            "three threshold-combined plaintext group points",
            "three bounded integer tallies",
        ],
        registered_weight,
        public_winner,
        aggregate_ciphertext_c1_hex: ciphertext_c1
            .iter()
            .map(|point| hex(&encode_point(point)))
            .collect(),
        records,
        private_package_bytes_emitted: false,
        private_openings_emitted: false,
        private_plaintext_groups_emitted: false,
        private_tallies_emitted: false,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let report = P3D1Result {
        gate: "G0.9-P3-D1",
        status: "PASS_P3D1_REAL_GROTH16_FIXED_123_COMPLETE_P3",
        classification: "real-groth16-over-complete-fixed-123-cp-package-threshold-bounded-tally-and-public-winner-relation",
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
        manifest_matches_p3d1,
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        selected_trustees: P3C0_LAGRANGE_SUBSET,
        lagrange_coefficients: P3C0_LAGRANGE_COEFFICIENTS,
        native_coefficients_match,
        native_cp_verified,
        native_packages_verified: verified_shares.len(),
        native_threshold_tally_matches_fixture,
        native_winner_matches_fixture,
        trustees_in_r1cs: valid.trustees,
        cp_equations_in_r1cs: valid.cp_equations,
        package_h2f_relations_in_r1cs: valid.package_h2f_relations,
        package_opening_equations_in_r1cs: valid.package_opening_equations,
        lagrange_combination_equations_in_r1cs: valid.lagrange_combination_equations,
        tally_group_binding_equations_in_r1cs: valid.tally_group_binding_equations,
        tally_range_relations_in_r1cs: valid.tally_range_relations,
        tally_sum_relations_in_r1cs: valid.tally_sum_relations,
        public_registered_weight: registered_weight,
        public_winner,
        winner_candidate_flags_in_r1cs: valid.winner_candidate_flags,
        winner_pairwise_relations_in_r1cs: valid.winner_pairwise_relations,
        private_plaintext_groups_emitted: false,
        private_tallies_emitted: false,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        witness_ms,
        peak_rss_mib: r1cs_peak_rss_mib,
        valid_relation_satisfied: valid.satisfied,
        wrong_public_winner_rejected,
        setup_seed: SETUP_SEED,
        proof_seed: PROOF_SEED,
        setup_ms,
        prove_ms,
        verify_ms,
        wrong_winner_verify_ms,
        peak_rss_after_setup_mib,
        peak_rss_after_prove_mib,
        peak_rss_after_verify_mib,
        proving_key_compressed_bytes: parameters.compressed_size(),
        verifying_key_compressed_bytes: parameters.vk.compressed_size(),
        proof_bytes: proof_encoding.len(),
        valid_proof_verified,
        proving_key_persisted: false,
        groth16_run: true,
        threshold_combination_gate_complete: true,
        full_p3d1: true,
        bounded_tally_binding_in_r1cs: true,
        tally_values_in_r1cs: true,
        winner_logic_in_r1cs: true,
        full_p3: true,
        limitations: vec![
            "P3-D1 fixes the selected subset to Trustees 1, 2 and 3 with coefficients (3,-3,1)",
            "tallies and plaintext group elements remain private; the registered-weight bound and winner are public",
            "the verifier must bind the public registered-weight input to the accepted registration close",
            "deterministic experiment seeds are reproducibility inputs, not a production trusted setup ceremony",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.9-P3-D1: trustees={:?} constraints={} public-inputs={} setup={:.3}ms prove={:.3}ms verify={:.3}ms proof={}B pk={}B vk={}B peak-rss={:.3}MiB valid={} wrong-public-winner-rejected={} groth16=true full-p3=true",
        report.selected_trustees,
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
        report.wrong_public_winner_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());
    Ok(())
}
