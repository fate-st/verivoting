//! G7 public archive verification/encoding ONLY. No setup, proving or private witness.
use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::ToConstraintField;
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde_json::{json, Value};
use std::{error::Error, fs, io::Write, path::Path, process::Command, time::Instant};
use vv_friendly_crypto::{
    decode_point, encode_point,
    evm_export::Eip2537PublicGroth16Proof,
    generator,
    p3ten_gadget::{G4D_INPUTS, G4D_LAYOUT, G4D_RELATION_ID, G4D_SELECTOR_INDEX, G4D_WINNER_INDEX},
};
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MANIFEST: &str = "configs/protocol_manifest.JUBJUB-G7-P3-EVM-W32.json";
const G6: &str = "fixtures/jubjub_g6_native_winner_only_public_v1.json";
const G4D: &str = "fixtures/jubjub_g4d_ten_subsets_p3_v2_public.json";
const OLD: &str = "fixtures/jubjub_g4c_partial_turnout_v2.json";
const OUTPUT: &str = "fixtures/g6_p3_eip2537_g7_v1.json";
const REPORT: &str = "experiments/g7_raw/public_export.json";

fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("missing {key}").into())
}
fn decode(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("odd hex length".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| Ok(u8::from_str_radix(std::str::from_utf8(p)?, 16)?))
        .collect()
}
fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut remaining = bytes;
    let value = T::deserialize_compressed(&mut remaining)?;
    let mut encoded = Vec::new();
    value.serialize_compressed(&mut encoded)?;
    if !remaining.is_empty() || encoded != bytes {
        return Err("noncanonical or trailing bytes".into());
    }
    Ok(value)
}
fn load(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn metadata(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd).args(args).output()?;
    if !out.status.success() {
        return Err(format!("{cmd} failed").into());
    }
    Ok(String::from_utf8(out.stdout)?.trim().into())
}
fn write_new(path: &str, v: &Value) -> Result<()> {
    fs::create_dir_all(Path::new(path).parent().ok_or("parent")?)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(v)?)?;
    file.write_all(b"\n")?;
    Ok(())
}
fn main() -> Result<()> {
    if Path::new(OUTPUT).exists() || Path::new(REPORT).exists() {
        return Err("export paths must be new; never overwrite archived results".into());
    }
    let config = load(MANIFEST)?;
    for pin in config["source_pins"].as_array().ok_or("pins")? {
        if hex(&sha256(&fs::read(string(pin, "path")?)?)) != string(pin, "sha256")? {
            return Err(format!("source pin changed: {}", pin["path"]).into());
        }
    }
    let source = load(G6)?;
    let old = load(OLD)?;
    let d = load(G4D)?;
    if source["schema"] != "VV/G6/NATIVE-WINNER-ONLY/PUBLIC/v1"
        || source["relation_id"] != G4D_RELATION_ID
        || source["input_layout"] != G4D_LAYOUT
        || source["verifying_key_hex"] != d["verifying_key_hex"]
        || source["verifying_key_sha256"] != config["verifying_key_sha256"]
        || source["proof_sha256"] != config["proof_sha256"]
        || source["subset"] != json!([1, 2, 3])
        || source["public_winner"] != 1
    {
        return Err("G6/G4-D archive or relation mismatch".into());
    }
    let vk_bytes = decode(string(&source, "verifying_key_hex")?)?;
    let proof_bytes = decode(string(&source, "proof_hex")?)?;
    if hex(&sha256(&vk_bytes)) != string(&config, "verifying_key_sha256")?
        || hex(&sha256(&proof_bytes)) != string(&config, "proof_sha256")?
        || proof_bytes.len() != 192
    {
        return Err("VK/proof digest mismatch".into());
    }
    let vk: VerifyingKey<Bls12_381> = canonical(&vk_bytes)?;
    let proof: Proof<Bls12_381> = canonical(&proof_bytes)?;
    let inputs = source["public_inputs_le_hex"]
        .as_array()
        .ok_or("inputs")?
        .iter()
        .map(|v| {
            let bytes = decode(v.as_str().ok_or("scalar hex")?)?;
            if bytes.len() != 32 {
                return Err("scalar length".into());
            }
            canonical::<Fr>(&bytes)
        })
        .collect::<Result<Vec<_>>>()?;
    if inputs.len() != G4D_INPUTS
        || vk.gamma_abc_g1.len() != 61
        || inputs[G4D_SELECTOR_INDEX] != Fr::from(0u64)
        || inputs[G4D_WINNER_INDEX] != Fr::from(1u64)
        || inputs[58] != Fr::from(32u64)
    {
        return Err("public input layout mismatch".into());
    }
    // Match the public canonical C1 encodings to the actual frozen CT2 before export.
    let ciphertexts = source["public_context"]["frozen_ciphertexts"]
        .as_array()
        .ok_or("CT2")?;
    if ciphertexts.len() != 3 {
        return Err("three candidates required".into());
    }
    for (j, ct) in ciphertexts.iter().enumerate() {
        let point = decode_point(&decode(string(ct, "c1_hex")?)?, true)?;
        let packed: Vec<Fr> = encode_point(&point).to_field_elements().ok_or("packing")?;
        if packed.len() != 2 || inputs[52 + j * 2..54 + j * 2] != packed {
            return Err("P3 C1 inputs differ from frozen CT2".into());
        }
    }
    // A canonical, in-subgroup alternative ciphertext component, not malformed field data.
    let c1 = decode_point(&decode(string(&ciphertexts[0], "c1_hex")?)?, true)?;
    let changed_c1 = encode_point(&(c1 + generator()));
    let packed: Vec<Fr> = changed_c1.to_field_elements().ok_or("changed packing")?;
    let mut selector = inputs.clone();
    selector[G4D_SELECTOR_INDEX] = Fr::from(1u64);
    let mut winner = inputs.clone();
    winner[G4D_WINNER_INDEX] = Fr::from(2u64);
    let mut ciphertext = inputs.clone();
    ciphertext[52..54].copy_from_slice(&packed);
    let started = Instant::now();
    let pvk = prepare_verifying_key(&vk);
    let prepare_ms = started.elapsed().as_secs_f64() * 1000.;
    let mut checks = Vec::new();
    for (name, values, expected) in [
        ("valid", &inputs, true),
        ("wrong_selector", &selector, false),
        ("wrong_winner", &winner, false),
        ("changed_ciphertext", &ciphertext, false),
    ] {
        let start = Instant::now();
        let accepted = Groth16::<Bls12_381>::verify_proof(&pvk, &proof, values)?;
        if accepted != expected {
            return Err(format!("native {name} unexpected result").into());
        }
        checks.push(json!({"case":name,"accepted":accepted,"verify_ms":start.elapsed().as_secs_f64()*1000.}));
    }
    let old_vk: VerifyingKey<Bls12_381> =
        canonical(&decode(string(&old, "verifying_key_ark_compressed_hex")?)?)?;
    let start = Instant::now();
    let accepted =
        Groth16::<Bls12_381>::verify_proof(&prepare_verifying_key(&old_vk), &proof, &inputs)?;
    if accepted {
        return Err("wrong actual fixed-123 VK accepted G6 proof".into());
    }
    checks.push(json!({"case":"wrong_vk","accepted":false,"prepare_and_verify_ms":start.elapsed().as_secs_f64()*1000.}));
    let convert = |v: &[Fr]| Eip2537PublicGroth16Proof::from_proof(&vk, &proof, v);
    let material = convert(&inputs)?;
    let old_material = Eip2537PublicGroth16Proof::from_proof(&old_vk, &proof, &inputs)?;
    if old_material.vk_eip2537_hex
        != load("fixtures/g4c_eip2537_fixture_v2.json")?["vk_eip2537_hex"]
    {
        return Err("old VK conversion differs from actual G5-D archived VK".into());
    }
    let fixture = json!({
        "schema":"VV/G7/G6-P3-EIP2537/v1","relation_id":G4D_RELATION_ID,"input_layout":G4D_LAYOUT,
        "curve":"BLS12-381","encoding":material.encoding,"public_input_count":60,
        "vk_eip2537_hex":material.vk_eip2537_hex,"verifying_key_ark_sha256":config["verifying_key_sha256"],
        "proof_neg_a_eip2537_hex":material.proof_neg_a_eip2537_hex,"proof_ark_sha256":config["proof_sha256"],
        "public_inputs_be_hex":material.public_inputs_be_hex,
        "wrong_selector_inputs_be_hex":convert(&selector)?.public_inputs_be_hex,
        "wrong_winner_inputs_be_hex":convert(&winner)?.public_inputs_be_hex,
        "changed_ciphertext_inputs_be_hex":convert(&ciphertext)?.public_inputs_be_hex,
        "changed_ciphertext_c1_hex":hex(&changed_c1),
        "ciphertext_mutation":"candidate 1 C1 := frozen C1 + Jubjub generator; only canonical packed C1 fields 52/53 replaced; CP prefix unchanged",
        "wrong_vk_ark_sha256":old["verifying_key_sha256"],"wrong_vk_eip2537_hex":old_material.vk_eip2537_hex,
        "source_fixture":G6,"source_fixture_sha256":hex(&sha256(&fs::read(G6)?)),
        "freeze_digest_hex":source["freeze_digest_hex"],"freeze_enforced_by_this_verifier":false,
        "setup_run":false,"proof_generation_run":false,"private_witness_emitted":false,"production_setup":false
    });
    let source_hashes = [
        MANIFEST,
        "crates/vv-friendly-dkg/src/bin/g7_p3_eip2537_export.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "crates/vv-friendly-crypto/src/evm_export.rs",
        "crates/vv-friendly-crypto/src/p3ten_gadget.rs",
        "Cargo.lock",
        "rust-toolchain.toml",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect::<Result<Vec<_>>>()?;
    write_new(OUTPUT, &fixture)?;
    write_new(
        REPORT,
        &json!({"gate":"G7-public-export","status":"PASS_G6_ARCHIVE_NATIVE_VERIFY_EIP2537_EXPORT",
        "git_commit":metadata("git", &["rev-parse","HEAD"] )?,
        "git_dirty":!metadata("git", &["status","--porcelain"] )?.is_empty(),
        "rustc_version":metadata("rustc", &["--version"] )?,"build_profile":if cfg!(debug_assertions){"dev"}else{"release-opt-level-1"},
        "verifying_key_sha256":config["verifying_key_sha256"],"proof_sha256":config["proof_sha256"],
        "source_pins":config["source_pins"],"source_hashes":source_hashes,
        "public_input_count":60,"prepare_vk_ms":prepare_ms,"native_checks":checks,
        "output_fixture":OUTPUT,"output_fixture_sha256":hex(&sha256(&fs::read(OUTPUT)?)),
        "setup_count":0,"proof_generation_count":0,"private_witness_emitted":false,"evm_run":false,"production_setup":false}),
    )?;
    println!("G7 public export PASS: same G6 proof, same G4-D VK; 1 positive / 4 native negatives; NO setup/prove");
    Ok(())
}
