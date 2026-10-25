use ark_bls12_381::Bls12_381;
use ark_ec::Group;
use ark_ff::{PrimeField, Zero};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap, env, error::Error, fs, path::PathBuf, process::Command, time::Instant,
};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    verify_decryption_share, GeneralizedCpContext, VerifiedDecryptionShare,
};
use vv_friendly_crypto::cp_h3_gadget::H3_TRUSTEE_ID_OFFSET;
use vv_friendly_crypto::cp_h4_gadget::H4ACircuit;
use vv_friendly_crypto::evm_export::Eip2537DisclosureFixture;
use vv_friendly_crypto::p3b_gadget::P3B1TrusteeCircuit;
use vv_friendly_crypto::p3c_gadget::{
    g4b_lagrange_coefficients, g4b_selected_subset, G4B_BASELINE_SUBSET, G4B_SECOND_SUBSET,
};
use vv_friendly_crypto::p3d_gadget::{
    check_winner_linkage_p3d0_for_bound_subset, p3d0_bound_subset_public_inputs,
    P3D0BoundSubsetCircuit, P3D0CircuitReport,
};
use vv_friendly_crypto::{
    encode_scalar, encrypt_weighted_vote, fold_ballots, generator, Ciphertext, Point, Scalar,
    CP_H2F_SUITE_ID, PACKAGE_H2F_SUITE_ID,
};
use vv_friendly_dkg::package::{create_private_package, verify_private_package};
use vv_friendly_dkg::registration::{
    verify_close, RegistrationAuthority, RegistrationEntry, RegistrationPolicy,
};
use vv_friendly_dkg::{
    lagrange_at_zero, run_dkg, threshold_decrypt_verified_shares_with_registration,
    AuthenticationKeypair, DkgConfig, DkgOutput, FaultPlan,
};
use vv_types::{derive_final_context, hex, sha256};

const RULE_ID: &str = "highest-weight-smallest-id-on-tie";
const DEFAULT_SEED: u64 = 20_261_026;
const EXPECTED_TALLY: [u64; 3] = [4, 2, 4];
const PUBLIC_WINNER: u64 = 1;
const LAYOUT_VERSION: &str = "VV/JUBJUB/G4-B/INPUT-LAYOUT/v1";
const SETUP_SEED: u64 = 0x4734_4253_4554_5550;
const PROOF_SEED: u64 = 0x4734_4250_524F_4F46;

#[derive(Clone)]
struct TrusteeMaterial {
    circuit: P3B1TrusteeCircuit,
    shares: Vec<Point>,
    verified_share: VerifiedDecryptionShare,
}

struct FixtureContext<'a> {
    output: &'a DkgOutput,
    aggregate: &'a [Ciphertext],
    materials: &'a HashMap<u32, TrusteeMaterial>,
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    registered_weight: u64,
}

struct SubsetFixture {
    selected_trustees: [u32; 3],
    public_second_subset: bool,
    trustees: Vec<P3B1TrusteeCircuit>,
    plaintext_groups: Vec<Point>,
    native_winner: usize,
    coefficients: [Scalar; 3],
}

#[derive(Serialize)]
struct ShapeMeasurement {
    selected_trustees: [u32; 3],
    public_second_subset: bool,
    lagrange_coefficient_labels: [&'static str; 3],
    lagrange_coefficients_canonical_le_hex: Vec<String>,
    native_winner: usize,
    satisfied: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
}

#[derive(Serialize)]
struct PublicLayoutFixture {
    layout_version: &'static str,
    protocol_version: String,
    supported_subsets: [[u32; 3]; 2],
    public_selector_mapping: [&'static str; 2],
    trustee_id_transcript_offset: usize,
    public_input_order: Vec<&'static str>,
    public_input_count: usize,
    selector_public_input_index: usize,
    setup_subset: [u32; 3],
    proof_subset: [u32; 3],
    private_plaintext_groups_emitted: bool,
    private_tallies_emitted: bool,
    proving_key_persisted: bool,
}

#[derive(Serialize)]
struct G4BResult {
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
    manifest_unchanged: bool,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    supported_subsets: [[u32; 3]; 2],
    public_selector_bound_to_transcript_ids: bool,
    coefficients_derived_from_public_selector: bool,
    coefficients_are_separate_public_values: bool,
    baseline: ShapeMeasurement,
    second_subset: ShapeMeasurement,
    same_constraints: bool,
    same_public_inputs: bool,
    same_witness_variables: bool,
    same_setup_parameters_used: bool,
    mismatched_selector_relation_rejected: bool,
    setup_seed: u64,
    proof_seed: u64,
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    wrong_selector_verify_ms: f64,
    peak_rss_after_setup_mib: f64,
    peak_rss_after_prove_mib: f64,
    peak_rss_after_verify_mib: f64,
    proving_key_compressed_bytes: usize,
    verifying_key_compressed_bytes: usize,
    proof_bytes: usize,
    valid_second_subset_proof_verified: bool,
    changed_public_selector_rejected: bool,
    selector_public_input_index: usize,
    changed_public_input_count: usize,
    proving_key_persisted: bool,
    groth16_run: bool,
    full_p3_for_supported_two_subset_gate: bool,
    full_arbitrary_subset_p3: bool,
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

fn build_subset_fixture(
    context: &FixtureContext<'_>,
    public_second_subset: bool,
) -> Result<SubsetFixture, Box<dyn Error>> {
    let selected_trustees = g4b_selected_subset(public_second_subset);
    let coefficients = g4b_lagrange_coefficients(public_second_subset);
    if lagrange_at_zero(
        &selected_trustees,
        context.output.config.n,
        context.output.config.tau,
    )? != coefficients
    {
        return Err("G4-B circuit coefficients differ from native interpolation".into());
    }
    let trustees = selected_trustees
        .iter()
        .map(|id| {
            context
                .materials
                .get(id)
                .map(|material| material.circuit.clone())
                .ok_or_else(|| format!("missing Trustee {id} circuit material"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let share_vectors = selected_trustees
        .iter()
        .map(|id| {
            context
                .materials
                .get(id)
                .map(|material| material.shares.clone())
                .ok_or_else(|| format!("missing Trustee {id} share material"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let verified_shares = selected_trustees
        .iter()
        .map(|id| {
            context
                .materials
                .get(id)
                .map(|material| material.verified_share.clone())
                .ok_or_else(|| format!("missing Trustee {id} verified package"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let plaintext_groups = context
        .aggregate
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
        context.output,
        &selected_trustees,
        context.aggregate,
        &verified_shares,
        context.registration_close_digest,
        context.disclosure_digest,
        context.registered_weight,
    )?;
    let native_winner = vv_types::winner(&native_tally, context.registered_weight)?;
    let fixture_matches = native_tally == EXPECTED_TALLY
        && native_winner == PUBLIC_WINNER as usize
        && plaintext_groups
            .iter()
            .zip(EXPECTED_TALLY)
            .all(|(point, tally)| {
                *point == generator().mul_bigint(Scalar::from(tally).into_bigint())
            });
    if !fixture_matches {
        return Err(format!("G4-B subset {selected_trustees:?} failed native consistency").into());
    }
    Ok(SubsetFixture {
        selected_trustees,
        public_second_subset,
        trustees,
        plaintext_groups,
        native_winner,
        coefficients,
    })
}

fn measure_shape(
    fixture: &SubsetFixture,
    ciphertext_c1: &[Point],
    registered_weight: u64,
    coefficient_labels: [&'static str; 3],
) -> Result<(ShapeMeasurement, P3D0CircuitReport), Box<dyn Error>> {
    let started = Instant::now();
    let report = check_winner_linkage_p3d0_for_bound_subset(
        &fixture.trustees,
        ciphertext_c1,
        &fixture.plaintext_groups,
        fixture.public_second_subset,
        &EXPECTED_TALLY,
        registered_weight,
        PUBLIC_WINNER,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !report.satisfied {
        return Err(format!(
            "G4-B subset {:?} failed R1CS preflight",
            fixture.selected_trustees
        )
        .into());
    }
    let measurement = ShapeMeasurement {
        selected_trustees: fixture.selected_trustees,
        public_second_subset: fixture.public_second_subset,
        lagrange_coefficient_labels: coefficient_labels,
        lagrange_coefficients_canonical_le_hex: fixture
            .coefficients
            .iter()
            .map(|coefficient| hex(&encode_scalar(coefficient)))
            .collect(),
        native_winner: fixture.native_winner,
        satisfied: report.satisfied,
        constraints: report.constraints,
        public_inputs: report.public_inputs,
        witness_variables: report.witness_variables,
        witness_ms,
    };
    Ok((measurement, report))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut output_path = PathBuf::from("experiments/g4_b_raw/public_subset_groth16.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_g4b_public_layout_v1.json");
    let mut evm_fixture_output_path = None;
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--layout-output" => layout_path = PathBuf::from(value),
            "--evm-fixture-output" => evm_fixture_output_path = Some(PathBuf::from(value)),
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
    let manifest_capability = manifest["capabilities"]["full_p3"]
        .as_str()
        .ok_or("prototype manifest lacks full_p3 capability")?
        .to_owned();
    let manifest_unchanged = manifest_capability == "READY_REAL_GROTH16_FIXED_123_COMPLETE_P3"
        && manifest["cp_hash_to_scalar_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(CP_H2F_SUITE_ID))
        && manifest["commitment_suite"]
            .as_str()
            .is_some_and(|suite| suite.contains(PACKAGE_H2F_SUITE_ID));
    if !manifest_unchanged {
        return Err("prototype manifest no longer describes the fixed-123 P3 baseline".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let election_id = sha256(b"VV/JUBJUB/G4-B/PUBLIC-SUBSET-GROTH16");
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
                sha256(format!("g4b-voter-credential-{index}").as_bytes()),
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

    let mut materials = HashMap::new();
    for trustee_id in 1..=4u32 {
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
        materials.insert(
            trustee_id,
            TrusteeMaterial {
                circuit: P3B1TrusteeCircuit {
                    cp: H4ACircuit {
                        transcript,
                        verification_share,
                        ciphertext_c0: ciphertext_c0.clone(),
                        decryption_shares: package.shares.clone(),
                        a0: package.proof.a0,
                        proof_coordinates: package.proof.coordinates,
                        response: package.proof.response,
                    },
                    package_commitment: public_record.commitment.point,
                    opening: private_package.opening(),
                },
                shares: package.shares,
                verified_share: verified.verified_share().clone(),
            },
        );
    }

    let fixture_context = FixtureContext {
        output: &dkg,
        aggregate: &aggregate,
        materials: &materials,
        registration_close_digest,
        disclosure_digest,
        registered_weight: registration.w_registered(),
    };
    let baseline_fixture = build_subset_fixture(&fixture_context, false)?;
    let second_fixture = build_subset_fixture(&fixture_context, true)?;
    let (baseline, baseline_report) = measure_shape(
        &baseline_fixture,
        &ciphertext_c1,
        registration.w_registered(),
        ["3", "-3", "1"],
    )?;
    let (second_subset, second_report) = measure_shape(
        &second_fixture,
        &ciphertext_c1,
        registration.w_registered(),
        ["8/3", "-2", "1/3"],
    )?;
    let same_constraints = baseline.constraints == second_subset.constraints;
    let same_public_inputs = baseline.public_inputs == second_subset.public_inputs;
    let same_witness_variables = baseline.witness_variables == second_subset.witness_variables;
    if !same_constraints || !same_public_inputs || !same_witness_variables {
        return Err("G4-B selector values do not synthesize one common R1CS shape".into());
    }

    let mismatched_selector_relation_rejected = !check_winner_linkage_p3d0_for_bound_subset(
        &second_fixture.trustees,
        &ciphertext_c1,
        &second_fixture.plaintext_groups,
        false,
        &EXPECTED_TALLY,
        registration.w_registered(),
        PUBLIC_WINNER,
    )?
    .satisfied;
    if !mismatched_selector_relation_rejected {
        return Err("G4-B failed to bind the public selector to transcript Trustee IDs".into());
    }

    let baseline_circuit = P3D0BoundSubsetCircuit {
        trustees: baseline_fixture.trustees,
        ciphertext_c1: ciphertext_c1.clone(),
        plaintext_groups: baseline_fixture.plaintext_groups,
        public_second_subset: false,
        tallies: EXPECTED_TALLY.to_vec(),
        registered_weight: registration.w_registered(),
        public_winner: PUBLIC_WINNER,
    };
    let second_circuit = P3D0BoundSubsetCircuit {
        trustees: second_fixture.trustees.clone(),
        ciphertext_c1: ciphertext_c1.clone(),
        plaintext_groups: second_fixture.plaintext_groups.clone(),
        public_second_subset: true,
        tallies: EXPECTED_TALLY.to_vec(),
        registered_weight: registration.w_registered(),
        public_winner: PUBLIC_WINNER,
    };
    let second_public_inputs = p3d0_bound_subset_public_inputs(
        &second_fixture.trustees,
        true,
        &ciphertext_c1,
        registration.w_registered(),
        PUBLIC_WINNER,
    )?;
    if second_public_inputs.len() != second_report.public_inputs
        || second_public_inputs.len() != baseline_report.public_inputs
    {
        return Err("G4-B public input packing differs from the R1CS layout".into());
    }

    let mut setup_rng = ChaCha20Rng::seed_from_u64(SETUP_SEED);
    let setup_started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        baseline_circuit,
        &mut setup_rng,
    )?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_setup_mib = peak_rss_mib();

    let mut proof_rng = ChaCha20Rng::seed_from_u64(PROOF_SEED);
    let prove_started = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        second_circuit,
        &parameters,
        &mut proof_rng,
    )?;
    let prove_ms = prove_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_prove_mib = peak_rss_mib();

    let prepared = prepare_verifying_key(&parameters.vk);
    let verify_started = Instant::now();
    let valid_second_subset_proof_verified =
        Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &second_public_inputs)?;
    let verify_ms = verify_started.elapsed().as_secs_f64() * 1000.0;

    let wrong_selector_inputs = p3d0_bound_subset_public_inputs(
        &second_fixture.trustees,
        false,
        &ciphertext_c1,
        registration.w_registered(),
        PUBLIC_WINNER,
    )?;
    let changed_public_input_indices = second_public_inputs
        .iter()
        .zip(&wrong_selector_inputs)
        .enumerate()
        .filter_map(|(index, (valid, changed))| (valid != changed).then_some(index))
        .collect::<Vec<_>>();
    if changed_public_input_indices.as_slice() != [51] {
        return Err("G4-B selector mutation changed an unexpected public-input position".into());
    }
    let selector_public_input_index = changed_public_input_indices[0];
    let wrong_selector_started = Instant::now();
    let changed_public_selector_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &wrong_selector_inputs)?;
    let wrong_selector_verify_ms = wrong_selector_started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_verify_mib = peak_rss_mib();
    if !valid_second_subset_proof_verified || !changed_public_selector_rejected {
        return Err("G4-B Groth16 public-selector boundary failed".into());
    }

    if let Some(path) = &evm_fixture_output_path {
        let fixture = Eip2537DisclosureFixture::from_two_subset_proof(
            &parameters.vk,
            &proof,
            &second_public_inputs,
            &wrong_selector_inputs,
        );
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(&fixture)?)?;
        println!(
            "wrote public-only EIP-2537 disclosure fixture {}",
            path.display()
        );
    }

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;
    let layout = PublicLayoutFixture {
        layout_version: LAYOUT_VERSION,
        protocol_version: protocol_version.clone(),
        supported_subsets: [G4B_BASELINE_SUBSET, G4B_SECOND_SUBSET],
        public_selector_mapping: ["false -> {1,2,3}", "true -> {1,2,4}"],
        trustee_id_transcript_offset: H3_TRUSTEE_ID_OFFSET,
        public_input_order: vec![
            "three repetitions of: CP public-prefix fields then package-commitment fields",
            "one Boolean second-subset selector",
            "three aggregate ciphertext C1 points",
            "registered-weight bound",
            "winner candidate identifier",
        ],
        public_input_count: second_public_inputs.len(),
        selector_public_input_index,
        setup_subset: G4B_BASELINE_SUBSET,
        proof_subset: G4B_SECOND_SUBSET,
        private_plaintext_groups_emitted: false,
        private_tallies_emitted: false,
        proving_key_persisted: false,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let report = G4BResult {
        gate: "G4-B",
        status: "PASS_G4B_REAL_GROTH16_TWO_SUBSET_PUBLIC_SELECTOR",
        classification: "one-groth16-setup-from-123-and-one-proof-for-124-over-complete-p3-with-public-two-subset-selector",
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
        manifest_unchanged,
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        supported_subsets: [G4B_BASELINE_SUBSET, G4B_SECOND_SUBSET],
        public_selector_bound_to_transcript_ids: true,
        coefficients_derived_from_public_selector: true,
        coefficients_are_separate_public_values: false,
        baseline,
        second_subset,
        same_constraints,
        same_public_inputs,
        same_witness_variables,
        same_setup_parameters_used: true,
        mismatched_selector_relation_rejected,
        setup_seed: SETUP_SEED,
        proof_seed: PROOF_SEED,
        setup_ms,
        prove_ms,
        verify_ms,
        wrong_selector_verify_ms,
        peak_rss_after_setup_mib,
        peak_rss_after_prove_mib,
        peak_rss_after_verify_mib,
        proving_key_compressed_bytes: parameters.compressed_size(),
        verifying_key_compressed_bytes: parameters.vk.compressed_size(),
        proof_bytes: proof_encoding.len(),
        valid_second_subset_proof_verified,
        changed_public_selector_rejected,
        selector_public_input_index,
        changed_public_input_count: changed_public_input_indices.len(),
        proving_key_persisted: false,
        groth16_run: true,
        full_p3_for_supported_two_subset_gate: true,
        full_arbitrary_subset_p3: false,
        limitations: vec![
            "the public selector supports only {1,2,3} and {1,2,4}, not all ten legal 3-of-5 subsets",
            "the setup is a deterministic experiment setup, not a production ceremony",
            "the proving key is measured in memory and not persisted",
            "the cryptographic Manifest remains limited to the prior fixed-123 capability",
            "private plaintext group elements and tally values are not emitted",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G4-B: setup-subset={:?} proof-subset={:?} constraints={} public-inputs={} witness-vars={} setup={:.3}ms prove={:.3}ms verify={:.3}ms proof={}B peak-rss={:.3}MiB valid={} wrong-selector-rejected={} groth16=true",
        report.baseline.selected_trustees,
        report.second_subset.selected_trustees,
        report.second_subset.constraints,
        report.second_subset.public_inputs,
        report.second_subset.witness_variables,
        report.setup_ms,
        report.prove_ms,
        report.verify_ms,
        report.proof_bytes,
        report.peak_rss_after_verify_mib,
        report.valid_second_subset_proof_verified,
        report.changed_public_selector_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());
    Ok(())
}
