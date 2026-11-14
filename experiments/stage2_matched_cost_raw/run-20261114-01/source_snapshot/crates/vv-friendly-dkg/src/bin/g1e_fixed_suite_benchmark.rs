//! G1-E: operation-level repetitions on the frozen D2 pool. No Groth16 setup/proving.
#[path = "common/g1e_fold.rs"]
mod fold;
#[allow(dead_code)]
#[path = "common/native_batch.rs"]
mod native_batch;
use bellperson::groth16::{
    aggregate::{
        aggregate_proofs, setup_fake_srs, verify_aggregate_proof, AggregateProof, AggregateVersion,
        GenericSRS, ProverSRS, VerifierSRS,
    },
    prepare_verifying_key, verify_proof, PreparedVerifyingKey, Proof, VerifyingKey,
};
use blstrs::{Bls12, Scalar};
use native_batch::{metadata, peak_rss_mib, transcript, write_new, Result};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    hint::black_box,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use vv_friendly_crypto::{decode_point, state_gadget::StateBallotRecord, Ciphertext};
use vv_types::{hex, sha256};
const RAW: &str = "experiments/g1_d2_raw/uniform_depth5_batch32.json";
const FIXTURE: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const RAW_SHA: &str = "1a8c11c29ec89b8cf4a58dc963d83f408cd6f3fef6ef28a91159bb6985952372";
const FIXTURE_SHA: &str = "6b9a1ca042e3ab1d637fb3b554029cc7a36a5b867430b2707539d1ea61928596";
const MANIFEST: &str = "configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json";
const MANIFEST_SHA: &str = "00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e";
const BATCHES: [usize; 5] = [2, 4, 8, 16, 32];
const SEED: u64 = 20261030;
const SRS_SEED: u64 = 0x4731_4430_5352_5320;
const TARGET_MS: f64 = 150.0;
const OPS: [&str; 3] = ["individual_verify", "aggregation", "aggregate_verify"];
fn bytes(s: &str) -> Result<Vec<u8>> {
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
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().ok_or_else(|| format!("missing {k}").into())
}
fn digest(v: &Value, k: &str) -> Result<[u8; 32]> {
    bytes(text(v, k)?)?
        .try_into()
        .map_err(|_| "wrong digest width".into())
}
fn check_hash(path: &str, expected: &str) -> Result<()> {
    if hex(&sha256(&fs::read(path)?)) != expected {
        return Err(format!("hash mismatch: {path}").into());
    }
    Ok(())
}
fn shuffled<T>(values: &mut [T], rng: &mut ChaCha20Rng) {
    for i in (1..values.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        values.swap(i, j);
    }
}
struct Pool {
    proofs: Vec<Proof<Bls12>>,
    inputs: Vec<Vec<Scalar>>,
    pvk: PreparedVerifyingKey<Bls12>,
    srs: GenericSRS<Bls12>,
    records: Vec<StateBallotRecord>,
    provenance: Value,
}
fn load() -> Result<Pool> {
    check_hash(RAW, RAW_SHA)?;
    check_hash(FIXTURE, FIXTURE_SHA)?;
    check_hash(MANIFEST, MANIFEST_SHA)?;
    let raw: Value = serde_json::from_slice(&fs::read(RAW)?)?;
    let f: Value = serde_json::from_slice(&fs::read(FIXTURE)?)?;
    let vk_bytes = bytes(text(&f, "groth16_vk_hex")?)?;
    let mut cursor = Cursor::new(&vk_bytes);
    let vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    let mut encoded = Vec::new();
    vk.write(&mut encoded)?;
    if cursor.position() as usize != vk_bytes.len()
        || encoded != vk_bytes
        || hex(&sha256(&vk_bytes)) != text(&raw, "ballot_vk_bellperson_wire_sha256")?
    {
        return Err("VK wire mismatch".into());
    }
    let pvk = prepare_verifying_key(&vk);
    let rows = f["source_public_inputs_le_hex"]
        .as_array()
        .ok_or("missing rows")?;
    let inputs = rows
        .iter()
        .map(|row| {
            row.as_array()
                .ok_or("invalid row")?
                .iter()
                .map(|x| {
                    let b: [u8; 32] = bytes(x.as_str().ok_or("not hex")?)?
                        .try_into()
                        .map_err(|_| "wrong scalar width")?;
                    Option::<Scalar>::from(Scalar::from_bytes_le(&b))
                        .ok_or_else(|| "noncanonical scalar".into())
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let proof_hex = f["source_proofs_ark_compressed_hex"]
        .as_array()
        .ok_or("missing proofs")?;
    let mut proofs = Vec::new();
    for s in proof_hex {
        let b = bytes(s.as_str().ok_or("not proof hex")?)?;
        let mut cursor = Cursor::new(&b);
        let proof = Proof::<Bls12>::read(&mut cursor)?;
        let mut encoded = Vec::new();
        proof.write(&mut encoded)?;
        if cursor.position() as usize != b.len() || encoded != b {
            return Err("proof canonical round trip failed".into());
        }
        proofs.push(proof);
    }
    if proofs.len() != 32
        || inputs.len() != 32
        || inputs.iter().any(|r| r.len() != 25)
        || proof_hex
            .iter()
            .map(Value::as_str)
            .collect::<HashSet<_>>()
            .len()
            != 32
    {
        return Err("pool shape/distinctness mismatch".into());
    }
    for (p, x) in proofs.iter().zip(&inputs) {
        if !verify_proof(&pvk, p, x)? {
            return Err("saved source proof verification failed".into());
        }
    }
    let srs = setup_fake_srs::<Bls12, _>(&mut ChaCha20Rng::seed_from_u64(SRS_SEED), 32);
    if hex(&srs.hash()) != text(&raw, "shared_aggregation_srs_sha256")? {
        return Err("SRS hash mismatch".into());
    }
    let mut srs_wire = Vec::new();
    srs.write(&mut srs_wire)?;
    let mut records = Vec::new();
    for r in f["source_public_records"]
        .as_array()
        .ok_or("missing records")?
    {
        let ciphertexts = r["ciphertexts"]
            .as_array()
            .ok_or("missing ciphertexts")?
            .iter()
            .map(|ct| {
                Ok(Ciphertext {
                    c0: decode_point(&bytes(text(ct, "c0_hex")?)?, false)?,
                    c1: decode_point(&bytes(text(ct, "c1_hex")?)?, true)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        records.push(StateBallotRecord {
            election_id: digest(r, "election_id_hex")?,
            final_context: digest(r, "final_context_hex")?,
            region: r["region"].as_u64().ok_or("missing region")?,
            registration_root: digest(r, "registration_root_hex")?,
            nullifier: digest(r, "nullifier_hex")?,
            key_set_hash: digest(r, "key_set_hash_hex")?,
            public_key: decode_point(&bytes(text(r, "public_key_hex")?)?, false)?,
            ciphertexts,
        });
    }
    if records.len() != 32 {
        return Err("record count mismatch".into());
    }
    for (r, x) in records.iter().zip(&inputs) {
        let packed =
            vv_friendly_crypto::state_gadget::ballot_public_inputs(r).ok_or("record packing")?;
        if native_batch::convert_inputs(&packed)? != *x {
            return Err("record does not match authenticated proof row".into());
        }
    }
    let provenance = json!({"raw":RAW,"raw_sha256":RAW_SHA,"fixture":FIXTURE,"fixture_sha256":FIXTURE_SHA,"manifest":MANIFEST,"manifest_sha256":MANIFEST_SHA,"ark_vk_sha256":raw["ballot_vk_ark_compressed_sha256"],"bellperson_vk_sha256":raw["ballot_vk_bellperson_wire_sha256"],"shared_srs_sha256":raw["shared_aggregation_srs_sha256"],"vk_wire_bytes":vk_bytes.len(),"generic_srs_wire_bytes":srs_wire.len(),"srs_kind":"deterministic setup_fake_srs research-only","source_proofs":32,"new_groth16_proofs":0,"groth16_setup_count":0,"rows_verified":32});
    Ok(Pool {
        proofs,
        inputs,
        pvk,
        srs,
        records,
        provenance,
    })
}
struct Batch {
    b: usize,
    ps: ProverSRS<Bls12>,
    vs: VerifierSRS<Bls12>,
    aggregate: AggregateProof<Bls12>,
    transcript: Vec<u8>,
    info: Value,
}
fn prepare(pool: &Pool, b: usize, negative: bool) -> Result<Batch> {
    let start = Instant::now();
    let (ps, vs) = pool.srs.specialize(b);
    let specialize_ms = start.elapsed().as_secs_f64() * 1000.0;
    let tr = transcript(&pool.inputs[..b]);
    let aggregate = aggregate_proofs::<Bls12>(&ps, &tr, &pool.proofs[..b], AggregateVersion::V2)?;
    if !verify_aggregate_proof(
        &vs,
        &pool.pvk,
        ChaCha20Rng::seed_from_u64(SEED + b as u64),
        &pool.inputs[..b],
        &aggregate,
        &tr,
        AggregateVersion::V2,
    )? {
        return Err("positive failed".into());
    }
    if negative {
        let mut changed = pool.inputs[..b].to_vec();
        let mut nf = changed[b - 1][7].to_bytes_le();
        nf[0] ^= 1;
        changed[b - 1][7] =
            Option::<Scalar>::from(Scalar::from_bytes_le(&nf)).ok_or("negative scalar")?;
        if verify_aggregate_proof(
            &vs,
            &pool.pvk,
            ChaCha20Rng::seed_from_u64(SEED + 100 + b as u64),
            &changed,
            &aggregate,
            &transcript(&changed),
            AggregateVersion::V2,
        )? {
            return Err("last-nullifier negative accepted".into());
        }
    }
    let old = vec![Ciphertext::zero(); 3];
    let expected = fold::reference(&old, &pool.records[..b]).ok_or("reference fold")?;
    let start = Instant::now();
    let actual = fold::actual(&old, &pool.records[..b]).ok_or("actual fold")?;
    let direct_fold_ms = start.elapsed().as_secs_f64() * 1000.0;
    if actual != expected {
        return Err("independent fold oracle mismatch".into());
    }
    let mut wire = Vec::new();
    aggregate.write(&mut wire)?;
    let vs_json = json!({"n":vs.n,"g":vs.g,"h":vs.h,"g_alpha":vs.g_alpha,"g_beta":vs.g_beta,"h_alpha":vs.h_alpha,"h_beta":vs.h_beta,"h_alpha_d":vs.h_alpha_d});
    let info = json!({"batch_size":b,"source_indices":(0..b).collect::<Vec<_>>(),"positive":true,"last_nullifier_negative_rejected":negative.then_some(true),"negative_row":b-1,"negative_field":7,"independent_fold_matches":true,"direct_fold_ms_unbenchmarked":direct_fold_ms,"specialize_ms_offline":specialize_ms,"individual_proof_bytes":b*192,"aggregate_proof_bytes":wire.len(),"aggregate_proof_sha256":hex(&sha256(&wire)),"matrix_bytes":b*25*32,"transcript_bytes":tr.len(),"individual_plus_matrix_bytes":b*192+b*25*32,"aggregate_plus_matrix_bytes":wire.len()+b*25*32,"verifier_srs_json_bytes":serde_json::to_vec(&vs_json)?.len(),"verifier_srs_json_sha256":hex(&sha256(&serde_json::to_vec(&vs_json)?)),"gas":null,"full_calldata_bytes":null,"full_da_bytes":null});
    Ok(Batch {
        b,
        ps,
        vs,
        aggregate,
        transcript: tr,
        info,
    })
}
fn operation(pool: &Pool, batch: &Batch, op: usize, nonce: u64) -> Result<()> {
    match op {
        0 => {
            for (p, x) in pool.proofs[..batch.b].iter().zip(&pool.inputs[..batch.b]) {
                if !verify_proof(&pool.pvk, p, x)? {
                    return Err("timed individual verify failed".into());
                }
            }
        }
        1 => {
            black_box(aggregate_proofs::<Bls12>(
                &batch.ps,
                &batch.transcript,
                &pool.proofs[..batch.b],
                AggregateVersion::V2,
            )?);
        }
        2 => {
            if !verify_aggregate_proof(
                &batch.vs,
                &pool.pvk,
                ChaCha20Rng::seed_from_u64(SEED.wrapping_add(nonce)),
                &pool.inputs[..batch.b],
                &batch.aggregate,
                &batch.transcript,
                AggregateVersion::V2,
            )? {
                return Err("timed aggregate verify failed".into());
            }
        }
        _ => return Err("invalid operation".into()),
    }
    Ok(())
}
fn timed(pool: &Pool, batch: &Batch, op: usize, k: usize, nonce: &mut u64) -> Result<Value> {
    let start = Instant::now();
    for _ in 0..k {
        *nonce += 1;
        operation(pool, batch, op, *nonce)?;
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    Ok(
        json!({"operation":OPS[op],"iterations":k,"elapsed_ms":elapsed,"ms_per_operation":elapsed/k as f64,"success":true}),
    )
}
fn calibration(pool: &Pool, batch: &Batch, op: usize, nonce: &mut u64) -> Result<usize> {
    let t = timed(pool, batch, op, 1, nonce)?;
    let ms = t["elapsed_ms"].as_f64().ok_or("timer")?;
    Ok((TARGET_MS / ms).ceil().clamp(1.0, 4096.0) as usize)
}
#[cfg(target_os = "macos")]
// Existing pinned libc exposes this Mach query; observer only, no new crypto dependency.
#[allow(deprecated)]
fn rss() -> Result<f64> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    let result = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if result != 0 {
        return Err("Mach RSS measurement unavailable".into());
    }
    let info = unsafe { info.assume_init() };
    Ok(info.resident_size as f64 / 1048576.0)
}
#[cfg(not(target_os = "macos"))]
fn rss() -> Result<f64> {
    Err("current RSS sampler currently requires macOS".into())
}
fn source_snapshot() -> Result<Value> {
    let paths = [
        "crates/vv-friendly-dkg/src/bin/g1e_fixed_suite_benchmark.rs",
        "crates/vv-friendly-dkg/src/bin/common/g1e_fold.rs",
        "crates/vv-friendly-dkg/src/bin/common/native_batch.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "Cargo.toml",
        "Cargo.lock",
        "scripts/g1e_run.py",
        "scripts/g1e_analyze.py",
        "scripts/g1e_csv.mjs",
    ];
    Ok(json!(paths
        .iter()
        .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
        .collect::<Result<Vec<_>>>()?))
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let commit = metadata("git", &["rev-parse", "HEAD"])?;
    if !metadata("git", &["diff", "HEAD", "--name-only"])?.is_empty() {
        return Err("tracked source changes during benchmark".into());
    }
    if args.first().is_some_and(|a| a == "--memory") {
        if args.len() != 5 {
            return Err("--memory B op repeat output".into());
        }
        let b: usize = args[1].parse()?;
        let op: usize = args[2].parse()?;
        let repeat: usize = args[3].parse()?;
        if !BATCHES.contains(&b) || op > 2 {
            return Err("invalid memory job".into());
        }
        let pool = load()?;
        let loaded = rss()?;
        let batch = prepare(&pool, b, false)?;
        let mut nonce = 0;
        let k = calibration(&pool, &batch, op, &mut nonce)?;
        let before = rss()?;
        let cumulative_before = peak_rss_mib();
        let active = Arc::new(AtomicBool::new(true));
        let running = active.clone();
        let sampler = std::thread::spawn(move || -> Option<(f64, usize)> {
            let mut peak = 0.0f64;
            let mut count = 0;
            while running.load(Ordering::Relaxed) {
                peak = peak.max(rss().ok()?);
                count += 1;
                std::thread::sleep(Duration::from_millis(1));
            }
            Some((peak, count))
        });
        let measured = timed(&pool, &batch, op, k, &mut nonce);
        active.store(false, Ordering::Relaxed);
        let (sampled, count) = sampler
            .join()
            .map_err(|_| "RSS sampler panicked")?
            .ok_or("RSS sample unavailable")?;
        let after = rss()?;
        let phase_peak = before.max(after).max(sampled);
        let report = json!({"gate":"G1-E-memory","git_commit":commit,"tracked_source_clean":true,"batch_size":b,"operation":OPS[op],"repeat":repeat,"pid":std::process::id(),"pool_loaded_rss_mib":loaded,"before_rss_mib":before,"after_rss_mib":after,"sampled_phase_peak_rss_mib":phase_peak,"sampled_phase_delta_mib":phase_peak-before,"rss_sample_count":count,"rss_interval_ms":1,"phase_peak_is_sampled_lower_bound":true,"cumulative_process_peak_before_mib":cumulative_before,"cumulative_process_peak_after_mib":peak_rss_mib(),"measurement":measured?,"provenance":pool.provenance});
        write_new(Path::new(&args[4]), &report)?;
        return Ok(());
    }
    if args.len() != 2 || args[0] != "--output" {
        return Err("--output NEW_DIRECTORY".into());
    }
    if !metadata("git", &["status", "--porcelain"])?.is_empty() {
        return Err("timing run requires clean committed tree before output creation".into());
    }
    let out = PathBuf::from(&args[1]);
    if out.exists() {
        return Err("new output directory required".into());
    }
    let snapshot = source_snapshot()?;
    let started = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    println!("G1-E clean source={commit}; git_dirty=false; frozen D2 pool; 5 warmups + 30 rounds");
    let start = Instant::now();
    let pool = load()?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.0;
    let batches = BATCHES
        .iter()
        .map(|&b| prepare(&pool, b, true))
        .collect::<Result<Vec<_>>>()?;
    println!("32 source proofs and 5 positive/negative aggregates PASS; independent reference folds PASS");
    let mut nonce = 0;
    let mut ks = [[1usize; 3]; 5];
    for (i, batch) in batches.iter().enumerate() {
        for (op, k) in ks[i].iter_mut().enumerate() {
            *k = calibration(&pool, batch, op, &mut nonce)?;
        }
    }
    let mut rng = ChaCha20Rng::seed_from_u64(SEED);
    let mut warmup = Vec::new();
    for round in 0..5 {
        let mut order = [0, 1, 2, 3, 4];
        shuffled(&mut order, &mut rng);
        for i in order {
            let mut ops = [0, 1, 2];
            shuffled(&mut ops, &mut rng);
            let mut samples = Vec::new();
            for op in ops {
                let t = timed(&pool, &batches[i], op, ks[i][op], &mut nonce)?;
                ks[i][op] = (TARGET_MS / t["ms_per_operation"].as_f64().ok_or("timer")?)
                    .ceil()
                    .clamp(1.0, 4096.0) as usize;
                samples.push(t);
            }
            warmup
                .push(json!({"warmup_round":round+1,"batch_size":BATCHES[i],"operations":samples}));
        }
        println!("warmup {}/5 complete (excluded)", round + 1);
    }
    for round in 0..30 {
        let mut order = [0, 1, 2, 3, 4];
        shuffled(&mut order, &mut rng);
        let batch_order = order.map(|i| BATCHES[i]);
        for i in order {
            let mut ops = [0, 1, 2];
            shuffled(&mut ops, &mut rng);
            let mut samples = Vec::new();
            for op in ops {
                samples.push(timed(&pool, &batches[i], op, ks[i][op], &mut nonce)?);
            }
            write_new(
                &out.join(format!("B{}/trial-{:03}.json", BATCHES[i], round + 1)),
                &json!({"gate":"G1-E","git_commit":commit,"batch_size":BATCHES[i],"round":round+1,"batch_order":batch_order,"operation_order":ops.map(|op|OPS[op]),"samples":samples,"aggregate_proof_sha256":batches[i].info["aggregate_proof_sha256"]}),
            )?;
        }
        println!(
            "formal round {:02}/30 complete; B order={batch_order:?}",
            round + 1
        );
    }
    if snapshot != source_snapshot()?
        || metadata("git", &["rev-parse", "HEAD"])? != commit
        || !metadata("git", &["diff", "HEAD", "--name-only"])?.is_empty()
    {
        return Err("source changed during measurement".into());
    }
    check_hash(RAW, RAW_SHA)?;
    check_hash(FIXTURE, FIXTURE_SHA)?;
    check_hash(MANIFEST, MANIFEST_SHA)?;
    write_new(&out.join("warmup.json"), &json!(warmup))?;
    write_new(
        &out.join("manifest.json"),
        &json!({"gate":"G1-E","status":"PASS_TIMING_150_BATCH_SAMPLES_450_OPERATION_WINDOWS","started_utc":started,"git_commit":commit,"git_dirty":false,"git_provenance_scope":"process start before generated output creation","tracked_source_clean_at_end":true,"source_hashes":snapshot,"seed":SEED,"warmup_rounds_per_batch":5,"formal_rounds_per_batch":30,"batch_sizes":BATCHES,"target_window_ms":TARGET_MS,"iterations_frozen_after_warmup":ks,"randomized_batch_and_operation_order":true,"load_verify_and_srs_rebuild_ms_offline":load_ms,"cases":batches.iter().map(|b|b.info.clone()).collect::<Vec<_>>(),"provenance":pool.provenance,"rustc":metadata("rustc",&["--version"])? ,"cpu":metadata("sysctl",&["-n","machdep.cpu.brand_string"])? ,"ram_bytes":metadata("sysctl",&["-n","hw.memsize"])? ,"build_profile":"release-opt-level-1","build_jobs":std::env::var("CARGO_BUILD_JOBS").ok(),"production_setup":false,"new_groth16_proof_count":0,"statistical_unit":"repeated operation windows on one fixed pool on one machine; not independent elections/proof generation","gas":null,"state_proof_run":false,"p3_run":false}),
    )?;
    println!(
        "G1-E timing PASS: 150 batch trials / 450 operation windows, 25 excluded warmup trials"
    );
    Ok(())
}
