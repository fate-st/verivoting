//! Public, proof-derived Fp12 multiplication vector. No setup/prove or secrets.
use bellperson::groth16::aggregate::AggregateProof;
use blstrs::{Bls12, Gt};
use serde_json::{json, Value};
use std::{env, error::Error, fs, io::Cursor, path::PathBuf, process::Command};
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn decode(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}

fn encode_fp12(value: &Gt) -> Result<String> {
    // blstrs GT serde exposes canonical Fp12 coefficients as 6 LE u64 limbs.
    // Convert directly to 12 padded 64-byte BE coefficients, without u64->f64.
    let value = serde_json::to_value(value)?;
    let mut bytes = Vec::with_capacity(768);
    for c12 in ["c0", "c1"] {
        for c6 in ["c0", "c1", "c2"] {
            for c2 in ["c0", "c1"] {
                let limbs = value[c12][c6][c2].as_array().ok_or("missing Fp limbs")?;
                if limbs.len() != 6 {
                    return Err("Fp needs six u64 limbs".into());
                }
                let mut le = Vec::with_capacity(48);
                for limb in limbs {
                    le.extend_from_slice(&limb.as_u64().ok_or("bad limb")?.to_le_bytes());
                }
                le.reverse();
                bytes.extend_from_slice(&[0u8; 16]);
                bytes.extend_from_slice(&le);
            }
        }
    }
    if bytes.len() != 768 {
        return Err("Fp12 encoding length mismatch".into());
    }
    Ok(format!("0x{}", hex(&bytes)))
}

fn metadata(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err("git metadata failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn main() -> Result<()> {
    let mut source_path = PathBuf::from("fixtures/snarkpack_g5c1_public_v1.json");
    let mut output_path = PathBuf::from("fixtures/snarkpack_g5c3_fp12_product_v1.json");
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match flag.as_str() {
            "--source" => source_path = value.into(),
            "--output" => output_path = value.into(),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    let source_bytes = fs::read(&source_path)?;
    let source: Value = serde_json::from_slice(&source_bytes)?;
    if source["schema"] != "VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1" {
        return Err("unexpected source fixture".into());
    }
    let wire = decode(
        source["aggregate_proof_hex"]
            .as_str()
            .ok_or("missing proof")?,
    )?;
    if source["aggregate_proof_sha256"] != hex(&sha256(&wire)) {
        return Err("proof digest mismatch".into());
    }
    let mut cursor = Cursor::new(&wire);
    let proof = AggregateProof::<Bls12>::read(&mut cursor)?;
    if cursor.position() as usize != wire.len() || proof.tmipp.gipa.nproofs != 2 {
        return Err("invalid B=2 wire boundary".into());
    }
    proof.parsing_check()?;
    let left = proof.com_ab.0;
    let right = proof.com_c.0;
    // blstrs uses additive group notation for GT: + means Fp12 multiplication.
    let product = left + right;
    let fixture = json!({
        "schema": "VV/G5-C3/FP12-PRODUCT/v1",
        "encoding": "VV/G5-C3/Fp12-12xFp64BE/v1",
        "source_fixture": source_path.display().to_string(),
        "source_fixture_sha256": hex(&sha256(&source_bytes)),
        "aggregate_proof_sha256": hex(&sha256(&wire)),
        "git_commit": metadata(&["rev-parse", "HEAD"] )?,
        "git_dirty": !metadata(&["status", "--porcelain"] )?.is_empty(),
        "native_implementation": "bellperson 0.27.0 wire reader; blstrs 0.7.0 GT addition (Fp12 multiplication)",
        "left_operand": "proof.com_ab.0", "right_operand": "proof.com_c.0",
        "left_wire_offset": 0, "right_wire_offset": 576,
        "left_fp12_hex": encode_fp12(&left)?,
        "right_fp12_hex": encode_fp12(&right)?,
        "product_fp12_hex": encode_fp12(&product)?,
        "relation": "Fp12 product on two real proof-derived GT operands, not a complete GIPA subexpression",
        "tower": "Fp2 u^2=-1; Fp6 v^3=1+u; Fp12 w^2=v",
        "setup_run": false, "prove_run": false, "private_witness_emitted": false,
        "pairing_to_gt_verified": false, "exact_snarkpack_evm_verified": false,
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&fixture)?)?;
    println!("G5-C3: real native proof-derived Fp12 product exported; setup/prove not run");
    Ok(())
}
