//! Isolated Stage1 materials. Compilation never runs Setup/Prove.
//! prepare uses a frozen-Setup validation worker and one immutable-PK range worker.
use ark_bls12_381::{Bls12_381, Fr, G1Affine, G2Affine};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, ProvingKey, VerifyingKey};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, OptimizationGoal};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::VerifyingKey as BellVk;
use blstrs::Bls12;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    env,
    error::Error,
    fs,
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use vv_friendly_crypto::evm_export::Eip2537PublicGroth16Proof;
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs, batch_root_v2_for_batch, fold_state_for_batch, state_public_inputs,
    StateBallotRecord, StateTransitionCircuitForBatch,
};
use vv_friendly_crypto::{decode_point, encode_point, Ciphertext};
use vv_types::{hex, sha256};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Circuit = StateTransitionCircuitForBatch<4>;
const CONFIG: &str = "configs/experiment.STAGE1-MULTIBATCH.json";
const POOL: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const OLD_STATE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";
const PK_HASH: &str = "448d9d92b9250aa2bcbf59c82036ab1615f2fe323bbb5a39b8f8d7ec9f73da1c";
const REQUEST_HASH: &str = "e5cf3a56500341f4a91da8dd7f0f6d3eed97431bc7d01eb265372edc5552f238";
const PK_LOAD_BUDGET_S: u64 = 660;
const PER_BATCH_BUDGET_S: u64 = 120;
const RANGE_WORKER_BUDGET_S: u64 = 1000;

fn load(p: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().ok_or_else(|| format!("missing {k}").into())
}
fn decode(s: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
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
fn fixed(v: &Value, k: &str) -> Result<[u8; 32]> {
    decode(string(v, k)?)?
        .try_into()
        .map_err(|_| format!("bad {k}").into())
}
fn serialized<T: CanonicalSerialize>(v: &T) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    v.serialize_compressed(&mut b)?;
    Ok(b)
}
fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(b: &[u8]) -> Result<T> {
    let mut rest = b;
    let v = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || serialized(&v)? != b {
        return Err("noncanonical bytes".into());
    }
    Ok(v)
}
fn write_new(p: impl AsRef<Path>, v: &Value) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)?;
    serde_json::to_writer_pretty(&mut f, v)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn hash_file(p: impl AsRef<Path>) -> Result<String> {
    let mut f = BufReader::new(fs::File::open(p)?);
    let mut h = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(hex(&h.finalize()))
}
fn config() -> Result<Value> {
    let c = load(CONFIG)?;
    for p in c["source_pins"].as_array().ok_or("source pins")? {
        if hash_file(string(p, "path")?)? != string(p, "sha256")? {
            return Err(format!("source changed: {}", p["path"]).into());
        }
    }
    Ok(c)
}
fn record(v: &Value) -> Result<StateBallotRecord> {
    let mut ciphertexts = Vec::new();
    for (j, ct) in v["ciphertexts"]
        .as_array()
        .ok_or("ciphertexts")?
        .iter()
        .enumerate()
    {
        if ct["candidate_id"] != j + 1 {
            return Err("candidate order".into());
        }
        ciphertexts.push(Ciphertext {
            c0: decode_point(&fixed(ct, "c0_hex")?, true)?,
            c1: decode_point(&fixed(ct, "c1_hex")?, true)?,
        });
    }
    if ciphertexts.len() != 3 {
        return Err("three candidates required".into());
    }
    Ok(StateBallotRecord {
        election_id: fixed(v, "election_id_hex")?,
        final_context: fixed(v, "final_context_hex")?,
        registration_root: fixed(v, "registration_root_hex")?,
        key_set_hash: fixed(v, "key_set_hash_hex")?,
        nullifier: fixed(v, "nullifier_hex")?,
        public_key: decode_point(&fixed(v, "public_key_hex")?, false)?,
        region: v["region"].as_u64().ok_or("region")?,
        ciphertexts,
    })
}
fn all_records(pool: &Value) -> Result<Vec<StateBallotRecord>> {
    let r = pool["source_public_records"]
        .as_array()
        .ok_or("records")?
        .iter()
        .map(record)
        .collect::<Result<Vec<_>>>()?;
    if r.len() != 32 || r.iter().map(|x| x.nullifier).collect::<HashSet<_>>().len() != 32 {
        return Err("need 32 unique nullifiers".into());
    }
    for x in &r {
        if x.region != 4
            || x.election_id != r[0].election_id
            || x.final_context != r[0].final_context
            || x.registration_root != r[0].registration_root
            || x.key_set_hash != r[0].key_set_hash
            || x.public_key != r[0].public_key
        {
            return Err("pool binding".into());
        }
    }
    Ok(r)
}
fn circuit(records: &[StateBallotRecord], index: usize) -> Result<Circuit> {
    if !(1..=8).contains(&index) {
        return Err("index range".into());
    }
    let mut old = vec![Ciphertext::zero(); 3];
    for chunk in records[..(index - 1) * 4].chunks(4) {
        old = fold_state_for_batch::<4>(&old, chunk).ok_or("fold")?;
    }
    let part = records[(index - 1) * 4..index * 4].to_vec();
    let a = &part[0];
    let root =
        batch_root_v2_for_batch::<4>(&a.election_id, &a.final_context, 4, index as u64, &part)
            .ok_or("root")?;
    let new = fold_state_for_batch::<4>(&old, &part).ok_or("new fold")?;
    Ok(Circuit {
        election_id: a.election_id,
        final_context: a.final_context,
        region: 4,
        batch_index: index as u64,
        batch_size: 4,
        batch_root: root,
        key_set_hash: a.key_set_hash,
        public_key: a.public_key,
        old_state: old,
        new_state: new,
        records: part,
    })
}
fn flat(ct: &[Ciphertext]) -> Vec<String> {
    ct.iter()
        .flat_map(|x| {
            [
                format!("0x{}", hex(&encode_point(&x.c0))),
                format!("0x{}", hex(&encode_point(&x.c1))),
            ]
        })
        .collect()
}
fn rss_mib() -> Result<f64> {
    let mut u = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, u.as_mut_ptr()) } != 0 {
        return Err("getrusage failed".into());
    }
    let u = unsafe { u.assume_init() };
    Ok(u.ru_maxrss as f64
        / if cfg!(target_os = "macos") {
            1048576.0
        } else {
            1024.0
        })
}
fn assert_root(c: &Circuit, cfg: &Value) -> Result<()> {
    if cfg["ordered_batch_roots"][c.batch_index as usize - 1] != format!("0x{}", hex(&c.batch_root))
    {
        return Err("root differs from frozen plan".into());
    }
    Ok(())
}
fn worker(mode: &str, run: &Path, phase: &str, index: usize, cfg: &Value) -> Result<()> {
    if mode == "setup-worker" && env::var_os("VV_REUSE_SETUP_DIR").is_some() {
        return reuse_frozen_setup(run, cfg);
    }
    let pool = load(POOL)?;
    let records = all_records(&pool)?;
    let c = circuit(&records, index)?;
    assert_root(&c, cfg)?;
    let manifest = load(run.join("request.json"))?;
    if manifest["phase"] != phase || manifest["config_sha256"] != hash_file(CONFIG)? {
        return Err("worker request binding".into());
    }
    if mode == "setup-worker" {
        let t = Instant::now();
        let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            c,
            &mut ChaCha20Rng::seed_from_u64(u64::from_str_radix(
                string(cfg, "state_setup_seed_hex")?,
                16,
            )?),
        )?;
        let setup_ms = t.elapsed().as_secs_f64() * 1000.0;
        let vk = serialized(&parameters.vk)?;
        if hex(&sha256(&vk)) != string(cfg, "state_vk_ark_sha256")? {
            return Err("historical state VK mismatch".into());
        }
        let f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(run.join("research_pk.bin"))?;
        let mut f = BufWriter::new(f);
        parameters.serialize_compressed(&mut f)?;
        f.flush()?;
        drop(f);
        write_new(
            run.join("setup.json"),
            &json!({"status":"MEASURED","setup_ms":setup_ms,"vk_sha256":hex(&sha256(&vk)),
            "pk_sha256":hash_file(run.join("research_pk.bin"))?,"pk_serialized_bytes":parameters.compressed_size(),
            "worker_peak_rss_mib":rss_mib()?,"production_setup":false,"setup_count":1}),
        )?;
    } else {
        let begin = Instant::now();
        let cs = ConstraintSystem::<Fr>::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);
        c.clone().generate_constraints(cs.clone())?;
        cs.finalize();
        if !cs.is_satisfied()? {
            return Err("invalid state witness".into());
        }
        let inputs = state_public_inputs(&c).ok_or("state input packing")?;
        if cs.borrow().ok_or("constraint system")?.instance_assignment[1..] != inputs {
            return Err("input packing differs".into());
        }
        let constraints = cs.num_constraints();
        let witness_ms = begin.elapsed().as_secs_f64() * 1000.0;
        drop(cs);
        let setup = load(run.join("setup.json"))?;
        if hash_file(run.join("research_pk.bin"))? != string(&setup, "pk_sha256")? {
            return Err("PK changed".into());
        }
        let mut reader = BufReader::new(fs::File::open(run.join("research_pk.bin"))?);
        let parameters = ProvingKey::<Bls12_381>::deserialize_compressed(&mut reader)?;
        let mut tail = [0u8; 1];
        if reader.read(&mut tail)? != 0 {
            return Err("PK trailing bytes".into());
        }
        if hex(&sha256(&serialized(&parameters.vk)?)) != string(cfg, "state_vk_ark_sha256")? {
            return Err("state VK changed".into());
        }
        let mut seed = fs::read(run.join("request.json"))?;
        seed.extend_from_slice(&(index as u64).to_be_bytes());
        let t = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            c.clone(),
            &parameters,
            &mut ChaCha20Rng::from_seed(sha256(&seed)),
        )?;
        let prove_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let prepared = prepare_verifying_key(&parameters.vk);
        let prepare_vk_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &inputs)? {
            return Err("fresh state proof rejected".into());
        }
        let verify_ms = t.elapsed().as_secs_f64() * 1000.0;
        let bytes = serialized(&proof)?;
        let eip = Eip2537PublicGroth16Proof::from_proof(&parameters.vk, &proof, &inputs)?;
        if hex(&sha256(&decode(&eip.vk_eip2537_hex)?)) != string(cfg, "state_vk_eip_sha256")? {
            return Err("EIP state VK changed".into());
        }
        let mut omitted = c.old_state.clone();
        for r in c.records.iter().take(3) {
            for (sum, ct) in omitted.iter_mut().zip(&r.ciphertexts) {
                *sum = sum.add(ct);
            }
        }
        write_new(
            run.join(format!("batch_{index}.json")),
            &json!({"phase":phase,"batch_index":index,"status":"MEASURED",
            "proof_origin":"NEW_THIS_RUN_SEPARATE_SERIAL_WORKER","proof_ark_sha256":hex(&sha256(&bytes)),"proof_ark_hex":hex(&bytes),
            "proof_ark_bytes":bytes.len(),"proof_eip_bytes":decode(&eip.proof_neg_a_eip2537_hex)?.len(),"eip":eip,
            "constraints":constraints,"witness_ms":witness_ms,"prove_ms":prove_ms,"verify_ms":verify_ms,"prepare_vk_ms":prepare_vk_ms,
            "witness_scope":cfg["witness_timing_scope"],"peak_rss_mib":rss_mib()?,
            "rss_scope":"OS process high-water mark for this sole batch worker, including witness, PK decode, Prove, Verify and export; not Prove-only interval peak",
            "old_state":flat(&c.old_state),"new_state":flat(&c.new_state),"omission_a_new_state":flat(&omitted),
            "root":format!("0x{}",hex(&c.batch_root)),"config_sha256":hash_file(CONFIG)?}),
        )?;
    }
    Ok(())
}
fn spawn_worker(
    run: &Path,
    phase: &str,
    mode: &str,
    index: usize,
    end: usize,
    cfg: &Value,
) -> Result<Value> {
    let label = format!("{mode}-{index}");
    let stdout = run.join(format!("{label}.stdout.log"));
    let stderr = run.join(format!("{label}.stderr.log"));
    let mut args = vec![
        mode.to_owned(),
        run.to_str().ok_or("run path")?.to_owned(),
        phase.to_owned(),
        index.to_string(),
    ];
    if mode == "prove-range" {
        args.push(end.to_string());
    }
    args.extend(["--approved".to_owned(), phase.to_owned()]);
    let t = Instant::now();
    let mut child = Command::new(env::current_exe()?)
        .args(&args)
        .stdout(Stdio::from(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(stdout)?,
        ))
        .stderr(Stdio::from(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(stderr)?,
        ))
        .spawn()?;
    loop {
        // Total deadline takes priority, including startup and final cleanup.
        let limit = if mode == "prove-range" {
            RANGE_WORKER_BUDGET_S
        } else {
            600
        };
        let mut reason = if t.elapsed() >= Duration::from_secs(limit) {
            Some("RANGE_OR_CHILD_TOTAL_BUDGET")
        } else {
            None
        };
        if mode == "prove-range" {
            if let Ok(progress) = load(run.join("range_progress.json")) {
                let start_ns = progress["stage_start_worker_ns"]
                    .as_u64()
                    .ok_or("progress timestamp")?;
                let budget = progress["stage_budget_s"]
                    .as_u64()
                    .ok_or("progress budget")?;
                if t.elapsed().as_nanos().saturating_sub(start_ns as u128)
                    >= Duration::from_secs(budget).as_nanos()
                {
                    reason = Some("PARENT_PHASE_BUDGET");
                }
            }
        }
        if reason.is_none() {
            if let Some(status) = child.try_wait()? {
                let v = json!({"label":label,"pid":child.id(),"exit_code":status.code(),"wall_ms":t.elapsed().as_secs_f64()*1000.0,"argv":args});
                write_new(run.join(format!("{label}.command.json")), &v)?;
                if !status.success() {
                    return Err(format!("worker failed: {label}; logs preserved").into());
                }
                return Ok(v);
            }
        }
        if let Ok(p) = Command::new("ps")
            .args(["-o", "rss=", "-p", &child.id().to_string()])
            .output()
        {
            if let Ok(kib) = String::from_utf8_lossy(&p.stdout).trim().parse::<f64>() {
                if kib / 1024.0
                    > cfg["budgets"]["child_rss_mib"]
                        .as_f64()
                        .ok_or("RSS budget")?
                {
                    reason = Some("RSS_BUDGET");
                }
            }
        }
        if let Some(reason) = reason {
            child.kill()?;
            child.wait()?;
            write_new(
                run.join(format!("{label}.blocked.json")),
                &json!({"status":"BLOCKED","reason":reason}),
            )?;
            return Err(reason.into());
        }
        thread::sleep(Duration::from_millis(250));
    }
}
fn ballot_vk(pool: &Value) -> Result<VerifyingKey<Bls12_381>> {
    let bytes = decode(string(pool, "groth16_vk_hex")?)?;
    let mut cursor = Cursor::new(&bytes);
    let vk = BellVk::<Bls12>::read(&mut cursor)?;
    let mut roundtrip = Vec::new();
    vk.write(&mut roundtrip)?;
    if cursor.position() as usize != bytes.len() || roundtrip != bytes || vk.ic.len() != 26 {
        return Err("ballot VK wire".into());
    }
    Ok(VerifyingKey {
        alpha_g1: canonical::<G1Affine>(&vk.alpha_g1.to_compressed())?,
        beta_g2: canonical::<G2Affine>(&vk.beta_g2.to_compressed())?,
        gamma_g2: canonical::<G2Affine>(&vk.gamma_g2.to_compressed())?,
        delta_g2: canonical::<G2Affine>(&vk.delta_g2.to_compressed())?,
        gamma_abc_g1: vk
            .ic
            .iter()
            .map(|p| canonical::<G1Affine>(&p.to_compressed()))
            .collect::<Result<Vec<_>>>()?,
    })
}
fn prepare(run: &Path, phase: &str, cfg: &Value) -> Result<()> {
    if run.exists() {
        return Err("fresh materials directory required; never overwrite or mix B/C".into());
    }
    fs::create_dir_all(run)?;
    let count = if phase == "B" { 2 } else { 8 };
    let start = Instant::now();
    let frozen = PathBuf::from(
        env::var("VV_REUSE_SETUP_DIR")
            .map_err(|_| "BLOCKED_FROZEN_SETUP_REQUIRED_NO_AUTOMATIC_SETUP")?,
    );
    if phase == "B" {
        let raw = fs::read(frozen.join("request.json"))?;
        if hex(&sha256(&raw)) != REQUEST_HASH {
            return Err("frozen request mismatch".into());
        }
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(run.join("request.json"))?;
        out.write_all(&raw)?;
    } else {
        write_new(
            run.join("request.json"),
            &json!({"phase":phase,"config_sha256":hash_file(CONFIG)?,
        "run_identity":run.to_str(),"pid":std::process::id(),"unix_nanos":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos().to_string(),
        "fresh_state_proofs_required":count,"archived_state_proofs_reused":0}),
        )?;
    }
    let pool = load(POOL)?;
    let records = all_records(&pool)?;
    let vk = ballot_vk(&pool)?;
    if hex(&sha256(&serialized(&vk)?)) != string(cfg, "ballot_vk_ark_sha256")? {
        return Err("ballot VK mismatch".into());
    }
    let prepared = prepare_verifying_key(&vk);
    let mut ballots = Vec::new();
    let mut per_vote_state = vec![Ciphertext::zero(); 3];
    for (i, r) in records.iter().enumerate() {
        let inputs = ballot_public_inputs(r).ok_or("ballot inputs")?;
        let row = pool["source_public_inputs_le_hex"][i]
            .as_array()
            .ok_or("row")?
            .iter()
            .map(|x| canonical::<Fr>(&decode(x.as_str().ok_or("field")?)?))
            .collect::<Result<Vec<_>>>()?;
        if row != inputs {
            return Err("record and original public inputs differ".into());
        }
        let bytes = decode(
            pool["source_proofs_ark_compressed_hex"][i]
                .as_str()
                .ok_or("proof")?,
        )?;
        let proof: Proof<Bls12_381> = canonical(&bytes)?;
        if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &row)? {
            return Err("original ballot proof failed".into());
        }
        let eip = Eip2537PublicGroth16Proof::from_proof(&vk, &proof, &row)?;
        if hex(&sha256(&decode(&eip.vk_eip2537_hex)?)) != string(cfg, "ballot_vk_eip_sha256")? {
            return Err("ballot EIP VK mismatch".into());
        }
        let old = flat(&per_vote_state);
        for (sum, ct) in per_vote_state.iter_mut().zip(&r.ciphertexts) {
            *sum = sum.add(ct);
        }
        ballots.push(json!({"index":i,"nullifier":format!("0x{}",hex(&r.nullifier)),"original_proof_sha256":hex(&sha256(&bytes)),
            "eip":eip,"old_state":old,"new_state":flat(&per_vote_state)}));
    }
    // The setup-worker validates/copies frozen material; it performs ZERO Setup generation.
    let commands = vec![
        spawn_worker(run, phase, "setup-worker", 1, 1, cfg)?,
        spawn_worker(run, phase, "prove-range", 1, count, cfg)?,
    ];
    let mut batches = Vec::new();
    for index in 1..=count {
        let value = load(run.join(format!("batch_{index}.json")))?;
        let c = circuit(&records, index)?;
        if value["old_state"] != json!(flat(&c.old_state))
            || value["new_state"] != json!(flat(&c.new_state))
        {
            return Err("worker state differs".into());
        }
        batches.push(value);
    }
    let historical = load(OLD_STATE)?;
    for (i, batch) in batches.iter().take(2).enumerate() {
        let old = &historical["batches"][i];
        for (field, key) in [("old_state", "old_state"), ("new_state", "new_state")] {
            let ct = old[field]
                .as_array()
                .ok_or("old CT")?
                .iter()
                .flat_map(|x| {
                    [
                        format!("0x{}", x["c0_hex"].as_str().unwrap_or("")),
                        format!("0x{}", x["c1_hex"].as_str().unwrap_or("")),
                    ]
                })
                .collect::<Vec<_>>();
            if batch[key] != json!(ct) {
                return Err("historical two-batch state differs".into());
            }
        }
    }
    write_new(
        run.join("materials.json"),
        &json!({"schema":"VV/STAGE1/MATERIALS/v1","phase":phase,"status":"MEASURED",
        "config_sha256":hash_file(CONFIG)?,"request_sha256":hash_file(run.join("request.json"))?,"setup":load(run.join("setup.json"))?,
        "ballots":ballots,"batches":batches,"commands":commands,"all_state_proofs_fresh":true,"serial_batches":true,"serial_workers":false,
        "total_wall_ms":start.elapsed().as_secs_f64()*1000.0,"original_ballot_proofs_reused":32,"original_ballot_proofs_generated":0,
        "state_proofs_generated":count,"archived_state_proofs_reused":0,"production_setup":false}),
    )?;
    write_new(
        run.join("prepare_summary.json"),
        &json!({"schema":"VV/STAGE1/PK-REUSE/v1","phase":phase,"commands":commands,
        "setup_generation_count":0,"setup_material_origin":"REUSED_FROZEN_B02","setup_reuse":load(run.join("setup_reuse.json"))?,
        "range":load(run.join("range_profile.json"))?,"batches":batches,"request_sha256":hash_file(run.join("request.json"))?,
        "pk_sha256":PK_HASH,"vk_ark_sha256":cfg["state_vk_ark_sha256"],"vk_eip_sha256":cfg["state_vk_eip_sha256"],
        "config_sha256":hash_file(CONFIG)?,"seed_policy":"SHA256(request_raw_bytes || u64_be(batch_index)); research reproduction only",
        "serial_batches":true,"serial_workers":false,"all_state_proofs_fresh":true,"total_wall_ms":start.elapsed().as_secs_f64()*1000.0}),
    )?;
    println!("{phase}: complete materials, no EVM execution; {count} fresh state proofs");
    Ok(())
}
fn reuse_frozen_setup(run: &Path, cfg: &Value) -> Result<()> {
    let begin = Instant::now();
    let source = PathBuf::from(env::var("VV_REUSE_SETUP_DIR")?);
    let setup = load(source.join("setup.json"))?;
    if string(&setup, "pk_sha256")? != PK_HASH
        || hash_file(source.join("research_pk.bin"))? != PK_HASH
        || string(&setup, "vk_sha256")? != string(cfg, "state_vk_ark_sha256")?
    {
        return Err("BLOCKED_FROZEN_SETUP_HASH_MISMATCH".into());
    }
    let input = fs::read(source.join("setup.json"))?;
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(run.join("setup.json"))?;
    out.write_all(&input)?;
    // Only the NEW run receives a symlink. Never write or chmod its historical target.
    std::os::unix::fs::symlink(source.join("research_pk.bin"), run.join("research_pk.bin"))?;
    write_new(
        run.join("setup_reuse.json"),
        &json!({"status":"PASS_FROZEN_SETUP_REUSE","pid":std::process::id(),
        "setup_generation_count":0,"source_directory":source,"setup_json_sha256":hex(&sha256(&input)),
        "pk_sha256":PK_HASH,"pk_bytes":fs::metadata(run.join("research_pk.bin"))?.len(),
        "vk_ark_sha256":cfg["state_vk_ark_sha256"],"reuse_validation_ms":begin.elapsed().as_secs_f64()*1000.0,
        "worker_peak_rss_mib":rss_mib()?}),
    )
}

struct HashingReader<R> {
    inner: R,
    hash: Sha256,
    bytes: u64,
}
impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(b)?;
        self.hash.update(&b[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
}

struct RangeClock<'a> {
    run: &'a Path,
    start: Instant,
    stage: Instant,
    stage_name: String,
    stage_budget: u64,
}
impl<'a> RangeClock<'a> {
    fn new(run: &'a Path, start: Instant) -> Self {
        Self {
            run,
            start,
            stage: Instant::now(),
            stage_name: "startup".to_owned(),
            stage_budget: PK_LOAD_BUDGET_S,
        }
    }
    fn check(&self) -> Result<()> {
        if self.start.elapsed() >= Duration::from_secs(RANGE_WORKER_BUDGET_S) {
            return Err("RANGE_TOTAL_BUDGET".into());
        }
        if self.stage.elapsed() >= Duration::from_secs(self.stage_budget) {
            return Err(format!("PHASE_BUDGET:{}", self.stage_name).into());
        }
        Ok(())
    }
    fn enter(&mut self, name: &str, budget: u64) -> Result<()> {
        self.check()?;
        self.stage = Instant::now();
        self.stage_name = name.to_owned();
        self.stage_budget = budget;
        let value = json!({"pid":std::process::id(),"stage":name,"stage_budget_s":budget,
            "stage_start_worker_ns":self.start.elapsed().as_nanos(),"range_worker_budget_s":RANGE_WORKER_BUDGET_S});
        let temporary = self.run.join("range_progress.tmp");
        fs::write(&temporary, serde_json::to_vec(&value)?)?;
        fs::rename(temporary, self.run.join("range_progress.json"))?;
        Ok(())
    }
}

#[allow(deprecated)]
fn instant_rss_mib() -> Result<f64> {
    #[cfg(target_os = "macos")]
    {
        let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info_data_t>::zeroed();
        let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
        let result = unsafe {
            libc::task_info(
                libc::mach_task_self(),
                libc::MACH_TASK_BASIC_INFO,
                info.as_mut_ptr().cast(),
                &mut count,
            )
        };
        if result != libc::KERN_SUCCESS || count != libc::MACH_TASK_BASIC_INFO_COUNT {
            return Err("MACH_RSS_SAMPLING_FAILED".into());
        }
        // libc's Mach structure is packed(4); do not create an aligned reference to this field.
        let resident =
            unsafe { std::ptr::addr_of!((*info.as_ptr()).resident_size).read_unaligned() };
        Ok(resident as f64 / 1048576.0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("INSTANT_RSS_PLATFORM_NOT_ADMITTED".into())
    }
}

fn load_range_pk(
    run: &Path,
    cfg: &Value,
    clock: &RangeClock<'_>,
    profile: &mut Value,
) -> Result<ProvingKey<Bls12_381>> {
    use std::os::unix::fs::MetadataExt;
    let total = Instant::now();
    // ONE descriptor for BOTH passes. The second pass hashes the exact bytes consumed by canonical decoding.
    let file = fs::File::open(run.join("research_pk.bin"))?;
    let before = file.metadata()?;
    let mut reader = BufReader::new(file);
    let begin = Instant::now();
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        clock.check()?;
        let n = reader.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    let digest = hex(&h.finalize());
    if digest != PK_HASH {
        return Err("PK_SHA256_MISMATCH".into());
    }
    profile["pk_sha256_ms"] = json!(begin.elapsed().as_secs_f64() * 1000.0);
    reader.seek(SeekFrom::Start(0))?;
    let mut actual = HashingReader {
        inner: reader,
        hash: Sha256::new(),
        bytes: 0,
    };
    let begin = Instant::now();
    clock.check()?;
    let parameters = ProvingKey::<Bls12_381>::deserialize_compressed(&mut actual)?;
    profile["pk_deserialize_ms"] = json!(begin.elapsed().as_secs_f64() * 1000.0);
    clock.check()?;
    let mut tail = [0u8; 1];
    if actual.read(&mut tail)? != 0 {
        return Err("PK_TRAILING_BYTES".into());
    }
    let loaded_digest = hex(&actual.hash.clone().finalize());
    let after = actual.inner.get_ref().metadata()?;
    if loaded_digest != PK_HASH
        || actual.bytes != before.len()
        || before.dev() != after.dev()
        || before.ino() != after.ino()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
    {
        return Err("PK_DESCRIPTOR_OR_LOADED_BYTES_CHANGED".into());
    }
    let begin = Instant::now();
    let vk_hash = hex(&sha256(&serialized(&parameters.vk)?));
    if vk_hash != string(cfg, "state_vk_ark_sha256")? {
        return Err("ARK_VK_HASH_MISMATCH".into());
    }
    profile["vk_ark_hash_ms"] = json!(begin.elapsed().as_secs_f64() * 1000.0);
    profile["pk_sha256"] = json!(digest);
    profile["loaded_pk_sha256"] = json!(loaded_digest);
    profile["vk_ark_sha256"] = json!(vk_hash);
    profile["pk_deserialize_bytes_read"] = json!(actual.bytes);
    profile["pk_load_count"] = json!(1);
    profile["pk_once_load_total_ms"] = json!(total.elapsed().as_secs_f64() * 1000.0);
    profile["single_readonly_descriptor"] = json!(true);
    clock.check()?;
    Ok(parameters)
}

fn prove_reuse_batch(
    run: &Path,
    phase: &str,
    index: usize,
    cfg: &Value,
    pk: &ProvingKey<Bls12_381>,
    clock: &RangeClock<'_>,
) -> Result<()> {
    let total = Instant::now();
    clock.check()?;
    let pool = load(POOL)?;
    let records = all_records(&pool)?;
    let c = circuit(&records, index)?;
    assert_root(&c, cfg)?;
    let request_raw = fs::read(run.join("request.json"))?;
    let request: Value = serde_json::from_slice(&request_raw)?;
    if request["phase"] != phase || request["config_sha256"] != hash_file(CONFIG)? {
        return Err("request binding".into());
    }
    let begin = Instant::now();
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    c.clone().generate_constraints(cs.clone())?;
    cs.finalize();
    if !cs.is_satisfied()? {
        return Err("invalid state witness".into());
    }
    let inputs = state_public_inputs(&c).ok_or("state inputs")?;
    if cs.borrow().ok_or("CS")?.instance_assignment[1..] != inputs {
        return Err("input packing differs".into());
    }
    let constraints = cs.num_constraints();
    let witness_ms = begin.elapsed().as_secs_f64() * 1000.0;
    drop(cs);
    clock.check()?;
    let mut seed = request_raw;
    seed.extend_from_slice(&(index as u64).to_be_bytes());
    let seed_hash = sha256(&seed);
    let begin = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        c.clone(),
        pk,
        &mut ChaCha20Rng::from_seed(seed_hash),
    )?;
    let prove_ms = begin.elapsed().as_secs_f64() * 1000.0;
    clock.check()?;
    let begin = Instant::now();
    let prepared = prepare_verifying_key(&pk.vk);
    let prepare_vk_ms = begin.elapsed().as_secs_f64() * 1000.0;
    let begin = Instant::now();
    if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &inputs)? {
        return Err("fresh proof rejected".into());
    }
    let verify_ms = begin.elapsed().as_secs_f64() * 1000.0;
    clock.check()?;
    let bytes = serialized(&proof)?;
    let eip = Eip2537PublicGroth16Proof::from_proof(&pk.vk, &proof, &inputs)?;
    let eip_bytes = decode(&eip.proof_neg_a_eip2537_hex)?;
    if hex(&sha256(&decode(&eip.vk_eip2537_hex)?)) != string(cfg, "state_vk_eip_sha256")? {
        return Err("EIP VK mismatch".into());
    }
    let mut omitted = c.old_state.clone();
    for r in c.records.iter().take(3) {
        for (sum, ct) in omitted.iter_mut().zip(&r.ciphertexts) {
            *sum = sum.add(ct);
        }
    }
    clock.check()?;
    write_new(
        run.join(format!("batch_{index}.json")),
        &json!({"phase":phase,"batch_index":index,"status":"MEASURED",
        "worker_pid":std::process::id(),"proof_origin":"NEW_THIS_RUN_SINGLE_PROCESS_PK_REUSE","pk_load_count":1,"batch_pk_deserialize_ms":0,
        "proof_ark_sha256":hex(&sha256(&bytes)),"proof_eip_sha256":hex(&sha256(&eip_bytes)),"proof_ark_hex":hex(&bytes),
        "proof_ark_bytes":bytes.len(),"proof_eip_bytes":eip_bytes.len(),"eip":eip,"proof_seed_sha256":hex(&seed_hash),
        "constraints":constraints,"witness_ms":witness_ms,"prove_ms":prove_ms,"verify_ms":verify_ms,"prepare_vk_ms":prepare_vk_ms,
        "batch_total_before_drop_ms":total.elapsed().as_secs_f64()*1000.0,"peak_rss_mib":rss_mib()?,
        "rss_scope":"Cumulative process ru_maxrss; NOT a per-batch peak","witness_scope":cfg["witness_timing_scope"],
        "old_state":flat(&c.old_state),"new_state":flat(&c.new_state),"omission_a_new_state":flat(&omitted),
        "root":format!("0x{}",hex(&c.batch_root)),"config_sha256":hash_file(CONFIG)?}),
    )?;
    // All batch-owned POOL/R1CS/witness/proof/export objects leave scope on return.
    Ok(())
}

fn prove_range(
    run: &Path,
    phase: &str,
    start: usize,
    end: usize,
    cfg: &Value,
    started: Instant,
) -> Result<()> {
    if !(1..=8).contains(&start) || end < start || end > 8 {
        return Err("invalid closed batch range".into());
    }
    let mut clock = RangeClock::new(run, started);
    let mut profile = json!({"schema":"VV/STAGE1/PK-REUSE-RANGE/v1","status":"RUNNING","pid":std::process::id(),
        "start_batch":start,"end_batch":end,"pk_load_count":0,"completed_batches":[],"rss_sequence":[],
        "budgets":{"pk_load_budget_s":PK_LOAD_BUDGET_S,"per_batch_budget_s":PER_BATCH_BUDGET_S,"range_worker_budget_s":RANGE_WORKER_BUDGET_S},
        "setup_generation_count":0,"rss_scope":"instant resident_size after batch Drop; separate lifetime ru_maxrss"});
    let result: Result<()> = (|| {
        clock.enter("pk_load", PK_LOAD_BUDGET_S)?;
        let pk = load_range_pk(run, cfg, &clock, &mut profile)?;
        for index in start..=end {
            clock.enter(&format!("batch_{index}"), PER_BATCH_BUDGET_S)?;
            let begin = Instant::now();
            prove_reuse_batch(run, phase, index, cfg, &pk, &clock)?;
            // Function returned: ALL large batch-owned objects have been dropped before this sample.
            let current = instant_rss_mib()?;
            let peak = rss_mib()?;
            clock.check()?;
            profile[format!("batch_{index}_instant_rss_mib")] = json!(current);
            profile[format!("batch_{index}_pk_deserialize_ms")] = json!(0);
            profile[format!("batch_{index}_total_ms")] =
                json!(begin.elapsed().as_secs_f64() * 1000.0);
            profile["completed_batches"]
                .as_array_mut()
                .ok_or("batches")?
                .push(json!(index));
            profile["rss_sequence"].as_array_mut().ok_or("rss")?.push(json!({"batch_index":index,"instant_rss_mib":current,"process_peak_rss_mib":peak,"after_batch_drop":true}));
        }
        drop(pk);
        clock.check()?;
        Ok(())
    })();
    profile["status"] = json!(if result.is_ok() {
        "PASS_RANGE_PK_REUSE"
    } else {
        "FAIL_RANGE_PK_REUSE"
    });
    profile["error"] = json!(result.as_ref().err().map(|e| e.to_string()));
    profile["worker_total_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
    profile["worker_peak_rss_mib"] = json!(rss_mib()?);
    write_new(run.join("range_profile.json"), &profile)?;
    result
}

fn main() -> Result<()> {
    let process_started = Instant::now();
    let a = env::args().skip(1).collect::<Vec<_>>();
    if a.len() < 5 {
        return Err("usage: prepare RUN B|C --approved B|C; internal worker: setup-worker|prove-worker RUN B|C INDEX --approved B|C".into());
    }
    let mode = &a[0];
    let run = PathBuf::from(&a[1]);
    let phase = &a[2];
    if !["B", "C"].contains(&phase.as_str())
        || a[a.len() - 2] != "--approved"
        || a[a.len() - 1] != *phase
    {
        return Err("explicit stage approval required; no automatic progression".into());
    }
    if !run.starts_with("experiments/stage1_multibatch_raw")
        || run
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("run must be inside isolated stage1 raw directory".into());
    }
    let cfg = config()?;
    match mode.as_str() {
        "prepare" if a.len() == 5 => prepare(&run, phase, &cfg),
        "prove-range" if a.len() == 7 => prove_range(
            &run,
            phase,
            a[3].parse()?,
            a[4].parse()?,
            &cfg,
            process_started,
        ),
        "setup-worker" | "prove-worker" if a.len() == 6 => {
            worker(mode, &run, phase, a[3].parse()?, &cfg)
        }
        _ => Err("invalid command".into()),
    }
}
