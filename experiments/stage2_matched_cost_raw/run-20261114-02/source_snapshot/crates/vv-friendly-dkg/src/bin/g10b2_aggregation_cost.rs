//! G10-B2: only the two existing B4 aggregation-generation costs, not new Groth16 proofs.
//! Cargo auto-discovers this bin; do not change the manifest pinned by G10-B1.
#![recursion_limit = "512"]
#[allow(dead_code)]
#[path = "common/native_batch.rs"]
mod helpers;
#[allow(dead_code)]
#[path = "common/g10b_intake.rs"]
mod intake;
#[allow(dead_code)]
#[path = "common/g10b_materials.rs"]
mod materials;
use bellperson::groth16::aggregate::{
    aggregate_proofs, setup_fake_srs, verify_aggregate_proof, AggregateVersion,
};
use blstrs::Bls12;
use helpers::{metadata, peak_rss_mib, write_new, Result};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{fs, hint::black_box, path::Path, time::Instant};
use vv_friendly_crypto::state_gadget::ballot_public_inputs;
use vv_types::{hex, sha256};
const SRS_SEED: u64 = 0x4731_4430_5352_5320;
const ROUNDS: usize = 12;
const SOURCES: &[&str] = &[
    file!(),
    "crates/vv-friendly-dkg/src/bin/common/g10b_materials.rs",
    "crates/vv-friendly-dkg/src/bin/common/g10b_intake.rs",
    "crates/vv-friendly-dkg/src/bin/common/native_batch.rs",
    "crates/vv-friendly-crypto/src/state_gadget.rs",
    "crates/vv-friendly-dkg/Cargo.toml",
    "Cargo.toml",
    "Cargo.lock",
];
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let output = match args.as_slice() {
        [flag, p] if flag == "--output" => p,
        _ => return Err("usage: g10b2_aggregation_cost --output NEW_PATH".into()),
    };
    if Path::new(output).exists() {
        return Err("refusing archive overwrite".into());
    }
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let started = Instant::now();
    let load_t = Instant::now();
    let data = materials::load()?;
    let load_ms = ms(load_t);
    let fixture: Value = serde_json::from_slice(&fs::read(
        "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json",
    )?)?;
    let origin: Value =
        serde_json::from_slice(&fs::read("experiments/g2_e_raw/multibatch_state_b4.json")?)?;
    let t = Instant::now();
    // Read the exact u64 constant from the original Rust design, never through a rounded JSON number.
    let generic = setup_fake_srs::<Bls12, _>(&mut ChaCha20Rng::seed_from_u64(SRS_SEED), 32);
    let srs_rebuild_ms = ms(t);
    let srs_hash = hex(&generic.hash());
    if origin["shared_aggregation_srs_sha256"] != srs_hash {
        return Err("generic research SRS identity".into());
    }
    let t = Instant::now();
    let (prover_srs, verifier_srs) = generic.specialize(4);
    let specialize_ms = ms(t);
    let srs_json = json!({"n":verifier_srs.n,"g":verifier_srs.g,"h":verifier_srs.h,
        "g_alpha":verifier_srs.g_alpha,"g_beta":verifier_srs.g_beta,
        "h_alpha":verifier_srs.h_alpha,"h_beta":verifier_srs.h_beta,"h_alpha_d":verifier_srs.h_alpha_d});
    if srs_json != fixture["source_verifier_srs"] {
        return Err("specialized SRS mismatch".into());
    }
    let t = Instant::now();
    let matrices = data
        .batches
        .iter()
        .map(|b| {
            b.circuit
                .records
                .iter()
                .map(|r| helpers::convert_inputs(&ballot_public_inputs(r).ok_or("ballot packing")?))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let transcripts = matrices
        .iter()
        .map(|m| helpers::transcript(m))
        .collect::<Vec<_>>();
    let packing_ms = ms(t);
    let expected = data
        .batches
        .iter()
        .map(|b| {
            let mut bytes = Vec::new();
            b.aggregate.write(&mut bytes)?;
            Ok(bytes)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut samples = Vec::new();
    let mut generated = 0;
    let mut checks = 0;
    // One warmup per batch, then 12 alternating paired windows. All results retained.
    for round in 0..=ROUNDS {
        let order = if round % 2 == 0 { [0, 1] } else { [1, 0] };
        let flow_started = Instant::now();
        let mut sum = 0.;
        for (position, i) in order.into_iter().enumerate() {
            let t = Instant::now();
            let aggregate = black_box(aggregate_proofs::<Bls12>(
                &prover_srs,
                &transcripts[i],
                &data.batches[i].ballot_proofs,
                AggregateVersion::V2,
            )?);
            let aggregation_ms = ms(t);
            generated += 1;
            sum += aggregation_ms;
            // Serialization/equality and actual verification are outside aggregation timing.
            let t = Instant::now();
            let mut bytes = Vec::new();
            aggregate.write(&mut bytes)?;
            if bytes != expected[i] {
                return Err("new aggregate differs from exact archived proof".into());
            }
            if !verify_aggregate_proof(
                &verifier_srs,
                &data.ballot_key,
                ChaCha20Rng::seed_from_u64(0x4731_3042_3241_4700 ^ ((round * 2 + i) as u64)),
                &matrices[i],
                &aggregate,
                &transcripts[i],
                AggregateVersion::V2,
            )? {
                return Err("generated aggregate rejected".into());
            }
            checks += 1;
            let check_ms = ms(t);
            if round > 0 {
                samples.push(json!({"round":round,"position":position+1,"batch_index":i+1,
                "aggregation_ms":aggregation_ms,"post_generation_serialize_compare_verify_ms":check_ms,
                "aggregate_sha256":hex(&sha256(&bytes)),"aggregate_bytes":bytes.len(),"byte_identical_to_archive":true,"verified":true}));
            }
        }
        if round > 0 {
            println!("round {round:02}: two B4 aggregate generation {sum:.6} ms (checks separate; wall {:.6} ms)",ms(flow_started));
        }
        if started.elapsed().as_secs() > 120 || peak_rss_mib() > 512. {
            return Err("bounded G10-B2 aggregation time/RSS guard".into());
        }
    }
    let sources = SOURCES
        .iter()
        .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
        .collect::<Result<Vec<_>>>()?;
    let pins = [
        "results/g10_b1/summary.json",
        "experiments/g10_b1_raw/matched.json",
        "experiments/g2_e_raw/multibatch_state_b4.json",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect::<Result<Vec<_>>>()?;
    let report = json!({"schema":"VV/G10-B2/AGGREGATION-COST/v1","status":"PASS_EXACT_TWO_B4_AGGREGATION_GENERATION",
        "started_utc":started_utc,"source_base_sha":metadata("git",&["rev-parse","HEAD"] )?,"git_dirty":!metadata("git",&["status","--porcelain"] )?.is_empty(),
        "sources":sources,"source_version_sha256":hex(&sha256(&serde_json::to_vec(&sources)?)),
        "binary_sha256":hex(&sha256(&fs::read(std::env::current_exe()?)?)),"fixture_pins":data.pins,"additional_pins":pins,
        "research_srs_seed_hex":"4731443053525320","generic_srs_sha256":srs_hash,"verifier_srs_json_sha256":hex(&sha256(&serde_json::to_vec(&srs_json)?)),
        "same_srs_and_archived_proofs":true,"research_srs_rebuild_ms_offline":srs_rebuild_ms,"research_srs_specialize_ms_offline":specialize_ms,
        "matrix_and_transcript_pack_ms_offline":packing_ms,"load_and_prepare_ms_offline":load_ms,
        "method":{"paired_rounds":ROUNDS,"warmup_per_batch":1,"aggregations_per_window":1,"alternating_order":true,"proof_pools":1,
            "distinct_ballots":8,"batch_sizes":[4,4],"all_samples_retained":true,"timed":"aggregate_proofs V2 only; serialization/equality/verification excluded",
            "two_batch_sum_is_aggregation_components_not_end_to_end":true},
        "samples":samples,"new_aggregate_count_including_warmup":generated,"generated_aggregate_verifications":checks,
        "new_ballot_proofs":0,"new_state_proofs":0,"new_p3_proofs":0,"new_groth16_setups":0,"research_srs_rebuilds":1,
        "production_setup":false,"proving_key_persisted":false,"prover_srs_persisted":false,"evm_executed":false,"gas":null,"calldata":null,
        "groth16_setup_ms":null,"ballot_prove_ms":null,"state_prove_ms":null,"p3_prove_ms":null,"complete_system_cost":null,
        "process_peak_rss_mib":peak_rss_mib(),"rss_scope":"whole aggregation process including shared archived material loading and research SRS; not per-call allocation or Groth16 prover RSS",
        "rustc":metadata("rustc",&["+1.99.1","-Vv"] )?,"hardware":metadata("sysctl",&["-n","machdep.cpu.brand_string"] )?,
        "hardware_memory_bytes":metadata("sysctl",&["-n","hw.memsize"] )?,"rayon_num_threads":std::env::var("RAYON_NUM_THREADS").ok(),
        "build":"release opt-level1; locked offline; Rust1.99.1; jobs2; strip=none","resource_budget":{"wall_seconds":120,"rss_mib":512,"generation_windows":12},
        "elapsed_ms":ms(started)});
    write_new(Path::new(output), &report)?;
    println!("PASS: 26 exact B4 aggregates checked; zero new Groth16 Setup/Prove/EVM");
    Ok(())
}
