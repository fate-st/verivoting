//! Function-first integration: live batch intake -> freeze -> five CP workers ->
//! private threshold decode -> NEW P3 -> one Winner; separate Full Tally diagnostic.
#![recursion_limit = "512"]
#[path = "common/g6_batches.rs"]
mod batches;
#[allow(dead_code)]
#[path = "common/native_batch.rs"]
mod helpers;
#[allow(dead_code)]
#[path = "common/g6_materials.rs"]
mod materials;
#[path = "common/g10c_packages.rs"]
mod packages;
#[path = "common/g2e_receiver.rs"]
mod receiver;
#[path = "common/g6_state.rs"]
mod state;
use ark_bls12_381::Bls12_381;
use ark_groth16::{Groth16, Proof, VerifyingKey};
use materials::{compressed, decode, fixed, input_hex, Result};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
    time::Instant,
};
use vv_friendly_crypto::p3ten_gadget::{G4D_INPUTS, G4D_LAYOUT, G4D_RELATION_ID};
use vv_friendly_crypto::{decode_point, evm_export::Eip2537PublicGroth16Proof, generator};
use vv_types::{hex, sha256};
const ARCHIVE: &str = "fixtures/jubjub_g6_native_winner_only_public_v1.json";
const ARCHIVE_SHA: &str = "ca58fb6cd23258e1edbbdcd96ad7a5d5f93ca20b7af0ab7f8b330f7c02cbef78";
const VK_SHA: &str = "0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29";
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}
fn read(p: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn original() -> Result<Value> {
    let b = fs::read(ARCHIVE)?;
    if hex(&sha256(&b)) != ARCHIVE_SHA {
        return Err("G6 original archive changed".into());
    }
    let j: Value = serde_json::from_slice(&b)?;
    if j["verifying_key_sha256"] != VK_SHA {
        return Err("fixed P3 VK".into());
    }
    Ok(j)
}
fn write(p: &Path, v: &Value, private: bool) -> Result<usize> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
        if private {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }
    let bytes = [serde_json::to_vec_pretty(v)?, b"\n".to_vec()].concat();
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(if private { 0o600 } else { 0o644 })
        .open(p)?;
    f.write_all(&bytes)?;
    Ok(bytes.len())
}
fn anchors(p: &Value) -> Result<Vec<(vv_friendly_crypto::Point, vv_friendly_crypto::Point)>> {
    p["trustee_roster"]
        .as_array()
        .ok_or("roster")?
        .iter()
        .map(|r| {
            Ok((
                decode_point(&fixed(r, "authentication_public_key_hex")?, false)?,
                decode_point(&fixed(r, "verification_share_hex")?, false)?,
            ))
        })
        .collect()
}
fn make_flow(
    verifier: &batches::BatchVerifier,
    binding: Value,
    auth: Vec<(vv_friendly_crypto::Point, vv_friendly_crypto::Point)>,
) -> Result<state::WinnerOnly> {
    state::WinnerOnly::new(
        verifier.batches[0].circuit.records[0].clone(),
        verifier
            .batches
            .iter()
            .map(|b| b.circuit.batch_root)
            .collect(),
        binding,
        auth,
        VK_SHA.into(),
    )
}
fn receive(flow: &mut state::WinnerOnly, verifier: &batches::BatchVerifier) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for (i, b) in verifier.batches.iter().enumerate() {
        let mut m = Value::Null;
        flow.accept(&b.circuit, || verifier.verify(i, &mut m))
            .map_err(|e| format!("receive {e:?}"))?;
        m["checkpoint"] = flow.snapshot();
        out.push(m);
    }
    Ok(out)
}
fn key_and_proof(p: &Value) -> Result<(VerifyingKey<Bls12_381>, Proof<Bls12_381>)> {
    let key = decode(p["verifying_key_hex"].as_str().ok_or("VK")?)?;
    let proof = decode(p["proof_hex"].as_str().ok_or("proof")?)?;
    if hex(&sha256(&key)) != VK_SHA || hex(&sha256(&proof)) != p["proof_sha256"] {
        return Err("public key/proof identity".into());
    }
    Ok((materials::canonical(&key)?, materials::canonical(&proof)?))
}
fn replay_winner(p: &Path, out: &Path) -> Result<()> {
    let original = original()?;
    let public = read(p)?;
    if public["schema"] != "VV/G6/NATIVE-WINNER-ONLY/PUBLIC/v1"
        || public["public_context"] != original["public_context"]
        || public["verifying_key_hex"] != original["verifying_key_hex"]
    {
        return Err("public frozen context/VK mismatch".into());
    }
    let sources = materials::load_sources()?;
    let verifier = batches::BatchVerifier::load(&sources)?;
    let mut flow = make_flow(
        &verifier,
        original["public_context"].clone(),
        anchors(&original)?,
    )?;
    let steps = receive(&mut flow, &verifier)?;
    let frozen = flow.freeze().map_err(|e| format!("freeze {e:?}"))?;
    if public["freeze_digest_hex"] != hex(&frozen.digest) {
        return Err("public freeze seal".into());
    }
    let roster = public["trustee_roster"].as_array().ok_or("roster")?.clone();
    flow.install_roster(roster.clone())
        .map_err(|e| format!("public commitments {e:?}"))?;
    let winner = public["public_winner"].as_u64().ok_or("winner")?;
    let inputs =
        materials::expected_inputs(&frozen.context, &roster, &frozen.ciphertexts, 0, winner)?;
    if public["public_inputs_le_hex"] != json!(input_hex(&inputs)?) {
        return Err("public inputs/storage".into());
    }
    let (vk, proof) = key_and_proof(&public)?;
    let t = Instant::now();
    flow.disclose(winner, &inputs, &proof, &vk)
        .map_err(|e| format!("public proof {e:?}"))?;
    let verify_ms = ms(t);
    let checkpoint = flow.snapshot();
    if checkpoint != public["final_checkpoint"] {
        return Err("public checkpoint".into());
    }
    if flow.disclose(winner, &inputs, &proof, &vk) != Err(state::Rejection::Disclosed)
        || flow.snapshot() != checkpoint
    {
        return Err("public repeat".into());
    }
    write(
        out,
        &json!({"status":"PASS_PUBLIC_WINNER_REPLAY_WITHOUT_PRIVATE_DATA","source_fixture_sha256":hex(&sha256(&fs::read(p)?)),"batches":steps,
        "public_winner":winner,"winner_write_count":1,"p3_verify_and_write_ms":verify_ms,"all_60_inputs_rebuilt":true,
        "private_dkg_or_cp_or_tally_loaded":false,"new_setup":0,"new_proofs":0,"evm_executed":false}),
        false,
    )?;
    println!("PASS public Winner replay: only public material, winner written once");
    Ok(())
}
fn replay_full(p: &Path, public_path: &Path, out: &Path) -> Result<()> {
    let original = original()?;
    let public = read(public_path)?;
    if public["public_context"] != original["public_context"] {
        return Err("diagnostic target".into());
    }
    let full = read(p)?;
    let t = Instant::now();
    let tally =
        packages::public_full_tally(&full, &anchors(&original)?, &public["public_context"])?;
    let verify_ms = ms(t);
    if vv_types::winner(&tally, 32)? as u64 != public["public_winner"].as_u64().ok_or("winner")? {
        return Err("different result".into());
    }
    let mut wrong = full.clone();
    wrong["tally"][0] = json!(tally[0] + 1);
    if packages::public_full_tally(&wrong, &anchors(&original)?, &public["public_context"]).is_ok()
    {
        return Err("wrong claimed public tally accepted".into());
    }
    write(
        out,
        &json!({"status":"PASS_PUBLIC_FULL_TALLY_DIAGNOSTIC_VERIFICATION","same_frozen_target":true,"same_winner":true,"claimed_wrong_tally_rejected":true,
        "usable_decryption_packages_public_in_this_counterfactual":true,"authorizes_actual_winner_only_release":false,
        "verification_ms":verify_ms,"full_payload_file_bytes":fs::metadata(p)?.len(),"full_payload_sha256":hex(&sha256(&fs::read(p)?)),
        "no_secret_keys_loaded":true,"tally_and_usable_shares_omitted_from_this_report":true}),
        false,
    )?;
    println!(
        "PASS Full Tally public diagnostic verification; private diagnostic payload not published"
    );
    Ok(())
}
fn run(out: &Path, private: &Path) -> Result<()> {
    if out.exists() || private.exists() {
        return Err("new separate output directories required".into());
    }
    if out.starts_with(private) || private.starts_with(out) {
        return Err("public and diagnostic branches must be disjoint".into());
    }
    let started = Instant::now();
    let started_utc = helpers::metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let source_base = helpers::metadata("git", &["rev-parse", "HEAD"])?;
    let source_files = [
        file!(),
        "crates/vv-friendly-dkg/src/bin/common/g10c_packages.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_materials.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_batches.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_state.rs",
        "crates/vv-friendly-dkg/src/bin/common/g2e_receiver.rs",
        "crates/vv-friendly-dkg/src/package.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-crypto/src/cp.rs",
        "crates/vv-friendly-crypto/src/p3ten_gadget.rs",
        "Cargo.toml",
        "Cargo.lock",
        "crates/vv-friendly-dkg/Cargo.toml",
    ];
    let sources = source_files
        .iter()
        .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
        .collect::<Result<Vec<_>>>()?;
    let original = original()?;
    let material = materials::load_sources()?;
    println!(
        "FLOW 1/7: authenticated DKG and registration reconstruction; no protocol/curve change"
    );
    let (dkg, _keys, registration, binding) = materials::rebuild_dkg(&material)?;
    let auth = anchors(&original)?;
    for id in 1u32..=5 {
        if auth[id as usize - 1]
            != (
                dkg.config.authentication_public_keys[id as usize - 1],
                *dkg.verification_share(id)?,
            )
        {
            return Err("certified public anchors".into());
        }
    }
    let verifier = batches::BatchVerifier::load(&material)?;
    let mut flow = make_flow(&verifier, binding.clone(), auth.clone())?;
    println!("FLOW 2/7: both live B4 SnarkPack/state verification -> CT1 -> CT2 -> one freeze");
    let batch_steps = receive(&mut flow, &verifier)?;
    let frozen = flow.freeze().map_err(|e| format!("freeze {e:?}"))?;
    if original["freeze_digest_hex"] != hex(&frozen.digest)
        || original["public_context"] != frozen.context
    {
        return Err("same original CT2/freeze".into());
    }
    println!("FLOW 3/7: five OS Trustee workers, private CP/package delivery over pipes");
    let t = Instant::now();
    let (wires, worker_evidence) = packages::deliver(&dkg, &frozen.context)?;
    let mut trustees = Vec::new();
    let mut verified = Vec::new();
    let mut roster = Vec::new();
    for (i, wire) in wires.iter().enumerate() {
        let p = packages::check(
            &frozen.context,
            &frozen.ciphertexts,
            wire,
            &auth[i].0,
            &auth[i].1,
        )?;
        trustees.push(p.trustee);
        verified.push(p.verified);
        roster.push(p.row);
    }
    let packages_ms = ms(t);
    flow.install_roster(roster.clone())
        .map_err(|e| format!("commitment install {e:?}"))?;
    let m = materials::Materials {
        trustees,
        verified,
        roster: roster.clone(),
        context: frozen.context.clone(),
    };
    let t = Instant::now();
    let circuit = materials::build_case(0, &dkg, &registration, &frozen.ciphertexts, &m)?;
    let private_decode_ms = ms(t);
    let winner = circuit.public_winner;
    let inputs = circuit.public_inputs()?;
    if inputs.len() != G4D_INPUTS
        || inputs
            != materials::expected_inputs(&frozen.context, &roster, &frozen.ciphertexts, 0, winner)?
    {
        return Err("new P3 live-input binding".into());
    }
    println!("FLOW 4/7: real private threshold decode complete; rebuild same G4-D Groth16 PK");
    let t = Instant::now();
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut ChaCha20Rng::seed_from_u64(0x4734_442D_5345_5455),
    )?;
    let setup_ms = ms(t);
    let vk_bytes = compressed(&params.vk)?;
    if hex(&sha256(&vk_bytes)) != VK_SHA || hex(&vk_bytes) != original["verifying_key_hex"] {
        return Err("P3 VK drift".into());
    }
    println!("FLOW 5/7: generate ONE new real P3 for this run's five-process packages; setup {setup_ms:.3} ms");
    if helpers::peak_rss_mib() > 16384. {
        return Err("setup resource limit".into());
    }
    let t = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit.clone(),
        &params,
        &mut ChaCha20Rng::seed_from_u64(0x4731_3043_5052_4F4F),
    )?;
    let prove_ms = ms(t);
    let proof_bytes = compressed(&proof)?;
    println!("FLOW 6/7: verify frozen binding, wrong Winner rejection, then one-time Winner write");
    let before = flow.snapshot();
    let mut changed = frozen.ciphertexts.clone();
    changed[0].c1 += generator();
    let changed_ctx = state::context_for(&frozen.context, &changed)?;
    let wrong_target = materials::expected_inputs(&changed_ctx, &roster, &changed, 0, winner)?;
    if flow.disclose(winner, &wrong_target, &proof, &params.vk) != Err(state::Rejection::Target)
        || flow.snapshot() != before
    {
        return Err("target rejection".into());
    }
    let wrong_winner = winner % 3 + 1;
    let wrong_inputs = materials::expected_inputs(
        &frozen.context,
        &roster,
        &frozen.ciphertexts,
        0,
        wrong_winner,
    )?;
    if flow.disclose(wrong_winner, &wrong_inputs, &proof, &params.vk)
        != Err(state::Rejection::Proof)
        || flow.snapshot() != before
    {
        return Err("wrong Winner must fail actual Groth16".into());
    }
    let t = Instant::now();
    flow.disclose(winner, &inputs, &proof, &params.vk)
        .map_err(|e| format!("disclose {e:?}"))?;
    let disclose_ms = ms(t);
    let final_checkpoint = flow.snapshot();
    if flow.disclose(winner, &inputs, &proof, &params.vk) != Err(state::Rejection::Disclosed)
        || flow.snapshot() != final_checkpoint
    {
        return Err("one-time disclosure".into());
    }
    let public = json!({"schema":"VV/G6/NATIVE-WINNER-ONLY/PUBLIC/v1","protocol_version":"VERIVOTING-JUBJUB-EXPERIMENT-G6-NATIVE-WINNER-ONLY-v1",
        "relation_id":G4D_RELATION_ID,"input_layout":G4D_LAYOUT,"source_g2e_fixture":material.config["g2e_fixture"],"source_g2e_fixture_sha256":material.config["g2e_fixture_sha256"],
        "verifying_key_hex":hex(&vk_bytes),"verifying_key_sha256":VK_SHA,"freeze_digest_hex":hex(&frozen.digest),"public_context":frozen.context,
        "trustee_roster":roster,"subset":[1,2,3],"public_winner":winner,"proof_hex":hex(&proof_bytes),"proof_sha256":hex(&sha256(&proof_bytes)),
        "public_inputs_le_hex":input_hex(&inputs)?,"final_checkpoint":final_checkpoint,"private_tally_or_shares_emitted":false,"production_setup":false});
    let public_path = out.join("winner_only/public.json");
    let public_bytes = write(&public_path, &public, false)?;
    let evm = Eip2537PublicGroth16Proof::from_proof(&params.vk, &proof, &inputs)?;
    write(
        &out.join("winner_only/evm.json"),
        &serde_json::to_value(&evm)?,
        false,
    )?;
    println!("FLOW 7/7: counterfactual Full Tally verification branch; keep usable shares outside public archive");
    let full = json!({"mode":"COUNTERFACTUAL_PUBLIC_DECRYPTION_BASELINE","authenticated_winner_only_policy_authorizes_this_release":false,
        "scope":"diagnostic baseline only, not a permitted transition of the certified Winner-only election", "frozen_context":frozen.context,
        "released_packages":&wires[..3],"tally":circuit.tallies,"winner":winner});
    let full_path = private.join("full_tally.json");
    let full_bytes = write(&full_path, &full, true)?;
    replay_winner(&public_path, &out.join("winner_public_replay.json"))?;
    replay_full(
        &full_path,
        &public_path,
        &out.join("full_tally_public_verification.json"),
    )?;
    for s in &sources {
        if s["sha256"]
            != hex(&sha256(&fs::read(
                s["path"].as_str().ok_or("source path")?,
            )?))
        {
            return Err("source changed during execution".into());
        }
    }
    if helpers::peak_rss_mib() > 16384. || started.elapsed().as_secs() > 600 {
        return Err("functional integration resource guard".into());
    }
    write(
        &out.join("report.json"),
        &json!({"status":"PASS_CONNECTED_NATIVE_FIVE_WORKER_NEW_P3_WINNER_ONCE_AND_FULL_TALLY_DIAGNOSTIC",
        "started_utc":started_utc,"source_base_sha":source_base,"source_was_dirty":true,"sources":sources,"source_version_sha256":hex(&sha256(&serde_json::to_vec(&sources)?)),
        "binary_sha256":hex(&sha256(&fs::read(std::env::current_exe()?)?)),"certified_dkg_and_registration":binding,"batches":batch_steps,
        "same_ct2_and_freeze_as_g6":true,"worker_evidence":worker_evidence,"five_cp_and_commitment_openings_verified":true,"packages_and_delivery_ms":packages_ms,
        "private_threshold_decode_ms":private_decode_ms,"new_groth16_setup_rebuilds":1,"new_p3_proofs":1,"setup_rebuild_ms":setup_ms,"p3_prove_ms":prove_ms,
        "p3_vk_sha256":VK_SHA,"p3_proof_sha256":hex(&sha256(&proof_bytes)),"winner_disclose_ms":disclose_ms,"public_winner":winner,"winner_writes":1,
        "necessary_negatives":{"changed_frozen_target_rejected_atomically":true,"wrong_winner_actual_groth16_rejected_atomically":true,"repeat_disclosure_rejected_atomically":true,"full_tally_wrong_claim_rejected":true},
        "release_comparison":{"same_ct2_same_dkg_same_selected_shares":true,"winner_only_fixture_json_bytes":public_bytes,"full_tally_diagnostic_json_bytes":full_bytes,
        "winner_proof_bytes":proof_bytes.len(),"winner_public_inputs_bytes":inputs.len()*32,"cached_p3_vk_bytes":vk_bytes.len(),
        "full_tally_payload_not_in_public_archive":true,"full_tally_policy_authorized":false,"full_tally_is_counterfactual_baseline":true,"privacy_theorem_proved":false},
        "reused_ballot_proofs":8,"reused_aggregates":2,"reused_state_proofs":2,"new_ballot_proofs":0,"new_state_proofs":0,"new_cp_packages":5,
        "new_p3_proof_from_current_private_witness":true,"not_json_concatenation":true,"all_60_inputs_from_live_frozen_state":true,
        "process_peak_rss_mib":helpers::peak_rss_mib(),"total_wall_ms":ms(started),"resource_budget":{"wall_seconds":600,"rss_mib":16384},
        "production_setup":false,"operational_privacy":false,"independent_distributed_dkg":false,"exact_snarkpack_evm":false,"full_target_architecture_complete":false,"gas":null,
        "limitations":["same-host workers receive shares from centralized deterministic research DKG bootstrap; not independent trustee key custody",
        "public research seeds invalidate operational secrecy; private pipe isolation is transport evidence, not a privacy theorem",
        "Full Tally is a local counterfactual diagnostic, not authorized by the Winner-only certificate and not published with Winner fixture",
        "native live state machine is in-memory; exact SnarkPack EVM and production deployment remain incomplete"]}),
        false,
    )?;
    println!("PASS CONNECTED NATIVE: five workers, one NEW P3, winner={winner} written once; Full Tally diagnostic verified separately");
    Ok(())
}
fn main() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    match a.as_slice(){
        [worker] if worker=="--trustee-worker"=>packages::worker(),
        [mode,p,flag,o] if mode=="--verify-winner"&&flag=="--output"=>replay_winner(Path::new(p),Path::new(o)),
        [mode,p,flag,w,of,o] if mode=="--verify-full"&&flag=="--winner-fixture"&&of=="--output"=>replay_full(Path::new(p),Path::new(w),Path::new(o)),
        [flag,o,pf,p] if flag=="--output-dir"&&pf=="--private-dir"=>run(Path::new(o),Path::new(p)),
        _=>Err("usage: g10c_complete_native --output-dir NEW_PUBLIC --private-dir NEW_DIAGNOSTIC (or --verify-winner/--verify-full)".into()),
    }
}
