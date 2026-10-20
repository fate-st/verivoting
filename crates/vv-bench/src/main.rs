use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use vv_crypto::{
    aggregate_vectors, bounded_discrete_log, ciphertext_vector_digest, combine_decryption_shares,
    commit_bytes, decryption_transcript_and_challenge, encrypt, generator, point_bytes,
    prove_decryption, scalar_bytes, verify_commitment, verify_decryption, verify_signature,
    Ciphertext, DecryptionProof, PedersenCommitment, SchnorrSignature, ThresholdContext,
};
use vv_disclosure::{
    cp_gadget::{
        decompress_ristretto, preflight_cp_equation, profile_cp_primitives,
        run_cp_equation_groth16, CpEquationCircuit, CpFeasibilityMetrics, CpPreflightMetrics,
        CpPrimitiveCosts, CpScalarMulVersion, EdwardsAffineValue,
    },
    friendly_curve::{run_friendly_curve_experiment, FriendlyCurveReport},
    run_p1_groth16, P1Metrics,
};
use vv_dkg::{run_dkg, DkgConfig, DkgOutput, FaultPlan};
use vv_registry::{
    disclosure_digest, policy_pre_digest, verify_close, RegistrationAuthority, RegistrationEntry,
};
use vv_types::{
    derive_dkg_session_id, derive_final_context, derive_prekey_context, hex, sha256, winner,
    CanonicalEncoder, ProtocolManifest,
};

const CSV_HEADER: &str = "run_id,experiment,mode,git_commit,manifest_sha256,curve_suite,os,arch,cpu,logical_threads,ram_gb,rustc_version,compiler_flags,n_voters,n_candidates,n_trustees,t_corrupt,tau,q_cert,batch_size,region_count,merkle_depth,w_registered,w_max,seed_id,proof_type,stage,success,abort_reason,proof_bytes,public_input_bytes,private_bytes,constraints,setup_ms,witness_ms,prove_ms,verify_ms,aggregation_ms,cp_ms,dlog_ms,peak_rss_mb,gas_kind,total_gas,calldata_bytes,da_bytes\n";

#[derive(Clone, Debug)]
struct Options {
    command: String,
    manifest: PathBuf,
    output: PathBuf,
    trials: usize,
    seed: u64,
    max_cp_constraints: usize,
    cp_implementation: String,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = env::args().skip(1);
        let command = args.next().unwrap_or_else(|| "help".to_owned());
        let mut options = Self {
            command,
            manifest: PathBuf::from("configs/protocol_manifest.json"),
            output: PathBuf::from("experiments/raw"),
            trials: 1,
            seed: 20261019,
            max_cp_constraints: 1_000_000,
            cp_implementation: "canonical_253".to_owned(),
        };
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--manifest" => options.manifest = PathBuf::from(value),
                "--output" => options.output = PathBuf::from(value),
                "--trials" => options.trials = value.parse()?,
                "--seed" => options.seed = value.parse()?,
                "--max-cp-constraints" => options.max_cp_constraints = value.parse()?,
                "--cp-implementation" => options.cp_implementation = value,
                _ => return Err(format!("unknown option {flag}").into()),
            }
        }
        Ok(options)
    }
}

#[derive(Serialize)]
struct DkgReport {
    classification: &'static str,
    n: usize,
    t: usize,
    tau: usize,
    q_cert: usize,
    elapsed_ms: f64,
    complaints: usize,
    corrected_complaints: usize,
    disqualified_dealers: Vec<u32>,
    extraction_recoveries: Vec<u32>,
    certificate_signers: usize,
    certificate_valid: bool,
    public_key: String,
    election_id: String,
    policy_pre: String,
    manifest_digest: String,
    prekey_context: String,
    session_id: String,
    key_core_digest: String,
    transcript_digest: String,
    estimated_protocol_bytes: usize,
    execution_model: &'static str,
}

#[derive(Serialize)]
struct DlogSample {
    bound: u64,
    expected: u64,
    decoded: u64,
    decode_ms: f64,
    algorithm: &'static str,
}

#[derive(Serialize)]
struct PublicWinnerEvent {
    protocol_version: String,
    classification: &'static str,
    election_id: String,
    context: String,
    policy_pre: String,
    dkg_session_id: String,
    key_core_digest: String,
    registration_root: String,
    w_registered: u64,
    w_max: u64,
    target: String,
    selected_trustees: Vec<u32>,
    public_commitments: Vec<String>,
    public_signatures: Vec<String>,
    winner: usize,
    p1_groth16_verified: bool,
    full_p3_generated: bool,
    public_tally_present: bool,
}

#[derive(Serialize)]
struct E2eReport {
    classification: &'static str,
    gate_status: &'static str,
    public_event: PublicWinnerEvent,
    dkg_ms: f64,
    ra_close_ms: f64,
    vote_encrypt_ms: f64,
    cp_prove_verify_ms: f64,
    combine_decode_ms: f64,
    p1_metrics: P1Metrics,
    all_legal_subsets_checked: usize,
    public_bytes: usize,
    private_package_bytes: usize,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct CpPointVector {
    compressed_hex: String,
    edwards_x_decimal: String,
    edwards_y_decimal: String,
}

#[derive(Serialize)]
struct CpCompatibilityVector {
    vector_id: &'static str,
    classification: &'static str,
    election_id: String,
    context: String,
    policy_pre: String,
    disclosure_digest: String,
    target: String,
    trustee: u32,
    generator: CpPointVector,
    verification_key: CpPointVector,
    ciphertext_c0: Vec<String>,
    decryption_shares: Vec<String>,
    commitment_a0: CpPointVector,
    coordinate_commitments: Vec<String>,
    response_scalar_le_hex: String,
    challenge_scalar_le_hex: String,
    rejection_counter: u16,
    transcript_hex: String,
    transcript_sha256: String,
    native_cp_verified: bool,
    circuit_equation: &'static str,
    circuit_limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct CpFeasibilityReport {
    gate: &'static str,
    status: &'static str,
    build_profile: &'static str,
    rustc_version: String,
    peak_rss_mb: f64,
    native_cp_verified: bool,
    max_cp_constraints: usize,
    preflight: CpPreflightMetrics,
    primitive_costs: CpPrimitiveCosts,
    groth16: Option<CpFeasibilityMetrics>,
    next_decision: &'static str,
}

#[derive(Serialize)]
struct FriendlyCurveDecisionReport {
    gate: &'static str,
    status: &'static str,
    classification: &'static str,
    build_profile: &'static str,
    rustc_version: String,
    baseline_commit: &'static str,
    experiment_revision: &'static str,
    manifest_sha256: String,
    native_curve_suite: String,
    manifest_changed: bool,
    elapsed_ms: f64,
    peak_rss_mb: f64,
    experiment: FriendlyCurveReport,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    if options.command == "help" || options.command == "--help" {
        print_help();
        return Ok(());
    }
    fs::create_dir_all(&options.output)?;
    let manifest = ProtocolManifest::from_path(&options.manifest)?;
    match options.command.as_str() {
        "manifest" => run_manifest(&manifest),
        "p1" => run_p1(&manifest, &options)?,
        "dkg" => run_dkg_experiment(&manifest, &options)?,
        "dlog" => run_dlog(&manifest, &options)?,
        "e2e" => run_e2e(&manifest, &options)?,
        "p3-feasibility" => run_p3_feasibility(&manifest, &options)?,
        "cp-optimize" => run_cp_optimization(&manifest, &options)?,
        "curve-compare" => run_curve_compare(&manifest, &options)?,
        "all" => {
            run_manifest(&manifest);
            run_dkg_experiment(&manifest, &options)?;
            run_p1(&manifest, &options)?;
            run_dlog(&manifest, &options)?;
            run_e2e(&manifest, &options)?;
        }
        command => return Err(format!("unknown command {command}").into()),
    }
    Ok(())
}

fn print_help() {
    println!(
        "vv-bench <manifest|p1|dkg|dlog|e2e|p3-feasibility|cp-optimize|curve-compare|all> [--manifest PATH] [--output DIR] [--trials N] [--seed N] [--max-cp-constraints N] [--cp-implementation baseline_256|canonical_253]"
    );
    println!("Classification: native core and real P1 Groth16; no SnarkPack, state proof, full P3, or chain gas claim.");
}

fn run_manifest(manifest: &ProtocolManifest) {
    println!("manifest core validation: PASS");
    println!("manifest_sha256={}", manifest.digest_hex());
    match manifest.validate_full_submission() {
        Ok(()) => println!("full submission validation: PASS"),
        Err(error) => println!("full submission validation: BLOCKED ({error})"),
    }
}

fn run_curve_compare(manifest: &ProtocolManifest, options: &Options) -> Result<(), Box<dyn Error>> {
    let rss_before = peak_rss_mb();
    let started = Instant::now();
    let experiment = run_friendly_curve_experiment(options.seed)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_mb = peak_rss_mb().max(rss_before);
    let accepted = experiment.cp.valid_proof_verified
        && experiment.elgamal_coordinate.valid_proof_verified
        && experiment.state_point_addition.valid_proof_verified
        && experiment.cp.changed_public_input_rejected
        && experiment.elgamal_coordinate.changed_public_input_rejected
        && experiment
            .state_point_addition
            .changed_public_input_rejected
        && experiment.cp_wrong_commitment_rejected
        && experiment.cp_wrong_challenge_rejected
        && experiment.cp_wrong_response_rejected
        && !experiment.manifest_or_native_protocol_changed
        && !experiment.full_p3;
    if !accepted {
        return Err("friendly-curve experiment failed an acceptance condition".into());
    }

    let report = FriendlyCurveDecisionReport {
        gate: "G0.6",
        status: "PARTIAL_PASS_ISOLATED_CURVE_EXPERIMENT_FULL_P3_BLOCKED",
        classification: "real-groth16-isolated-curve-decision-data",
        build_profile: build_profile(),
        rustc_version: rustc_version(),
        baseline_commit: "92d147001fe84c58f85d7692ff9980b4684cd193",
        experiment_revision: "uncommitted_g0_6_worktree",
        manifest_sha256: manifest.digest_hex(),
        native_curve_suite: manifest.encryption_group.clone(),
        manifest_changed: false,
        elapsed_ms,
        peak_rss_mb,
        experiment,
    };
    write_json(&options.output.join("p3-curve-friendly.json"), &report)?;
    println!(
        "G0.6 isolated friendly curve: CP={} constraints, ElGamal={} constraints, state-add={} constraints, total={:.3}ms peak_rss={:.1}MiB full_p3=false",
        report.experiment.cp.constraints,
        report.experiment.elgamal_coordinate.constraints,
        report.experiment.state_point_addition.constraints,
        report.elapsed_ms,
        report.peak_rss_mb,
    );
    Ok(())
}

fn run_p1(manifest: &ProtocolManifest, options: &Options) -> Result<(), Box<dyn Error>> {
    for trial in 0..options.trials {
        let metrics = run_p1_groth16(&[4, 2, 4], 10, options.seed + trial as u64)?;
        let path = options.output.join(format!("p1-trial-{trial}.json"));
        write_json(&path, &metrics)?;
        append_csv(
            &options.output.join("runs.csv"),
            manifest,
            &format!("p1-{trial}"),
            "p1",
            4,
            10,
            "P1_GROTH16",
            metrics.verified,
            metrics.proof_bytes,
            0,
            metrics.constraints,
            metrics.setup_ms,
            metrics.prove_ms,
            metrics.verify_ms,
            0.0,
            options.seed + trial as u64,
        )?;
        println!(
            "P1 trial {trial}: constraints={} setup={:.3}ms prove={:.3}ms verify={:.3}ms verified={}",
            metrics.constraints,
            metrics.setup_ms,
            metrics.prove_ms,
            metrics.verify_ms,
            metrics.verified
        );
    }
    Ok(())
}

fn dkg_config(
    manifest: &ProtocolManifest,
    election_id: [u8; 32],
    policy_pre: [u8; 32],
) -> DkgConfig {
    let manifest_digest = sha256(&manifest.canonical_bytes());
    let prekey_context = derive_prekey_context(election_id, policy_pre, manifest_digest);
    DkgConfig {
        election_id,
        policy_pre,
        manifest_digest,
        prekey_context,
        session_id: derive_dkg_session_id(prekey_context),
        n: manifest.n,
        t: manifest.t,
        q_cert: manifest.q_cert,
    }
}

fn run_dkg_experiment(
    manifest: &ProtocolManifest,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
    for trial in 0..options.trials {
        let mut rng = ChaCha20Rng::seed_from_u64(options.seed + trial as u64);
        let election_id = sha256(b"VV/EXPERIMENT/ELECTION/20261020/POLICY-V2");
        let authority = RegistrationAuthority::generate(&mut rng);
        let policy_pre = policy_pre_digest(
            election_id,
            10,
            &manifest.winning_rule,
            sha256(&manifest.canonical_bytes()),
            &authority.keypair.public,
        );
        let started = Instant::now();
        let output = run_dkg(
            dkg_config(manifest, election_id, policy_pre),
            FaultPlan {
                bad_private_share: Some((1, 3)),
                bad_feldman_broadcast_dealer: Some(2),
                ..FaultPlan::default()
            },
            &mut rng,
        )?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let report = dkg_report(&output, elapsed_ms, manifest.tau);
        write_json(
            &options.output.join(format!("dkg-trial-{trial}.json")),
            &report,
        )?;
        append_csv(
            &options.output.join("runs.csv"),
            manifest,
            &format!("dkg-{trial}"),
            "dkg",
            0,
            0,
            "PEDERSEN_VSS_GJKR_PHASES",
            report.certificate_valid,
            0,
            0,
            0,
            0.0,
            elapsed_ms,
            0.0,
            0.0,
            options.seed + trial as u64,
        )?;
        println!(
            "DKG trial {trial}: {:.3}ms complaints={} recoveries={} certificate={}",
            elapsed_ms,
            report.complaints,
            report.extraction_recoveries.len(),
            report.certificate_valid
        );
    }
    Ok(())
}

fn dkg_report(output: &DkgOutput, elapsed_ms: f64, tau: usize) -> DkgReport {
    DkgReport {
        classification: "real-group-single-process-protocol-simulation",
        n: output.config.n,
        t: output.config.t,
        tau,
        q_cert: output.config.q_cert,
        elapsed_ms,
        complaints: output.transcript.complaints,
        corrected_complaints: output.transcript.corrected_complaints,
        disqualified_dealers: output.transcript.disqualified_dealers.clone(),
        extraction_recoveries: output.transcript.extraction_recoveries.clone(),
        certificate_signers: output.certificate.signatures.len(),
        certificate_valid: output.verify_certificate(),
        public_key: hex(&point_bytes(&output.public_key)),
        election_id: hex(&output.config.election_id),
        policy_pre: hex(&output.config.policy_pre),
        manifest_digest: hex(&output.config.manifest_digest),
        prekey_context: hex(&output.config.prekey_context),
        session_id: hex(&output.config.session_id),
        key_core_digest: hex(&output.certificate.key_core_digest),
        transcript_digest: hex(&output.transcript.public_digest),
        estimated_protocol_bytes: output.transcript.estimated_bytes,
        execution_model:
            "one OS process with independent trustee state objects; multiprocess G3 remains open",
    }
}

fn run_dlog(manifest: &ProtocolManifest, options: &Options) -> Result<(), Box<dyn Error>> {
    let mut samples = Vec::new();
    for bound in [1_000u64, 10_000, 100_000, 1_000_000] {
        let expected = bound - 1;
        let target = Scalar::from(expected) * generator();
        let started = Instant::now();
        let decoded = bounded_discrete_log(&target, bound)?;
        let decode_ms = started.elapsed().as_secs_f64() * 1000.0;
        samples.push(DlogSample {
            bound,
            expected,
            decoded,
            decode_ms,
            algorithm: "baby-step-giant-step",
        });
        append_csv(
            &options.output.join("runs.csv"),
            manifest,
            &format!("dlog-{bound}"),
            "dlog",
            0,
            bound,
            "BOUNDED_DLOG_BSGS",
            decoded == expected,
            0,
            0,
            0,
            0.0,
            0.0,
            0.0,
            decode_ms,
            options.seed,
        )?;
        println!("DLog bound={bound}: {decode_ms:.3}ms decoded={decoded}");
    }
    write_json(&options.output.join("dlog.json"), &samples)?;
    Ok(())
}

fn run_e2e(manifest: &ProtocolManifest, options: &Options) -> Result<(), Box<dyn Error>> {
    let mut rng = ChaCha20Rng::seed_from_u64(options.seed ^ 0x0045_3245);
    let election_id = sha256(b"VV/EXPERIMENT/ELECTION/20261020/POLICY-V2");
    let authority = RegistrationAuthority::generate(&mut rng);
    let manifest_digest = sha256(&manifest.canonical_bytes());
    let w_max = 10u64;
    let policy_pre = policy_pre_digest(
        election_id,
        w_max,
        &manifest.winning_rule,
        manifest_digest,
        &authority.keypair.public,
    );
    // The policy and all data needed to derive the DKG session are fixed before
    // the first DKG message is generated.
    let config = dkg_config(manifest, election_id, policy_pre);
    let dkg_started = Instant::now();
    let dkg = run_dkg(config, FaultPlan::default(), &mut rng)?;
    let dkg_ms = dkg_started.elapsed().as_secs_f64() * 1000.0;
    let context = derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);
    let weights = [3u64, 2, 1, 4];
    let choices = [1usize, 2, 1, 3];
    let entries: Vec<_> = weights
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            RegistrationEntry::new(
                sha256(format!("fixture-credential-{index}").as_bytes()),
                *weight,
                Scalar::random(&mut rng),
            )
        })
        .collect();
    let ra_started = Instant::now();
    let certificate =
        authority.close(entries, election_id, context, policy_pre, w_max, &mut rng)?;
    verify_close(
        &certificate,
        &authority.keypair.public,
        election_id,
        context,
        policy_pre,
        w_max,
    )?;
    let ra_close_ms = ra_started.elapsed().as_secs_f64() * 1000.0;
    let disclose = disclosure_digest(&certificate, &manifest.winning_rule);

    let vote_started = Instant::now();
    let mut ballots = Vec::new();
    for (weight, choice) in weights.iter().zip(choices) {
        let mut ballot = Vec::new();
        for candidate in 1..=manifest.candidate_count {
            let message = if candidate == choice { *weight } else { 0 };
            ballot.push(encrypt(&dkg.public_key, message, &mut rng).0);
        }
        ballots.push(ballot);
    }
    let final_ciphertext = aggregate_vectors(&ballots)?;
    let vote_encrypt_ms = vote_started.elapsed().as_secs_f64() * 1000.0;
    let mut target_encoding = CanonicalEncoder::new(b"VV/TALLY/TARGET/v1");
    target_encoding.fixed(&election_id);
    target_encoding.fixed(&context);
    target_encoding.fixed(&policy_pre);
    target_encoding.fixed(&disclose);
    target_encoding.fixed(&certificate.close.root_close);
    target_encoding.fixed(&ciphertext_vector_digest(&final_ciphertext));
    let target = sha256(&target_encoding.finish());
    let threshold_context = ThresholdContext {
        election_id,
        context,
        policy_pre,
        disclose,
        target,
    };

    let selected = vec![1u32, 3, 5];
    let cp_started = Instant::now();
    let mut private_shares = Vec::new();
    let mut private_packages = Vec::new();
    let mut commitments: Vec<PedersenCommitment> = Vec::new();
    let mut signatures: Vec<SchnorrSignature> = Vec::new();
    for trustee in &selected {
        let index = *trustee as usize - 1;
        let (shares, proof) = prove_decryption(
            &threshold_context,
            *trustee,
            dkg.secret_shares[index],
            &dkg.verification_shares[index],
            &final_ciphertext,
            &mut rng,
        )?;
        assert!(verify_decryption(
            &threshold_context,
            *trustee,
            &dkg.verification_shares[index],
            &final_ciphertext,
            &shares,
            &proof
        ));
        let package = private_package_bytes(
            &threshold_context,
            *trustee,
            &dkg.verification_shares[index],
            &shares,
            &proof,
        );
        let opening = Scalar::random(&mut rng);
        let commitment = commit_bytes(b"VV/TH/PRIVATE-PACKAGE/v1", &package, opening)?;
        let package_scalar = vv_crypto::hash_to_scalar(b"VV/TH/PRIVATE-PACKAGE/v1", &package)?.0;
        assert!(verify_commitment(&commitment, package_scalar, opening));
        let commitment_message = commitment_message(&threshold_context, *trustee, &commitment);
        let signature = dkg.authentication_keys[index].sign(
            b"VV/TH/COMMIT/SIGNATURE/v1",
            &commitment_message,
            &mut rng,
        )?;
        assert!(verify_signature(
            &dkg.authentication_keys[index].public,
            b"VV/TH/COMMIT/SIGNATURE/v1",
            &commitment_message,
            &signature
        ));
        private_shares.push(shares);
        private_packages.push(package);
        commitments.push(commitment);
        signatures.push(signature);
    }
    let cp_prove_verify_ms = cp_started.elapsed().as_secs_f64() * 1000.0;

    let combine_started = Instant::now();
    let combined = combine_decryption_shares(&selected, &private_shares)?;
    let mut tally = Vec::new();
    for (ciphertext, decryption) in final_ciphertext.iter().zip(combined) {
        tally.push(bounded_discrete_log(
            &(ciphertext.c1 - decryption),
            certificate.close.w_registered,
        )?);
    }
    let declared_winner = winner(&tally, certificate.close.w_registered)?;
    if tally != [4, 2, 4] || declared_winner != 1 {
        return Err("four-voter fixture produced the wrong private tally/winner".into());
    }
    let all_legal_subsets_checked = verify_all_subsets(&dkg, &final_ciphertext, &[4, 2, 4], 3)?;
    let combine_decode_ms = combine_started.elapsed().as_secs_f64() * 1000.0;
    let p1_metrics = run_p1_groth16(
        &tally,
        certificate.close.w_registered,
        options.seed ^ 0x5031,
    )?;
    let private_package_bytes = private_packages.iter().map(Vec::len).sum();
    let public_event = PublicWinnerEvent {
        protocol_version: manifest.protocol_version.clone(),
        classification: "winner-only-public-view-core-no-full-p3",
        election_id: hex(&election_id),
        context: hex(&context),
        policy_pre: hex(&policy_pre),
        dkg_session_id: hex(&dkg.config.session_id),
        key_core_digest: hex(&dkg.certificate.key_core_digest),
        registration_root: hex(&certificate.close.root_close),
        w_registered: certificate.close.w_registered,
        w_max: certificate.close.w_max,
        target: hex(&target),
        selected_trustees: selected,
        public_commitments: commitments
            .iter()
            .map(|commitment| hex(&point_bytes(&commitment.point)))
            .collect(),
        public_signatures: signatures
            .iter()
            .map(|signature| hex(&signature.canonical_bytes()))
            .collect(),
        winner: declared_winner,
        p1_groth16_verified: p1_metrics.verified,
        full_p3_generated: false,
        public_tally_present: false,
    };
    let public_bytes = serde_json::to_vec(&public_event)?.len();
    let report = E2eReport {
        classification: "end-to-end-native-core-with-real-p1-not-official-p3",
        gate_status: "G0-core/P1 pass; G1/G2/G3-multiprocess/G4/G5 blocked",
        public_event,
        dkg_ms,
        ra_close_ms,
        vote_encrypt_ms,
        cp_prove_verify_ms,
        combine_decode_ms,
        p1_metrics,
        all_legal_subsets_checked,
        public_bytes,
        private_package_bytes,
        limitations: vec![
            "DKG executes real Pedersen/Feldman/GJKR phases in one process, not five OS processes",
            "ballot Groth16 and SnarkPack are not integrated",
            "state R1CS is not integrated",
            "P1 is not full P3: CP and package binding are verified natively, not inside Groth16",
            "no target-chain verifier or gas result is claimed",
            "the experiment does not prove Function-Equal privacy or the joint extraction theorem",
        ],
    };
    let output_path = options.output.join("e2e-public.json");
    write_json(&output_path, &report)?;
    append_csv(
        &options.output.join("runs.csv"),
        manifest,
        "e2e-core",
        "e2e",
        4,
        10,
        "CORE_PLUS_P1_NOT_P3",
        report.public_event.p1_groth16_verified,
        report.p1_metrics.proof_bytes,
        report.private_package_bytes,
        report.p1_metrics.constraints,
        report.p1_metrics.setup_ms,
        report.p1_metrics.prove_ms,
        report.p1_metrics.verify_ms,
        report.combine_decode_ms,
        options.seed,
    )?;
    println!(
        "E2E core: winner={} subsets={} public_bytes={} private_bytes={} full_p3=false",
        report.public_event.winner,
        report.all_legal_subsets_checked,
        report.public_bytes,
        report.private_package_bytes
    );
    Ok(())
}

fn cp_point_vector(point: &RistrettoPoint, value: EdwardsAffineValue) -> CpPointVector {
    CpPointVector {
        compressed_hex: hex(&point_bytes(point)),
        edwards_x_decimal: value.x_decimal(),
        edwards_y_decimal: value.y_decimal(),
    }
}

fn run_p3_feasibility(
    manifest: &ProtocolManifest,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
    run_cp_experiment(manifest, options, CpScalarMulVersion::Baseline256, false)
}

fn run_cp_optimization(
    manifest: &ProtocolManifest,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
    let version = match options.cp_implementation.as_str() {
        "baseline_256" => CpScalarMulVersion::Baseline256,
        "canonical_253" => CpScalarMulVersion::Canonical253,
        value => {
            return Err(format!(
                "unknown CP implementation {value}; use baseline_256 or canonical_253"
            )
            .into())
        }
    };
    run_cp_experiment(manifest, options, version, true)
}

fn run_cp_experiment(
    manifest: &ProtocolManifest,
    options: &Options,
    scalar_mul_version: CpScalarMulVersion,
    optimization_run: bool,
) -> Result<(), Box<dyn Error>> {
    let mut rng = ChaCha20Rng::seed_from_u64(options.seed ^ 0x4730_355F_4350);
    let secret = Scalar::from(23u64);
    let verification_key = secret * generator();
    let (ciphertext, _) = encrypt(&verification_key, 3, &mut rng);
    let election_id = sha256(b"VV/G0.5/ELECTION/PREPOLICY-V2");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy_pre = policy_pre_digest(
        election_id,
        10,
        &manifest.winning_rule,
        sha256(&manifest.canonical_bytes()),
        &authority.keypair.public,
    );
    let prekey_context =
        derive_prekey_context(election_id, policy_pre, sha256(&manifest.canonical_bytes()));
    let mut cp_key_core = CanonicalEncoder::new(b"VV/G0.5/KEYCORE/v2");
    cp_key_core.fixed(&election_id);
    cp_key_core.fixed(&prekey_context);
    cp_key_core.fixed(&point_bytes(&verification_key));
    let context_digest = derive_final_context(prekey_context, sha256(&cp_key_core.finish()));
    let context = ThresholdContext {
        election_id,
        context: context_digest,
        policy_pre,
        disclose: sha256(b"VV/G0.5/DISCLOSURE/v1"),
        target: sha256(b"VV/G0.5/TARGET/v1"),
    };
    let ciphertexts = vec![ciphertext];
    let (shares, proof) = prove_decryption(
        &context,
        1,
        secret,
        &verification_key,
        &ciphertexts,
        &mut rng,
    )?;
    let native_cp_verified = verify_decryption(
        &context,
        1,
        &verification_key,
        &ciphertexts,
        &shares,
        &proof,
    );
    if !native_cp_verified {
        return Err("native CP vector did not verify".into());
    }
    let (transcript, challenge, rejection_counter) = decryption_transcript_and_challenge(
        &context,
        1,
        &verification_key,
        &ciphertexts,
        &shares,
        &proof,
    )?;
    if rejection_counter != proof.rejection_counter {
        return Err("native CP rejection counter mismatch".into());
    }

    let generator_value = decompress_ristretto(point_bytes(&generator()))?;
    let verification_key_value = decompress_ristretto(point_bytes(&verification_key))?;
    let commitment_value = decompress_ristretto(point_bytes(&proof.a0))?;
    let primitive_costs = profile_cp_primitives(generator_value, verification_key_value)?;
    let circuit = CpEquationCircuit {
        generator: generator_value,
        commitment: commitment_value,
        verification_key: verification_key_value,
        challenge_le: scalar_bytes(&challenge),
        response_le: scalar_bytes(&proof.response),
        scalar_mul_version,
    };

    let rss_before = peak_rss_mb();
    let preflight = preflight_cp_equation(circuit.clone())?;
    if !preflight.valid_relation_satisfied {
        return Err("real CP vector did not satisfy the R1CS equation".into());
    }
    let groth16 = if preflight.constraints <= options.max_cp_constraints {
        Some(run_cp_equation_groth16(
            circuit,
            options.seed ^ 0x4730_355F_5052,
        )?)
    } else {
        None
    };
    let peak_rss_mb = peak_rss_mb().max(rss_before);
    let (status, next_decision) = if groth16.is_some() {
        (
            "PARTIAL_PASS_SINGLE_EQUATION_GROTH16_FULL_P3_BLOCKED",
            "Optimize the non-native circuit, then add compressed Ristretto decoding and exact SHA-512/rejection sampling before P2/P3-a",
        )
    } else {
        (
            "BLOCKED_RESOURCE_GUARD_NO_GROTH16_PROOF",
            "Do not begin P2/P3-a; first reduce non-native scalar-multiplication cost, then add compressed Ristretto decoding and exact SHA-512/rejection sampling",
        )
    };
    let report = CpFeasibilityReport {
        gate: "G0.5",
        status,
        build_profile: build_profile(),
        rustc_version: rustc_version(),
        peak_rss_mb,
        native_cp_verified,
        max_cp_constraints: options.max_cp_constraints,
        preflight,
        primitive_costs,
        groth16,
        next_decision,
    };
    if optimization_run {
        write_json(
            &options.output.join(format!(
                "cp-optimization-{}.json",
                scalar_mul_version.label()
            )),
            &report,
        )?;
        write_cp_optimization_csv(
            &options.output.join("cp_optimization.csv"),
            options.seed,
            &report,
        )?;
    } else {
        write_json(&options.output.join("p3-feasibility.json"), &report)?;
        write_cp_bench_csv(
            &options.output.join("cp_circuit_bench.csv"),
            options.seed,
            &report,
        )?;
    }

    let vector = CpCompatibilityVector {
        vector_id: "VV-G0.5-CP-SINGLE-COORDINATE-PREPOLICY-v2",
        classification: "real-native-generalized-cp-vector-used-by-partial-r1cs-equation",
        election_id: hex(&context.election_id),
        context: hex(&context.context),
        policy_pre: hex(&context.policy_pre),
        disclosure_digest: hex(&context.disclose),
        target: hex(&context.target),
        trustee: 1,
        generator: cp_point_vector(&generator(), generator_value),
        verification_key: cp_point_vector(&verification_key, verification_key_value),
        ciphertext_c0: ciphertexts
            .iter()
            .map(|value| hex(&point_bytes(&value.c0)))
            .collect(),
        decryption_shares: shares
            .iter()
            .map(|value| hex(&point_bytes(value)))
            .collect(),
        commitment_a0: cp_point_vector(&proof.a0, commitment_value),
        coordinate_commitments: proof
            .coordinates
            .iter()
            .map(|value| hex(&point_bytes(value)))
            .collect(),
        response_scalar_le_hex: hex(&scalar_bytes(&proof.response)),
        challenge_scalar_le_hex: hex(&scalar_bytes(&challenge)),
        rejection_counter,
        transcript_hex: hex(&transcript),
        transcript_sha256: hex(&sha256(&transcript)),
        native_cp_verified,
        circuit_equation: "z*G = A0 + d*VK",
        circuit_limitations: vec![
            "challenge d is a canonical public scalar input, not yet recomputed from SHA-512 in R1CS",
            "public Edwards coordinates are not yet bound to compressed Ristretto bytes in R1CS",
            "the coordinate equation z*CT0=A1+d*D and generalized vector loop are not yet compiled",
            "this is G0.5 feasibility, not P2, P3-a, or full P3",
        ],
    };
    write_json(Path::new("fixtures/cp_vectors.json"), &vector)?;
    if let Some(metrics) = &report.groth16 {
        println!(
            "G0.5 {} partial Groth16: constraints={} public_inputs={} setup={:.3}ms prove={:.3}ms verify={:.3}ms peak_rss={:.1}MiB full_p3=false",
            metrics.implementation,
            metrics.constraints,
            metrics.public_inputs,
            metrics.setup_ms,
            metrics.prove_ms,
            metrics.verify_ms,
            report.peak_rss_mb,
        );
    } else {
        println!(
            "G0.5 {} preflight: constraints={} public_inputs={} witness={:.3}ms guard={} Groth16_setup=false Groth16_prove=false Groth16_verify=false peak_rss={:.1}MiB full_p3=false",
            report.preflight.implementation,
            report.preflight.constraints,
            report.preflight.public_inputs,
            report.preflight.synthesis_witness_ms,
            report.max_cp_constraints,
            report.peak_rss_mb,
        );
    }
    Ok(())
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev-unoptimized"
    } else {
        "release-opt-level-1"
    }
}

fn rustc_version() -> String {
    let executable = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    std::process::Command::new(executable)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".to_owned())
}

#[cfg(target_os = "macos")]
fn peak_rss_mb() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: getrusage initializes the supplied rusage structure on success.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status == 0 {
        // SAFETY: status == 0 means the operating system initialized `usage`.
        unsafe { usage.assume_init().ru_maxrss as f64 / (1024.0 * 1024.0) }
    } else {
        0.0
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn peak_rss_mb() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: getrusage initializes the supplied rusage structure on success.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status == 0 {
        // Linux and most other Unix targets report ru_maxrss in KiB.
        unsafe { usage.assume_init().ru_maxrss as f64 / 1024.0 }
    } else {
        0.0
    }
}

#[cfg(not(unix))]
fn peak_rss_mb() -> f64 {
    0.0
}

fn write_cp_bench_csv(
    path: &Path,
    seed: u64,
    report: &CpFeasibilityReport,
) -> Result<(), Box<dyn Error>> {
    const HEADER: &str = "timestamp,gate,status,build_profile,rustc_version,os,arch,seed,equation,max_cp_constraints,constraints,public_inputs,input_and_scalar_range_constraints,curve_membership_constraints,nonnative_double_scalar_mul_constraints,ristretto_quotient_equality_constraints,valid_relation_satisfied,groth16_setup_run,groth16_prove_run,groth16_verify_run,setup_ms,prove_ms,verify_ms,proof_bytes,proving_key_bytes,verifying_key_bytes,peak_rss_mb,native_cp_verified,valid_proof_verified,changed_public_challenge_rejected,altered_response_relation_unsatisfied,compressed_encoding_bound_in_circuit,sha512_recomputed_in_circuit,full_p3\n";
    let new_file = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if new_file {
        file.write_all(HEADER.as_bytes())?;
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let preflight = &report.preflight;
    let breakdown = &preflight.breakdown;
    let optional = |value: Option<String>| value.unwrap_or_default();
    let columns = vec![
        timestamp.to_string(),
        report.gate.to_owned(),
        report.status.to_owned(),
        report.build_profile.to_owned(),
        report.rustc_version.replace(',', ";"),
        env::consts::OS.to_owned(),
        env::consts::ARCH.to_owned(),
        seed.to_string(),
        preflight.equation.clone(),
        report.max_cp_constraints.to_string(),
        preflight.constraints.to_string(),
        preflight.public_inputs.to_string(),
        breakdown.input_and_scalar_range.to_string(),
        breakdown.curve_membership.to_string(),
        breakdown.nonnative_double_scalar_mul.to_string(),
        breakdown.ristretto_quotient_equality.to_string(),
        preflight.valid_relation_satisfied.to_string(),
        report.groth16.is_some().to_string(),
        report.groth16.is_some().to_string(),
        report.groth16.is_some().to_string(),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| format!("{:.6}", value.setup_ms)),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| format!("{:.6}", value.prove_ms)),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| format!("{:.6}", value.verify_ms)),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| value.proof_bytes.to_string()),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| value.proving_key_bytes.to_string()),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| value.verifying_key_bytes.to_string()),
        ),
        format!("{:.3}", report.peak_rss_mb),
        report.native_cp_verified.to_string(),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| value.valid_proof_verified.to_string()),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| value.changed_public_challenge_rejected.to_string()),
        ),
        optional(
            report
                .groth16
                .as_ref()
                .map(|value| value.altered_response_relation_unsatisfied.to_string()),
        ),
        report
            .groth16
            .as_ref()
            .is_some_and(|value| value.compressed_encoding_bound_in_circuit)
            .to_string(),
        report
            .groth16
            .as_ref()
            .is_some_and(|value| value.sha512_recomputed_in_circuit)
            .to_string(),
        report
            .groth16
            .as_ref()
            .is_some_and(|value| value.full_p3)
            .to_string(),
    ];
    writeln!(file, "{}", columns.join(","))?;
    Ok(())
}

fn write_cp_optimization_csv(
    path: &Path,
    seed: u64,
    report: &CpFeasibilityReport,
) -> Result<(), Box<dyn Error>> {
    const HEADER: &str = "timestamp,implementation,status,build_profile,rustc_version,os,arch,seed,constraints,public_inputs,input_and_scalar_range_constraints,curve_membership_constraints,nonnative_double_scalar_mul_constraints,ristretto_quotient_equality_constraints,nonnative_field_mul_profile_constraints,extended_point_add_profile_constraints,ristretto_equality_profile_constraints,synthesis_witness_ms,peak_rss_mb,max_cp_constraints,valid_relation_satisfied,native_cp_verified,groth16_setup_run,groth16_prove_run,groth16_verify_run,setup_ms,prove_ms,verify_ms,proof_bytes,full_p3\n";
    let new_file = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if new_file {
        file.write_all(HEADER.as_bytes())?;
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let preflight = &report.preflight;
    let breakdown = &preflight.breakdown;
    let groth16_value = |extract: fn(&CpFeasibilityMetrics) -> String| {
        report.groth16.as_ref().map(extract).unwrap_or_default()
    };
    let columns = vec![
        timestamp.to_string(),
        preflight.implementation.clone(),
        report.status.to_owned(),
        report.build_profile.to_owned(),
        report.rustc_version.replace(',', ";"),
        env::consts::OS.to_owned(),
        env::consts::ARCH.to_owned(),
        seed.to_string(),
        preflight.constraints.to_string(),
        preflight.public_inputs.to_string(),
        breakdown.input_and_scalar_range.to_string(),
        breakdown.curve_membership.to_string(),
        breakdown.nonnative_double_scalar_mul.to_string(),
        breakdown.ristretto_quotient_equality.to_string(),
        report
            .primitive_costs
            .nonnative_field_mul_with_output_check
            .to_string(),
        report
            .primitive_costs
            .extended_point_add_with_output_check
            .to_string(),
        report
            .primitive_costs
            .ristretto_quotient_equality
            .to_string(),
        format!("{:.6}", preflight.synthesis_witness_ms),
        format!("{:.3}", report.peak_rss_mb),
        report.max_cp_constraints.to_string(),
        preflight.valid_relation_satisfied.to_string(),
        report.native_cp_verified.to_string(),
        report.groth16.is_some().to_string(),
        report.groth16.is_some().to_string(),
        report.groth16.is_some().to_string(),
        groth16_value(|value| format!("{:.6}", value.setup_ms)),
        groth16_value(|value| format!("{:.6}", value.prove_ms)),
        groth16_value(|value| format!("{:.6}", value.verify_ms)),
        groth16_value(|value| value.proof_bytes.to_string()),
        report
            .groth16
            .as_ref()
            .is_some_and(|value| value.full_p3)
            .to_string(),
    ];
    writeln!(file, "{}", columns.join(","))?;
    Ok(())
}

fn private_package_bytes(
    context: &ThresholdContext,
    trustee: u32,
    verification_share: &RistrettoPoint,
    shares: &[RistrettoPoint],
    proof: &DecryptionProof,
) -> Vec<u8> {
    let mut encoded = CanonicalEncoder::new(b"VV/TH/PRIVATE-PACKAGE/v1");
    encoded.fixed(&context.election_id);
    encoded.fixed(&context.context);
    encoded.fixed(&context.policy_pre);
    encoded.fixed(&context.disclose);
    encoded.fixed(&context.target);
    encoded.u32(trustee);
    encoded.fixed(&point_bytes(verification_share));
    encoded.u64(shares.len() as u64);
    for share in shares {
        encoded.fixed(&point_bytes(share));
    }
    encoded.fixed(&point_bytes(&proof.a0));
    for coordinate in &proof.coordinates {
        encoded.fixed(&point_bytes(coordinate));
    }
    encoded.fixed(&scalar_bytes(&proof.response));
    encoded.u16(proof.rejection_counter);
    encoded.finish()
}

fn commitment_message(
    context: &ThresholdContext,
    trustee: u32,
    commitment: &PedersenCommitment,
) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/TH/COMMIT/v1");
    encoded.fixed(&context.election_id);
    encoded.fixed(&context.context);
    encoded.fixed(&context.target);
    encoded.u32(trustee);
    encoded.fixed(&context.policy_pre);
    encoded.fixed(&context.disclose);
    encoded.fixed(&point_bytes(&commitment.point));
    sha256(&encoded.finish())
}

fn verify_all_subsets(
    dkg: &DkgOutput,
    ciphertexts: &[Ciphertext],
    expected: &[u64],
    tau: usize,
) -> Result<usize, Box<dyn Error>> {
    let ids: Vec<u32> = (1..=dkg.config.n as u32).collect();
    let subsets = combinations(&ids, tau);
    for subset in &subsets {
        let shares: Vec<Vec<RistrettoPoint>> = subset
            .iter()
            .map(|id| {
                ciphertexts
                    .iter()
                    .map(|ciphertext| dkg.secret_shares[*id as usize - 1] * ciphertext.c0)
                    .collect()
            })
            .collect();
        let combined = combine_decryption_shares(subset, &shares)?;
        let decoded: Vec<_> = ciphertexts
            .iter()
            .zip(combined)
            .map(|(ciphertext, decryption)| bounded_discrete_log(&(ciphertext.c1 - decryption), 10))
            .collect::<Result<_, _>>()?;
        if decoded != expected {
            return Err(format!("subset {subset:?} decoded incorrectly").into());
        }
    }
    Ok(subsets.len())
}

fn combinations(values: &[u32], choose: usize) -> Vec<Vec<u32>> {
    fn visit(
        values: &[u32],
        choose: usize,
        start: usize,
        current: &mut Vec<u32>,
        output: &mut Vec<Vec<u32>>,
    ) {
        if current.len() == choose {
            output.push(current.clone());
            return;
        }
        for index in start..values.len() {
            current.push(values[index]);
            visit(values, choose, index + 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    visit(values, choose, 0, &mut Vec::new(), &mut output);
    output
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_csv(
    path: &Path,
    manifest: &ProtocolManifest,
    run_id: &str,
    experiment: &str,
    n_voters: usize,
    w_registered: u64,
    proof_type: &str,
    success: bool,
    proof_bytes: usize,
    private_bytes: usize,
    constraints: usize,
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    dlog_ms: f64,
    seed: u64,
) -> Result<(), Box<dyn Error>> {
    let new_file = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if new_file {
        file.write_all(CSV_HEADER.as_bytes())?;
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let columns = vec![
        format!("{run_id}-{timestamp}"),
        experiment.to_owned(),
        "native-time".to_owned(),
        "NO_GIT_REPOSITORY".to_owned(),
        manifest.digest_hex(),
        "Ristretto255+BLS12-381".to_owned(),
        env::consts::OS.to_owned(),
        env::consts::ARCH.to_owned(),
        String::new(),
        std::thread::available_parallelism().map_or(String::new(), |n| n.get().to_string()),
        String::new(),
        String::new(),
        build_profile().to_owned(),
        n_voters.to_string(),
        manifest.candidate_count.to_string(),
        manifest.n.to_string(),
        manifest.t.to_string(),
        manifest.tau.to_string(),
        manifest.q_cert.to_string(),
        String::new(),
        "1".to_owned(),
        String::new(),
        w_registered.to_string(),
        "10".to_owned(),
        seed.to_string(),
        proof_type.to_owned(),
        experiment.to_owned(),
        success.to_string(),
        String::new(),
        proof_bytes.to_string(),
        String::new(),
        private_bytes.to_string(),
        constraints.to_string(),
        format!("{setup_ms:.6}"),
        String::new(),
        format!("{prove_ms:.6}"),
        format!("{verify_ms:.6}"),
        String::new(),
        String::new(),
        format!("{dlog_ms:.6}"),
        String::new(),
        "none-native-only".to_owned(),
        String::new(),
        String::new(),
        String::new(),
    ];
    writeln!(file, "{}", columns.join(","))?;
    Ok(())
}
