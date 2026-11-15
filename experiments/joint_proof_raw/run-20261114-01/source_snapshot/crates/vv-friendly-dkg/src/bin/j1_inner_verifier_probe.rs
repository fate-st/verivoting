//! J1 public-fixture preflight only. NOT an in-circuit verifier.
//! No voter witness, Setup, Prove, DKG or R1CS construction is performed.
use ark_bls12_381::{Bls12_381, Fq, Fr, G1Affine, G2Affine};
use ark_ff::PrimeField;
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::VerifyingKey as BellVk;
use blstrs::Bls12;
use serde_json::{json, Value};
use std::{error::Error, fs, io::Cursor, io::Write, path::Path, time::Instant};
use vv_friendly_crypto::evm_export::Eip2537PublicGroth16Proof;
use vv_types::{hex, sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const POOL: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const STATE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";
const EIP: &str = "fixtures/g8_receiver_eip2537_v1.json";

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("missing {key}").into())
}

fn unhex(s: &str) -> Result<Vec<u8>> {
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

fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    value.serialize_compressed(&mut out)?;
    Ok(out)
}

fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut remaining = bytes;
    let value = T::deserialize_compressed(&mut remaining)?;
    if !remaining.is_empty() || compressed(&value)? != bytes {
        return Err("noncanonical public archive".into());
    }
    Ok(value)
}

fn main() -> Result<()> {
    let output = std::env::args().nth(1).ok_or("NEW_OUTPUT_JSON required")?;
    if Path::new(&output).exists() {
        return Err("refuse overwrite".into());
    }
    let started = Instant::now();
    let pool: Value = serde_json::from_slice(&fs::read(POOL)?)?;
    let state: Value = serde_json::from_slice(&fs::read(STATE)?)?;
    let eip: Value = serde_json::from_slice(&fs::read(EIP)?)?;
    let vk_wire = unhex(text(&pool, "groth16_vk_hex")?)?;
    if state["source_ballot_vk_hex"] != pool["groth16_vk_hex"] {
        return Err("source VK differs".into());
    }
    let mut cursor = Cursor::new(&vk_wire);
    let bell = BellVk::<Bls12>::read(&mut cursor)?;
    let mut roundtrip = Vec::new();
    bell.write(&mut roundtrip)?;
    if cursor.position() as usize != vk_wire.len() || roundtrip != vk_wire {
        return Err("noncanonical bellperson VK".into());
    }
    let vk = VerifyingKey::<Bls12_381> {
        alpha_g1: canonical::<G1Affine>(&bell.alpha_g1.to_compressed())?,
        beta_g2: canonical::<G2Affine>(&bell.beta_g2.to_compressed())?,
        gamma_g2: canonical::<G2Affine>(&bell.gamma_g2.to_compressed())?,
        delta_g2: canonical::<G2Affine>(&bell.delta_g2.to_compressed())?,
        gamma_abc_g1: bell
            .ic
            .iter()
            .map(|p| canonical::<G1Affine>(&p.to_compressed()))
            .collect::<Result<Vec<_>>>()?,
    };
    let ark_vk = compressed(&vk)?;
    if hex(&sha256(&ark_vk)) != text(&pool, "groth16_vk_ark_compressed_sha256")?
        || eip["ballot_vk_ark_sha256"] != pool["groth16_vk_ark_compressed_sha256"]
        || vk.gamma_abc_g1.len() != 26
    {
        return Err("original ballot VK/25-input layout differs".into());
    }
    let batch = &state["batches"][0];
    if batch["source_ballot_proofs_hex"][0] != pool["source_proofs_ark_compressed_hex"][0]
        || batch["source_public_inputs_le_hex"][0] != pool["source_public_inputs_le_hex"][0]
    {
        return Err("not same first D5/W32 ballot".into());
    }
    let proof_bytes = unhex(
        batch["source_ballot_proofs_hex"][0]
            .as_str()
            .ok_or("proof")?,
    )?;
    let proof: Proof<Bls12_381> = canonical(&proof_bytes)?;
    let inputs = batch["source_public_inputs_le_hex"][0]
        .as_array()
        .ok_or("inputs")?
        .iter()
        .map(|v| canonical::<Fr>(&unhex(v.as_str().ok_or("scalar")?)?))
        .collect::<Result<Vec<_>>>()?;
    if inputs.len() != 25 || proof_bytes.len() != 192 {
        return Err("archive shape".into());
    }
    // This native check validates the PUBLIC fixture, not the recursive claim.
    // Its result is NEVER allocated as an R1CS witness or counted as a J1 pass.
    let preflight =
        Groth16::<Bls12_381>::verify_proof(&prepare_verifying_key(&vk), &proof, &inputs)?;
    if !preflight {
        return Err("original public proof invalid".into());
    }
    // Use the original exporter: Ark proof stores A; EIP wire stores -A,B,C.
    let exported = Eip2537PublicGroth16Proof::from_proof(&vk, &proof, &inputs)?;
    let first = &eip["batches"][0];
    if first["ballot_proofs_eip2537_hex"][0] != exported.proof_neg_a_eip2537_hex
        || first["ballot_inputs_be_hex"][0] != json!(exported.public_inputs_be_hex)
        || eip["ballot_vk_eip2537_hex"] != exported.vk_eip2537_hex
        || unhex(&exported.proof_neg_a_eip2537_hex)?.len() != 512
    {
        return Err("original EIP-2537 -A proof/VK/input encoding differs".into());
    }
    let report = json!({
        "schema":"VV/J1/PUBLIC-PREFLIGHT/v1", "status":"PASS_PUBLIC_FIXTURE_PREFLIGHT_ONLY",
        "source_ballot_index":0, "source_batch":1, "registered_voters":32,
        "public_input_count":25, "native_fixture_preflight_verified":preflight,
        "ark_proof_bytes":192, "eip2537_proof_bytes":512,
        "same_original_proof_vk_and_inputs":true, "negative_a_export_matches":true,
        "proof_ark_sha256":hex(&sha256(&proof_bytes)), "vk_ark_sha256":hex(&sha256(&ark_vk)),
        "proof_eip_sha256":hex(&sha256(&unhex(&exported.proof_neg_a_eip2537_hex)?)),
        "sources":[POOL,STATE,EIP].iter().map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))}))).collect::<Result<Vec<_>>>()?,
        "inner_proof_field_bits":Fr::MODULUS_BIT_SIZE, "native_pairing_constraint_field_bits":Fq::MODULUS_BIT_SIZE,
        "current_outer_constraint_field_bits":Fr::MODULUS_BIT_SIZE,
        "in_circuit_verification_executed":false, "host_bool_allocated_as_witness":false,
        "constraint_count":null, "constraint_satisfaction":null,
        "setup_count":0, "proof_generation_count":0, "private_voter_witness_read":false,
        "elapsed_ms":started.elapsed().as_secs_f64()*1000.0,
    });
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&serde_json::to_vec_pretty(&report)?)?;
    file.write_all(b"\n")?;
    println!("PASS public fixture only: original 192-byte Ark proof / 512-byte EIP -A proof / VK / 25 inputs. NOT an inner R1CS pass.");
    Ok(())
}
