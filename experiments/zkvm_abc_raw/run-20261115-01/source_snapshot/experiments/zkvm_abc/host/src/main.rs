use ark_bls12_381::{Bls12_381, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{Proof, VerifyingKey};
use g17_common::{canonical, compressed, sha, unhex, verify_batch, BatchInput, ORIGINAL_VK_SHA256};
use serde_json::{json, Value};
use std::{error::Error, fs, io::Write, path::Path, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn write_new(path: &Path, value: &Value) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string_pretty(value)?)?;
    Ok(())
}
fn bytes(v: &Value) -> Result<Vec<u8>> {
    Ok(unhex(v.as_str().ok_or("expected hex string")?)?)
}
fn fq(wire: &[u8]) -> Result<Fq> {
    if wire.len() != 64 {
        return Err("Fp64 required".into());
    }
    let f = Fq::from_be_bytes_mod_order(wire);
    let b = f.into_bigint().to_bytes_be();
    let mut canonical = vec![0; 64 - b.len()];
    canonical.extend(b);
    if canonical != wire {
        return Err("noncanonical EIP Fp".into());
    }
    Ok(f)
}
fn g1(wire: &[u8]) -> Result<G1Affine> {
    if wire.len() != 128 {
        return Err("G1 length".into());
    }
    let p = G1Affine::new_unchecked(fq(&wire[..64])?, fq(&wire[64..])?);
    if !p.is_on_curve() || !p.is_in_correct_subgroup_assuming_on_curve() {
        return Err("invalid G1".into());
    }
    Ok(p)
}
fn g2(wire: &[u8]) -> Result<G2Affine> {
    if wire.len() != 256 {
        return Err("G2 length".into());
    }
    let p = G2Affine::new_unchecked(
        Fq2::new(fq(&wire[..64])?, fq(&wire[64..128])?),
        Fq2::new(fq(&wire[128..192])?, fq(&wire[192..])?),
    );
    if !p.is_on_curve() || !p.is_in_correct_subgroup_assuming_on_curve() {
        return Err("invalid G2".into());
    }
    Ok(p)
}
fn prepare(repo: &Path, output: &Path) -> Result<()> {
    fs::create_dir(output)?;
    let pool: Value = serde_json::from_slice(&fs::read(
        repo.join("fixtures/snarkpack_g1d0_depth5_public_v1.json"),
    )?)?;
    let eip: Value = serde_json::from_slice(&fs::read(
        repo.join("fixtures/g8_receiver_eip2537_v1.json"),
    )?)?;
    let eip_vk = bytes(&eip["ballot_vk_eip2537_hex"])?;
    if eip_vk.len() != 896 + 26 * 128 {
        return Err("original EIP VK length".into());
    }
    let vk = VerifyingKey::<Bls12_381> {
        alpha_g1: g1(&eip_vk[..128])?,
        beta_g2: g2(&eip_vk[128..384])?,
        gamma_g2: g2(&eip_vk[384..640])?,
        delta_g2: g2(&eip_vk[640..896])?,
        gamma_abc_g1: eip_vk[896..]
            .chunks_exact(128)
            .map(g1)
            .collect::<Result<Vec<_>>>()?,
    };
    let vk_bytes = compressed(&vk)?;
    if sha(&vk_bytes) != ORIGINAL_VK_SHA256
        || pool["groth16_vk_ark_compressed_sha256"] != ORIGINAL_VK_SHA256
        || eip["ballot_vk_ark_sha256"] != ORIGINAL_VK_SHA256
    {
        return Err("original VK mismatch".into());
    }
    let mut proofs = Vec::new();
    let mut rows = Vec::new();
    for i in 0..2 {
        let original = pool["source_proofs_ark_compressed_hex"][i]
            .as_str()
            .ok_or("proof")?;
        let p: Proof<Bls12_381> = canonical(&unhex(original)?)?;
        let e = bytes(&eip["batches"][0]["ballot_proofs_eip2537_hex"][i])?;
        if e.len() != 512
            || -g1(&e[..128])? != p.a
            || g2(&e[128..384])? != p.b
            || g1(&e[384..])? != p.c
        {
            return Err("192B Ark / 512B EIP negative-A mismatch".into());
        }
        let row: Vec<String> = pool["source_public_inputs_le_hex"][i]
            .as_array()
            .ok_or("row")?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or("scalar string"))
            .collect::<std::result::Result<_, _>>()?;
        for (j, scalar) in row.iter().enumerate() {
            let mut b = unhex(scalar)?;
            b.reverse();
            if bytes(&eip["batches"][0]["ballot_inputs_be_hex"][i][j])? != b {
                return Err("original LE/BE inputs mismatch".into());
            }
        }
        proofs.push(original.to_owned());
        rows.push(row);
    }
    let mut cases = Vec::new();
    for size in [1, 2] {
        let good = BatchInput {
            vk_ark_hex: hex::encode(&vk_bytes),
            proofs_ark_hex: proofs[..size].to_vec(),
            inputs_le_hex: rows[..size].to_vec(),
        };
        verify_batch(&good)?;
        let mut bad = good.clone();
        let f = canonical::<Fr>(&unhex(&bad.inputs_le_hex[size - 1][7])?)? + Fr::from(1u64);
        bad.inputs_le_hex[size - 1][7] = hex::encode(compressed(&f)?);
        let expected_error = format!("G17_INVALID_GROTH16_{}", size - 1);
        if verify_batch(&bad).err().as_deref() != Some(&expected_error) {
            return Err("native negative did not reach pairing rejection".into());
        }
        for (suffix, input, accepted) in [("valid", good, true), ("changed-input", bad, false)] {
            let file = format!("b{size}-{suffix}.json");
            write_new(&output.join(&file), &serde_json::to_value(&input)?)?;
            cases.push(json!({"file":file,"batch_size":size,"expected_accept":accepted,
                "mutation":if accepted{Value::Null}else{json!({"row":size-1,"column":7,"operation":"Fr + 1","proof_vk_unchanged":true})},
                "input_sha256":sha(&fs::read(output.join(&file))?)}));
        }
    }
    write_new(
        &output.join("materials.json"),
        &json!({"status":"PASS_NATIVE_PUBLIC_MATERIAL_PREPARATION_NOT_GUEST",
        "original_vk_sha256":ORIGINAL_VK_SHA256,"ark_proof_bytes":192,"eip_proof_bytes":512,
        "negative_a_conversion_matches":true,"public_input_count":25,"cases":cases,
        "new_setup":0,"new_proofs":0,"private_witness_read":false}),
    )?;
    println!("PASS native input preparation only; NOT guest execution");
    Ok(())
}
fn test(input: &Path, output: &Path, guest: bool) -> Result<()> {
    let batch: BatchInput = serde_json::from_slice(&fs::read(input)?)?;
    let start = Instant::now();
    let native = verify_batch(&batch);
    let native_ms = start.elapsed().as_secs_f64() * 1000.0;
    let native_error = native.as_ref().err().cloned();
    if native_error
        .as_ref()
        .is_some_and(|s| !s.starts_with("G17_INVALID_GROTH16_"))
    {
        return Err("unexpected native parse/validation error".into());
    }
    let mut report = json!({"schema":"VV/G17/Z1/EXECUTION/v1","input_sha256":sha(&fs::read(input)?),
        "batch_size":batch.proofs_ark_hex.len(),"native_accept":native.is_ok(),"native_error":native_error,
        "native_verification_ms":native_ms,"guest_executed":false,"guest_accept":null,
        "guest_error":null,"guest_wall_ms":null,"guest_cycles":null,"segments":null,
        "journal_bytes":null,"program_id":null,"guest_elf_sha256":null,
        "receipt_kind":"NONE_EXECUTION_ONLY","real_proof_generated":false,"state_binding_implemented":false,
        "evm_gas":null});
    if guest {
        #[cfg(feature = "zkvm")]
        {
            use g17_common::VerificationOutput;
            use g17_methods::G17_GUEST_B_ELF;
            use risc0_zkvm::{compute_image_id, default_executor, ExecutorEnv, ExitCode};
            let elf = G17_GUEST_B_ELF;
            let image = compute_image_id(elf)?;
            let env = ExecutorEnv::builder().write(&batch)?.build()?;
            report["program_id"] = json!(image.to_string());
            report["guest_elf_sha256"] = json!(sha(elf));
            let started = Instant::now();
            let executed = default_executor().execute(env, elf);
            report["guest_executed"] = json!(true);
            report["guest_wall_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
            match executed {
                Ok(session) => {
                    report["guest_cycles"] = json!(session.cycles());
                    report["segments"] = json!(session.segments.len());
                    report["journal_bytes"] = json!(session.journal.bytes.len());
                    if session.exit_code != ExitCode::Halted(0) {
                        return Err("non-success session exit".into());
                    }
                    let journal: VerificationOutput = session.journal.decode()?;
                    if native.as_ref().ok() != Some(&journal) {
                        return Err("native/guest journal disagreement".into());
                    }
                    report["guest_accept"] = json!(true);
                    report["status"] = json!("PASS_GUEST");
                }
                Err(e) => {
                    let reason = format!("{e:#}");
                    report["guest_error"] = json!(reason);
                    if let Some(expected) = native.as_ref().err() {
                        if reason.contains(expected) {
                            report["guest_accept"] = json!(false);
                            report["status"] = json!("PASS_GUEST_CRYPTOGRAPHIC_REJECTION");
                        } else {
                            report["status"] = json!("BLOCKED_GUEST_EXECUTION");
                        }
                    } else {
                        report["status"] = json!("BLOCKED_GUEST_EXECUTION");
                    }
                }
            }
        }
        #[cfg(not(feature = "zkvm"))]
        {
            return Err("guest mode requires --features zkvm; no native substitute".into());
        }
    } else {
        report["status"] = json!(if native.is_ok() {
            "PASS_NATIVE_REFERENCE_ONLY"
        } else {
            "PASS_NATIVE_CRYPTOGRAPHIC_REJECTION_ONLY"
        });
    }
    write_new(output, &report)?;
    println!("{}", report["status"]);
    if guest
        && !report["status"]
            .as_str()
            .is_some_and(|s| s.starts_with("PASS_GUEST"))
    {
        return Err("guest execution did not meet acceptance".into());
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "usage: g17-host prepare REPO NEW_MATERIALS_DIR | native/guest INPUT NEW_RESULT_JSON"
                .into(),
        );
    }
    match args[0].as_str() {
        "prepare" => prepare(Path::new(&args[1]), Path::new(&args[2])),
        "native" => test(Path::new(&args[1]), Path::new(&args[2]), false),
        "guest" => test(Path::new(&args[1]), Path::new(&args[2]), true),
        _ => Err("unknown mode".into()),
    }
}
