use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::{encode_point, encrypt_weighted_vote, fold_ballots, SUITE_ID};
use vv_friendly_dkg::{
    combinations, interpolate_verification_key, run_dkg, threshold_decrypt_tally,
    AuthenticationKeypair, DkgConfig, FaultPlan,
};
use vv_types::{hex, sha256, winner};

#[derive(Serialize)]
struct SubsetResult {
    trustees: Vec<u32>,
    interpolated_public_key_matches: bool,
    private_tally: Vec<u64>,
    winner: usize,
}

#[derive(Serialize)]
struct ExperimentResult {
    classification: &'static str,
    suite_id: &'static str,
    seed: u64,
    n: usize,
    t: usize,
    tau: usize,
    q_cert: usize,
    qualified_count: usize,
    qualified_trustees: Vec<u32>,
    complaints: usize,
    corrected_complaints: usize,
    extraction_recoveries: Vec<u32>,
    certificate_signers: Vec<u32>,
    estimated_dkg_bytes: usize,
    dkg_ms: f64,
    vote_encrypt_and_fold_ms: f64,
    ten_subset_threshold_decrypt_ms: f64,
    election_id: String,
    policy_pre: String,
    prekey_context: String,
    dkg_session_id: String,
    key_core_digest: String,
    public_key_hex: String,
    verification_shares_hex: Vec<String>,
    registered_weight: u64,
    subset_results: Vec<SubsetResult>,
    all_ten_subsets_passed: bool,
    generalized_cp_implemented: bool,
    full_g08: bool,
    full_p3: bool,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct RunEnvelope {
    gate: &'static str,
    status: &'static str,
    git_commit: String,
    git_dirty: String,
    manifest_file_sha256: String,
    build_profile: &'static str,
    release_strip: String,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    experiment: ExperimentResult,
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
    let mut output = PathBuf::from("experiments/g0_8_dkg_raw/g08_dkg_threshold.json");
    let mut manifest = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut seed = 20261020u64;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--output" => output = PathBuf::from(value),
            "--manifest" => manifest = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    // Capture provenance before creating experiment output files.
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
    let manifest_bytes = fs::read(&manifest)?;
    let manifest_digest = sha256(&manifest_bytes);

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let authentication_keys: Vec<_> = (0..5)
        .map(|_| AuthenticationKeypair::generate(&mut rng))
        .collect();
    let election_id = sha256(b"VV/JUBJUB/G0.8/DKG-THRESHOLD/FOUR-VOTER");
    let config = DkgConfig::new(
        election_id,
        manifest_digest,
        authentication_keys.iter().map(|key| key.public).collect(),
        2,
        3,
    )?;

    let dkg_started = Instant::now();
    let dkg = run_dkg(config, &authentication_keys, FaultPlan::default(), &mut rng)?;
    let dkg_ms = dkg_started.elapsed().as_secs_f64() * 1000.0;

    let weights = [3u64, 2, 1, 4];
    let choices = [1usize, 2, 1, 3];
    let registered_weight = weights.iter().sum();
    let encryption_started = Instant::now();
    let ballots = weights
        .iter()
        .zip(choices)
        .map(|(weight, choice)| {
            encrypt_weighted_vote(
                &dkg.public_key,
                *weight,
                choice,
                3,
                registered_weight,
                &mut rng,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let aggregate = fold_ballots(&ballots)?;
    let vote_encrypt_and_fold_ms = encryption_started.elapsed().as_secs_f64() * 1000.0;

    let threshold_started = Instant::now();
    let mut subset_results = Vec::new();
    for subset in combinations(&[1, 2, 3, 4, 5], 3) {
        let interpolated_public_key_matches =
            interpolate_verification_key(&dkg, &subset)? == dkg.public_key;
        let tally = threshold_decrypt_tally(&dkg, &subset, &aggregate, registered_weight)?;
        let selected_winner = winner(&tally, registered_weight)?;
        subset_results.push(SubsetResult {
            trustees: subset,
            interpolated_public_key_matches,
            private_tally: tally,
            winner: selected_winner,
        });
    }
    let ten_subset_threshold_decrypt_ms = threshold_started.elapsed().as_secs_f64() * 1000.0;
    let all_ten_subsets_passed = subset_results.len() == 10
        && subset_results.iter().all(|result| {
            result.interpolated_public_key_matches
                && result.private_tally == [4, 2, 4]
                && result.winner == 1
        });
    if !all_ten_subsets_passed || !dkg.verify_certificate() {
        return Err("G0.8 DKG/threshold acceptance failed".into());
    }

    let report = RunEnvelope {
        gate: "G0.8-3",
        status: "PASS_NATIVE_DKG_THRESHOLD_NO_CP",
        git_commit,
        git_dirty,
        manifest_file_sha256: hex(&manifest_digest),
        build_profile: if cfg!(debug_assertions) {
            "dev-unoptimized"
        } else {
            "release-opt-level-1"
        },
        release_strip: env::var("CARGO_PROFILE_RELEASE_STRIP")
            .unwrap_or_else(|_| "default".to_owned()),
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
        experiment: ExperimentResult {
            classification: "single-process-real-jubjub-gjkr-and-threshold-decryption-no-cp",
            suite_id: SUITE_ID,
            seed,
            n: dkg.config.n,
            t: dkg.config.t,
            tau: dkg.config.tau,
            q_cert: dkg.config.q_cert,
            qualified_count: dkg.qual.len(),
            qualified_trustees: dkg.qual.clone(),
            complaints: dkg.transcript.complaints,
            corrected_complaints: dkg.transcript.corrected_complaints,
            extraction_recoveries: dkg.transcript.extraction_recoveries.clone(),
            certificate_signers: dkg
                .certificate
                .signatures
                .iter()
                .map(|(id, _)| *id)
                .collect(),
            estimated_dkg_bytes: dkg.transcript.estimated_bytes,
            dkg_ms,
            vote_encrypt_and_fold_ms,
            ten_subset_threshold_decrypt_ms,
            election_id: hex(&dkg.config.election_id),
            policy_pre: hex(&dkg.config.policy_pre),
            prekey_context: hex(&dkg.config.prekey_context),
            dkg_session_id: hex(&dkg.config.session_id),
            key_core_digest: hex(&dkg.certificate.key_core_digest),
            public_key_hex: hex(&encode_point(&dkg.public_key)),
            verification_shares_hex: dkg
                .verification_shares
                .iter()
                .map(|point| hex(&encode_point(point)))
                .collect(),
            registered_weight,
            subset_results,
            all_ten_subsets_passed,
            generalized_cp_implemented: false,
            full_g08: false,
            full_p3: false,
            limitations: vec![
                "single process; not five independently deployed trustee processes",
                "decryption shares do not yet carry generalized Chaum-Pedersen proofs",
                "no private package commitment/signature workflow and no full P3",
            ],
        },
    };

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.8-3 Jubjub DKG/threshold: n=5 tau=3 subsets=10 tally=(4,2,4) winner=1 dkg={dkg_ms:.3}ms threshold-all={ten_subset_threshold_decrypt_ms:.3}ms full_g08=false full_p3=false"
    );
    println!("wrote {}", output.display());
    Ok(())
}
