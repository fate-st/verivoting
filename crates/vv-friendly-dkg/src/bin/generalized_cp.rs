use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{
    derive_cp_target, prove_decryption_share, transcript_and_challenge, verify_decryption_share,
    DecryptionSharePackage, GeneralizedCpContext,
};
use vv_friendly_crypto::{
    encode_point, encode_scalar, encrypt_weighted_vote, fold_ballots, CP_H2F_SUITE_ID, SUITE_ID,
};
use vv_friendly_dkg::{
    combinations, run_dkg, threshold_decrypt_verified_shares, AuthenticationKeypair, DkgConfig,
    FaultPlan,
};
use vv_types::{derive_final_context, hex, sha256, winner};

#[derive(Serialize)]
struct CpTrusteeResult {
    trustee_id: u32,
    verification_share_hex: String,
    transcript_sha256: String,
    challenge_scalar_hex: String,
    coordinate_count: usize,
    proof_bytes: usize,
    share_vector_bytes: usize,
    verified: bool,
}

#[derive(Serialize)]
struct SubsetResult {
    trustees: Vec<u32>,
    private_tally: Vec<u64>,
    winner: usize,
}

#[derive(Serialize)]
struct ExperimentResult {
    classification: &'static str,
    suite_id: &'static str,
    cp_challenge_suite_id: &'static str,
    seed: u64,
    n: usize,
    t: usize,
    tau: usize,
    candidate_count: usize,
    election_id: String,
    policy_pre: String,
    final_context: String,
    target: String,
    public_key_hex: String,
    dkg_ms: f64,
    vote_encrypt_and_fold_ms: f64,
    cp_prove_all_five_ms: f64,
    cp_verify_all_five_ms: f64,
    ten_subset_verified_threshold_decrypt_ms: f64,
    trustee_proofs: Vec<CpTrusteeResult>,
    subset_results: Vec<SubsetResult>,
    all_five_cp_verified: bool,
    all_ten_subsets_passed: bool,
    challenge_recomputed_by_native_verifier: bool,
    only_verified_share_type_combined: bool,
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

fn proof_bytes(package: &DecryptionSharePackage) -> usize {
    // a0, Aj vector and response scalar. CP-H2F-v3 has no rejection counter.
    32 + package.proof.coordinates.len() * 32 + 32
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut output = PathBuf::from("experiments/g0_9_h2d_raw/h2d_generalized_cp.json");
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
    let election_id = sha256(b"VV/JUBJUB/G0.9/H2D/GENERALIZED-CP/FOUR-VOTER");
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
    let contexts = (1..=5u32)
        .map(|trustee_id| GeneralizedCpContext {
            election_id: dkg.config.election_id,
            policy_pre: dkg.config.policy_pre,
            final_context,
            registration_close_digest: [0; 32],
            disclosure_digest: [0; 32],
            target,
            trustee_id,
        })
        .collect::<Vec<_>>();

    let prove_started = Instant::now();
    let packages = contexts
        .iter()
        .map(|context| {
            prove_decryption_share(
                context,
                dkg.secret_share(context.trustee_id)?,
                dkg.verification_share(context.trustee_id)?,
                &aggregate,
                &mut rng,
            )
            .map_err(Into::into)
        })
        .collect::<Result<Vec<DecryptionSharePackage>, Box<dyn Error>>>()?;
    let cp_prove_all_five_ms = prove_started.elapsed().as_secs_f64() * 1000.0;

    let verify_started = Instant::now();
    let verified = contexts
        .iter()
        .zip(&packages)
        .map(|(context, package)| {
            Ok(verify_decryption_share(
                context,
                dkg.verification_share(context.trustee_id)?,
                &aggregate,
                package,
            )?)
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let cp_verify_all_five_ms = verify_started.elapsed().as_secs_f64() * 1000.0;

    let trustee_proofs = contexts
        .iter()
        .zip(&packages)
        .map(|(context, package)| {
            let (transcript, challenge) = transcript_and_challenge(
                context,
                dkg.verification_share(context.trustee_id)?,
                &aggregate,
                package,
            )?;
            Ok(CpTrusteeResult {
                trustee_id: context.trustee_id,
                verification_share_hex: hex(&encode_point(
                    dkg.verification_share(context.trustee_id)?,
                )),
                transcript_sha256: hex(&sha256(&transcript)),
                challenge_scalar_hex: hex(&encode_scalar(&challenge)),
                coordinate_count: aggregate.len(),
                proof_bytes: proof_bytes(package),
                share_vector_bytes: package.shares.len() * 32,
                verified: true,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    let threshold_started = Instant::now();
    let mut subset_results = Vec::new();
    for subset in combinations(&[1, 2, 3, 4, 5], 3) {
        let selected = subset
            .iter()
            .map(|id| verified[*id as usize - 1].clone())
            .collect::<Vec<_>>();
        let tally = threshold_decrypt_verified_shares(
            &dkg,
            &subset,
            &aggregate,
            &selected,
            registered_weight,
        )?;
        let selected_winner = winner(&tally, registered_weight)?;
        subset_results.push(SubsetResult {
            trustees: subset,
            private_tally: tally,
            winner: selected_winner,
        });
    }
    let ten_subset_verified_threshold_decrypt_ms =
        threshold_started.elapsed().as_secs_f64() * 1000.0;
    let all_five_cp_verified = verified.len() == 5;
    let all_ten_subsets_passed = subset_results.len() == 10
        && subset_results
            .iter()
            .all(|result| result.private_tally == [4, 2, 4] && result.winner == 1);
    if !all_five_cp_verified || !all_ten_subsets_passed {
        return Err("G0.9-H2-D generalized CP acceptance failed".into());
    }

    let report = RunEnvelope {
        gate: "G0.9-H2-D",
        status: "PASS_H2D_CP_H2F_V3_NATIVE_ADOPTION",
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
            classification: "single-process-native-jubjub-generalized-cp-with-adopted-cp-h2f-v3",
            suite_id: SUITE_ID,
            cp_challenge_suite_id: CP_H2F_SUITE_ID,
            seed,
            n: dkg.config.n,
            t: dkg.config.t,
            tau: dkg.config.tau,
            candidate_count: aggregate.len(),
            election_id: hex(&dkg.config.election_id),
            policy_pre: hex(&dkg.config.policy_pre),
            final_context: hex(&final_context),
            target: hex(&target),
            public_key_hex: hex(&encode_point(&dkg.public_key)),
            dkg_ms,
            vote_encrypt_and_fold_ms,
            cp_prove_all_five_ms,
            cp_verify_all_five_ms,
            ten_subset_verified_threshold_decrypt_ms,
            trustee_proofs,
            subset_results,
            all_five_cp_verified,
            all_ten_subsets_passed,
            challenge_recomputed_by_native_verifier: true,
            only_verified_share_type_combined: true,
            full_g08: false,
            full_p3: false,
            limitations: vec![
                "native CP verification uses adopted CP-H2F-v3; CP group equations are not in R1CS",
                "private package commitments and package signatures are not implemented",
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
        "G0.9-H2-D CP-H2F-v3: trustees=5 coordinates=3 CP-verified=5/5 subsets=10/10 tally=(4,2,4) winner=1 prove-all={cp_prove_all_five_ms:.3}ms verify-all={cp_verify_all_five_ms:.3}ms full_h2=true full_p3=false"
    );
    println!("wrote {}", output.display());
    Ok(())
}
