//! Isolated Stage1 materials. Compilation never runs Setup/Prove.
//! prepare RUN B|C --approved B|C; each setup/prove worker is a separate serial process.
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
    io::{BufReader, BufWriter, Cursor, Read, Write},
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

const PROFILE_STAGES: [&str; 16] = [
    "worker_input",
    "request_bind",
    "witness_diagnostic",
    "pk_sha256",
    "pk_open",
    "pk_deserialize",
    "pk_tail_check",
    "vk_ark_hash",
    "proof_seed",
    "prove",
    "prepare_vk",
    "ark_verify",
    "proof_serialize",
    "eip_export",
    "omission_check",
    "output_write",
];
#[derive(Default)]
struct Profile {
    stages: std::collections::BTreeMap<&'static str, u128>,
    process_config_ns: Option<u128>,
    worker_total_ns: Option<u128>,
    deserialize_bytes: Option<u64>,
    deserialize_user_cpu_ms: Option<f64>,
    deserialize_sys_cpu_ms: Option<f64>,
}
impl Profile {
    fn mark(&mut self, name: &'static str, t: Instant) {
        assert!(self.stages.insert(name, t.elapsed().as_nanos()).is_none());
    }
    fn save(&self, result: &Result<()>) -> Result<()> {
        let output = env::var("VV_PROFILE_OUT_JSON")?;
        let sum_ns: u128 = self.stages.values().copied().sum();
        let residual = self.worker_total_ns.map(|total| {
            total
                .checked_sub(sum_ns)
                .expect("Invalid worker timing hierarchy")
        });
        let mut v = json!({
            "status": if result.is_ok() { "PASS_PROFILE_ONLY" } else { "FAIL_PROFILE" },
            "error": result.as_ref().err().map(|e| e.to_string()),
            "completed_stages": PROFILE_STAGES.iter().filter(|n| self.stages.contains_key(**n)).collect::<Vec<_>>(),
            "stage_order": PROFILE_STAGES,
            "sum_completed_stages_ns": sum_ns,
            "pk_deserialize_bytes_read": self.deserialize_bytes,
            "pk_deserialize_user_cpu_ms": self.deserialize_user_cpu_ms,
            "pk_deserialize_sys_cpu_ms": self.deserialize_sys_cpu_ms,
            "worker_peak_rss_mib": rss_mib().ok(),
            "wall_clock_unit": "integer nanoseconds; milliseconds derived by division by 1e6",
            "worker_total_scope": "outer worker() call in main, stopped after return including local Drops",
            "unattributed_scope": "setup.json read, explicit ConstraintSystem Drop, other local Drops and uninstrumented bookkeeping",
            "output_write_scope": "write_new batch JSON file; output JSON construction is outside this stage",
            "pk_deserialize_scope": "streamed Read + compressed point decoding + unchanged canonical validation; active cache, not cold I/O",
            "cpu_scope": "getrusage RUSAGE_SELF interval around deserialize; summed CPU across threads, not an additive wall-clock stage",
            "gate_c": "NOT_AUTHORIZED_NOT_STARTED",
            "setup_count": 0,
            "proof_generation_count": if self.stages.contains_key("prove") { 1 } else { 0 },
            "evm_transaction_count": 0
        });
        for (name, ns) in [
            ("process_config", self.process_config_ns),
            ("worker_total", self.worker_total_ns),
            ("unattributed_worker", residual),
        ] {
            v[format!("{name}_ns")] = json!(ns);
            v[format!("{name}_ms")] = json!(ns.map(|n| n as f64 / 1_000_000.0));
        }
        for name in PROFILE_STAGES {
            let ns = self.stages.get(name).copied();
            v[format!("{name}_ns")] = json!(ns);
            v[format!("{name}_ms")] = json!(ns.map(|n| n as f64 / 1_000_000.0));
        }
        write_new(output, &v)
    }
}
struct CountingReader<R> {
    inner: R,
    bytes_read: u64,
}
impl<R> CountingReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            bytes_read: 0,
        }
    }
}
impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buffer)?;
        self.bytes_read = self
            .bytes_read
            .checked_add(n as u64)
            .expect("read counter overflow");
        Ok(n)
    }
}
fn cpu_usage_ms() -> Result<(f64, f64)> {
    let mut u = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, u.as_mut_ptr()) } != 0 {
        return Err("getrusage CPU sampling failed".into());
    }
    let u = unsafe { u.assume_init() };
    let ms = |v: libc::timeval| v.tv_sec as f64 * 1000.0 + v.tv_usec as f64 / 1000.0;
    Ok((ms(u.ru_utime), ms(u.ru_stime)))
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Circuit = StateTransitionCircuitForBatch<4>;
const CONFIG: &str = "configs/experiment.STAGE1-MULTIBATCH.json";
const POOL: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const OLD_STATE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";

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
fn worker(
    mode: &str,
    run: &Path,
    phase: &str,
    index: usize,
    cfg: &Value,
    profile: &mut Profile,
) -> Result<()> {
    let profile_t = Instant::now();
    let pool = load(POOL)?;
    let records = all_records(&pool)?;
    let c = circuit(&records, index)?;
    assert_root(&c, cfg)?;
    profile.mark("worker_input", profile_t);
    let profile_t = Instant::now();
    let manifest = load(run.join("request.json"))?;
    if manifest["phase"] != phase || manifest["config_sha256"] != hash_file(CONFIG)? {
        return Err("worker request binding".into());
    }
    profile.mark("request_bind", profile_t);
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
        profile.mark("witness_diagnostic", begin);
        drop(cs);
        let setup = load(run.join("setup.json"))?;
        let profile_t = Instant::now();
        if hash_file(run.join("research_pk.bin"))? != string(&setup, "pk_sha256")? {
            return Err("PK changed".into());
        }
        profile.mark("pk_sha256", profile_t);
        let profile_t = Instant::now();
        let mut reader =
            CountingReader::new(BufReader::new(fs::File::open(run.join("research_pk.bin"))?));
        profile.mark("pk_open", profile_t);
        let cpu_before = cpu_usage_ms()?;
        let profile_t = Instant::now();
        let parameters = ProvingKey::<Bls12_381>::deserialize_compressed(&mut reader)?;
        profile.deserialize_bytes = Some(reader.bytes_read);
        profile.mark("pk_deserialize", profile_t);
        let cpu_after = cpu_usage_ms()?;
        profile.deserialize_user_cpu_ms = Some(cpu_after.0 - cpu_before.0);
        profile.deserialize_sys_cpu_ms = Some(cpu_after.1 - cpu_before.1);
        let profile_t = Instant::now();
        let mut tail = [0u8; 1];
        if reader.read(&mut tail)? != 0 {
            return Err("PK trailing bytes".into());
        }
        profile.mark("pk_tail_check", profile_t);
        let profile_t = Instant::now();
        if hex(&sha256(&serialized(&parameters.vk)?)) != string(cfg, "state_vk_ark_sha256")? {
            return Err("state VK changed".into());
        }
        profile.mark("vk_ark_hash", profile_t);
        let profile_t = Instant::now();
        let mut seed = fs::read(run.join("request.json"))?;
        seed.extend_from_slice(&(index as u64).to_be_bytes());
        profile.mark("proof_seed", profile_t);
        let t = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            c.clone(),
            &parameters,
            &mut ChaCha20Rng::from_seed(sha256(&seed)),
        )?;
        let prove_ms = t.elapsed().as_secs_f64() * 1000.0;
        profile.mark("prove", t);
        let t = Instant::now();
        let prepared = prepare_verifying_key(&parameters.vk);
        let prepare_vk_ms = t.elapsed().as_secs_f64() * 1000.0;
        profile.mark("prepare_vk", t);
        let t = Instant::now();
        if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &inputs)? {
            return Err("fresh state proof rejected".into());
        }
        let verify_ms = t.elapsed().as_secs_f64() * 1000.0;
        profile.mark("ark_verify", t);
        let profile_t = Instant::now();
        let bytes = serialized(&proof)?;
        profile.mark("proof_serialize", profile_t);
        let profile_t = Instant::now();
        let eip = Eip2537PublicGroth16Proof::from_proof(&parameters.vk, &proof, &inputs)?;
        if hex(&sha256(&decode(&eip.vk_eip2537_hex)?)) != string(cfg, "state_vk_eip_sha256")? {
            return Err("EIP state VK changed".into());
        }
        profile.mark("eip_export", profile_t);
        let profile_t = Instant::now();
        let mut omitted = c.old_state.clone();
        for r in c.records.iter().take(3) {
            for (sum, ct) in omitted.iter_mut().zip(&r.ciphertexts) {
                *sum = sum.add(ct);
            }
        }
        profile.mark("omission_check", profile_t);
        let output = json!({"phase":phase,"batch_index":index,"status":"MEASURED",
            "proof_origin":"NEW_THIS_RUN_SEPARATE_SERIAL_WORKER","proof_ark_sha256":hex(&sha256(&bytes)),"proof_ark_hex":hex(&bytes),
            "proof_ark_bytes":bytes.len(),"proof_eip_bytes":decode(&eip.proof_neg_a_eip2537_hex)?.len(),"eip":eip,
            "constraints":constraints,"witness_ms":witness_ms,"prove_ms":prove_ms,"verify_ms":verify_ms,"prepare_vk_ms":prepare_vk_ms,
            "witness_scope":cfg["witness_timing_scope"],"peak_rss_mib":rss_mib()?,
            "rss_scope":"OS process high-water mark for this sole batch worker, including witness, PK decode, Prove, Verify and export; not Prove-only interval peak",
            "old_state":flat(&c.old_state),"new_state":flat(&c.new_state),"omission_a_new_state":flat(&omitted),
            "root":format!("0x{}",hex(&c.batch_root)),"config_sha256":hash_file(CONFIG)?});
        let profile_t = Instant::now();
        write_new(run.join(format!("batch_{index}.json")), &output)?;
        profile.mark("output_write", profile_t);
    }
    Ok(())
}
fn spawn_worker(run: &Path, phase: &str, mode: &str, index: usize, cfg: &Value) -> Result<Value> {
    let label = format!("{mode}-{index}");
    let stdout = run.join(format!("{label}.stdout.log"));
    let stderr = run.join(format!("{label}.stderr.log"));
    let args = [
        mode,
        run.to_str().ok_or("run path")?,
        phase,
        &index.to_string(),
        "--approved",
        phase,
    ];
    let t = Instant::now();
    let mut child = Command::new(env::current_exe()?)
        .args(args)
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
        if let Some(status) = child.try_wait()? {
            let v = json!({"label":label,"pid":child.id(),"exit_code":status.code(),"wall_ms":t.elapsed().as_secs_f64()*1000.0,"argv":args});
            write_new(run.join(format!("{label}.command.json")), &v)?;
            if !status.success() {
                return Err(format!("worker failed: {label}; logs preserved").into());
            }
            return Ok(v);
        }
        let mut reason = None;
        if t.elapsed().as_secs()
            > cfg["budgets"]["child_seconds"]
                .as_u64()
                .ok_or("time budget")?
        {
            reason = Some("WALL_BUDGET");
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
    write_new(
        run.join("request.json"),
        &json!({"phase":phase,"config_sha256":hash_file(CONFIG)?,
        "run_identity":run.to_str(),"pid":std::process::id(),"unix_nanos":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos().to_string(),
        "fresh_state_proofs_required":count,"archived_state_proofs_reused":0}),
    )?;
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
    // Setup occurs once in its own process; each Prove has a distinct process afterwards.
    let mut commands = vec![spawn_worker(run, phase, "setup-worker", 1, cfg)?];
    let mut batches = Vec::new();
    for index in 1..=count {
        println!("{phase}: generating fresh state proof {index}/{count} in one independent worker");
        commands.push(spawn_worker(run, phase, "prove-worker", index, cfg)?);
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
        "ballots":ballots,"batches":batches,"commands":commands,"all_state_proofs_fresh":true,"serial_workers":true,
        "total_wall_ms":start.elapsed().as_secs_f64()*1000.0,"original_ballot_proofs_reused":32,"original_ballot_proofs_generated":0,
        "state_proofs_generated":count,"archived_state_proofs_reused":0,"production_setup":false}),
    )?;
    println!("{phase}: complete materials, no EVM execution; {count} fresh state proofs");
    Ok(())
}
fn main() -> Result<()> {
    let process_start = Instant::now();
    let mut profile = Profile::default();
    let result: Result<()> = (|| {
        let a = env::args().skip(1).collect::<Vec<_>>();
        if a.len() < 5 {
            return Err("usage: diagnostic prove-worker RUN B 1 --approved B".into());
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
        // Diagnostic-only guard: prevent prepare, Setup, batch 2 or Gate C execution.
        if a.len() != 6
            || mode != "prove-worker"
            || phase != "B"
            || a[3] != "1"
            || run != Path::new("experiments/stage1_multibatch_raw/pk-profile-batch1")
        {
            return Err("only the authorized single Batch 1 diagnostic is permitted".into());
        }
        let cfg = config()?;
        profile.process_config_ns = Some(process_start.elapsed().as_nanos());
        let worker_start = Instant::now();
        let result = worker(mode, &run, phase, a[3].parse()?, &cfg, &mut profile);
        profile.worker_total_ns = Some(worker_start.elapsed().as_nanos());
        result
    })();
    if let Err(e) = profile.save(&result) {
        eprintln!("Could not persist profile: {e}");
        if result.is_ok() {
            return Err(e);
        }
    }
    result
}
