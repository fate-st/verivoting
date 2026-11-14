//! One real B=2 TIPP endpoint, not a complete SnarkPack or EVM verifier.
//! Reuses C2's captured official challenge; the JS runner independently
//! reconstructs all challenges from the original proof and external statement.
use bellperson::groth16::aggregate::AggregateProof;
use blstrs::{pairing, Bls12, G1Affine, Gt, Scalar};
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

fn scalar(bytes: &[u8]) -> Result<Scalar> {
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| "scalar length")?;
    Option::<Scalar>::from(Scalar::from_bytes_le(&bytes))
        .ok_or_else(|| "noncanonical scalar".into())
}

fn encode_fp12(value: &Gt) -> Result<String> {
    // Exact six-u64 LE limbs per Fp. Never round a u64 through a JSON float.
    let value = serde_json::to_value(value)?;
    let mut bytes = Vec::with_capacity(768);
    for c12 in ["c0", "c1"] {
        for c6 in ["c0", "c1", "c2"] {
            for c2 in ["c0", "c1"] {
                let limbs = value[c12][c6][c2].as_array().ok_or("Fp limbs")?;
                if limbs.len() != 6 {
                    return Err("Fp limb count".into());
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
        return Err("Fp12 encoding length".into());
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
    let source_path = "fixtures/snarkpack_g5c1_public_v1.json";
    let trace_path = "fixtures/snarkpack_g5c2_transcript_v1.json";
    let mut output_path = PathBuf::from("fixtures/snarkpack_g5c3_tipp_endpoint_v1.json");
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--output" => output_path = args.next().ok_or("missing output")?.into(),
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    let source_bytes = fs::read(source_path)?;
    let source: Value = serde_json::from_slice(&source_bytes)?;
    let trace_bytes = fs::read(trace_path)?;
    let trace: Value = serde_json::from_slice(&trace_bytes)?;
    if source["schema"] != "VV/G5-C1/SNARKPACK-PUBLIC-FIXTURE/v1"
        || trace["schema"] != "VV/G5-C2/SNARKPACK-INTERNAL-TRANSCRIPT/v1"
        || trace["fixture"] != source_path
        || trace["fixture_sha256"] != hex(&sha256(&source_bytes))
        || trace["counter_bytes"] != 8
    {
        return Err("unbound source/C2 trace".into());
    }
    let wire = decode(source["aggregate_proof_hex"].as_str().ok_or("proof")?)?;
    let proof_hash = hex(&sha256(&wire));
    if source["aggregate_proof_sha256"] != proof_hash
        || trace["aggregate_proof_sha256"] != proof_hash
    {
        return Err("proof digest mismatch".into());
    }
    let mut cursor = Cursor::new(&wire);
    let proof = AggregateProof::<Bls12>::read(&mut cursor)?;
    if cursor.position() as usize != wire.len()
        || proof.tmipp.gipa.nproofs != 2
        || proof.tmipp.gipa.z_ab.len() != 1
    {
        return Err("requires exact B=2 boundary and one GIPA round".into());
    }
    proof.parsing_check()?;
    let rows = trace["valid"].as_array().ok_or("C2 rows")?;
    if rows.len() != 6 || rows[3]["id"] != "gipa-0-round" || rows[4]["id"] != "extra-link" {
        return Err("unexpected official challenge rows".into());
    }
    let c_inv = scalar(&decode(
        rows[3]["challenge_le_hex"].as_str().ok_or("c_inv")?,
    )?)?;
    // Official extra-link serializes challenges.last() == inverse(c_inv) first.
    let extra_base = decode(rows[4]["base_hex"].as_str().ok_or("extra base")?)?;
    let prefix = b"snarkpack-v1gipa-extra-link";
    if !extra_base.starts_with(prefix) || extra_base.len() < prefix.len() + 32 {
        return Err("bad extra-link base".into());
    }
    let c = scalar(&extra_base[prefix.len()..prefix.len() + 32])?;
    if c * c_inv != Scalar::from(1u64) {
        return Err("c/c_inv mismatch".into());
    }
    let gipa = &proof.tmipp.gipa;
    // blstrs GT additive notation: + is Fp12 multiplication; * is exponentiation.
    // verify.rs initializes zab = ip_ab, then adds z_l*c and z_r*c_inv.
    let target = proof.ip_ab + gipa.z_ab[0].0 * c + gipa.z_ab[0].1 * c_inv;
    let paired = pairing(&gipa.final_a, &gipa.final_b);
    let identity = pairing(&G1Affine::default(), &gipa.final_b);
    // Remain in GT: multiply only the expected target by the existing ip_ab.
    let wrong_target = target + proof.ip_ab;
    if target != paired
        || target == identity
        || proof.ip_ab == identity
        || wrong_target == paired
        || wrong_target == identity
    {
        return Err("endpoint positive/negative failed".into());
    }
    let fixture = json!({
        "schema": "VV/G5-C3/TIPP-ENDPOINT/v1",
        "encoding": "VV/G5-C3/Fp12-12xFp64BE/v1",
        "source_fixture": source_path, "source_fixture_sha256": hex(&sha256(&source_bytes)),
        "aggregate_proof_sha256": proof_hash,
        "c2_trace_fixture": trace_path, "c2_trace_fixture_sha256": hex(&sha256(&trace_bytes)),
        "git_commit": metadata(&["rev-parse", "HEAD"] )?,
        "git_dirty": !metadata(&["status", "--porcelain"] )?.is_empty(),
        "challenge_source": "saved official C2 trace; independently recomputed by g5c3_tipp_endpoint.mjs",
        "c_le_hex": hex(&c.to_bytes_le()), "c_inv_le_hex": hex(&c_inv.to_bytes_le()),
        "relation": "e(final_a,final_b) = ip_ab * z_ab[0].0^c * z_ab[0].1^c_inv",
        "wire_offsets": { "ip_ab": 1152, "z_ab_left": 3796, "z_ab_right": 4084,
            "final_a": 4468, "final_b": 4516 },
        "final_a_compressed_hex": hex(&gipa.final_a.to_compressed()),
        "final_b_compressed_hex": hex(&gipa.final_b.to_compressed()),
        "final_a_uncompressed_hex": hex(&gipa.final_a.to_uncompressed()),
        "final_b_uncompressed_hex": hex(&gipa.final_b.to_uncompressed()),
        "target_fp12_hex": encode_fp12(&target)?, "pairing_fp12_hex": encode_fp12(&paired)?,
        "wrong_target_fp12_hex": encode_fp12(&wrong_target)?,
        "wrong_target_mutation": "target multiplied by proof.ip_ab; all points and challenges unchanged",
        "native_endpoint_verified": true, "native_wrong_gt_target_rejected": true,
        "target_is_nonunit_gt": true, "wrong_target_is_nonunit_gt": true,
        "native_implementation": "bellperson 0.27.0 wire reader; blstrs 0.7.0 pairing and GT operations",
        "pairing_to_gt_evm_verified": false, "exact_snarkpack_evm_verified": false,
        "setup_run": false, "prove_run": false, "private_witness_emitted": false,
    });
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&fixture)?)?;
    println!("G5-C3-B endpoint fixture: native equality PASS; wrong in-subgroup target rejected; no setup/prove");
    Ok(())
}
