//! G8 phase 1: verify/export archived ballot+state proofs. No setup/prover/witness.
use ark_bls12_381::{Bls12_381, Fr, G1Affine, G2Affine};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::VerifyingKey as BellVk;
use blstrs::Bls12;
use serde_json::{json, Value};
use std::{
    error::Error,
    fs,
    io::{Cursor, Write},
    path::Path,
    process::Command,
};
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs, batch_root_v2_for_batch, state_public_inputs, StateBallotRecord,
    StateTransitionCircuitForBatch,
};
use vv_friendly_crypto::{decode_point, evm_export::Eip2537PublicGroth16Proof, Ciphertext};
use vv_types::{hex, sha256};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MANIFEST: &str = "configs/protocol_manifest.JUBJUB-G8-RECEIVER-FALLBACK-W32.json";
const STATE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";
const POOL: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const G6: &str = "fixtures/jubjub_g6_native_winner_only_public_v1.json";
const OUTPUT: &str = "fixtures/g8_receiver_eip2537_v1.json";
const REPORT: &str = "experiments/g8_raw/public_export.json";
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().ok_or_else(|| format!("missing {k}").into())
}
fn decode(s: &str) -> Result<Vec<u8>> {
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
fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut remaining = bytes;
    let v = T::deserialize_compressed(&mut remaining)?;
    let mut encoded = Vec::new();
    v.serialize_compressed(&mut encoded)?;
    if !remaining.is_empty() || encoded != bytes {
        return Err("noncanonical archive".into());
    }
    Ok(v)
}
fn compressed<T: CanonicalSerialize>(v: &T) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    v.serialize_compressed(&mut b)?;
    Ok(b)
}
fn load(p: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn fixed(v: &Value, k: &str) -> Result<[u8; 32]> {
    decode(string(v, k)?)?
        .try_into()
        .map_err(|_| format!("bad {k} length").into())
}
fn states(v: &Value) -> Result<Vec<Ciphertext>> {
    v.as_array()
        .ok_or("states")?
        .iter()
        .map(|r| {
            Ok(Ciphertext {
                c0: decode_point(&fixed(r, "c0_hex")?, true)?,
                c1: decode_point(&fixed(r, "c1_hex")?, true)?,
            })
        })
        .collect()
}
fn record(v: &Value) -> Result<StateBallotRecord> {
    Ok(StateBallotRecord {
        election_id: fixed(v, "election_id_hex")?,
        final_context: fixed(v, "final_context_hex")?,
        registration_root: fixed(v, "registration_root_hex")?,
        key_set_hash: fixed(v, "key_set_hash_hex")?,
        nullifier: fixed(v, "nullifier_hex")?,
        region: v["region"].as_u64().ok_or("region")?,
        public_key: decode_point(&fixed(v, "public_key_hex")?, false)?,
        ciphertexts: states(&v["ciphertexts"])?,
    })
}
fn le_inputs(v: &[Fr]) -> Result<Value> {
    Ok(json!(v
        .iter()
        .map(|x| Ok(hex(&compressed(x)?)))
        .collect::<Result<Vec<_>>>()?))
}
fn write_new(p: &str, v: &Value) -> Result<()> {
    fs::create_dir_all(Path::new(p).parent().ok_or("parent")?)?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)?;
    f.write_all(&serde_json::to_vec_pretty(v)?)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn main() -> Result<()> {
    if Path::new(OUTPUT).exists() || Path::new(REPORT).exists() {
        return Err("new export paths required".into());
    }
    let config = load(MANIFEST)?;
    for pin in config["source_pins"].as_array().ok_or("pins")? {
        if hex(&sha256(&fs::read(string(pin, "path")?)?)) != string(pin, "sha256")? {
            return Err(format!("changed {}", pin["path"]).into());
        }
    }
    let s = load(STATE)?;
    let pool = load(POOL)?;
    let g6 = load(G6)?;
    if s["source_ballot_vk_hex"] != pool["groth16_vk_hex"]
        || s["final_state"] != g6["public_context"]["frozen_ciphertexts"]
    {
        return Err("G2-E/D2/G6 differ".into());
    }
    let wire = decode(string(&pool, "groth16_vk_hex")?)?;
    let mut reader = Cursor::new(&wire);
    let bell = BellVk::<Bls12>::read(&mut reader)?;
    let mut roundtrip = Vec::new();
    bell.write(&mut roundtrip)?;
    if reader.position() as usize != wire.len() || wire != roundtrip || bell.ic.len() != 26 {
        return Err("ballot VK encoding".into());
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
    let vk_hash = hex(&sha256(&compressed(&vk)?));
    if pool["groth16_vk_ark_compressed_sha256"] != vk_hash {
        return Err("converted ballot VK differs from D2 Ark VK".into());
    }
    let svk_bytes = decode(string(&s, "state_vk_hex")?)?;
    let svk: VerifyingKey<Bls12_381> = canonical(&svk_bytes)?;
    if s["state_vk_sha256"] != hex(&sha256(&svk_bytes)) || svk.gamma_abc_g1.len() != 38 {
        return Err("state VK".into());
    }
    let bpvk = prepare_verifying_key(&vk);
    let spvk = prepare_verifying_key(&svk);
    let mut batches = Vec::new();
    let mut checks = Vec::new();
    let mut ballot_eip_vk = None;
    let mut state_eip_vk = None;
    for (i, b) in s["batches"].as_array().ok_or("batches")?.iter().enumerate() {
        let rows = b["source_public_inputs_le_hex"].as_array().ok_or("rows")?;
        let records = b["source_public_records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .map(record)
            .collect::<Result<Vec<_>>>()?;
        if records.len() != 4 || rows.len() != 4 {
            return Err("B4 required".into());
        }
        let c = StateTransitionCircuitForBatch::<4> {
            election_id: records[0].election_id,
            final_context: records[0].final_context,
            region: records[0].region,
            key_set_hash: records[0].key_set_hash,
            public_key: records[0].public_key,
            batch_index: (i + 1) as u64,
            batch_size: 4,
            batch_root: fixed(b, "batch_root_hex")?,
            old_state: states(&b["old_state"])?,
            new_state: states(&b["new_state"])?,
            records,
        };
        if batch_root_v2_for_batch::<4>(
            &c.election_id,
            &c.final_context,
            c.region,
            c.batch_index,
            &c.records,
        ) != Some(c.batch_root)
        {
            return Err("batch root".into());
        }
        let mut proofs = Vec::new();
        let mut inputs = Vec::new();
        let mut hashes = Vec::new();
        for (j, r) in c.records.iter().enumerate() {
            let packed = ballot_public_inputs(r).ok_or("ballot inputs")?;
            let pool_index = i * 4 + j;
            if le_inputs(&packed)? != rows[j]
                || rows[j] != pool["source_public_inputs_le_hex"][pool_index]
                || b["source_ballot_proofs_hex"][j]
                    != pool["source_proofs_ark_compressed_hex"][pool_index]
            {
                return Err("not exact archived D2 prefix".into());
            }
            let bytes = decode(b["source_ballot_proofs_hex"][j].as_str().ok_or("proof")?)?;
            let proof: Proof<Bls12_381> = canonical(&bytes)?;
            if !Groth16::<Bls12_381>::verify_proof(&bpvk, &proof, &packed)? {
                return Err("ballot proof".into());
            }
            if j == 0 {
                let other = canonical::<Proof<Bls12_381>>(&decode(
                    b["source_ballot_proofs_hex"][1]
                        .as_str()
                        .ok_or("other proof")?,
                )?)?;
                if Groth16::<Bls12_381>::verify_proof(&bpvk, &other, &packed)? {
                    return Err("wrong ballot proof accepted".into());
                }
            }
            let m = Eip2537PublicGroth16Proof::from_proof(&vk, &proof, &packed)?;
            ballot_eip_vk = Some(m.vk_eip2537_hex);
            proofs.push(m.proof_neg_a_eip2537_hex);
            inputs.push(m.public_inputs_be_hex);
            hashes.push(hex(&sha256(&bytes)));
        }
        let si = state_public_inputs(&c).ok_or("state inputs")?;
        if le_inputs(&si)? != b["state_public_inputs_le_hex"] {
            return Err("state input packing".into());
        }
        let bytes = decode(string(b, "state_proof_hex")?)?;
        let proof: Proof<Bls12_381> = canonical(&bytes)?;
        if hex(&sha256(&bytes)) != string(b, "state_proof_sha256")?
            || !Groth16::<Bls12_381>::verify_proof(&spvk, &proof, &si)?
        {
            return Err("state proof".into());
        }
        let m = Eip2537PublicGroth16Proof::from_proof(&svk, &proof, &si)?;
        state_eip_vk = Some(m.vk_eip2537_hex);
        checks.push(json!({"batch":i+1,"ballot_proofs_verified":4,"state_proof_verified":true,"wrong_ballot_proof_rejected":true}));
        batches.push(json!({"batch_index":i+1,"root_hex":b["batch_root_hex"],"old_state":b["old_state"],"new_state":b["new_state"],
            "ballot_proofs_eip2537_hex":proofs,"ballot_inputs_be_hex":inputs,"ballot_proof_ark_sha256":hashes,
            "state_proof_eip2537_hex":m.proof_neg_a_eip2537_hex,"state_inputs_be_hex":m.public_inputs_be_hex,"state_proof_ark_sha256":b["state_proof_sha256"]}));
    }
    if batches.len() != 2 {
        return Err("two batches required".into());
    }
    let output = json!({"schema":"VV/G8/RECEIVER-FREEZE-EIP2537/v1","phase":"receiver-and-freeze-only",
        "batch_validity_route":"individual-groth16-not-snarkpack","ballot_vk_eip2537_hex":ballot_eip_vk.ok_or("ballot VK")?,
        "state_vk_eip2537_hex":state_eip_vk.ok_or("state VK")?,"ballot_vk_ark_sha256":vk_hash,"state_vk_ark_sha256":s["state_vk_sha256"],
        "public_context":g6["public_context"],"p3_vk_ark_sha256":g6["verifying_key_sha256"],
        "expected_freeze_digest_hex":g6["freeze_digest_hex"],"expected_final_state":s["final_state"],"batches":batches,
        "setup_count":0,"proof_generation_count":0,"private_witness_emitted":false,"production_setup":false,"source_pins":config["source_pins"]});
    write_new(OUTPUT, &output)?;
    let source_hashes = [
        MANIFEST,
        "crates/vv-friendly-dkg/src/bin/g8_receiver_export.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "crates/vv-friendly-crypto/src/evm_export.rs",
        "crates/vv-friendly-crypto/src/state_gadget.rs",
        "Cargo.lock",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect::<Result<Vec<_>>>()?;
    let git = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    write_new(
        REPORT,
        &json!({"gate":"G8-public-export","status":"PASS_EIGHT_BALLOTS_TWO_STATE_PROOFS_NATIVE_REPLAY_AND_EIP2537_EXPORT",
        "git_commit":String::from_utf8(git.stdout)?.trim(),"git_dirty":!Command::new("git").args(["status","--porcelain"]).output()?.stdout.is_empty(),
        "source_pins":config["source_pins"],"source_hashes":source_hashes,"checks":checks,"setup_count":0,"proof_generation_count":0,
        "output_fixture":OUTPUT,"output_fixture_sha256":hex(&sha256(&fs::read(OUTPUT)?)),"private_witness_emitted":false}),
    )?;
    println!("G8 phase 1 export PASS: 8 archived ballot proofs + 2 state proofs verified; no Setup/Prove; NOT SnarkPack EVM");
    Ok(())
}
