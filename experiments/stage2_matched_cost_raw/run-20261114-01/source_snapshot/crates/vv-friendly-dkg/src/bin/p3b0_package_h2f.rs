use ark_ec::Group;
use ark_ff::PrimeField;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, GeneralizedCpContext,
};
use vv_friendly_crypto::hash_to_field_gadget::check_package_hash_to_field_v1;
use vv_friendly_crypto::{
    encode_scalar, encrypt, generator, package_hash_to_field_v1, random_nonzero_scalar,
    PACKAGE_H2F_SUITE_ID,
};
use vv_friendly_dkg::package::create_private_package;
use vv_friendly_dkg::AuthenticationKeypair;
use vv_types::{hex, sha256};

const DEFAULT_SEED: u64 = 20_261_024;

#[derive(Serialize)]
struct PackageVector {
    trustee_id: u32,
    package_bytes: usize,
    package_sha256: String,
    package_scalar_le_hex: String,
    commitment_hex: String,
    r1cs_constraints: usize,
    r1cs_public_inputs: usize,
    r1cs_witness_variables: usize,
    r1cs_witness_ms: f64,
    r1cs_satisfied: bool,
}

#[derive(Serialize)]
struct VectorFile {
    suite_id: &'static str,
    protocol_version: String,
    seed: u64,
    selected_trustees: Vec<u32>,
    operation_domain: &'static str,
    mapping: &'static str,
    vectors: Vec<PackageVector>,
    private_package_bytes_emitted: bool,
}

#[derive(Serialize)]
struct P3B0Result {
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
    manifest_matches_p3b0: bool,
    suite_id: &'static str,
    vector_file: String,
    selected_trustees: Vec<u32>,
    package_bytes_per_trustee: usize,
    h2f_input_bytes: usize,
    sha512_blocks: usize,
    constraints_per_package: usize,
    public_inputs_per_package: usize,
    witness_variables_per_package: usize,
    total_witness_ms: f64,
    peak_rss_mib: f64,
    same_fixed_shape: bool,
    native_r1cs_match: bool,
    changed_package_rejected: bool,
    private_package_bytes_emitted: bool,
    native_package_commitment_migrated: bool,
    package_opening_equation_in_p3_r1cs: bool,
    groth16_run: bool,
    full_p3b: bool,
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
    let mut output_path = PathBuf::from("experiments/g0_9_p3b0_raw/package_h2f_adoption.json");
    let mut vector_path = PathBuf::from("fixtures/jubjub_package_h2f_v1.json");
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--vector-output" => vector_path = PathBuf::from(value),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }

    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_capability = manifest["capabilities"]["p3b0_package_h2f"]
        .as_str()
        .ok_or("prototype manifest lacks P3-B0 capability")?
        .to_owned();
    let manifest_matches_p3b0 = protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G0.9-")
        && manifest_capability == "READY_FIXED_SHAPE_PACKAGE_H2F_V1_NATIVE_AND_R1CS"
        && manifest["commitment_suite"]
            .as_str()
            .is_some_and(|suite| suite.contains(PACKAGE_H2F_SUITE_ID));
    if !manifest_matches_p3b0 {
        return Err("prototype manifest does not describe P3-B0 package H2F".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let aggregate_secret = random_nonzero_scalar(&mut rng);
    let public_key = generator().mul_bigint(aggregate_secret.into_bigint());
    let ciphertexts = (0u64..3)
        .map(|message| encrypt(&public_key, message, &mut rng))
        .collect::<Result<Vec<_>, _>>()?;
    let election_id = sha256(b"VV/JUBJUB/G0.9/P3B0/ELECTION");
    let policy_pre = sha256(b"VV/JUBJUB/G0.9/P3B0/POLICY-PRE");
    let final_context = sha256(b"VV/JUBJUB/G0.9/P3B0/FINAL-CONTEXT");
    let registration_close_digest = sha256(b"VV/JUBJUB/G0.9/P3B0/REGISTRATION-CLOSE");
    let disclosure_digest = sha256(b"VV/JUBJUB/G0.9/P3B0/DISCLOSURE-CLOSE");
    let target = derive_cp_target_with_registration(
        election_id,
        policy_pre,
        final_context,
        registration_close_digest,
        disclosure_digest,
        &ciphertexts,
    )?;

    let selected_trustees = vec![1u32, 2, 3];
    let mut vectors = Vec::with_capacity(selected_trustees.len());
    let mut reports = Vec::with_capacity(selected_trustees.len());
    let started = Instant::now();
    for trustee_id in &selected_trustees {
        let secret = random_nonzero_scalar(&mut rng);
        let verification_share = generator().mul_bigint(secret.into_bigint());
        let context = GeneralizedCpContext {
            election_id,
            policy_pre,
            final_context,
            registration_close_digest,
            disclosure_digest,
            target,
            trustee_id: *trustee_id,
        };
        let cp_package = prove_decryption_share(
            &context,
            secret,
            &verification_share,
            &ciphertexts,
            &mut rng,
        )?;
        let authentication_key = AuthenticationKeypair::generate(&mut rng);
        let (private, public) = create_private_package(
            context,
            verification_share,
            cp_package,
            &authentication_key,
            &mut rng,
        )?;
        let encoded = private.encoded_bytes();
        let scalar = package_hash_to_field_v1(&encoded);
        let scalar_bytes = encode_scalar(&scalar);
        let case_started = Instant::now();
        let report = check_package_hash_to_field_v1(&encoded, &scalar_bytes)?;
        let r1cs_witness_ms = case_started.elapsed().as_secs_f64() * 1000.0;
        vectors.push(PackageVector {
            trustee_id: *trustee_id,
            package_bytes: encoded.len(),
            package_sha256: hex(&sha256(&encoded)),
            package_scalar_le_hex: hex(&scalar_bytes),
            commitment_hex: hex(&vv_friendly_crypto::encode_point(&public.commitment.point)),
            r1cs_constraints: report.constraints,
            r1cs_public_inputs: report.public_inputs,
            r1cs_witness_variables: report.witness_variables,
            r1cs_witness_ms,
            r1cs_satisfied: report.satisfied,
        });
        reports.push(report);
    }
    let total_witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let peak_rss_mib = peak_rss_mib();

    let same_fixed_shape = reports.windows(2).all(|pair| {
        pair[0].constraints == pair[1].constraints
            && pair[0].public_inputs == pair[1].public_inputs
            && pair[0].witness_variables == pair[1].witness_variables
    });
    let native_r1cs_match = reports.iter().all(|report| report.satisfied);
    let first_scalar = encode_scalar(&package_hash_to_field_v1(b"P3B0-ORIGINAL-PACKAGE"));
    let changed_package_rejected =
        !check_package_hash_to_field_v1(b"P3B0-CHANGED--PACKAGE", &first_scalar)?.satisfied;
    if !same_fixed_shape || !native_r1cs_match || !changed_package_rejected {
        return Err("P3-B0 package H2F acceptance boundary failed".into());
    }
    let first = reports.first().ok_or("P3-B0 produced no reports")?;
    let package_bytes_per_trustee = vectors
        .first()
        .ok_or("P3-B0 produced no vectors")?
        .package_bytes;
    if vectors
        .iter()
        .any(|vector| vector.package_bytes != package_bytes_per_trustee)
    {
        return Err("P3-B0 package encodings do not have one fixed length".into());
    }

    let vector_file = VectorFile {
        suite_id: PACKAGE_H2F_SUITE_ID,
        protocol_version: protocol_version.clone(),
        seed,
        selected_trustees: selected_trustees.clone(),
        operation_domain: "PRIVATE-PACKAGE",
        mapping: "SHA-512 full digest OS2IP-BE mod Jubjub-Fr",
        vectors,
        private_package_bytes_emitted: false,
    };
    if let Some(parent) = vector_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&vector_path, serde_json::to_vec_pretty(&vector_file)?)?;

    let result = P3B0Result {
        gate: "G0.9-P3-B0",
        status: "PASS_P3B0_PACKAGE_H2F_V1_ADOPTED",
        classification: "fixed-shape-private-package-hash-to-field-native-and-r1cs-adoption-no-opening-equation-no-groth16-not-full-p3",
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
        manifest_matches_p3b0,
        suite_id: PACKAGE_H2F_SUITE_ID,
        vector_file: vector_path.display().to_string(),
        selected_trustees,
        package_bytes_per_trustee,
        h2f_input_bytes: first.h2f_input_bytes,
        sha512_blocks: first.padded_blocks,
        constraints_per_package: first.constraints,
        public_inputs_per_package: first.public_inputs,
        witness_variables_per_package: first.witness_variables,
        total_witness_ms,
        peak_rss_mib,
        same_fixed_shape,
        native_r1cs_match,
        changed_package_rejected,
        private_package_bytes_emitted: false,
        native_package_commitment_migrated: true,
        package_opening_equation_in_p3_r1cs: false,
        groth16_run: false,
        full_p3b: false,
        full_p3: false,
        limitations: vec![
            "P3-B0 adopts and measures the package message mapping but does not yet constrain Pedersen openings inside P3",
            "private package bytes are hashed in R1CS but are not emitted in the public vector file",
            "Lagrange, tally and winner relations remain outside this gate",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&result)?)?;
    println!(
        "G0.9-P3-B0: trustees={:?} package={}B h2f-input={}B blocks={} constraints/package={} public-inputs/package={} witness-vars/package={} total-witness={:.3}ms peak-rss={:.3}MiB fixed-shape={} native-r1cs-match={} changed-package-rejected={} opening-in-p3=false groth16=false full-p3=false",
        result.selected_trustees,
        result.package_bytes_per_trustee,
        result.h2f_input_bytes,
        result.sha512_blocks,
        result.constraints_per_package,
        result.public_inputs_per_package,
        result.witness_variables_per_package,
        result.total_witness_ms,
        result.peak_rss_mib,
        result.same_fixed_shape,
        result.native_r1cs_match,
        result.changed_package_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", vector_path.display());
    Ok(())
}
