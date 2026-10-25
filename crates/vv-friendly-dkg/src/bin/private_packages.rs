use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{derive_cp_target, prove_decryption_share, GeneralizedCpContext};
use vv_friendly_crypto::{encode_point, encrypt_weighted_vote, fold_ballots, SUITE_ID};
use vv_friendly_dkg::package::{
    create_private_package, select_verified_packages, verify_private_package, PublicPackageRecord,
};
use vv_friendly_dkg::{
    run_dkg, threshold_decrypt_verified_shares, AuthenticationKeypair, DkgConfig, FaultPlan,
};
use vv_types::{derive_final_context, hex, sha256, winner};

#[derive(Serialize)]
struct PublicPackageResult {
    trustee_id: u32,
    commitment_hex: String,
    signature_hex: String,
    signature_bytes: usize,
}

impl From<&PublicPackageRecord> for PublicPackageResult {
    fn from(record: &PublicPackageRecord) -> Self {
        let signature = record.signature.canonical_bytes();
        Self {
            trustee_id: record.trustee_id,
            commitment_hex: hex(&encode_point(&record.commitment.point)),
            signature_hex: hex(&signature),
            signature_bytes: signature.len(),
        }
    }
}

#[derive(Serialize)]
struct WinnerOnlyEvent {
    event_version: &'static str,
    election_id: String,
    policy_pre: String,
    final_context: String,
    dkg_session_id: String,
    key_core_digest: String,
    cp_target: String,
    selected_trustees: Vec<u32>,
    selected_package_records: Vec<PublicPackageResult>,
    winner: usize,
    private_tally_present: bool,
    decryption_shares_present: bool,
    cp_proofs_present: bool,
    commitment_openings_present: bool,
}

#[derive(Serialize)]
struct ExperimentResult {
    classification: &'static str,
    suite_id: &'static str,
    seed: u64,
    n: usize,
    t: usize,
    tau: usize,
    candidate_count: usize,
    registered_weight: u64,
    public_key_hex: String,
    dkg_ms: f64,
    vote_encrypt_and_fold_ms: f64,
    cp_prove_and_package_all_five_ms: f64,
    verify_all_five_and_select_ms: f64,
    verified_threshold_decrypt_ms: f64,
    packages_created: usize,
    packages_verified: usize,
    packages_selected: usize,
    private_package_payload_bytes_total: usize,
    all_packages_verified_before_selection: bool,
    only_verified_share_type_combined: bool,
    winner_only_public_event: WinnerOnlyEvent,
    registration_close_implemented: bool,
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
    let mut output = PathBuf::from("experiments/g0_8_packages_raw/e2e-public.json");
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
    let election_id = sha256(b"VV/JUBJUB/G0.8/PRIVATE-PACKAGES/FOUR-VOTER");
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
    let registered_weight = 10u64;
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

    let final_context =
        derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);
    let target = derive_cp_target(
        dkg.config.election_id,
        dkg.config.policy_pre,
        final_context,
        &aggregate,
    )?;

    let package_started = Instant::now();
    let mut private_packages = Vec::with_capacity(dkg.config.n);
    let mut public_records = Vec::with_capacity(dkg.config.n);
    for trustee_id in 1..=dkg.config.n as u32 {
        let context = GeneralizedCpContext {
            election_id: dkg.config.election_id,
            policy_pre: dkg.config.policy_pre,
            final_context,
            registration_close_digest: [0; 32],
            disclosure_digest: [0; 32],
            target,
            trustee_id,
        };
        let cp_package = prove_decryption_share(
            &context,
            dkg.secret_share(trustee_id)?,
            dkg.verification_share(trustee_id)?,
            &aggregate,
            &mut rng,
        )?;
        let (private, public) = create_private_package(
            context,
            *dkg.verification_share(trustee_id)?,
            cp_package,
            &authentication_keys[trustee_id as usize - 1],
            &mut rng,
        )?;
        private_packages.push(private);
        public_records.push(public);
    }
    let cp_prove_and_package_all_five_ms = package_started.elapsed().as_secs_f64() * 1000.0;

    let verify_started = Instant::now();
    let verified_packages = private_packages
        .iter()
        .zip(&public_records)
        .map(|(private, public)| verify_private_package(&dkg, &aggregate, private, public))
        .collect::<Result<Vec<_>, _>>()?;
    let selected = select_verified_packages(&verified_packages, dkg.config.tau)?;
    let verify_all_five_and_select_ms = verify_started.elapsed().as_secs_f64() * 1000.0;

    let selected_trustees = selected
        .iter()
        .map(|package| package.trustee_id())
        .collect::<Vec<_>>();
    let selected_shares = selected
        .iter()
        .map(|package| package.verified_share().clone())
        .collect::<Vec<_>>();
    let threshold_started = Instant::now();
    let private_tally = threshold_decrypt_verified_shares(
        &dkg,
        &selected_trustees,
        &aggregate,
        &selected_shares,
        registered_weight,
    )?;
    let public_winner = winner(&private_tally, registered_weight)?;
    let verified_threshold_decrypt_ms = threshold_started.elapsed().as_secs_f64() * 1000.0;

    let accepted = verified_packages.len() == 5
        && selected_trustees == [1, 2, 3]
        && private_tally == [4, 2, 4]
        && public_winner == 1;
    if !accepted {
        return Err("G0.8 private-package acceptance failed".into());
    }

    let selected_package_records = selected
        .iter()
        .map(|package| PublicPackageResult::from(package.public_record()))
        .collect();
    let private_package_payload_bytes_total = private_packages
        .iter()
        .map(|package| package.encoded_len())
        .sum();
    let report = RunEnvelope {
        gate: "G0.8-E1",
        status: "PASS_NATIVE_PRIVATE_PACKAGES_NO_REGISTRATION_CLOSE",
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
            classification:
                "single-process-native-jubjub-private-packages-winner-only-no-registration-close",
            suite_id: SUITE_ID,
            seed,
            n: dkg.config.n,
            t: dkg.config.t,
            tau: dkg.config.tau,
            candidate_count: aggregate.len(),
            registered_weight,
            public_key_hex: hex(&encode_point(&dkg.public_key)),
            dkg_ms,
            vote_encrypt_and_fold_ms,
            cp_prove_and_package_all_five_ms,
            verify_all_five_and_select_ms,
            verified_threshold_decrypt_ms,
            packages_created: private_packages.len(),
            packages_verified: verified_packages.len(),
            packages_selected: selected.len(),
            private_package_payload_bytes_total,
            all_packages_verified_before_selection: true,
            only_verified_share_type_combined: true,
            winner_only_public_event: WinnerOnlyEvent {
                event_version: "VV/JUBJUB/WINNER-ONLY-EVENT/v3",
                election_id: hex(&dkg.config.election_id),
                policy_pre: hex(&dkg.config.policy_pre),
                final_context: hex(&final_context),
                dkg_session_id: hex(&dkg.config.session_id),
                key_core_digest: hex(&dkg.certificate.key_core_digest),
                cp_target: hex(&target),
                selected_trustees,
                selected_package_records,
                winner: public_winner,
                private_tally_present: false,
                decryption_shares_present: false,
                cp_proofs_present: false,
                commitment_openings_present: false,
            },
            registration_close_implemented: false,
            full_g08: false,
            full_p3: false,
            limitations: vec![
                "Jubjub registration weight commitments and authenticated close are not implemented",
                "native CP verification only; Fiat-Shamir is not recomputed in R1CS",
                "single process; not five independently deployed trustee processes",
                "this result is not full P3",
            ],
        },
    };

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G0.8-E1 private packages: created=5 verified=5 selected={:?} winner={} public-tally=false public-shares=false registration-close=false full-g08=false full-p3=false",
        report
            .experiment
            .winner_only_public_event
            .selected_trustees,
        report.experiment.winner_only_public_event.winner,
    );
    println!("wrote {}", output.display());
    Ok(())
}
