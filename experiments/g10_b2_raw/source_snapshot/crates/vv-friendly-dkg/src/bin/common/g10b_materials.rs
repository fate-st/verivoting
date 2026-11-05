//! Archived G10-A loader checks reused verbatim, extended only for individual proofs and v2 membership paths.
use super::helpers;
use ark_bls12_381::Bls12_381;
use ark_groth16::{prepare_verifying_key, PreparedVerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::{
    aggregate::{AggregateProof, VerifierSRS},
    prepare_verifying_key as prepare_ballot_key, PreparedVerifyingKey as BallotKey, VerifyingKey,
};
use blstrs::Bls12;
use helpers::Result;
use serde_json::{json, Value};
use std::{fs, io::Cursor};
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs, state_public_inputs, StateBallotRecord,
    StateTransitionCircuitForBatch as Circuit, STATE_B4_LAYOUT_VERSION,
};
use vv_friendly_crypto::{decode_point, Ciphertext};
use vv_types::{hex, sha256};
const FIXTURE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";
const FIXTURE_SHA: &str = "42e68fa9bee72fb77e47c292dd3e65386cf4a348192c1ee0a9731cfe2f5eff19";
const POOL: &str = "fixtures/snarkpack_g1d2_depth5_batch32_public_v1.json";
const POOL_SHA: &str = "6b9a1ca042e3ab1d637fb3b554029cc7a36a5b867430b2707539d1ea61928596";
const MANIFEST: &str = "configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json";
const MANIFEST_SHA: &str = "00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e";

fn bytes(v: &Value) -> Result<Vec<u8>> {
    let s = v.as_str().ok_or("missing hex string")?;
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
fn fixed(v: &Value, key: &str) -> Result<[u8; 32]> {
    bytes(&v[key])?
        .try_into()
        .map_err(|_| "wrong 32-byte encoding".into())
}
fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(b: &[u8]) -> Result<T> {
    let mut rest = b;
    let v = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || helpers::compressed(&v)? != b {
        return Err("noncanonical Ark encoding".into());
    }
    Ok(v)
}
fn states(v: &Value) -> Result<Vec<Ciphertext>> {
    let a = v.as_array().ok_or("ciphertext array")?;
    if a.len() != 3 {
        return Err("three candidates required".into());
    }
    a.iter()
        .enumerate()
        .map(|(j, c)| {
            if c["candidate_id"] != j + 1 {
                return Err("candidate order".into());
            }
            Ok(Ciphertext {
                c0: decode_point(&fixed(c, "c0_hex")?, true)?,
                c1: decode_point(&fixed(c, "c1_hex")?, true)?,
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
pub(super) struct Batch {
    pub circuit: Circuit<4>,
    pub aggregate: AggregateProof<Bls12>,
    pub proof: ark_groth16::Proof<Bls12_381>,
    pub ballot_proofs: Vec<bellperson::groth16::Proof<Bls12>>,
    pub paths: Vec<[[u8; 32]; 2]>,
}
pub(super) struct Loaded {
    pub batches: Vec<Batch>,
    pub ballot_key: BallotKey<Bls12>,
    pub state_key: PreparedVerifyingKey<Bls12_381>,
    pub srs: VerifierSRS<Bls12>,
    pub pins: Value,
    pub communication: Value,
}
pub(super) fn load() -> Result<Loaded> {
    let mut pins = Vec::new();
    for (p, h) in [
        (FIXTURE, FIXTURE_SHA),
        (POOL, POOL_SHA),
        (MANIFEST, MANIFEST_SHA),
    ] {
        let actual = hex(&sha256(&fs::read(p)?));
        if actual != h {
            return Err(format!("source hash mismatch: {p}").into());
        }
        pins.push(json!({"path":p,"sha256":actual}));
    }
    let f: Value = serde_json::from_slice(&fs::read(FIXTURE)?)?;
    let pool: Value = serde_json::from_slice(&fs::read(POOL)?)?;
    if f["schema"] != "VV/G2-E/MULTIBATCH-B4-PUBLIC-FIXTURE/v1"
        || f["state_layout"] != STATE_B4_LAYOUT_VERSION
        || f["source_ballot_vk_hex"] != pool["groth16_vk_hex"]
        || f["batches"].as_array().ok_or("batches")?.len() != 2
    {
        return Err("wrong W32 state fixture".into());
    }
    let vk_bytes = bytes(&f["source_ballot_vk_hex"])?;
    let mut cursor = Cursor::new(&vk_bytes);
    let ballot_vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
    let mut wire = Vec::new();
    ballot_vk.write(&mut wire)?;
    if cursor.position() as usize != vk_bytes.len() || wire != vk_bytes || ballot_vk.ic.len() != 26
    {
        return Err("ballot VK encoding/arity".into());
    }
    let state_bytes = bytes(&f["state_vk_hex"])?;
    if f["state_vk_sha256"] != hex(&sha256(&state_bytes)) {
        return Err("state VK hash".into());
    }
    let state_vk: ark_groth16::VerifyingKey<Bls12_381> = canonical(&state_bytes)?;
    if state_vk.gamma_abc_g1.len() != 38 {
        return Err("state VK arity".into());
    }
    let v = &f["source_verifier_srs"];
    let srs = VerifierSRS {
        n: v["n"].as_u64().ok_or("SRS n")? as usize,
        g: serde_json::from_value(v["g"].clone())?,
        h: serde_json::from_value(v["h"].clone())?,
        g_alpha: serde_json::from_value(v["g_alpha"].clone())?,
        g_beta: serde_json::from_value(v["g_beta"].clone())?,
        h_alpha: serde_json::from_value(v["h_alpha"].clone())?,
        h_beta: serde_json::from_value(v["h_beta"].clone())?,
        h_alpha_d: serde_json::from_value(v["h_alpha_d"].clone())?,
    };
    if srs.n != 4 {
        return Err("B4 SRS required".into());
    }
    let mut batches = Vec::new();
    let mut old = vec![Ciphertext::zero(); 3];
    let mut communication = Vec::new();
    for (i, b) in f["batches"].as_array().ok_or("batches")?.iter().enumerate() {
        let rows = pool["source_public_records"]
            .as_array()
            .ok_or("pool records")?;
        if b["source_public_records"] != json!(&rows[i * 4..i * 4 + 4])
            || states(&b["old_state"])? != old
            || b["batch_index"] != i + 1
        {
            return Err("not exact continuous D2 prefix".into());
        }
        let records = b["source_public_records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .map(record)
            .collect::<Result<Vec<_>>>()?;
        let binding = &records[0];
        let circuit = Circuit::<4> {
            election_id: binding.election_id,
            final_context: binding.final_context,
            region: binding.region,
            key_set_hash: binding.key_set_hash,
            public_key: binding.public_key,
            batch_index: (i + 1) as u64,
            batch_size: 4,
            batch_root: fixed(b, "batch_root_hex")?,
            old_state: old,
            new_state: states(&b["new_state"])?,
            records,
        };
        let packed = circuit
            .records
            .iter()
            .map(|r| helpers::convert_inputs(&ballot_public_inputs(r).ok_or("ballot packing")?))
            .collect::<Result<Vec<_>>>()?;
        if json!(packed
            .iter()
            .map(|r| r.iter().map(|x| hex(&x.to_bytes_le())).collect::<Vec<_>>())
            .collect::<Vec<_>>())
            != b["source_public_inputs_le_hex"]
            || hex(&helpers::transcript(&packed)) != b["source_external_transcript_hex"]
        {
            return Err("record/matrix/transcript binding".into());
        }
        let input = state_public_inputs(&circuit).ok_or("state packing")?;
        if json!(input
            .iter()
            .map(|x| Ok(hex(&helpers::compressed(x)?)))
            .collect::<Result<Vec<_>>>()?)
            != b["state_public_inputs_le_hex"]
        {
            return Err("state public inputs".into());
        }
        let wire = bytes(&b["source_aggregate_hex"])?;
        let mut cursor = Cursor::new(&wire);
        let aggregate = AggregateProof::<Bls12>::read(&mut cursor)?;
        aggregate.parsing_check()?;
        let mut back = Vec::new();
        aggregate.write(&mut back)?;
        if back != wire
            || cursor.position() as usize != wire.len()
            || aggregate.tmipp.gipa.nproofs != 4
            || b["source_aggregate_sha256"] != hex(&sha256(&wire))
        {
            return Err("aggregate canonical bytes/hash".into());
        }
        let proof_bytes = bytes(&b["state_proof_hex"])?;
        if b["state_proof_sha256"] != hex(&sha256(&proof_bytes)) {
            return Err("state proof hash".into());
        }
        communication.push(json!({"batch_index":i+1,"aggregate_bytes":wire.len(),"matrix_bytes":4*25*32,
            "old_and_new_ct_bytes":2*3*2*32,"state_statement_bytes":37*32,"state_proof_bytes":proof_bytes.len(),
            "state_statement_is_derived_not_added_to_matrix_and_CT":true,"full_network_or_calldata_bytes":null}));
        let archived = pool["source_proofs_ark_compressed_hex"]
            .as_array()
            .ok_or("proof pool")?;
        if b["source_ballot_proofs_hex"] != json!(&archived[i * 4..i * 4 + 4]) {
            return Err("individual proof pool mismatch".into());
        }
        let ballot_proofs = archived[i * 4..i * 4 + 4]
            .iter()
            .map(|v| {
                let p: ark_groth16::Proof<Bls12_381> = canonical(&bytes(v)?)?;
                helpers::convert_proof(&p)
            })
            .collect::<Result<Vec<_>>>()?;
        let path_started = std::time::Instant::now();
        let paths = (0..4)
            .map(|j| {
                super::intake::membership_path(&circuit.records, circuit.batch_index, j)
                    .ok_or_else(|| "membership path".into())
            })
            .collect::<Result<Vec<_>>>()?;
        communication
            .last_mut()
            .ok_or("missing communication row")?["sender_membership_path_prepare_ms"] =
            json!(path_started.elapsed().as_secs_f64() * 1000.);
        for (j, path) in paths.iter().enumerate() {
            if !super::intake::verify_membership(
                &circuit.records[j],
                circuit.batch_index,
                j,
                path,
                &circuit.batch_root,
            ) {
                return Err("membership encoding differs from original v2 root".into());
            }
        }
        let proof = canonical(&proof_bytes)?;
        old = circuit.new_state.clone();
        batches.push(Batch {
            circuit,
            aggregate,
            proof,
            ballot_proofs,
            paths,
        });
    }
    if states(&f["final_state"])? != old {
        return Err("final CT2 mismatch".into());
    }
    pins.push(json!({"ballot_vk_sha256":hex(&sha256(&vk_bytes)),"state_vk_sha256":hex(&sha256(&state_bytes)),
        "verifier_srs_json_sha256":hex(&sha256(&serde_json::to_vec(v)?))}));
    Ok(Loaded {
        batches,
        ballot_key: prepare_ballot_key(&ballot_vk),
        state_key: prepare_verifying_key(&state_vk),
        srs,
        pins: json!(pins),
        communication: json!(communication),
    })
}
