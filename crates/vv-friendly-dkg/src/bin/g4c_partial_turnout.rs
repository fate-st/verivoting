//! Isolated G4-C tally-v2 experiment: one fresh setup, two turnout witnesses.
//! Private tallies, slack, packages, shares, openings and the proving key are
//! retained only in memory; the optional fixture contains public proof data.

#![recursion_limit = "256"]

use ark_bls12_381::{Bls12_381, Fr};
use ark_ec::Group;
use ark_ff::{PrimeField, Zero};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{
    env, error::Error, fs, io::Write, path::Path, path::PathBuf, process::Command, time::Instant,
};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    verify_decryption_share, GeneralizedCpContext,
};
use vv_friendly_crypto::cp_h4_gadget::H4ACircuit;
use vv_friendly_crypto::p3b_gadget::P3B1TrusteeCircuit;
use vv_friendly_crypto::p3c1_gadget::P3_TALLY_RELATION_ID;
use vv_friendly_crypto::p3c_gadget::{g4b_lagrange_coefficients, G4B_BASELINE_SUBSET};
use vv_friendly_crypto::p3d_gadget::{p3d0_bound_subset_public_inputs, P3D0BoundSubsetCircuit};
use vv_friendly_crypto::{
    encrypt_weighted_vote, fold_ballots, generator, Ciphertext, Point, Scalar, CP_H2F_SUITE_ID,
    PACKAGE_H2F_SUITE_ID,
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
const DEFAULT_SEED: u64 = 20_261_028;
const EXPECTED_PROTOCOL: &str = "VERIVOTING-JUBJUB-PROTOTYPE-G4C-3.0";
const LAYOUT_VERSION: &str = "VV/JUBJUB/G4-C/INPUT-LAYOUT/v2";
const SETUP_SEED: u64 = 0x4734_4353_4554_5550;
const FULL_PROOF_SEED: u64 = 0x4734_4346_554C_4C32;
const PARTIAL_PROOF_SEED: u64 = 0x4734_4350_4152_5432;
const WINNER: u64 = 1;

struct CaseContext<'a> {
    dkg: &'a DkgOutput,
    authentication_keys: &'a [AuthenticationKeypair],
    final_context: [u8; 32],
    registration_close_digest: [u8; 32],
    disclosure_digest: [u8; 32],
    registered_weight: u64,
}

struct CaseFixture {
    circuit: P3D0BoundSubsetCircuit,
    public_inputs: Vec<Fr>,
    native_tally_matches_expected: bool,
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    #[cfg(target_os = "macos")]
    let divisor = 1024.0 * 1024.0;
    #[cfg(not(target_os = "macos"))]
    let divisor = 1024.0;
    usage.ru_maxrss as f64 / divisor
}

fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

fn input_hex(inputs: &[Fr]) -> Result<Vec<String>, Box<dyn Error>> {
    inputs
        .iter()
        .map(|input| Ok(hex(&compressed(input)?)))
        .collect()
}

fn write_new_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn build_case(
    context: &CaseContext<'_>,
    ballots: &[Vec<Ciphertext>],
    expected_tally: [u64; 3],
    rng: &mut ChaCha20Rng,
) -> Result<CaseFixture, Box<dyn Error>> {
    let aggregate = fold_ballots(ballots)?;
    let target = derive_cp_target_with_registration(
        context.dkg.config.election_id,
        context.dkg.config.policy_pre,
        context.final_context,
        context.registration_close_digest,
        context.disclosure_digest,
        &aggregate,
    )?;
    let ciphertext_c0 = aggregate.iter().map(|ct| ct.c0).collect::<Vec<_>>();
    let ciphertext_c1 = aggregate.iter().map(|ct| ct.c1).collect::<Vec<_>>();
    let coefficients = g4b_lagrange_coefficients(false);
    if lagrange_at_zero(
        &G4B_BASELINE_SUBSET,
        context.dkg.config.n,
        context.dkg.config.tau,
    )? != coefficients
    {
        return Err("G4-C interpolation differs from the circuit's subset 123".into());
    }
    let mut trustees = Vec::new();
    let mut verified_shares = Vec::new();
    let mut share_vectors = Vec::new();
    for trustee_id in G4B_BASELINE_SUBSET {
        let cp_context = GeneralizedCpContext {
            election_id: context.dkg.config.election_id,
            policy_pre: context.dkg.config.policy_pre,
            final_context: context.final_context,
            registration_close_digest: context.registration_close_digest,
            disclosure_digest: context.disclosure_digest,
            target,
            trustee_id,
        };
        let verification_share = *context.dkg.verification_share(trustee_id)?;
        let package = prove_decryption_share(
            &cp_context,
            context.dkg.secret_share(trustee_id)?,
            &verification_share,
            &aggregate,
            rng,
        )?;
        verify_decryption_share(&cp_context, &verification_share, &aggregate, &package)?;
        let (transcript, _) =
            transcript_and_challenge(&cp_context, &verification_share, &aggregate, &package)?;
        let (private_package, public_record) = create_private_package(
            cp_context,
            verification_share,
            package.clone(),
            &context.authentication_keys[usize::try_from(trustee_id)? - 1],
            rng,
        )?;
        let verified =
            verify_private_package(context.dkg, &aggregate, &private_package, &public_record)?;
        verified_shares.push(verified.verified_share().clone());
        share_vectors.push(package.shares.clone());
        trustees.push(P3B1TrusteeCircuit {
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
    let native_tally = threshold_decrypt_verified_shares_with_registration(
        context.dkg,
        &G4B_BASELINE_SUBSET,
        &aggregate,
        &verified_shares,
        context.registration_close_digest,
        context.disclosure_digest,
        context.registered_weight,
    )?;
    let plaintext_groups = aggregate
        .iter()
        .enumerate()
        .map(|(coordinate, ciphertext)| {
            let combined = share_vectors.iter().zip(&coefficients).fold(
                Point::zero(),
                |sum, (shares, coefficient)| {
                    sum + shares[coordinate].mul_bigint(coefficient.into_bigint())
                },
            );
            ciphertext.c1 - combined
        })
        .collect::<Vec<_>>();
    let native_tally_matches_expected = native_tally == expected_tally
        && vv_types::winner(&native_tally, context.registered_weight)? == WINNER as usize
        && plaintext_groups
            .iter()
            .zip(expected_tally)
            .all(|(point, tally)| {
                *point == generator().mul_bigint(Scalar::from(tally).into_bigint())
            });
    if !native_tally_matches_expected {
        return Err("G4-C native decryption, group binding or expected winner differs".into());
    }
    let public_inputs = p3d0_bound_subset_public_inputs(
        &trustees,
        false,
        &ciphertext_c1,
        context.registered_weight,
        WINNER,
    )?;
    Ok(CaseFixture {
        circuit: P3D0BoundSubsetCircuit {
            trustees,
            ciphertext_c1,
            plaintext_groups,
            public_second_subset: false,
            tallies: expected_tally.to_vec(),
            registered_weight: context.registered_weight,
            public_winner: WINNER,
        },
        public_inputs,
        native_tally_matches_expected,
    })
}

fn measure_relation(
    circuit: &P3D0BoundSubsetCircuit,
    expected_public_inputs: &[Fr],
) -> Result<Value, Box<dyn Error>> {
    let started = Instant::now();
    let cs = ConstraintSystem::<Fr>::new_ref();
    circuit.clone().generate_constraints(cs.clone())?;
    cs.finalize();
    let satisfied = cs.is_satisfied()?;
    let actual_public_inputs = cs
        .borrow()
        .ok_or("G4-C constraint system was unexpectedly absent")?
        .instance_assignment[1..]
        .to_vec();
    let public_input_packing_matches = actual_public_inputs == expected_public_inputs;
    if !public_input_packing_matches {
        return Err("G4-C public input values/order differ from R1CS instance assignment".into());
    }
    Ok(json!({
        "satisfied": satisfied,
        "constraints": cs.num_constraints(),
        "public_inputs": cs.num_instance_variables() - 1,
        "witness_variables": cs.num_witness_variables(),
        "witness_ms": started.elapsed().as_secs_f64() * 1000.0,
        "witness_timing_scope": "synthesis, finalize, satisfaction check and exact public-input packing check; not witness generation alone",
        "peak_rss_mib": peak_rss_mib(),
        "public_input_packing_matches_r1cs": public_input_packing_matches
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-P3-WEIGHT-V2.json");
    let mut output_path = PathBuf::from("experiments/g4_c_raw/partial_turnout_groth16.json");
    let mut fixture_path = PathBuf::from("fixtures/jubjub_g4c_partial_turnout_v2.json");
    let mut seed = DEFAULT_SEED;
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--fixture-output" => fixture_path = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    if output_path == fixture_path || output_path.exists() || fixture_path.exists() {
        return Err("G4-C output paths must be distinct and must not already exist".into());
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let capability = "EXPERIMENT_ONLY_G4C_TALLY_V2_REQUIRES_FRESH_SETUP";
    if manifest["protocol_version"].as_str() != Some(EXPECTED_PROTOCOL)
        || manifest["p3_tally_relation_id"].as_str() != Some(P3_TALLY_RELATION_ID)
        || manifest["input_layout_version"].as_str() != Some(LAYOUT_VERSION)
        || manifest["capabilities"]["full_p3"].as_str() != Some(capability)
        || manifest["production_setup"].as_bool() != Some(false)
        || !manifest["cp_hash_to_scalar_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(CP_H2F_SUITE_ID))
        || !manifest["commitment_suite"]
            .as_str()
            .is_some_and(|suite| suite.contains(PACKAGE_H2F_SUITE_ID))
        || manifest["n"].as_u64() != Some(5)
        || manifest["t"].as_u64() != Some(2)
        || manifest["tau"].as_u64() != Some(3)
        || manifest["candidate_count"].as_u64() != Some(3)
        || manifest["registered_weight"].as_u64() != Some(10)
    {
        return Err("G4-C requires its isolated tally-v2 experimental manifest; old v1 keys are incompatible".into());
    }
    let base_path = manifest["base_manifest_path"]
        .as_str()
        .ok_or("G4-C descriptor lacks base_manifest_path")?;
    let base_manifest_sha = hex(&sha256(&fs::read(base_path)?));
    if manifest["base_manifest_sha256"].as_str() != Some(base_manifest_sha.as_str()) {
        return Err("G4-C base manifest differs from its pinned SHA-256".into());
    }

    println!("G4-C [1/7] freeze registration policy, run one authenticated 5/3 DKG and close registration");
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let election_id = sha256(b"VV/JUBJUB/G4-C/PARTIAL-TURNOUT/TALLY-v2");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy = RegistrationPolicy::new(authority.public_key(), 10, RULE_ID)?;
    let authentication_keys = (0..5)
        .map(|_| AuthenticationKeypair::generate(&mut rng))
        .collect::<Vec<_>>();
    let dkg_config = DkgConfig::new_with_registration_policy(
        election_id,
        sha256(&manifest_bytes),
        authentication_keys.iter().map(|key| key.public).collect(),
        policy.digest(election_id),
        2,
        3,
    )?;
    let dkg = run_dkg(
        dkg_config,
        &authentication_keys,
        FaultPlan::default(),
        &mut rng,
    )?;
    let final_context =
        derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);
    let weights = [3u64, 2, 1, 4];
    let choices = [1usize, 2, 1, 3];
    let entries = weights
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            RegistrationEntry::new(
                sha256(format!("g4c-voter-credential-{index}").as_bytes()),
                *weight,
                &mut rng,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let close = authority.close(
        entries,
        dkg.config.election_id,
        final_context,
        dkg.config.policy_pre,
        &policy,
        &mut rng,
    )?;
    let registration = verify_close(
        &close,
        &policy,
        dkg.config.election_id,
        final_context,
        dkg.config.policy_pre,
    )?;
    if registration.w_registered() != 10 {
        return Err(
            "G4-C registration fixture did not authenticate the required weight bound".into(),
        );
    }
    let ballots = weights
        .iter()
        .zip(choices)
        .map(|(weight, choice)| {
            encrypt_weighted_vote(&dkg.public_key, *weight, choice, 3, 10, &mut rng)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let context = CaseContext {
        dkg: &dkg,
        authentication_keys: &authentication_keys,
        final_context,
        registration_close_digest: registration.registration_close_digest(),
        disclosure_digest: registration.disclosure_digest(),
        registered_weight: registration.w_registered(),
    };
    println!("G4-C [2/7] build full/partial turnout CP packages and privately check threshold decryption (subset 123)");
    let full = build_case(&context, &ballots, [4, 2, 4], &mut rng)?;
    let partial = build_case(&context, &ballots[..2], [3, 2, 0], &mut rng)?;
    println!("G4-C [3/7] synthesize both complete tally-v2 relations and verify exact public-input packing");
    let full_shape = measure_relation(&full.circuit, &full.public_inputs)?;
    let partial_shape = measure_relation(&partial.circuit, &partial.public_inputs)?;
    if full_shape["satisfied"] != true || partial_shape["satisfied"] != true {
        return Err("G4-C a valid turnout relation was unsatisfied".into());
    }
    for key in ["constraints", "public_inputs", "witness_variables"] {
        if full_shape[key] != partial_shape[key] {
            return Err(format!("G4-C turnout witnesses changed the R1CS {key} shape").into());
        }
    }
    println!(
        "G4-C [4/7] test one sum-over-bound witness using the same partial-turnout CP material"
    );
    if !partial.circuit.tallies.iter().all(|tally| *tally <= 4)
        || partial.circuit.tallies.iter().sum::<u64>() <= 4
    {
        return Err(
            "G4-C negative fixture must have each tally within bound but its sum above bound"
                .into(),
        );
    }
    let mut over_bound = partial.circuit.clone();
    over_bound.registered_weight = 4;
    let over_bound_inputs = p3d0_bound_subset_public_inputs(
        &over_bound.trustees,
        false,
        &over_bound.ciphertext_c1,
        4,
        WINNER,
    )?;
    let over_bound_shape = measure_relation(&over_bound, &over_bound_inputs)?;
    let over_bound_rejected = over_bound_shape["satisfied"] == false;
    if !over_bound_rejected {
        return Err("G4-C sum-over-bound witness was not rejected".into());
    }

    println!(
        "G4-C [5/7] one fresh deterministic research Groth16 setup for tally-v2 (no old key reuse)"
    );
    let mut setup_rng = ChaCha20Rng::seed_from_u64(SETUP_SEED);
    let started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        full.circuit.clone(),
        &mut setup_rng,
    )?;
    let setup_ms = started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_after_setup_mib = peak_rss_mib();
    let verifying_key_bytes = compressed(&parameters.vk)?;
    let verifying_key_sha256 = hex(&sha256(&verifying_key_bytes));
    let prepared = prepare_verifying_key(&parameters.vk);
    if parameters.vk.gamma_abc_g1.len() != full.public_inputs.len() + 1 {
        return Err("G4-C verifying key does not match the measured public-input layout".into());
    }
    let mut case_results = Vec::new();
    let mut public_cases = Vec::new();
    let mut wrong_winner_verify_ms = 0.0;
    let mut wrong_winner_rejected = false;
    let mut winner_input_index = None;
    println!(
        "G4-C [6/7] prove and verify full then partial turnout serially with the same fresh key"
    );
    for (name, fixture, shape, proof_seed) in [
        ("full_turnout", &full, full_shape, FULL_PROOF_SEED),
        (
            "partial_turnout",
            &partial,
            partial_shape,
            PARTIAL_PROOF_SEED,
        ),
    ] {
        println!("G4-C {name}: creating real Groth16 proof");
        let mut proof_rng = ChaCha20Rng::seed_from_u64(proof_seed);
        let started = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            fixture.circuit.clone(),
            &parameters,
            &mut proof_rng,
        )?;
        let prove_ms = started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_prove_mib = peak_rss_mib();
        let started = Instant::now();
        let verified =
            Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &fixture.public_inputs)?;
        let verify_ms = started.elapsed().as_secs_f64() * 1000.0;
        if !verified {
            return Err(format!("G4-C {name} proof did not verify").into());
        }
        let proof_bytes = compressed(&proof)?;
        let mut public_case = json!({
            "case_name": name,
            "public_winner": WINNER,
            "public_inputs_ark_compressed_le_hex": input_hex(&fixture.public_inputs)?,
            "proof_ark_compressed_hex": hex(&proof_bytes),
            "proof_sha256": hex(&sha256(&proof_bytes))
        });
        if name == "partial_turnout" {
            let wrong_winner_inputs = p3d0_bound_subset_public_inputs(
                &fixture.circuit.trustees,
                false,
                &fixture.circuit.ciphertext_c1,
                10,
                2,
            )?;
            let changed = fixture
                .public_inputs
                .iter()
                .zip(&wrong_winner_inputs)
                .enumerate()
                .filter_map(|(index, (a, b))| (a != b).then_some(index))
                .collect::<Vec<_>>();
            if changed.as_slice() != [fixture.public_inputs.len() - 1] {
                return Err("G4-C wrong-winner mutation changed more than the winner input".into());
            }
            winner_input_index = changed.first().copied();
            let started = Instant::now();
            wrong_winner_rejected =
                !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &wrong_winner_inputs)?;
            wrong_winner_verify_ms = started.elapsed().as_secs_f64() * 1000.0;
            public_case["changed_winner_public_inputs_ark_compressed_le_hex"] =
                json!(input_hex(&wrong_winner_inputs)?);
        }
        case_results.push(json!({
            "case_name": name,
            "native_tally_matches_expected": fixture.native_tally_matches_expected,
            "shape": shape,
            "public_winner": WINNER,
            "proof_seed": proof_seed,
            "prove_ms": prove_ms,
            "verify_ms": verify_ms,
            "peak_rss_after_prove_mib": peak_rss_after_prove_mib,
            "peak_rss_after_verify_mib": peak_rss_mib(),
            "proof_bytes": proof_bytes.len(),
            "proof_verified": verified,
            "verifying_key_sha256": verifying_key_sha256
        }));
        public_cases.push(public_case);
        println!("G4-C {name}: valid={verified}, prove={prove_ms:.3}ms verify={verify_ms:.3}ms proof={}B peak-rss={:.3}MiB", proof_bytes.len(), peak_rss_mib());
    }
    if !wrong_winner_rejected {
        return Err("G4-C valid partial-turnout proof accepted a changed public winner".into());
    }
    if fs::read(&manifest_path)? != manifest_bytes {
        return Err("G4-C manifest changed during the experiment".into());
    }
    println!("G4-C [7/7] write public-only v2 fixture and measured experiment record");
    let fixture = json!({
        "layout_version": LAYOUT_VERSION,
        "p3_tally_relation_id": P3_TALLY_RELATION_ID,
        "protocol_version": EXPECTED_PROTOCOL,
        "manifest_sha256": hex(&sha256(&manifest_bytes)),
        "encoding": "arkworks-0.4 canonical compressed; scalar public inputs are 32-byte little-endian; not EIP-2537 calldata",
        "curve": "BLS12-381",
        "selected_trustees": G4B_BASELINE_SUBSET,
        "public_second_subset": false,
        "public_input_count": full.public_inputs.len(),
        "public_input_order": ["three repetitions: CP public-prefix fields then package commitment fields", "Boolean two-subset selector", "three aggregate ciphertext C1 points", "registered weight bound", "winner candidate identifier"],
        "winner_public_input_index": winner_input_index,
        "verifying_key_ark_compressed_hex": hex(&verifying_key_bytes),
        "verifying_key_sha256": verifying_key_sha256,
        "cases": public_cases,
        "private_witness_emitted": false,
        "private_tallies_emitted": false,
        "private_slack_emitted": false,
        "private_plaintext_groups_emitted": false,
        "private_packages_emitted": false,
        "proving_key_persisted": false,
        "production_setup": false
    });
    let source_paths = [
        "crates/vv-friendly-dkg/src/bin/g4c_partial_turnout.rs",
        "crates/vv-friendly-crypto/src/p3c1_gadget.rs",
        "crates/vv-friendly-crypto/src/p3d_gadget.rs",
    ];
    let source_hashes = source_paths
        .iter()
        .map(|path| Ok(json!({"path": path, "sha256": hex(&sha256(&fs::read(path)?))})))
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let report = json!({
        "gate": "G4-C",
        "status": "PASS_G4C_REAL_GROTH16_PARTIAL_TURNOUT_TALLY_V2",
        "classification": "one-fresh-v2-setup-two-real-proofs-full-and-partial-turnout-fixed-123",
        "git_commit": command_output("git", &["rev-parse", "HEAD"]).unwrap_or_else(|| "UNKNOWN".to_owned()),
        "git_dirty": command_output("git", &["status", "--porcelain", "--untracked-files=normal"]).map(|value| !value.is_empty()),
        "build_profile": if cfg!(debug_assertions) { "dev-unoptimized" } else { "release-opt-level-1" },
        "rustc_version": command_output(&env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()), &["--version"]).unwrap_or_else(|| "UNKNOWN".to_owned()),
        "os": env::consts::OS,
        "arch": env::consts::ARCH,
        "cpu": command_output("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|| "UNKNOWN".to_owned()),
        "prototype_manifest": manifest_path,
        "manifest_sha256": hex(&sha256(&manifest_bytes)),
        "protocol_version": EXPECTED_PROTOCOL,
        "manifest_capability": capability,
        "base_manifest_sha256": base_manifest_sha,
        "p3_tally_relation_id": P3_TALLY_RELATION_ID,
        "layout_version": LAYOUT_VERSION,
        "fixture_file": fixture_path,
        "source_hashes": source_hashes,
        "seed": seed,
        "selected_trustees": G4B_BASELINE_SUBSET,
        "public_second_subset": false,
        "registered_weight": registration.w_registered(),
        "same_dkg_and_registration_close": true,
        "same_constraints": true,
        "same_public_input_count": true,
        "same_witness_variables": true,
        "same_setup_parameters_used": true,
        "fresh_setup_for_tally_v2": true,
        "old_v1_verifying_key_reused": false,
        "setup_seed": SETUP_SEED,
        "setup_ms": setup_ms,
        "peak_rss_after_setup_mib": peak_rss_after_setup_mib,
        "peak_rss_after_verify_mib": peak_rss_mib(),
        "proving_key_compressed_bytes": parameters.compressed_size(),
        "verifying_key_compressed_bytes": verifying_key_bytes.len(),
        "verifying_key_sha256": verifying_key_sha256,
        "cases": case_results,
        "negative_tests": {
            "changed_partial_public_winner_rejected": wrong_winner_rejected,
            "wrong_winner_verify_ms": wrong_winner_verify_ms,
            "winner_public_input_index": winner_input_index,
            "same_partial_cp_material_sum_over_public_bound_rejected": over_bound_rejected,
            "sum_over_bound_shape": over_bound_shape,
            "sum_over_bound_public_registered_weight": 4,
            "sum_over_bound_each_tally_within_bound_checked": partial.circuit.tallies.iter().all(|tally| *tally <= 4)
        },
        "setup_count": 1,
        "proof_count": 2,
        "groth16_run": true,
        "evm_run": false,
        "all_ten_subsets_tested": false,
        "production_setup": false,
        "private_tallies_emitted": false,
        "private_slack_emitted": false,
        "private_plaintext_groups_emitted": false,
        "private_packages_emitted": false,
        "private_decryption_shares_emitted": false,
        "private_openings_emitted": false,
        "proving_key_persisted": false,
        "limitations": [
            "only subset 123 was measured; the existing two-subset relation is not arbitrary-subset P3",
            "deterministic publicly known research setup seeds are unsuitable for production",
            "this gate checks complete P3 with authenticated registration weight, not a composed ballot/state/settlement proof",
            "no v2 on-chain verifier or SnarkPack EVM verification was measured",
            "two witnesses are functional correctness cases, not repeated performance samples"
        ]
    });
    write_new_json(&fixture_path, &fixture)?;
    write_new_json(&output_path, &report)?;
    println!("G4-C PASS: one fresh setup, two verified proofs, changed winner rejected, sum-over-bound rejected; private tallies not emitted");
    println!("wrote {}", output_path.display());
    println!("wrote {}", fixture_path.display());
    Ok(())
}
