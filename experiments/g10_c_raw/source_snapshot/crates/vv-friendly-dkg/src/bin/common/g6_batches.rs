//! G6 reloads public material but executes both real verifiers in the live receiver.
use super::materials::{canonical, compressed, decode, fixed, input_hex, states, Result, Sources};
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey};
use bellperson::groth16::{
    aggregate::{verify_aggregate_proof, AggregateProof, AggregateVersion, VerifierSRS},
    prepare_verifying_key as prepare_ballot_vk, PreparedVerifyingKey as BallotPrepared,
    VerifyingKey,
};
use blstrs::{Bls12, Scalar};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{io::Cursor, time::Instant};
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs, batch_root_v2_for_batch, state_public_inputs, StateBallotRecord,
    StateTransitionCircuitForBatch, STATE_B4_LAYOUT_VERSION,
};
use vv_friendly_crypto::{decode_point, Ciphertext};
use vv_types::{hex, sha256};

pub(super) struct Batch {
    pub(super) circuit: StateTransitionCircuitForBatch<4>,
    matrix: Vec<Vec<Scalar>>,
    transcript: Vec<u8>,
    aggregate: AggregateProof<Bls12>,
    proof: ark_groth16::Proof<Bls12_381>,
    inputs: Vec<Fr>,
}
pub(super) struct BatchVerifier {
    pub(super) batches: Vec<Batch>,
    state_key: PreparedVerifyingKey<Bls12_381>,
    ballot_key: BallotPrepared<Bls12>,
    srs: VerifierSRS<Bls12>,
}

fn record(v: &Value) -> Result<StateBallotRecord> {
    Ok(StateBallotRecord {
        election_id: fixed(v, "election_id_hex")?,
        final_context: fixed(v, "final_context_hex")?,
        registration_root: fixed(v, "registration_root_hex")?,
        nullifier: fixed(v, "nullifier_hex")?,
        key_set_hash: fixed(v, "key_set_hash_hex")?,
        public_key: decode_point(&fixed(v, "public_key_hex")?, false)?,
        region: v["region"].as_u64().ok_or("record region")?,
        ciphertexts: states(&v["ciphertexts"])?,
    })
}

impl BatchVerifier {
    pub(super) fn load(s: &Sources) -> Result<Self> {
        let f = &s.state;
        if f["schema"] != "VV/G2-E/MULTIBATCH-B4-PUBLIC-FIXTURE/v1"
            || f["state_layout"] != STATE_B4_LAYOUT_VERSION
            || f["batches"].as_array().ok_or("batches")?.len() != 2
            || f["source_ballot_vk_hex"] != s.pool["groth16_vk_hex"]
        {
            return Err("G2-E trust anchors/layout differ".into());
        }
        let vk_bytes = decode(f["state_vk_hex"].as_str().ok_or("state VK")?)?;
        if f["state_vk_sha256"] != hex(&sha256(&vk_bytes)) {
            return Err("state VK digest".into());
        }
        let vk: ark_groth16::VerifyingKey<Bls12_381> = canonical(&vk_bytes)?;
        if vk.gamma_abc_g1.len() != 38 {
            return Err("state VK arity".into());
        }
        let ballot_bytes = decode(f["source_ballot_vk_hex"].as_str().ok_or("ballot VK")?)?;
        let mut cursor = Cursor::new(&ballot_bytes);
        let ballot_vk = VerifyingKey::<Bls12>::read(&mut cursor)?;
        let mut roundtrip = Vec::new();
        ballot_vk.write(&mut roundtrip)?;
        if roundtrip != ballot_bytes
            || cursor.position() as usize != ballot_bytes.len()
            || ballot_vk.ic.len() != 26
        {
            return Err("noncanonical ballot VK".into());
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
            return Err("B4 verifier SRS required".into());
        }
        let records = s.pool["source_public_records"]
            .as_array()
            .ok_or("D2 records")?;
        let rows = s.pool["source_public_inputs_le_hex"]
            .as_array()
            .ok_or("D2 matrix")?;
        if records.len() != 32 || rows.len() != 32 {
            return Err("D2 count".into());
        }
        let mut batches = Vec::new();
        let mut old = vec![Ciphertext::zero(); 3];
        for (i, b) in f["batches"].as_array().ok_or("batches")?.iter().enumerate() {
            let start = i * 4;
            if b["source_public_records"] != json!(&records[start..start + 4])
                || b["source_public_inputs_le_hex"] != json!(&rows[start..start + 4])
                || b["batch_index"] != i + 1
                || states(&b["old_state"])? != old
            {
                return Err("batch not the exact ordered D2 prefix/continuous state".into());
            }
            let records = records[start..start + 4]
                .iter()
                .map(record)
                .collect::<Result<Vec<_>>>()?;
            let mut matrix = Vec::new();
            for (r, row) in records.iter().zip(&rows[start..start + 4]) {
                let packed = ballot_public_inputs(r).ok_or("ballot packing")?;
                if row != &json!(input_hex(&packed)?) {
                    return Err("ballot record/matrix mismatch".into());
                }
                let fields = packed
                    .iter()
                    .map(|x| {
                        let bytes: [u8; 32] =
                            compressed(x)?.try_into().map_err(|_| "field length")?;
                        Option::<Scalar>::from(Scalar::from_bytes_le(&bytes))
                            .ok_or_else(|| "field encoding".into())
                    })
                    .collect::<Result<Vec<_>>>()?;
                matrix.push(fields);
            }
            let binding = &records[0];
            let circuit = StateTransitionCircuitForBatch::<4> {
                election_id: binding.election_id,
                final_context: binding.final_context,
                region: binding.region,
                batch_index: (i + 1) as u64,
                batch_size: 4,
                batch_root: fixed(b, "batch_root_hex")?,
                key_set_hash: binding.key_set_hash,
                public_key: binding.public_key,
                old_state: old,
                new_state: states(&b["new_state"])?,
                records,
            };
            if batch_root_v2_for_batch::<4>(
                &circuit.election_id,
                &circuit.final_context,
                circuit.region,
                circuit.batch_index,
                &circuit.records,
            ) != Some(circuit.batch_root)
            {
                return Err("batch root mismatch".into());
            }
            let inputs = state_public_inputs(&circuit).ok_or("state inputs")?;
            if b["state_public_inputs_le_hex"] != json!(input_hex(&inputs)?) {
                return Err("state inputs mismatch".into());
            }
            let mut transcript = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1".to_vec();
            transcript.extend_from_slice(&(matrix.len() as u32).to_be_bytes());
            for row in &matrix {
                transcript.extend_from_slice(&(row.len() as u32).to_be_bytes());
                for f in row {
                    transcript.extend_from_slice(&f.to_bytes_le());
                }
            }
            if b["source_external_transcript_hex"] != hex(&transcript) {
                return Err("SP transcript mismatch".into());
            }
            let bytes = decode(b["source_aggregate_hex"].as_str().ok_or("aggregate")?)?;
            let mut cursor = Cursor::new(&bytes);
            let aggregate = AggregateProof::<Bls12>::read(&mut cursor)?;
            aggregate.parsing_check()?;
            let mut wire = Vec::new();
            aggregate.write(&mut wire)?;
            if cursor.position() as usize != bytes.len()
                || wire != bytes
                || aggregate.tmipp.gipa.nproofs != 4
            {
                return Err("aggregate boundary".into());
            }
            let proof = canonical(&decode(
                b["state_proof_hex"].as_str().ok_or("state proof")?,
            )?)?;
            old = circuit.new_state.clone();
            batches.push(Batch {
                circuit,
                matrix,
                transcript,
                aggregate,
                proof,
                inputs,
            });
        }
        Ok(Self {
            batches,
            state_key: prepare_verifying_key(&vk),
            ballot_key: prepare_ballot_vk(&ballot_vk),
            srs,
        })
    }

    pub(super) fn verify(&self, index: usize, metrics: &mut Value) -> Result<bool> {
        let b = self.batches.get(index).ok_or("batch index")?;
        let t = Instant::now();
        let aggregate_ok = verify_aggregate_proof(
            &self.srs,
            &self.ballot_key,
            ChaCha20Rng::seed_from_u64(0x4736_2D56_4552_0000 + index as u64),
            &b.matrix,
            &b.aggregate,
            &b.transcript,
            AggregateVersion::V2,
        )?;
        let aggregate_ms = t.elapsed().as_secs_f64() * 1000.;
        let t = Instant::now();
        let state_ok = Groth16::<Bls12_381>::verify_proof(&self.state_key, &b.proof, &b.inputs)?;
        *metrics = json!({"batch_index":index+1,"snarkpack_verified":aggregate_ok,"state_proof_verified":state_ok,
            "snarkpack_verify_ms":aggregate_ms,"state_verify_ms":t.elapsed().as_secs_f64()*1000.,"proofs_reused":true});
        Ok(aggregate_ok && state_ok)
    }
}
