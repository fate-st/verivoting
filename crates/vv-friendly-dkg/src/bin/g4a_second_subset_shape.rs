use ark_ec::Group;
use ark_ff::{Field, PrimeField, Zero};
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
use vv_friendly_crypto::cp_h4_gadget::H4ACircuit;
use vv_friendly_crypto::p3b_gadget::P3B1TrusteeCircuit;
use vv_friendly_crypto::p3d_gadget::{check_winner_linkage_p3d0_for_subset, P3D0CircuitReport};
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
const BASELINE_SUBSET: [u32; 3] = [1, 2, 3];
const SECOND_SUBSET: [u32; 3] = [1, 2, 4];
const EXPECTED_TALLY: [u64; 3] = [4, 2, 4];
const PUBLIC_WINNER: u64 = 1;

#[derive(Clone)]
struct TrusteeMaterial {
    circuit: P3B1TrusteeCircuit,
    shares: Vec<Point>,
    verified_share: VerifiedDecryptionShare,
}

struct MeasurementContext<'a> {
    output: &'a DkgOutput,
    aggregate: &'a [Ciphertext],
    ciphertext_c1: &'a [Point],
    materials: &'a HashMap<u32, TrusteeMaterial>,
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    registered_weight: u64,
}

#[derive(Serialize)]
struct HistoricalFixedBaseline {
    artifact: String,
    status: String,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
}

#[derive(Serialize)]
struct SubsetMeasurement {
    selected_trustees: [u32; 3],
    lagrange_coefficient_labels: [&'static str; 3],
    lagrange_coefficients_canonical_le_hex: Vec<String>,
    native_coefficients_match_expected: bool,
    native_cp_verified: usize,
    native_packages_verified: usize,
    native_threshold_tally: Vec<u64>,
    native_threshold_tally_matches_fixture: bool,
    native_winner: usize,
    native_winner_matches_fixture: bool,
    relation_satisfied: bool,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    trustees_in_r1cs: usize,
    cp_equations_in_r1cs: usize,
    package_h2f_relations_in_r1cs: usize,
    package_opening_equations_in_r1cs: usize,
    lagrange_combination_equations_in_r1cs: usize,
    tally_group_binding_equations_in_r1cs: usize,
    tally_range_relations_in_r1cs: usize,
    tally_sum_relations_in_r1cs: usize,
    winner_candidate_flags_in_r1cs: usize,
    winner_pairwise_relations_in_r1cs: usize,
}

#[derive(Serialize)]
struct G4AResult {
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
    seed: u64,
    historical_fixed_123: HistoricalFixedBaseline,
    generalized_123: SubsetMeasurement,
    generalized_124: SubsetMeasurement,
    same_constraints: bool,
    same_public_inputs: bool,
    same_witness_variables: bool,
    same_relation_component_counts: bool,
    common_count_shape_confirmed: bool,
    public_input_layout_preserved: bool,
    setup_matrices_proven_identical: bool,
    coefficient_binding_is_public: bool,
    g4b_parameterization_required: bool,
    groth16_run: bool,
    full_p3: bool,
    peak_rss_mib: f64,
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

fn expected_coefficients(subset: [u32; 3]) -> Result<[Scalar; 3], Box<dyn Error>> {
    if subset == BASELINE_SUBSET {
        return Ok([Scalar::from(3u64), -Scalar::from(3u64), Scalar::from(1u64)]);
    }
    if subset == SECOND_SUBSET {
        let inverse_three = Scalar::from(3u64)
            .inverse()
            .ok_or("three is not invertible in the Jubjub scalar field")?;
        return Ok([
            Scalar::from(8u64) * inverse_three,
            -Scalar::from(2u64),
            inverse_three,
        ]);
    }
    Err("G4-A only measures subsets 123 and 124".into())
}

fn measure_subset(
    context: &MeasurementContext<'_>,
    subset: [u32; 3],
    coefficient_labels: [&'static str; 3],
) -> Result<SubsetMeasurement, Box<dyn Error>> {
    let coefficients_vec =
        lagrange_at_zero(&subset, context.output.config.n, context.output.config.tau)?;
    let coefficients: [Scalar; 3] = coefficients_vec
        .clone()
        .try_into()
        .map_err(|_| "unexpected Lagrange coefficient count")?;
    let native_coefficients_match_expected = coefficients == expected_coefficients(subset)?;

    let circuits = subset
        .iter()
        .map(|id| {
            context
                .materials
                .get(id)
                .map(|material| material.circuit.clone())
                .ok_or_else(|| format!("missing Trustee {id} circuit material"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let share_vectors = subset
        .iter()
        .map(|id| {
            context
                .materials
                .get(id)
                .map(|material| material.shares.clone())
                .ok_or_else(|| format!("missing Trustee {id} share material"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let verified_shares = subset
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
        &subset,
        context.aggregate,
        &verified_shares,
        context.registration_close_digest,
        context.disclosure_digest,
        context.registered_weight,
    )?;
    let native_threshold_tally_matches_fixture = native_tally == EXPECTED_TALLY
        && plaintext_groups
            .iter()
            .zip(EXPECTED_TALLY)
            .all(|(point, tally)| {
                *point == generator().mul_bigint(Scalar::from(tally).into_bigint())
            });
    let native_winner = vv_types::winner(&native_tally, context.registered_weight)?;
    let native_winner_matches_fixture = native_winner == PUBLIC_WINNER as usize;

    let started = Instant::now();
    let report = check_winner_linkage_p3d0_for_subset(
        &circuits,
        context.ciphertext_c1,
        &plaintext_groups,
        coefficients,
        &EXPECTED_TALLY,
        context.registered_weight,
        PUBLIC_WINNER,
    )?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    if !native_coefficients_match_expected
        || !native_threshold_tally_matches_fixture
        || !native_winner_matches_fixture
        || !report.satisfied
    {
        return Err(format!("G4-A subset {subset:?} failed native or R1CS consistency").into());
    }

    Ok(subset_measurement(
        subset,
        coefficient_labels,
        coefficients,
        native_tally,
        native_winner,
        native_coefficients_match_expected,
        native_threshold_tally_matches_fixture,
        native_winner_matches_fixture,
        report,
        witness_ms,
    ))
}

#[allow(clippy::too_many_arguments)]
fn subset_measurement(
    selected_trustees: [u32; 3],
    lagrange_coefficient_labels: [&'static str; 3],
    coefficients: [Scalar; 3],
    native_threshold_tally: Vec<u64>,
    native_winner: usize,
    native_coefficients_match_expected: bool,
    native_threshold_tally_matches_fixture: bool,
    native_winner_matches_fixture: bool,
    report: P3D0CircuitReport,
    witness_ms: f64,
) -> SubsetMeasurement {
    SubsetMeasurement {
        selected_trustees,
        lagrange_coefficient_labels,
        lagrange_coefficients_canonical_le_hex: coefficients
            .iter()
            .map(|coefficient| hex(&encode_scalar(coefficient)))
            .collect(),
        native_coefficients_match_expected,
        native_cp_verified: selected_trustees.len(),
        native_packages_verified: selected_trustees.len(),
        native_threshold_tally,
        native_threshold_tally_matches_fixture,
        native_winner,
        native_winner_matches_fixture,
        relation_satisfied: report.satisfied,
        constraints: report.constraints,
        public_inputs: report.public_inputs,
        witness_variables: report.witness_variables,
        witness_ms,
        trustees_in_r1cs: report.trustees,
        cp_equations_in_r1cs: report.cp_equations,
        package_h2f_relations_in_r1cs: report.package_h2f_relations,
        package_opening_equations_in_r1cs: report.package_opening_equations,
        lagrange_combination_equations_in_r1cs: report.lagrange_combination_equations,
        tally_group_binding_equations_in_r1cs: report.tally_group_binding_equations,
        tally_range_relations_in_r1cs: report.tally_range_relations,
        tally_sum_relations_in_r1cs: report.tally_sum_relations,
        winner_candidate_flags_in_r1cs: report.winner_candidate_flags,
        winner_pairwise_relations_in_r1cs: report.winner_pairwise_relations,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut baseline_path = PathBuf::from("experiments/g0_9_p3d1_raw/p3d1_full_groth16.json");
    let mut output_path = PathBuf::from("experiments/g4_a_raw/second_subset_p3_shape.json");
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--baseline" => baseline_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    if manifest["p3_tally_relation_id"].as_str()
        != Some(vv_friendly_crypto::p3c1_gadget::P3_TALLY_RELATION_ID)
    {
        return Err("P3 tally relation changed to v2; archived v1 manifests/keys cannot be reused. Run friendly-g4c-partial-turnout with its v2 experiment manifest.".into());
    }
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

    let baseline_bytes = fs::read(&baseline_path)?;
    let baseline: Value = serde_json::from_slice(&baseline_bytes)?;
    let historical_fixed_123 = HistoricalFixedBaseline {
        artifact: baseline_path.display().to_string(),
        status: baseline["status"].as_str().unwrap_or("UNKNOWN").to_owned(),
        constraints: baseline["constraints"]
            .as_u64()
            .ok_or("baseline lacks constraints")? as usize,
        public_inputs: baseline["public_inputs"]
            .as_u64()
            .ok_or("baseline lacks public_inputs")? as usize,
        witness_variables: baseline["witness_variables"]
            .as_u64()
            .ok_or("baseline lacks witness_variables")? as usize,
    };

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let election_id = sha256(b"VV/JUBJUB/G4-A/SECOND-SUBSET-P3-SHAPE");
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
                sha256(format!("g4a-voter-credential-{index}").as_bytes()),
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

    let measurement_context = MeasurementContext {
        output: &dkg,
        aggregate: &aggregate,
        ciphertext_c1: &ciphertext_c1,
        materials: &materials,
        registration_close_digest,
        disclosure_digest,
        registered_weight: registration.w_registered(),
    };
    let generalized_123 = measure_subset(&measurement_context, BASELINE_SUBSET, ["3", "-3", "1"])?;
    let generalized_124 =
        measure_subset(&measurement_context, SECOND_SUBSET, ["8/3", "-2", "1/3"])?;

    let same_constraints = generalized_123.constraints == generalized_124.constraints;
    let same_public_inputs = generalized_123.public_inputs == generalized_124.public_inputs;
    let same_witness_variables =
        generalized_123.witness_variables == generalized_124.witness_variables;
    let same_relation_component_counts = generalized_123.trustees_in_r1cs
        == generalized_124.trustees_in_r1cs
        && generalized_123.cp_equations_in_r1cs == generalized_124.cp_equations_in_r1cs
        && generalized_123.package_h2f_relations_in_r1cs
            == generalized_124.package_h2f_relations_in_r1cs
        && generalized_123.package_opening_equations_in_r1cs
            == generalized_124.package_opening_equations_in_r1cs
        && generalized_123.lagrange_combination_equations_in_r1cs
            == generalized_124.lagrange_combination_equations_in_r1cs
        && generalized_123.tally_group_binding_equations_in_r1cs
            == generalized_124.tally_group_binding_equations_in_r1cs
        && generalized_123.tally_range_relations_in_r1cs
            == generalized_124.tally_range_relations_in_r1cs
        && generalized_123.tally_sum_relations_in_r1cs
            == generalized_124.tally_sum_relations_in_r1cs
        && generalized_123.winner_candidate_flags_in_r1cs
            == generalized_124.winner_candidate_flags_in_r1cs
        && generalized_123.winner_pairwise_relations_in_r1cs
            == generalized_124.winner_pairwise_relations_in_r1cs;
    let common_count_shape_confirmed = generalized_123.relation_satisfied
        && generalized_124.relation_satisfied
        && same_constraints
        && same_public_inputs
        && same_witness_variables
        && same_relation_component_counts;
    if !common_count_shape_confirmed {
        return Err("G4-A did not find a common generalized R1CS count shape".into());
    }
    let public_input_layout_preserved = generalized_123.public_inputs
        == historical_fixed_123.public_inputs
        && generalized_124.public_inputs == historical_fixed_123.public_inputs;

    let report = G4AResult {
        gate: "G4-A",
        status: "PASS_G4A_SECOND_SUBSET_COMMON_R1CS_COUNT_SHAPE",
        classification: "complete-p3-r1cs-for-fixed-123-and-fixed-124-through-one-generic-coefficient-path-no-groth16-no-public-subset-binding",
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
        seed,
        historical_fixed_123,
        generalized_123,
        generalized_124,
        same_constraints,
        same_public_inputs,
        same_witness_variables,
        same_relation_component_counts,
        common_count_shape_confirmed,
        public_input_layout_preserved,
        setup_matrices_proven_identical: false,
        coefficient_binding_is_public: false,
        g4b_parameterization_required: true,
        groth16_run: false,
        full_p3: false,
        peak_rss_mib: peak_rss_mib(),
        limitations: vec![
            "G4-A measures only the two legal subsets {1,2,3} and {1,2,4}; it does not test all ten 3-of-5 subsets",
            "Lagrange coefficients are constrained setup-time circuit parameters, not verifier-selected public inputs",
            "equal counts and public layout do not prove identical R1CS matrices or proving-key reuse",
            "G4-A performs no Groth16 setup, proof or verification",
            "the cryptographic Manifest remains limited to the prior fixed-123 complete P3 proof",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G4-A: baseline={:?} second={:?} constraints={}/{} public-inputs={}/{} witness-vars={}/{} witness-ms={:.3}/{:.3} peak-rss={:.3}MiB common-count-shape={} groth16=false",
        report.generalized_123.selected_trustees,
        report.generalized_124.selected_trustees,
        report.generalized_123.constraints,
        report.generalized_124.constraints,
        report.generalized_123.public_inputs,
        report.generalized_124.public_inputs,
        report.generalized_123.witness_variables,
        report.generalized_124.witness_variables,
        report.generalized_123.witness_ms,
        report.generalized_124.witness_ms,
        report.peak_rss_mib,
        report.common_count_shape_confirmed,
    );
    println!("wrote {}", output_path.display());
    Ok(())
}
