//! G6: one live native receiver -> frozen CT2 -> private CP/decode -> new P3 -> one winner write.
#![recursion_limit = "512"]
#[path = "common/g6_batches.rs"]
mod batches;
#[path = "common/g6_materials.rs"]
mod materials;
#[path = "common/g2e_receiver.rs"]
mod receiver;
#[path = "common/g6_state.rs"]
mod state;
use ark_bls12_381::Bls12_381;
use ark_groth16::Groth16;
use ark_serialize::CanonicalSerialize;
use materials::{compressed, decode, fixed, input_hex, Result};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use vv_friendly_crypto::p3ten_gadget::{G4D_INPUTS, G4D_LAYOUT, G4D_RELATION_ID, G4D_SUBSETS};
use vv_friendly_crypto::{decode_point, generator};
use vv_types::{hex, sha256};
const DESCRIPTOR: &str = "configs/protocol_manifest.JUBJUB-G6-NATIVE-WINNER-ONLY-W32.json";
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G6-NATIVE-WINNER-ONLY-v1";
// Reconstruct exactly the G4-D key, but generate a NEW G6 proof with NEW CP packages.
const SETUP_SEED: u64 = 0x4734_442D_5345_5455;
const PROOF_SEED: u64 = 0x4736_2D50_524F_4F46;

fn metadata(program: &str, args: &[&str]) -> Result<String> {
    let x = Command::new(program).args(args).output()?;
    if !x.status.success() {
        return Err(format!("{program} metadata failed").into());
    }
    Ok(String::from_utf8(x.stdout)?.trim().to_owned())
}
fn write_new(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(&serde_json::to_vec_pretty(value)?)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn rss() -> f64 {
    let mut u = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, u.as_mut_ptr()) } != 0 {
        return 0.;
    }
    let u = unsafe { u.assume_init() };
    #[cfg(target_os = "macos")]
    let divisor = 1024. * 1024.;
    #[cfg(not(target_os = "macos"))]
    let divisor = 1024.;
    u.ru_maxrss as f64 / divisor
}
fn source_hashes() -> Result<Vec<Value>> {
    [
        "crates/vv-friendly-dkg/src/bin/g6_native_winner_only.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_materials.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_batches.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_state.rs",
        "crates/vv-friendly-dkg/src/bin/common/g2e_receiver.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-dkg/src/package.rs",
        "crates/vv-friendly-dkg/src/registration.rs",
        "crates/vv-friendly-crypto/src/p3ten_gadget.rs",
        "crates/vv-friendly-crypto/src/p3c1_gadget.rs",
        "crates/vv-friendly-crypto/src/p3d_gadget.rs",
        "crates/vv-friendly-crypto/src/p3b_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h4_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h3_gadget.rs",
        "crates/vv-friendly-crypto/src/hash_to_field_gadget.rs",
        "crates/vv-friendly-crypto/src/state_gadget.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "Cargo.toml",
        "Cargo.lock",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect()
}
struct Baseline {
    descriptor: Value,
    p3: Value,
    raw: Value,
    pins: Vec<Value>,
}
fn baseline() -> Result<Baseline> {
    let bytes = fs::read(DESCRIPTOR)?;
    let d: Value = serde_json::from_slice(&bytes)?;
    if d["protocol_version"] != PROTOCOL
        || d["release_policy"] != state::RELEASE
        || d["subset"] != json!(G4D_SUBSETS[0])
        || d["relation_id"] != G4D_RELATION_ID
        || d["input_layout"] != G4D_LAYOUT
        || d["production_setup"] != false
    {
        return Err("G6 descriptor mismatch".into());
    }
    let mut pins = vec![json!({"path":DESCRIPTOR,"sha256":hex(&sha256(&bytes))})];
    let mut read = |key: &str| -> Result<Value> {
        let p = d[key].as_str().ok_or("pin path")?;
        let b = fs::read(p)?;
        let h = hex(&sha256(&b));
        if d[format!("{key}_sha256")] != h {
            return Err(format!("G6 pin changed {p}").into());
        }
        pins.push(json!({"path":p,"sha256":h}));
        Ok(serde_json::from_slice(&b)?)
    };
    let p3 = read("p3_fixture")?;
    let raw = read("p3_raw")?;
    let _ = read("p3_descriptor")?;
    if p3["verifying_key_sha256"] != d["expected_p3_vk_sha256"]
        || raw["verifying_key_sha256"] != d["expected_p3_vk_sha256"]
        || raw["matrix_sha256"] != d["expected_matrix_sha256"]
        || raw["status"] != "PASS_TEN_P3_V2_SUBSETS_ONE_CIRCUIT_ONE_VK_FROZEN_W32_CT2"
    {
        return Err("G4-D relation/key evidence mismatch".into());
    }
    // Enforce unchanged cryptographic implementation; Cargo only registers the new runner.
    for p in raw["source_hashes"]
        .as_array()
        .ok_or("G4-D source hashes")?
    {
        let path = p["path"].as_str().ok_or("source path")?;
        if path.ends_with("Cargo.toml") {
            continue;
        }
        let h = hex(&sha256(&fs::read(path)?));
        if p["sha256"] != h {
            return Err(format!("G4-D source changed {path}").into());
        }
        pins.push(json!({"path":path,"sha256":h}));
    }
    Ok(Baseline {
        descriptor: d,
        p3,
        raw,
        pins,
    })
}
fn anchors(roster: &Value) -> Result<Vec<(vv_friendly_crypto::Point, vv_friendly_crypto::Point)>> {
    let rows = roster.as_array().ok_or("roster")?;
    if rows.len() != 5 {
        return Err("five roster entries".into());
    }
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            if r["trustee_id"] != i + 1 {
                return Err("roster order".into());
            }
            Ok((
                decode_point(&fixed(r, "authentication_public_key_hex")?, false)?,
                decode_point(&fixed(r, "verification_share_hex")?, false)?,
            ))
        })
        .collect()
}
fn receive(flow: &mut state::WinnerOnly, verifier: &batches::BatchVerifier) -> Result<Vec<Value>> {
    let mut metrics = Vec::new();
    for (i, b) in verifier.batches.iter().enumerate() {
        let mut m = Value::Null;
        let t = Instant::now();
        flow.accept(&b.circuit, || verifier.verify(i, &mut m))
            .map_err(|e| format!("live receiver: {e:?}"))?;
        m["receiver_check_verify_apply_ms"] = json!(t.elapsed().as_secs_f64() * 1000.);
        m["checkpoint"] = flow.snapshot();
        metrics.push(m);
    }
    Ok(metrics)
}
fn replay(path: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        return Err("new replay output required".into());
    }
    let b = baseline()?;
    let s = materials::load_sources()?;
    let batches = batches::BatchVerifier::load(&s)?;
    let bytes = fs::read(path)?;
    let p: Value = serde_json::from_slice(&bytes)?;
    if p["schema"] != "VV/G6/NATIVE-WINNER-ONLY/PUBLIC/v1"
        || p["verifying_key_hex"] != b.p3["verifying_key_hex"]
        || p["relation_id"] != G4D_RELATION_ID
        || p["protocol_version"] != PROTOCOL
        || p["input_layout"] != G4D_LAYOUT
        || p["subset"] != json!(G4D_SUBSETS[0])
        || p["source_g2e_fixture"] != s.config["g2e_fixture"]
        || p["source_g2e_fixture_sha256"] != s.config["g2e_fixture_sha256"]
    {
        return Err("public G6 schema/VK differs from pinned G4-D key".into());
    }
    let mut flow = state::WinnerOnly::new(
        batches.batches[0].circuit.records[0].clone(),
        batches
            .batches
            .iter()
            .map(|x| x.circuit.batch_root)
            .collect(),
        b.p3["public_context"].clone(),
        anchors(&b.p3["trustee_roster"])?,
        b.p3["verifying_key_sha256"]
            .as_str()
            .ok_or("VK hash")?
            .into(),
    )?;
    let received = receive(&mut flow, &batches)?;
    let f = flow.freeze().map_err(|e| format!("freeze: {e:?}"))?;
    if p["freeze_digest_hex"] != hex(&f.digest) || p["public_context"] != f.context {
        return Err("public G6 frozen binding differs".into());
    }
    let roster = p["trustee_roster"]
        .as_array()
        .ok_or("public roster")?
        .clone();
    flow.install_roster(roster.clone())
        .map_err(|e| format!("roster: {e:?}"))?;
    let winner = p["public_winner"].as_u64().ok_or("winner")?;
    let expected = materials::expected_inputs(&f.context, &roster, &f.ciphertexts, 0, winner)?;
    if p["public_inputs_le_hex"] != json!(input_hex(&expected)?) {
        return Err("P3 inputs not equal to live freeze".into());
    }
    let proof_bytes = decode(p["proof_hex"].as_str().ok_or("proof")?)?;
    if p["proof_sha256"] != hex(&sha256(&proof_bytes)) {
        return Err("proof hash".into());
    }
    let proof = materials::canonical(&proof_bytes)?;
    let vk = materials::canonical(&decode(p["verifying_key_hex"].as_str().ok_or("VK")?)?)?;
    let t = Instant::now();
    flow.disclose(winner, &expected, &proof, &vk)
        .map_err(|e| format!("public disclose: {e:?}"))?;
    let disclosure_ms = t.elapsed().as_secs_f64() * 1000.;
    let snapshot = flow.snapshot();
    if p["final_checkpoint"] != snapshot {
        return Err("public checkpoint differs from actual receiver/freeze/disclosure".into());
    }
    if flow.disclose(winner, &expected, &proof, &vk) != Err(state::Rejection::Disclosed)
        || flow.snapshot() != snapshot
    {
        return Err("public repeated disclosure accepted".into());
    }
    write_new(
        output,
        &json!({"gate":"G6-public-replay","status":"PASS_PUBLIC_RECEIVE_FREEZE_AND_WINNER_ONCE","fixture_sha256":hex(&sha256(&bytes)),"batches":received,"final_checkpoint":snapshot,"winner_disclosure_verify_ms":disclosure_ms,"setup_count":0,"new_p3_proof_count":0,"no_private_material_loaded":true,"scope":"research public replay against pinned certified G4-D public anchors, not a production registry","evm_run":false}),
    )?;
    println!("G6 PUBLIC REPLAY PASS: both batches -> live freeze -> new P3 -> one winner write; zero setup/prove/private input");
    Ok(())
}
fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--verify-fixture") {
        if args.len() != 4 || args[2] != "--output" {
            return Err("use --verify-fixture PATH --output NEW_PATH".into());
        }
        return replay(Path::new(&args[1]), Path::new(&args[3]));
    }
    let mut output = PathBuf::from("experiments/g6_raw/native_winner_only.json");
    let mut fixture = PathBuf::from("fixtures/jubjub_g6_native_winner_only_public_v1.json");
    for pair in args.chunks(2) {
        if pair.len() != 2 {
            return Err("option value".into());
        }
        match pair[0].as_str() {
            "--output" => output = pair[1].clone().into(),
            "--fixture-output" => fixture = pair[1].clone().into(),
            _ => return Err("unknown option".into()),
        }
    }
    if output.exists() || fixture.exists() || output == fixture {
        return Err("distinct NEW outputs required".into());
    }
    let started = Instant::now();
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let git_commit = metadata("git", &["rev-parse", "HEAD"])?;
    let git_status = metadata("git", &["status", "--porcelain", "--untracked-files=all"])?;
    let hashes = source_hashes()?;
    let baseline = baseline()?;
    let sources = materials::load_sources()?;
    println!("G6 [1/7] rebuild same certified DKG and registration; exact D2 context/root/first-eight ciphertexts");
    let (dkg, keys, registration, binding) = materials::rebuild_dkg(&sources)?;
    let anchors = (1u32..=5)
        .map(|id| {
            Ok((
                dkg.config.authentication_public_keys[id as usize - 1],
                *dkg.verification_share(id)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let verifier = batches::BatchVerifier::load(&sources)?;
    let mut flow = state::WinnerOnly::new(
        verifier.batches[0].circuit.records[0].clone(),
        verifier
            .batches
            .iter()
            .map(|b| b.circuit.batch_root)
            .collect(),
        binding.clone(),
        anchors,
        baseline.p3["verifying_key_sha256"]
            .as_str()
            .ok_or("VK hash")?
            .into(),
    )?;
    println!("G6 [2/7] REAL G2-E Receiver: verify both archived SnarkPack + state proofs, sequential check-then-write");
    let batch_metrics = receive(&mut flow, &verifier)?;
    let t = Instant::now();
    let frozen = flow.freeze().map_err(|e| format!("live freeze: {e:?}"))?;
    let freeze_ms = t.elapsed().as_secs_f64() * 1000.;
    if sources.state["final_state"] != json!(materials::ciphertexts(&frozen.ciphertexts)) {
        return Err("live CT2 differs from historical regression target".into());
    }
    let frozen_checkpoint = flow.snapshot();
    if !matches!(flow.freeze(), Err(state::Rejection::Frozen))
        || flow.snapshot() != frozen_checkpoint
    {
        return Err("second freeze modified storage".into());
    }
    if flow.accept(&verifier.batches[0].circuit, || {
        panic!("post-freeze must reject before verification")
    }) != Err(state::Rejection::Frozen)
        || flow.snapshot() != frozen_checkpoint
    {
        return Err("post-freeze mutation accepted".into());
    }
    println!(
        "G6 [3/7] freeze CT2 ONCE; create/deliver/verify five NEW private CP packages in process"
    );
    let t = Instant::now();
    let m = materials::build_materials(
        &dkg,
        &keys,
        &registration,
        &frozen.ciphertexts,
        frozen.context.clone(),
    )?;
    flow.install_roster(m.roster.clone())
        .map_err(|e| format!("authenticated roster: {e:?}"))?;
    let packages_ms = t.elapsed().as_secs_f64() * 1000.;
    let t = Instant::now();
    let circuit = materials::build_case(0, &dkg, &registration, &frozen.ciphertexts, &m)?;
    let private_decode_and_witness_ms = t.elapsed().as_secs_f64() * 1000.;
    let winner = circuit.public_winner;
    let inputs = circuit.public_inputs()?;
    println!("G6 [4/7] private 3-of-5 decode; complete unchanged P3 v2 R1CS for live frozen CT2");
    let shape = materials::measure(&circuit)?;
    if shape["satisfied"] != true
        || shape["public_inputs"] != G4D_INPUTS
        || shape["matrix_sha256"] != baseline.raw["matrix_sha256"]
    {
        return Err("G6 changed G4-D relation".into());
    }
    println!("G6 [5/7] rebuild ONE research setup; require exact full G4-D VK; PK stays in memory");
    let t = Instant::now();
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let setup_ms = t.elapsed().as_secs_f64() * 1000.;
    let vk_bytes = compressed(&params.vk)?;
    let vk_hash = hex(&sha256(&vk_bytes));
    if baseline.p3["verifying_key_hex"] != hex(&vk_bytes)
        || baseline.descriptor["expected_p3_vk_sha256"] != vk_hash
    {
        return Err("rebuilt full G4-D VK differs".into());
    }
    println!(
        "  Setup MATCH: {setup_ms:.3} ms; VK={vk_hash}; peakRSS={:.3}MiB",
        rss()
    );
    println!(
        "G6 [6/7] create ONE NEW P3 proof from this run's private packages and decoded witness"
    );
    let t = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit,
        &params,
        &mut ChaCha20Rng::seed_from_u64(PROOF_SEED),
    )?;
    let prove_ms = t.elapsed().as_secs_f64() * 1000.;
    let proof_bytes = compressed(&proof)?;
    let mut altered = frozen.ciphertexts.clone();
    altered[0].c1 += generator();
    let altered_context = state::context_for(&frozen.context, &altered)?;
    let wrong_inputs =
        materials::expected_inputs(&altered_context, &m.roster, &altered, 0, winner)?;
    let before = flow.snapshot();
    let t = Instant::now();
    let target_rejected =
        flow.disclose(winner, &wrong_inputs, &proof, &params.vk) == Err(state::Rejection::Target);
    let target_reject_ms = t.elapsed().as_secs_f64() * 1000.;
    if !target_rejected || flow.snapshot() != before {
        return Err("frozen-target substitution not rejected atomically".into());
    }
    let t = Instant::now();
    flow.disclose(winner, &inputs, &proof, &params.vk)
        .map_err(|e| format!("real winner proof: {e:?}"))?;
    let disclosure_ms = t.elapsed().as_secs_f64() * 1000.;
    let final_checkpoint = flow.snapshot();
    let t = Instant::now();
    let repeat_rejected =
        flow.disclose(winner, &inputs, &proof, &params.vk) == Err(state::Rejection::Disclosed);
    let repeat_reject_ms = t.elapsed().as_secs_f64() * 1000.;
    if !repeat_rejected
        || flow.snapshot() != final_checkpoint
        || final_checkpoint["winner_write_count"] != 1
    {
        return Err("winner repeated write".into());
    }
    if hashes != source_hashes()? || git_commit != metadata("git", &["rev-parse", "HEAD"])? {
        return Err("source changed during execution".into());
    }
    let mut pins = sources.pins;
    pins.extend(baseline.pins);
    for p in &pins {
        if p["sha256"] != hex(&sha256(&fs::read(p["path"].as_str().ok_or("pin")?)?)) {
            return Err("input changed during execution".into());
        }
    }
    let public = json!({"schema":"VV/G6/NATIVE-WINNER-ONLY/PUBLIC/v1","protocol_version":PROTOCOL,"relation_id":G4D_RELATION_ID,"input_layout":G4D_LAYOUT,
        "source_g2e_fixture":sources.config["g2e_fixture"],"source_g2e_fixture_sha256":sources.config["g2e_fixture_sha256"],
        "verifying_key_hex":hex(&vk_bytes),"verifying_key_sha256":vk_hash,"freeze_digest_hex":hex(&frozen.digest),"public_context":frozen.context,
        "trustee_roster":m.roster,"subset":G4D_SUBSETS[0],"public_winner":winner,"proof_hex":hex(&proof_bytes),"proof_sha256":hex(&sha256(&proof_bytes)),
        "public_inputs_le_hex":input_hex(&inputs)?,"final_checkpoint":final_checkpoint,"private_tally_or_shares_emitted":false,"production_setup":false});
    write_new(&fixture, &public)?;
    write_new(
        &output,
        &json!({"gate":"G6","status":"PASS_LIVE_RECEIVER_FREEZE_PRIVATE_DECRYPT_P3_WINNER_ONCE","protocol_version":PROTOCOL,
        "started_utc":started_utc,"git_commit":git_commit,"git_dirty":!git_status.is_empty(),"git_status_at_start":git_status,"source_hashes":hashes,"source_artifact_pins":pins,"sources_unchanged_before_archival":true,
        "certified_dkg_and_registration":binding,"batches":batch_metrics,"frozen_checkpoint":frozen_checkpoint,"freeze_digest_hex":hex(&frozen.digest),"freeze_ms":freeze_ms,
        "new_private_packages":5,"private_packages_verified":5,"private_delivery":"in-process private objects, no network/private serialization","selected_subset":G4D_SUBSETS[0],"packages_create_deliver_verify_ms":packages_ms,
        "private_threshold_decode_and_witness_ms":private_decode_and_witness_ms,"shape":shape,"g4d_relation_and_vk_unchanged":true,"setup_rebuild_count":1,"setup_rebuild_ms":setup_ms,
        "setup_seed_hex":format!("{SETUP_SEED:016x}"),"proof_seed_hex":format!("{PROOF_SEED:016x}"),"package_seed_hex":format!("{:016x}",materials::PACKAGE_SEED),
        "new_p3_proof_count":1,"prove_ms":prove_ms,"winner_disclosure_check_prepare_verify_write_ms":disclosure_ms,"proof_bytes":proof_bytes.len(),"public_input_bytes":inputs.len()*32,
        "verifying_key_sha256":vk_hash,"verifying_key_bytes":vk_bytes.len(),"proving_key_compressed_bytes":params.compressed_size(),"peak_rss_mib":rss(),
        "negative_tests":{"frozen_target_substitution_rejected":target_rejected,"target_reject_ms":target_reject_ms,"repeat_disclosure_rejected":repeat_rejected,"repeat_reject_ms":repeat_reject_ms,"second_freeze_rejected":true,"post_freeze_batch_rejected":true,"rejected_operations_do_not_write":true},
        "final_checkpoint":final_checkpoint,"public_winner":winner,"public_fixture":fixture,"public_fixture_sha256":hex(&sha256(&fs::read(&fixture)?)),
        "reused_ballot_proof_count":8,"reused_snarkpack_proof_count":2,"reused_state_proof_count":2,"new_ballot_proof_count":0,"new_state_proof_count":0,"complete_native_receiver_to_winner_flow":true,
        "production_setup":false,"private_tally_or_shares_emitted":false,"proving_key_persisted":false,"evm_run":false,"gas_total":null,"calldata_bytes":null,"full_settlement":false,
        "cpu":metadata("sysctl",&["-n","machdep.cpu.brand_string"]).unwrap_or_else(|_|"UNKNOWN".into()),"os":env::consts::OS,"arch":env::consts::ARCH,"rustc_version":metadata("rustc",&["--version"])?,"build_profile":if cfg!(debug_assertions){"dev"}else{"release-opt-level-1"},
        "total_run_ms_excluding_compilation":started.elapsed().as_secs_f64()*1000.,
        "limitations":["serial in-memory state machine, not a durable concurrent service or a smart contract",
            "fresh five CP packages are privately delivered as same-process objects, not independent trustee network processes",
            "archived validity/state proofs are reused but both real verifiers execute in this run before CT2 is frozen",
            "one functional integration, not repeated performance statistics or a completed joint-knowledge/privacy theorem",
            "deterministic research setup and prior fake aggregation SRS; published research seeds do not provide operational privacy",
            "freeze seal and one-time release checks are native host enforcement, not additional P3 public inputs or a new circuit",
            "accepted batch roots remain the local pinned schedule, not an independent censorship-resistant intake log",
            "G4-D VK not tested on EVM; exact SnarkPack EVM and complete settlement/Gas remain unfinished"]}),
    )?;
    println!("G6 [7/7] PASS: winner={winner}, writes=1; substituted target/repeated disclosure REJECT; prove={prove_ms:.3}ms verify+write={disclosure_ms:.6}ms RSS={:.3}MiB",rss());
    println!("archived {} and {}", output.display(), fixture.display());
    Ok(())
}
