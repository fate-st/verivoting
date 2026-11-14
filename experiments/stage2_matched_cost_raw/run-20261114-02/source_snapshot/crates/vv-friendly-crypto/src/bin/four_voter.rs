use serde::Serialize;
use std::{env, error::Error, fs, path::PathBuf, process::Command};
use vv_friendly_crypto::{run_four_voter_fixture, suite_vectors, FourVoterReport};

#[derive(Serialize)]
struct RunEnvelope {
    gate: &'static str,
    status: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    release_strip: String,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    experiment: FourVoterReport,
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
    let mut output =
        PathBuf::from("experiments/g0_8_suite_v2_raw/friendly-elgamal-four-voter.json");
    let mut vectors_output = PathBuf::from("fixtures/jubjub_suite_vectors.v2.json");
    let mut seed = 20261020u64;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--output" => output = PathBuf::from(value),
            "--vectors-output" => vectors_output = PathBuf::from(value),
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
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let experiment = run_four_voter_fixture(seed)?;
    let report = RunEnvelope {
        gate: "G0.8-2",
        status: "PARTIAL_WEIGHTED_ELGAMAL_ONLY",
        git_commit,
        git_dirty,
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
        experiment,
    };
    fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    if let Some(parent) = vectors_output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &vectors_output,
        serde_json::to_vec_pretty(&suite_vectors()?)?,
    )?;
    println!(
        "G0.8-2 native Jubjub ElGamal: tally={:?} winner={} encrypt={:.3}ms aggregate/decrypt/decode={:.3}ms full_g08=false full_p3=false",
        report.experiment.private_tally,
        report.experiment.public_winner,
        report.experiment.encryption_ms,
        report.experiment.aggregate_decrypt_decode_ms,
    );
    println!("wrote {}", output.display());
    println!("wrote {}", vectors_output.display());
    Ok(())
}
