//! Executed Rust reference vectors from existing public ciphertexts only.
use ark_ff::Zero;
use serde_json::{json, Value};
use std::{error::Error, fs, io::Write};
use vv_friendly_crypto::{decode_point, encode_point, generator, Point};
use vv_types::{hex, sha256};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn bytes(s: &str) -> Result<Vec<u8>> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
        .collect()
}
fn point(v: &Value) -> Result<Point> {
    Ok(decode_point(&bytes(v.as_str().ok_or("point hex")?)?, true)?)
}
fn flat(v: &Value) -> Result<Vec<Point>> {
    v.as_array()
        .ok_or("ciphertexts")?
        .iter()
        .flat_map(|ct| [&ct["c0_hex"], &ct["c1_hex"]])
        .map(point)
        .collect()
}
fn main() -> Result<()> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: stage2_point_vectors NEW_JSON")?;
    let source = "fixtures/g8_receiver_eip2537_v1.json";
    let source_bytes = fs::read(source)?;
    let f: Value = serde_json::from_slice(&source_bytes)?;
    let pool: Value = serde_json::from_slice(&fs::read(
        "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json",
    )?)?;
    let p = point(&pool["batches"][0]["source_public_records"][0]["ciphertexts"][0]["c0_hex"])?;
    let q = point(&pool["batches"][0]["source_public_records"][1]["ciphertexts"][0]["c0_hex"])?;
    let zero = Point::zero();
    let cases = [("identity",p,zero),("inverse",p,-p),("two_real_ciphertext_points",p,q),("doubling",p,p)]
        .into_iter().map(|(label,a,b)|json!({"label":label,"a":hex(&encode_point(&a)),"b":hex(&encode_point(&b)),"sum":hex(&encode_point(&(a+b)))})).collect::<Vec<_>>();
    let mut current = vec![zero; 6];
    let mut folds = Vec::new();
    for i in 0..2 {
        if flat(&f["batches"][i]["old_state"])? != current {
            return Err("reference old state mismatch".into());
        }
        for row in pool["batches"][i]["source_public_records"]
            .as_array()
            .ok_or("rows")?
        {
            for (sum, p) in current.iter_mut().zip(flat(&row["ciphertexts"])?) {
                *sum += p;
            }
        }
        if flat(&f["batches"][i]["new_state"])? != current {
            return Err("reference CT mismatch".into());
        }
        folds.push(json!({"batch_index":i+1,"state_hex":current.iter().map(|p|hex(&encode_point(p))).collect::<Vec<_>>()}));
    }
    let rounds = [zero, generator(), p, -p]
        .iter()
        .map(|p| hex(&encode_point(p)))
        .collect::<Vec<_>>();
    let result = json!({"schema":"VV/STAGE2/JUBJUB-POINT-REFERENCE/v1","status":"PASS_RUST_PUBLIC_REFERENCE",
        "source_fixture":source,"source_sha256":hex(&sha256(&source_bytes)),"roundtrips":rounds,"additions":cases,"folds":folds,
        "group_generator_not_zero":!generator().is_zero(),
        "new_setup":0,"new_proofs":0,"dkg_executed":false});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&serde_json::to_vec_pretty(&result)?)?;
    file.write_all(b"\n")?;
    println!(
        "PASS Rust Jubjub reference: four roundtrips, four additions, CT1/CT2; zero Setup/Prove"
    );
    Ok(())
}
