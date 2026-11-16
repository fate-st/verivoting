//! Verifies archived original inputs only. This is NOT SnarkMux.
use bellperson::groth16::{prepare_verifying_key, verify_proof, Proof, VerifyingKey};
use blstrs::{Bls12, Scalar};
use serde_json::{json, Value};
use std::{error::Error, fs, io::Cursor, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn bytes(v: &Value) -> Result<Vec<u8>> {
    let s = v.as_str().ok_or("hex string required")?;
    if !s.len().is_multiple_of(2) {
        return Err("odd hex encoding".into());
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        return Err("usage: snarkmux-input-check INPUT_BUNDLE.json".into());
    }
    let input: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    let raw_vk = bytes(&input["vk_hex"])?;
    let mut reader = Cursor::new(&raw_vk);
    let vk = VerifyingKey::<Bls12>::read(&mut reader)?;
    let mut back = Vec::new();
    vk.write(&mut back)?;
    if reader.position() as usize != raw_vk.len() || back != raw_vk || vk.ic.len() != 26 {
        return Err("noncanonical or wrong-arity original VK".into());
    }
    let rows = input["public_inputs_le_hex"]
        .as_array()
        .ok_or("input rows")?;
    let wires = input["proofs_hex"].as_array().ok_or("proof rows")?;
    if rows.len() != 5 || wires.len() != 5 {
        return Err("four ballots plus one valid replacement control required".into());
    }
    let mut inputs = Vec::new();
    let mut proofs = Vec::new();
    let mut sizes = Vec::new();
    for (row, wire) in rows.iter().zip(wires) {
        let row = row.as_array().ok_or("scalar row")?;
        if row.len() != 25 {
            return Err("exactly 25 public inputs required".into());
        }
        let scalars = row
            .iter()
            .map(|x| {
                let b: [u8; 32] = bytes(x)?.try_into().map_err(|_| "scalar width")?;
                Option::<Scalar>::from(Scalar::from_bytes_le(&b))
                    .ok_or_else(|| "noncanonical scalar; no reduction allowed".into())
            })
            .collect::<Result<Vec<_>>>()?;
        let raw = bytes(wire)?;
        let mut reader = Cursor::new(&raw);
        let proof = Proof::<Bls12>::read(&mut reader)?;
        let mut back = Vec::new();
        proof.write(&mut back)?;
        if reader.position() as usize != raw.len() || back != raw {
            return Err("noncanonical original proof".into());
        }
        sizes.push(back.len());
        proofs.push(proof);
        inputs.push(scalars);
    }
    let t = Instant::now();
    let pvk = prepare_verifying_key(&vk);
    let prepare_vk_ms = t.elapsed().as_secs_f64() * 1000.;
    let mut checks = Vec::new();
    for i in 0..4 {
        let t = Instant::now();
        let valid = verify_proof(&pvk, &proofs[i], &inputs[i])?;
        let verify_ms = t.elapsed().as_secs_f64() * 1000.;
        if !valid {
            return Err(format!("original ballot {} rejected", i + 1).into());
        }
        checks.push(
            json!({"ballot_position":i+1,"verified":valid,"verify_ms":verify_ms,
            "proof_bytes":sizes[i],"public_inputs":inputs[i].len()}),
        );
    }
    let mut changed = inputs[1].clone();
    changed[7] += Scalar::from(1u64);
    let changed_rejected = !verify_proof(&pvk, &proofs[1], &changed)?;
    let swapped_rejected = !verify_proof(&pvk, &proofs[0], &inputs[1])?;
    let control_valid = verify_proof(&pvk, &proofs[4], &inputs[4])?;
    let replacement_rejected = !verify_proof(&pvk, &proofs[4], &inputs[2])?;
    if !changed_rejected || !swapped_rejected || !control_valid || !replacement_rejected {
        return Err("original Groth16 binding control failed".into());
    }
    let total: f64 = checks
        .iter()
        .map(|x| x["verify_ms"].as_f64().unwrap())
        .sum();
    println!(
        "{}",
        json!({"status":"PASS_ORIGINAL_B4_GROTH16_INPUTS","proof_system":"Groth16",
        "pairing_curve":"BLS12-381","implementation":"bellperson 0.27.0 / blstrs 0.7.0",
        "original_valid_count":4,"original_ballot_count":4,"public_inputs":vk.ic.len()-1,
        "vk_bytes":raw_vk.len(),"checks":checks,"vk_prepare_ms":prepare_vk_ms,
        "four_individual_verify_ms":total,"samples":1,
        "native_binding_controls":{"modified_ballot2_nullifier_field_rejected":changed_rejected,
            "swapped_inputs_rejected":swapped_rejected,"fifth_ballot_valid_under_own_inputs":control_valid,
            "fifth_proof_rejected_under_original_third_inputs":replacement_rejected},
        "controls_are_not_snarkmux_tests":true,"new_proofs":0,"new_setups":0,"evm_executed":false})
    );
    Ok(())
}
