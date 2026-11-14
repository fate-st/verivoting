use ark_ec::Group;
use ark_ff::{PrimeField, Zero};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashSet,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use vv_friendly_crypto::{
    bounded_discrete_log, encode_point, encrypt_weighted_vote, fold_ballots, Ciphertext, Point,
};
use vv_friendly_dkg::{
    lagrange_at_zero,
    multiprocess::{
        aggregate_path, auth_path, deal_path, deal_trustee, decrypt_trustee, decryption_path,
        final_path, finalize_trustee, init_trustee, load_and_verify_decryption_shares,
        load_public_dkg, private_path, public_transport_bytes, record_resume, resume_path,
        verify_resume_record, write_aggregate, G3A_N, G3A_Q_CERT, G3A_SUBSET, G3A_T, G3A_TAU,
    },
};
use vv_types::{hex, sha256, winner};

const DEFAULT_SEED: u64 = 20_261_024;
const FILE_WAIT_LIMIT: Duration = Duration::from_secs(120);

#[derive(Serialize)]
struct G3Result {
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
    manifest_sha256_hex: String,
    seed: u64,
    n: usize,
    t: usize,
    tau: usize,
    q_cert: usize,
    trustee_process_count: usize,
    unique_os_process_ids: usize,
    total_os_process_instances: usize,
    initial_process_ids: Vec<u32>,
    process_ids: Vec<u32>,
    all_processes_exited_successfully: bool,
    independent_private_state_files: usize,
    private_state_files_persisted: bool,
    authenticated_deal_messages: usize,
    authenticated_decryption_share_messages: usize,
    authenticated_resume_messages: usize,
    restart_tested: bool,
    restarted_trustee: Option<u32>,
    initial_restarted_process_id: Option<u32>,
    resumed_process_id: Option<u32>,
    process_restart_observed: bool,
    persisted_private_state_reused: bool,
    restart_checkpoint_sha256_hex: Option<String>,
    public_transport_bytes: u64,
    local_transport: &'static str,
    qualified_trustees: Vec<u32>,
    complaints: usize,
    corrected_complaints: usize,
    disqualified_dealers: Vec<u32>,
    extraction_recoveries: Vec<u32>,
    estimated_dkg_bytes: usize,
    transcript_digest_hex: String,
    key_core_digest_hex: String,
    public_key_hex: String,
    public_coefficients: usize,
    verification_shares: usize,
    certificate_signers: Vec<u32>,
    certificate_quorum_verified: bool,
    all_trustees_agreed_on_public_state: bool,
    dkg_to_final_ms: f64,
    vote_encrypt_and_fold_ms: f64,
    fixed_subset_threshold_decrypt_ms: f64,
    total_wall_ms: f64,
    coordinator_peak_rss_mib: f64,
    child_peak_rss_mib: f64,
    registered_weight: u64,
    selected_trustees: [u32; 3],
    interpolated_public_key_matches: bool,
    private_tally: Vec<u64>,
    winner: usize,
    expected_tally_matches: bool,
    expected_winner_matches: bool,
    full_g3: bool,
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
fn rss_mib(who: libc::c_int) -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(who, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    unsafe { usage.assume_init() }.ru_maxrss as f64 / (1024.0 * 1024.0)
}

#[cfg(not(target_os = "macos"))]
fn rss_mib(who: libc::c_int) -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(who, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    unsafe { usage.assume_init() }.ru_maxrss as f64 / 1024.0
}

fn wait_for_files(paths: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    while paths.iter().any(|path| !path.is_file()) {
        if started.elapsed() > FILE_WAIT_LIMIT {
            return Err("timed out waiting for a G3-A process artifact".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn trustee_process(
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
    resume: bool,
    stop_after_init: bool,
) -> Result<(), Box<dyn Error>> {
    if resume {
        record_resume(trustee_id, seed, work_dir, manifest_path)?;
    } else {
        init_trustee(trustee_id, seed, work_dir, manifest_path)?;
        if stop_after_init {
            return Ok(());
        }
    }
    wait_for_files(
        &(1..=G3A_N)
            .map(|id| auth_path(work_dir, id as u32))
            .collect::<Vec<_>>(),
    )?;
    deal_trustee(trustee_id, seed, work_dir, manifest_path)?;
    wait_for_files(
        &(1..=G3A_N)
            .map(|dealer| deal_path(work_dir, dealer as u32, trustee_id))
            .collect::<Vec<_>>(),
    )?;
    finalize_trustee(trustee_id, seed, work_dir, manifest_path)?;
    if G3A_SUBSET.contains(&trustee_id) {
        wait_for_files(&[aggregate_path(work_dir)])?;
        decrypt_trustee(trustee_id, seed, work_dir, manifest_path)?;
    }
    Ok(())
}

fn spawn_trustees(
    executable: &Path,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
    restart_trustee: Option<u32>,
) -> Result<Vec<Child>, Box<dyn Error>> {
    (1..=G3A_N)
        .map(|id| {
            let mut command = Command::new(executable);
            command
                .arg("trustee")
                .arg("--id")
                .arg(id.to_string())
                .arg("--seed")
                .arg(seed.to_string())
                .arg("--work-dir")
                .arg(work_dir)
                .arg("--manifest")
                .arg(manifest_path);
            if restart_trustee == Some(id as u32) {
                command.arg("--stop-after-init").arg("true");
            }
            Ok(command
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()?)
        })
        .collect()
}

fn spawn_resumed_trustee(
    executable: &Path,
    trustee_id: u32,
    seed: u64,
    work_dir: &Path,
    manifest_path: &Path,
) -> Result<Child, Box<dyn Error>> {
    Ok(Command::new(executable)
        .arg("trustee")
        .arg("--id")
        .arg(trustee_id.to_string())
        .arg("--seed")
        .arg(seed.to_string())
        .arg("--work-dir")
        .arg(work_dir)
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--resume")
        .arg("true")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?)
}

fn combine_threshold_shares(
    aggregate: &[Ciphertext],
    shares: &[Vec<Point>],
    bound: u64,
) -> Result<Vec<u64>, Box<dyn Error>> {
    let coefficients = lagrange_at_zero(&G3A_SUBSET, G3A_N, G3A_TAU)?;
    aggregate
        .iter()
        .enumerate()
        .map(|(coordinate, ciphertext)| {
            let combined = shares.iter().zip(&coefficients).fold(
                Point::zero(),
                |sum, (trustee_shares, coefficient)| {
                    sum + trustee_shares[coordinate].mul_bigint(coefficient.into_bigint())
                },
            );
            Ok(bounded_discrete_log(&(ciphertext.c1 - combined), bound)?)
        })
        .collect()
}

fn coordinator_main(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut output = PathBuf::from("experiments/g3_a_raw/five_process_dkg.json");
    let mut output_was_set = false;
    let mut manifest = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut seed = DEFAULT_SEED;
    let mut restart_trustee = None;
    let mut iterator = args.into_iter();
    while let Some(flag) = iterator.next() {
        let value = iterator
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--output" => {
                output = PathBuf::from(value);
                output_was_set = true;
            }
            "--manifest" => manifest = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            "--restart-trustee" => restart_trustee = Some(value.parse()?),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    if restart_trustee.is_some_and(|id| id == 0 || id as usize > G3A_N) {
        return Err("--restart-trustee must be between 1 and 5".into());
    }
    if restart_trustee.is_some() && !output_was_set {
        output = PathBuf::from("experiments/g3_b_raw/one_trustee_restart.json");
    }

    let manifest = fs::canonicalize(manifest)?;
    let manifest_bytes = fs::read(&manifest)?;
    let manifest_json: Value = serde_json::from_slice(&manifest_bytes)?;
    let protocol_version = manifest_json["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let work_label = if restart_trustee.is_some() {
        "g3b"
    } else {
        "g3a"
    };
    let work_dir = PathBuf::from(format!("target/{work_label}-work-{}", std::process::id()));
    fs::create_dir_all(&work_dir)?;
    let work_dir = fs::canonicalize(work_dir)?;
    let executable = env::current_exe()?;

    let total_started = Instant::now();
    let dkg_started = Instant::now();
    let mut children = spawn_trustees(&executable, seed, &work_dir, &manifest, restart_trustee)?;
    let mut initial_restart_exited_successfully = true;
    let restart_evidence = if let Some(trustee_id) = restart_trustee {
        wait_for_files(
            &(1..=G3A_N)
                .map(|id| auth_path(&work_dir, id as u32))
                .collect::<Vec<_>>(),
        )?;
        wait_for_files(&[private_path(&work_dir, trustee_id)])?;
        let initial_status = children[trustee_id as usize - 1].wait()?;
        initial_restart_exited_successfully = initial_status.success();
        if !initial_restart_exited_successfully {
            return Err("the checkpointing Trustee process failed".into());
        }
        let checkpoint = sha256(&fs::read(private_path(&work_dir, trustee_id))?);
        children[trustee_id as usize - 1] =
            spawn_resumed_trustee(&executable, trustee_id, seed, &work_dir, &manifest)?;
        wait_for_files(&[resume_path(&work_dir, trustee_id)])?;
        Some(verify_resume_record(
            trustee_id, checkpoint, &work_dir, &manifest,
        )?)
    } else {
        None
    };
    wait_for_files(
        &(1..=G3A_N)
            .map(|id| final_path(&work_dir, id as u32))
            .collect::<Vec<_>>(),
    )?;
    let public_dkg = load_public_dkg(&work_dir, &manifest)?;
    let dkg_to_final_ms = dkg_started.elapsed().as_secs_f64() * 1000.0;

    let weights = [3u64, 2, 1, 4];
    let choices = [1usize, 2, 1, 3];
    let registered_weight = weights.iter().sum();
    let mut vote_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x4733_412D_564F_5445);
    let vote_started = Instant::now();
    let ballots = weights
        .iter()
        .zip(choices)
        .map(|(weight, choice)| {
            encrypt_weighted_vote(
                &public_dkg.public_key,
                *weight,
                choice,
                3,
                registered_weight,
                &mut vote_rng,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let aggregate = fold_ballots(&ballots)?;
    let vote_encrypt_and_fold_ms = vote_started.elapsed().as_secs_f64() * 1000.0;

    let threshold_started = Instant::now();
    write_aggregate(
        &work_dir,
        &protocol_version,
        public_dkg.config.session_id,
        &aggregate,
    )?;
    wait_for_files(
        &G3A_SUBSET
            .iter()
            .map(|id| decryption_path(&work_dir, *id))
            .collect::<Vec<_>>(),
    )?;
    let decryption_shares = load_and_verify_decryption_shares(&work_dir, &public_dkg, &aggregate)?;
    let private_tally =
        combine_threshold_shares(&aggregate, &decryption_shares, registered_weight)?;
    let selected_winner = winner(&private_tally, registered_weight)?;
    let fixed_subset_threshold_decrypt_ms = threshold_started.elapsed().as_secs_f64() * 1000.0;

    let all_processes_exited_successfully = initial_restart_exited_successfully
        && children
            .iter_mut()
            .map(Child::wait)
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .all(|status| status.success());
    if !all_processes_exited_successfully {
        return Err("one or more Trustee processes failed".into());
    }

    let coefficients = lagrange_at_zero(&G3A_SUBSET, G3A_N, G3A_TAU)?;
    let interpolated_public_key =
        G3A_SUBSET
            .iter()
            .zip(coefficients)
            .fold(Point::zero(), |sum, (id, coefficient)| {
                sum + public_dkg.verification_shares[*id as usize - 1]
                    .mul_bigint(coefficient.into_bigint())
            });
    let interpolated_public_key_matches = interpolated_public_key == public_dkg.public_key;
    let expected_tally_matches = private_tally == [4, 2, 4];
    let expected_winner_matches = selected_winner == 1;
    let unique_os_process_ids = public_dkg
        .process_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .len();
    let total_os_process_instances = public_dkg
        .initial_process_ids
        .iter()
        .chain(&public_dkg.process_ids)
        .copied()
        .collect::<HashSet<_>>()
        .len();
    let process_restart_observed = restart_evidence.as_ref().is_some_and(|evidence| {
        evidence.initial_process_id != evidence.resumed_process_id
            && public_dkg.initial_process_ids[evidence.trustee_id as usize - 1]
                == evidence.initial_process_id
            && public_dkg.process_ids[evidence.trustee_id as usize - 1]
                == evidence.resumed_process_id
    });
    let persisted_private_state_reused = restart_evidence.is_some();
    let process_identity_boundary_matches = public_dkg
        .initial_process_ids
        .iter()
        .zip(&public_dkg.process_ids)
        .enumerate()
        .all(|(index, (initial, final_id))| {
            if restart_trustee == Some((index + 1) as u32) {
                initial != final_id
            } else {
                initial == final_id
            }
        });
    let private_state_files = (1..=G3A_N)
        .filter(|id| private_path(&work_dir, *id as u32).is_file())
        .count();
    let certificate_quorum_verified = public_dkg.certificate_signers.len() >= G3A_Q_CERT;
    if unique_os_process_ids != G3A_N
        || private_state_files != G3A_N
        || !certificate_quorum_verified
        || !interpolated_public_key_matches
        || !expected_tally_matches
        || !expected_winner_matches
        || !process_identity_boundary_matches
        || (restart_trustee.is_some()
            && (!process_restart_observed
                || !persisted_private_state_reused
                || total_os_process_instances != G3A_N + 1))
        || (restart_trustee.is_none() && total_os_process_instances != G3A_N)
    {
        return Err("G3 process acceptance boundary failed".into());
    }

    let (gate, status, classification, limitations) = if restart_trustee.is_some() {
        (
            "G3-B",
            "PASS_G3B_ONE_TRUSTEE_RESTART_RECOVERY",
            "five-trustee-jubjub-dkg-with-one-persisted-state-process-restart-and-fixed-123-threshold-decryption",
            vec![
                "G3-B uses authenticated atomic local-file transport, not remote sockets or TLS",
                "only one orderly post-checkpoint Trustee restart is measured; mid-write crashes and timeout matrices are not tested",
                "only the fixed Trustee subset 1-2-3 is measured",
            ],
        )
    } else {
        (
            "G3-A",
            "PASS_G3A_FIVE_OS_PROCESS_DKG_HAPPY_PATH",
            "five-concurrent-os-process-jubjub-pedersen-vss-gjkr-happy-path-with-signed-local-file-transport-and-fixed-123-threshold-decryption",
            vec![
                "G3-A uses authenticated atomic local-file transport, not remote sockets or TLS",
                "only the honest happy path and fixed Trustee subset 1-2-3 are measured",
                "restart recovery, timeout handling and adversarial process tests are deferred to G3-B",
            ],
        )
    };
    let report = G3Result {
        gate,
        status,
        classification,
        git_commit: command_output("git", &["rev-parse", "HEAD"])
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
        git_dirty: command_output(
            "git",
            &["status", "--porcelain", "--untracked-files=normal"],
        )
        .map_or_else(
            || "UNKNOWN".to_owned(),
            |value| (!value.is_empty()).to_string(),
        ),
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
        prototype_manifest: manifest.display().to_string(),
        protocol_version,
        manifest_sha256_hex: hex(&sha256(&manifest_bytes)),
        seed,
        n: G3A_N,
        t: G3A_T,
        tau: G3A_TAU,
        q_cert: G3A_Q_CERT,
        trustee_process_count: G3A_N,
        unique_os_process_ids,
        total_os_process_instances,
        initial_process_ids: public_dkg.initial_process_ids,
        process_ids: public_dkg.process_ids,
        all_processes_exited_successfully,
        independent_private_state_files: private_state_files,
        private_state_files_persisted: true,
        authenticated_deal_messages: G3A_N * G3A_N,
        authenticated_decryption_share_messages: G3A_SUBSET.len(),
        authenticated_resume_messages: usize::from(restart_evidence.is_some()),
        restart_tested: restart_trustee.is_some(),
        restarted_trustee: restart_evidence
            .as_ref()
            .map(|evidence| evidence.trustee_id),
        initial_restarted_process_id: restart_evidence
            .as_ref()
            .map(|evidence| evidence.initial_process_id),
        resumed_process_id: restart_evidence
            .as_ref()
            .map(|evidence| evidence.resumed_process_id),
        process_restart_observed,
        persisted_private_state_reused,
        restart_checkpoint_sha256_hex: restart_evidence
            .as_ref()
            .map(|evidence| evidence.private_state_sha256_hex.clone()),
        public_transport_bytes: public_transport_bytes(&work_dir)?,
        local_transport: "atomic local JSON files with session-bound Jubjub Schnorr signatures",
        qualified_trustees: (1..=G3A_N).map(|id| id as u32).collect(),
        complaints: 0,
        corrected_complaints: 0,
        disqualified_dealers: Vec::new(),
        extraction_recoveries: Vec::new(),
        estimated_dkg_bytes: public_dkg.estimated_dkg_bytes,
        transcript_digest_hex: hex(&public_dkg.transcript_digest),
        key_core_digest_hex: hex(&public_dkg.key_core_digest),
        public_key_hex: hex(&encode_point(&public_dkg.public_key)),
        public_coefficients: public_dkg.public_coefficients.len(),
        verification_shares: public_dkg.verification_shares.len(),
        certificate_signers: public_dkg.certificate_signers,
        certificate_quorum_verified,
        all_trustees_agreed_on_public_state: true,
        dkg_to_final_ms,
        vote_encrypt_and_fold_ms,
        fixed_subset_threshold_decrypt_ms,
        total_wall_ms: total_started.elapsed().as_secs_f64() * 1000.0,
        coordinator_peak_rss_mib: rss_mib(libc::RUSAGE_SELF),
        child_peak_rss_mib: rss_mib(libc::RUSAGE_CHILDREN),
        registered_weight,
        selected_trustees: G3A_SUBSET,
        interpolated_public_key_matches,
        private_tally,
        winner: selected_winner,
        expected_tally_matches,
        expected_winner_matches,
        full_g3: false,
        limitations,
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{}: processes={} process-instances={} restart={} signed-deals={} signed-decryptions={} dkg={:.3}ms threshold={:.3}ms tally={:?} winner={} transport={}B full-g3=false",
        report.gate,
        report.trustee_process_count,
        report.total_os_process_instances,
        report.process_restart_observed,
        report.authenticated_deal_messages,
        report.authenticated_decryption_share_messages,
        report.dkg_to_final_ms,
        report.fixed_subset_threshold_decrypt_ms,
        report.private_tally,
        report.winner,
        report.public_transport_bytes,
    );
    println!("wrote {}", output.display());
    Ok(())
}

fn trustee_main(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut trustee_id = None;
    let mut seed = DEFAULT_SEED;
    let mut work_dir = None;
    let mut manifest = None;
    let mut resume = false;
    let mut stop_after_init = false;
    let mut iterator = args.into_iter();
    while let Some(flag) = iterator.next() {
        let value = iterator
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--id" => trustee_id = Some(value.parse()?),
            "--seed" => seed = value.parse()?,
            "--work-dir" => work_dir = Some(PathBuf::from(value)),
            "--manifest" => manifest = Some(PathBuf::from(value)),
            "--resume" => resume = value.parse()?,
            "--stop-after-init" => stop_after_init = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    if resume && stop_after_init {
        return Err("Trustee cannot resume and stop after init simultaneously".into());
    }
    trustee_process(
        trustee_id.ok_or("missing --id")?,
        seed,
        &work_dir.ok_or("missing --work-dir")?,
        &manifest.ok_or("missing --manifest")?,
        resume,
        stop_after_init,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|value| value == "trustee") {
        args.remove(0);
        trustee_main(args)
    } else {
        coordinator_main(args)
    }
}
