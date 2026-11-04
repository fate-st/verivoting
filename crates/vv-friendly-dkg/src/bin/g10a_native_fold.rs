//! G10-A: matched full native receive paths on the same two archived W32 B4 batches.
//! No setup, proving, aggregation, EVM or changes to historical receivers/relations.
#![recursion_limit = "512"]
#[allow(dead_code)]
#[path = "common/g1e_fold.rs"]
mod fold;
#[allow(dead_code)]
#[path = "common/native_batch.rs"]
mod helpers;
#[path = "common/g2e_receiver.rs"]
mod receiver;
use ark_bls12_381::Bls12_381;
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::{
    aggregate::{verify_aggregate_proof, AggregateProof, AggregateVersion, VerifierSRS},
    prepare_verifying_key as prepare_ballot_key, PreparedVerifyingKey as BallotKey, VerifyingKey,
};
use blstrs::Bls12;
use helpers::{metadata, peak_rss_mib, write_new, Result};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{fs, hint::black_box, io::Cursor, path::Path, time::Instant};
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs, state_public_inputs, StateBallotRecord,
    StateTransitionCircuitForBatch as Circuit, STATE_B4_LAYOUT_VERSION,
};
use vv_friendly_crypto::{decode_point, encode_point, generator, Ciphertext};
use vv_types::{hex, sha256};
const FIXTURE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";
const FIXTURE_SHA: &str = "42e68fa9bee72fb77e47c292dd3e65386cf4a348192c1ee0a9731cfe2f5eff19";
const POOL: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const POOL_SHA: &str = "6b9a1ca042e3ab1d637fb3b554029cc7a36a5b867430b2707539d1ea61928596";
const MANIFEST: &str = "configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json";
const MANIFEST_SHA: &str = "00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e";
const ROUNDS: usize = 10;
const REPS: usize = 8;
const SEED: u64 = 0x4731_3041_464F_4C44;

fn bytes(v: &Value) -> Result<Vec<u8>> {
    let s = v.as_str().ok_or("missing hex string")?;
    if !s.len().is_multiple_of(2) {
        return Err("odd hex".into());
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
        .collect()
}
fn fixed(v: &Value, key: &str) -> Result<[u8; 32]> {
    bytes(&v[key])?
        .try_into()
        .map_err(|_| "wrong 32-byte encoding".into())
}
fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(b: &[u8]) -> Result<T> {
    let mut rest = b;
    let v = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || helpers::compressed(&v)? != b {
        return Err("noncanonical Ark encoding".into());
    }
    Ok(v)
}
fn states(v: &Value) -> Result<Vec<Ciphertext>> {
    let a = v.as_array().ok_or("ciphertext array")?;
    if a.len() != 3 {
        return Err("three candidates required".into());
    }
    a.iter()
        .enumerate()
        .map(|(j, c)| {
            if c["candidate_id"] != j + 1 {
                return Err("candidate order".into());
            }
            Ok(Ciphertext {
                c0: decode_point(&fixed(c, "c0_hex")?, true)?,
                c1: decode_point(&fixed(c, "c1_hex")?, true)?,
            })
        })
        .collect()
}
fn record(v: &Value) -> Result<StateBallotRecord> {
    Ok(StateBallotRecord {
        election_id: fixed(v, "election_id_hex")?,
        final_context: fixed(v, "final_context_hex")?,
        registration_root: fixed(v, "registration_root_hex")?,
        key_set_hash: fixed(v, "key_set_hash_hex")?,
        nullifier: fixed(v, "nullifier_hex")?,
        region: v["region"].as_u64().ok_or("region")?,
        public_key: decode_point(&fixed(v, "public_key_hex")?, false)?,
        ciphertexts: states(&v["ciphertexts"])?,
    })
}
struct Batch {
    circuit: Circuit<4>,
    aggregate: AggregateProof<Bls12>,
    proof: ark_groth16::Proof<Bls12_381>,
}
struct Loaded {
    batches: Vec<Batch>,
    ballot_key: BallotKey<Bls12>,
    state_key: PreparedVerifyingKey<Bls12_381>,
    srs: VerifierSRS<Bls12>,
    pins: Value,
    communication: Value,
}
fn load() -> Result<Loaded> {
    let mut pins = Vec::new();
    for (p, h) in [
        (FIXTURE, FIXTURE_SHA),
        (POOL, POOL_SHA),
        (MANIFEST, MANIFEST_SHA),
    ] {
        let actual = hex(&sha256(&fs::read(p)?));
        if actual != h {
            return Err(format!("source hash mismatch: {p}").into());
        }
        pins.push(json!({"path":p,"sha256":actual}));
    }
    let f: Value = serde_json::from_slice(&fs::read(FIXTURE)?)?;
    let pool: Value = serde_json::from_slice(&fs::read(POOL)?)?;
    if f["schema"] != "VV/G2-E/MULTIBATCH-B4-PUBLIC-FIXTURE/v1"
        || f["state_layout"] != STATE_B4_LAYOUT_VERSION
        || f["source_ballot_vk_hex"] != pool["groth16_vk_hex"]
        || f["batches"].as_array().ok_or("batches")?.len() != 2
    {
        return Err("wrong W32 state fixture".into());
    }
    let vk_bytes = bytes(&f["source_ballot_vk_hex"])?;
    let mut cursor = Cursor::new(&vk_bytes);
    let ballot_vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    let mut wire = Vec::new();
    ballot_vk.write(&mut wire)?;
    if cursor.position() as usize != vk_bytes.len() || wire != vk_bytes || ballot_vk.ic.len() != 26
    {
        return Err("ballot VK encoding/arity".into());
    }
    let state_bytes = bytes(&f["state_vk_hex"])?;
    if f["state_vk_sha256"] != hex(&sha256(&state_bytes)) {
        return Err("state VK hash".into());
    }
    let state_vk: ark_groth16::VerifyingKey<Bls12_381> = canonical(&state_bytes)?;
    if state_vk.gamma_abc_g1.len() != 38 {
        return Err("state VK arity".into());
    }
    let v = &f["source_verifier_srs"];
    let srs = VerifierSRS {
        n: v["n"].as_u64().ok_or("SRS n")? as usize,
        g: serde_json::from_value(v["g"].clone())?,
        h: serde_json::from_value(v["h"].clone())?,
        g_alpha: serde_json::from_value(v["g_alpha"].clone())?,
        g_beta: serde_json::from_value(v["g_beta"].clone())?,
        h_alpha: serde_json::from_value(v["h_alpha"].clone())?,
        h_beta: serde_json::from_value(v["h_beta"].clone())?,
        h_alpha_d: serde_json::from_value(v["h_alpha_d"].clone())?,
    };
    if srs.n != 4 {
        return Err("B4 SRS required".into());
    }
    let mut batches = Vec::new();
    let mut old = vec![Ciphertext::zero(); 3];
    let mut communication = Vec::new();
    for (i, b) in f["batches"].as_array().ok_or("batches")?.iter().enumerate() {
        let rows = pool["source_public_records"]
            .as_array()
            .ok_or("pool records")?;
        if b["source_public_records"] != json!(&rows[i * 4..i * 4 + 4])
            || states(&b["old_state"])? != old
            || b["batch_index"] != i + 1
        {
            return Err("not exact continuous D2 prefix".into());
        }
        let records = b["source_public_records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .map(record)
            .collect::<Result<Vec<_>>>()?;
        let binding = &records[0];
        let circuit = Circuit::<4> {
            election_id: binding.election_id,
            final_context: binding.final_context,
            region: binding.region,
            key_set_hash: binding.key_set_hash,
            public_key: binding.public_key,
            batch_index: (i + 1) as u64,
            batch_size: 4,
            batch_root: fixed(b, "batch_root_hex")?,
            old_state: old,
            new_state: states(&b["new_state"])?,
            records,
        };
        let packed = circuit
            .records
            .iter()
            .map(|r| helpers::convert_inputs(&ballot_public_inputs(r).ok_or("ballot packing")?))
            .collect::<Result<Vec<_>>>()?;
        if json!(packed
            .iter()
            .map(|r| r.iter().map(|x| hex(&x.to_bytes_le())).collect::<Vec<_>>())
            .collect::<Vec<_>>())
            != b["source_public_inputs_le_hex"]
            || hex(&helpers::transcript(&packed)) != b["source_external_transcript_hex"]
        {
            return Err("record/matrix/transcript binding".into());
        }
        let input = state_public_inputs(&circuit).ok_or("state packing")?;
        if json!(input
            .iter()
            .map(|x| Ok(hex(&helpers::compressed(x)?)))
            .collect::<Result<Vec<_>>>()?)
            != b["state_public_inputs_le_hex"]
        {
            return Err("state public inputs".into());
        }
        let wire = bytes(&b["source_aggregate_hex"])?;
        let mut cursor = Cursor::new(&wire);
        let aggregate = AggregateProof::<Bls12>::read(&mut cursor)?;
        aggregate.parsing_check()?;
        let mut back = Vec::new();
        aggregate.write(&mut back)?;
        if back != wire
            || cursor.position() as usize != wire.len()
            || aggregate.tmipp.gipa.nproofs != 4
            || b["source_aggregate_sha256"] != hex(&sha256(&wire))
        {
            return Err("aggregate canonical bytes/hash".into());
        }
        let proof_bytes = bytes(&b["state_proof_hex"])?;
        if b["state_proof_sha256"] != hex(&sha256(&proof_bytes)) {
            return Err("state proof hash".into());
        }
        communication.push(json!({"batch_index":i+1,"aggregate_bytes":wire.len(),"matrix_bytes":4*25*32,
            "old_and_new_ct_bytes":2*3*2*32,"state_statement_bytes":37*32,"state_proof_bytes":proof_bytes.len(),
            "state_statement_is_derived_not_added_to_matrix_and_CT":true,"full_network_or_calldata_bytes":null}));
        let proof = canonical(&proof_bytes)?;
        old = circuit.new_state.clone();
        batches.push(Batch {
            circuit,
            aggregate,
            proof,
        });
    }
    if states(&f["final_state"])? != old {
        return Err("final CT2 mismatch".into());
    }
    pins.push(json!({"ballot_vk_sha256":hex(&sha256(&vk_bytes)),"state_vk_sha256":hex(&sha256(&state_bytes)),
        "verifier_srs_json_sha256":hex(&sha256(&serde_json::to_vec(v)?))}));
    Ok(Loaded {
        batches,
        ballot_key: prepare_ballot_key(&ballot_vk),
        state_key: prepare_verifying_key(&state_vk),
        srs,
        pins: json!(pins),
        communication: json!(communication),
    })
}
#[derive(Clone, Copy)]
enum Route {
    Direct,
    State,
}
impl Route {
    fn name(self) -> &'static str {
        match self {
            Self::Direct => "snarkpack_direct_fold",
            Self::State => "snarkpack_state_proof",
        }
    }
}
#[derive(Default)]
struct Parts {
    packing_ms: f64,
    sp_ms: f64,
    extra_ms: f64,
    sp_ok: bool,
    extra_ok: bool,
}
impl Loaded {
    fn receiver(&self) -> receiver::Receiver {
        receiver::Receiver::new(
            self.batches[0].circuit.records[0].clone(),
            self.batches.iter().map(|b| b.circuit.batch_root).collect(),
        )
    }
    fn verify(
        &self,
        c: &Circuit<4>,
        b: &Batch,
        route: Route,
        nonce: u64,
        m: &mut Parts,
    ) -> Result<bool> {
        let t = Instant::now();
        // Rebuild from the actual submitted records, not a cached positive statement.
        let matrix = c
            .records
            .iter()
            .map(|r| helpers::convert_inputs(&ballot_public_inputs(r).ok_or("ballot packing")?))
            .collect::<Result<Vec<_>>>()?;
        let transcript = helpers::transcript(&matrix);
        m.packing_ms = t.elapsed().as_secs_f64() * 1000.;
        let t = Instant::now();
        m.sp_ok = verify_aggregate_proof(
            &self.srs,
            &self.ballot_key,
            ChaCha20Rng::seed_from_u64(SEED ^ nonce ^ c.batch_index),
            &matrix,
            &b.aggregate,
            &transcript,
            AggregateVersion::V2,
        )?;
        m.sp_ms = t.elapsed().as_secs_f64() * 1000.;
        if !m.sp_ok {
            return Ok(false);
        }
        let t = Instant::now();
        m.extra_ok = match route {
            Route::Direct => {
                fold::reference(&c.old_state, &c.records).ok_or("fold shape")? == c.new_state
            }
            Route::State => Groth16::<Bls12_381>::verify_proof(
                &self.state_key,
                &b.proof,
                &state_public_inputs(c).ok_or("state packing")?,
            )?,
        };
        m.extra_ms = t.elapsed().as_secs_f64() * 1000.;
        Ok(m.extra_ok)
    }
    fn flow(&self, route: Route, nonce: u64) -> Result<Parts> {
        let mut r = self.receiver();
        let mut sum = Parts::default();
        for b in &self.batches {
            let mut m = Parts::default();
            r.apply(&b.circuit, || {
                self.verify(&b.circuit, b, route, nonce, &mut m)
            })
            .map_err(|e| format!("receive rejected: {e:?}"))?;
            sum.packing_ms += m.packing_ms;
            sum.sp_ms += m.sp_ms;
            sum.extra_ms += m.extra_ms;
        }
        if r.state() != self.batches[1].circuit.new_state || r.next() != 3 || r.spent_count() != 8 {
            return Err("final checkpoint".into());
        }
        Ok(sum)
    }
    fn negative(&self, route: Route) -> Result<Value> {
        let b = &self.batches[0];
        let mut bad = b.circuit.clone();
        bad.new_state[0].c1 += generator();
        let mut r = self.receiver();
        let mut m = Parts::default();
        let rejected = r.apply(&bad, || self.verify(&bad, b, route, 0, &mut m));
        if rejected != Err(receiver::Rejection::ProofVerification)
            || !m.sp_ok
            || m.extra_ok
            || r.next() != 1
            || r.spent_count() != 0
            || r.state() != b.circuit.old_state
        {
            return Err("negative did not reach the intended atomic rejection".into());
        }
        Ok(
            json!({"route":route.name(),"case":"changed_first_new_state_candidate1_c1_before_acceptance","snarkpack_verified":m.sp_ok,
            "route_specific_check_rejected":!m.extra_ok,"rejection":"ProofVerification","checkpoint_unchanged":true,"frozen_short_circuit":false}),
        )
    }
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let output = match args.as_slice() {
        [] => "experiments/g10_a_raw/native_direct_fold_b4.json",
        [flag, p] if flag == "--output" => p,
        _ => return Err("usage: friendly-g10a-native-fold [--output NEW_PATH]".into()),
    };
    if Path::new(output).exists() {
        return Err("refusing to overwrite an archived run".into());
    }
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let started = Instant::now();
    let load_started = Instant::now();
    let data = load()?;
    let load_ms = load_started.elapsed().as_secs_f64() * 1000.;
    let negatives = [data.negative(Route::Direct)?, data.negative(Route::State)?];
    for route in [Route::Direct, Route::State] {
        for i in 0..2 {
            black_box(data.flow(route, 0x1000 + i)?);
        }
    }
    let mut windows = Vec::new();
    for round in 0..ROUNDS {
        let order = if round % 2 == 0 {
            [Route::Direct, Route::State]
        } else {
            [Route::State, Route::Direct]
        };
        for (position, route) in order.into_iter().enumerate() {
            let t = Instant::now();
            let mut parts = Parts::default();
            for rep in 0..REPS {
                let p = black_box(data.flow(route, ((round * REPS + rep) as u64) << 8)?);
                parts.packing_ms += p.packing_ms;
                parts.sp_ms += p.sp_ms;
                parts.extra_ms += p.extra_ms;
            }
            let window_ms = t.elapsed().as_secs_f64() * 1000.;
            let flow_ms = window_ms / REPS as f64;
            windows.push(json!({"round":round+1,"position":position+1,"route":route.name(),"repetitions":REPS,
                "window_ms":window_ms,"two_batch_flow_ms":flow_ms,"per_batch_ms":flow_ms/2.,"per_vote_ms":flow_ms/8.,
                "votes_per_second":8000./flow_ms,"packing_ms_per_flow":parts.packing_ms/REPS as f64,
                "snarkpack_ms_per_flow":parts.sp_ms/REPS as f64,"route_specific_ms_per_flow":parts.extra_ms/REPS as f64}));
            println!(
                "round {:02} {}: two B4 receive/apply {:.6} ms",
                round + 1,
                route.name(),
                flow_ms
            );
            if started.elapsed().as_secs() > 120 || peak_rss_mib() > 2048. {
                return Err("bounded G10-A runtime/RSS guard".into());
            }
        }
    }
    let mut sources = Vec::new();
    for p in [
        file!(),
        "crates/vv-friendly-dkg/src/bin/common/g2e_receiver.rs",
        "crates/vv-friendly-dkg/src/bin/common/g1e_fold.rs",
        "crates/vv-friendly-dkg/src/bin/common/native_batch.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "Cargo.lock",
    ] {
        sources.push(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))}));
    }
    let exe = std::env::current_exe()?;
    let report = json!({"schema":"VV/G10-A/NATIVE-MATCHED-B4/v1","status":"PASS_MATCHED_TWO_B4_NATIVE_RECEIVE_PATHS",
        "started_utc":started_utc,"source_base_sha":metadata("git",&["rev-parse","HEAD"] )?,
        "git_dirty":!metadata("git",&["status","--porcelain"] )?.is_empty(),"sources":sources,"fixture_pins":data.pins,
        "binary_sha256":hex(&sha256(&fs::read(exe)?)),"rustc":metadata("rustc",&["+1.99.1","-Vv"] )?,
        "hardware":metadata("sysctl",&["-n","machdep.cpu.brand_string"] )?,"hardware_memory_bytes":metadata("sysctl",&["-n","hw.memsize"] )?,
        "build":"release; locked; offline; CARGO_BUILD_JOBS=2; toolchain 1.99.1","rayon_num_threads":std::env::var("RAYON_NUM_THREADS").ok(),
        "parameters":{"merkle_depth":5,"registered_weight":32,"batch_size":4,"consecutive_batches":2,"distinct_ballots":8},
        "method":{"paired_rounds":ROUNDS,"repetitions_per_window":REPS,"warmup_flows_per_route":2,"alternating_AB_BA":true,
            "same_rng_seed_per_matched_flow":true,"deterministic_research_rng_not_production_randomness":true,
            "timed":"receiver allocation, preflight including exact root/context/NF/old-state checks, statement/transcript rebuilding, real SnarkPack, direct fold or real state verification, checkpoint writes, final CT2 equality",
            "offline_excluded":"file parsing/canonical decoding, VK preparation; setup/proving/aggregation not rerun",
            "fixed_proof_pool_not_independent_elections":true,"no_outlier_filter":true,"total_time_not_component_sum":true},
        "windows":windows,"necessary_negatives":negatives,"both_paths_final_ct2_equal":true,
        "final_state":data.batches[1].circuit.new_state.iter().map(|c| json!({"c0_hex":hex(&encode_point(&c.c0)),"c1_hex":hex(&encode_point(&c.c1))})).collect::<Vec<_>>(),
        "load_and_prepare_ms":load_ms,"process_peak_rss_mib":peak_rss_mib(),"rss_scope":"shared cumulative process, including both paths; not per-route memory comparison",
        "communication_components":data.communication,"setup_count":0,"proof_generation_count":0,"aggregate_generation_count":0,
        "reused_state_proofs":2,"reused_snarkpack_proofs":2,"setup_ms":null,"prove_ms":null,"aggregation_ms":null,
        "generation_cost_status":"REUSED_NOT_RETIMED_NOT_ZERO","gas":null,"calldata":null,"evm_executed":false,
        "full_winner_only_or_full_tally_comparison":false,"exact_snarkpack_evm_verified":false,"complete_privacy_proof":false,
        "resource_budget":{"wall_seconds":120,"rss_mib":2048,"runs":1}});
    write_new(Path::new(output), &report)?;
    println!(
        "PASS: matched native Direct Fold/State Proof; {} windows; no Setup/Prove/EVM",
        ROUNDS * 2
    );
    Ok(())
}
