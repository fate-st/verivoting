//! One official loop-program proof smoke test, not a ballot or A/B benchmark.
use risc0_binfmt::ProgramBinary;
use risc0_zkos_v1compat::V1COMPAT_ELF;
use risc0_zkvm::{compute_image_id, ExecutorEnv, ExternalProver, InnerReceipt, Prover, ProverOpts};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
use std::{error::Error, fs, io::Write, path::Path, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn save(path: &Path, b: &[u8]) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    f.write_all(b)?;
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: g18-prover-admission prepare|smoke RUN_DIR OFFICIAL_LOOP_ELF".into());
    }
    let run = Path::new(&args[1]);
    let config: Value = serde_json::from_slice(&fs::read("configs/experiment.G18-DECISION.json")?)?;
    if std::env::var("RISC0_DEV_MODE").as_deref() != Ok("0") {
        return Err("RISC0_DEV_MODE must be explicitly 0".into());
    }
    let user_elf = fs::read(&args[2])?;
    if config["official_smoke"]["user_elf_sha256"] != hash(&user_elf)
        || config["official_smoke"]["kernel_elf_sha256"] != hash(V1COMPAT_ELF)
    {
        return Err("official binary/hash mismatch".into());
    }
    let program = ProgramBinary::new(&user_elf, V1COMPAT_ELF).encode();
    let image = compute_image_id(&program)?;
    if args[0] == "prepare" {
        fs::create_dir(run.join("official_smoke"))?;
        save(&run.join("official_smoke/guest.elf"), &user_elf)?;
        save(&run.join("official_smoke/program.bin"), &program)?;
        save(&run.join("official_smoke/input.bin"), &0u32.to_le_bytes())?;
        let j = json!({"image_id":image.to_string(),"user_elf_sha256":hash(&user_elf),
            "kernel_elf_sha256":hash(V1COMPAT_ELF),"program_sha256":hash(&program),
            "input_sha256":hash(&0u32.to_le_bytes()),"expected_journal_hex":"",
            "official_source":"risc0-zkvm 3.1.0 examples/datasheet.rs + loop.bin; loop count 0",
            "ballot_proof_test":false,"expected_receipt_kind":"Composite"});
        save(
            &run.join("official_smoke/program.json"),
            serde_json::to_string_pretty(&j)?.as_bytes(),
        )?;
        println!("{}", serde_json::to_string_pretty(&j)?);
        return Ok(());
    }
    if args[0] != "smoke" {
        return Err("expected prepare|smoke".into());
    }
    let server = std::env::var("RISC0_SERVER_PATH")?;
    let env = ExecutorEnv::builder()
        .write_slice(&0u32.to_le_bytes())
        .build()?;
    let opts = ProverOpts::composite().with_dev_mode(false);
    let prover = ExternalProver::new("G18_PINNED_LOCAL_R0VM", server);
    let mut j = json!({"schema":"VV/G18/OFFICIAL-SMOKE/v1","image_id":image.to_string(),
        "program_sha256":hash(&program),"dev_mode":false,"remote_prover":false,
        "proof_generated":false,"receipt_verified":false,"journal_verified":false,
        "proof_ms":null,"verify_ms":null,"receipt_bytes":null,"receipt_sha256":null,
        "stats":null,"error":null,"ballot_proof_test":false,"evm_wrapper_test":false});
    let started = Instant::now();
    let result = prover.prove_with_opts(env, &program, &opts);
    j["attempt_wall_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
    match result {
        Err(e) => {
            j["status"] = json!("FAIL_OFFICIAL_PROVER_SMOKE");
            j["error"] = json!(format!("{e:#}"));
        }
        Ok(info) => {
            j["proof_ms"] = j["attempt_wall_ms"].clone();
            j["stats"] = serde_json::to_value(&info.stats)?;
            if !matches!(&info.receipt.inner, InnerReceipt::Composite(_)) {
                return Err("not a real Composite receipt; fake/other kinds forbidden".into());
            }
            let bytes = bincode::serialize(&info.receipt)?;
            save(&run.join("official_smoke/receipt.bincode"), &bytes)?;
            j["proof_generated"] = json!(true);
            j["receipt_bytes"] = json!(bytes.len());
            j["receipt_sha256"] = json!(hash(&bytes));
            let verifying = Instant::now();
            match info.receipt.verify(image) {
                Err(e) => {
                    j["status"] = json!("FAIL_RECEIPT_CRYPTOGRAPHIC_VERIFICATION");
                    j["error"] = json!(format!("{e:#}"));
                }
                Ok(()) => {
                    j["receipt_verified"] = json!(true);
                    j["journal_verified"] = json!(info.receipt.journal.bytes.is_empty());
                    j["status"] = json!(if info.receipt.journal.bytes.is_empty() {
                        "PASS_REAL_OFFICIAL_COMPOSITE_RECEIPT"
                    } else {
                        "FAIL_OFFICIAL_JOURNAL"
                    });
                }
            }
            j["verify_ms"] = json!(verifying.elapsed().as_secs_f64() * 1000.0);
        }
    }
    save(
        &run.join("official_smoke/result.json"),
        serde_json::to_string_pretty(&j)?.as_bytes(),
    )?;
    println!("{}", serde_json::to_string_pretty(&j)?);
    if j["status"] != "PASS_REAL_OFFICIAL_COMPOSITE_RECEIPT" {
        return Err("official proof smoke did not pass; inspect original error/logs".into());
    }
    Ok(())
}
