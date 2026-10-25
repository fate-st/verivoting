//! G2-A/G2-B: fixed-B=2 regional encrypted-state transition.
//!
//! The relation binds the paper's ordered state statement to the exact two
//! ballot public records already authenticated by G1-C1. It recomputes the
//! canonical batch root and folds all six Jubjub ciphertext components. This
//! G2-A measures the R1CS relation; G2-B proves the unchanged relation with
//! Groth16.

use crate::{
    ballot_gadget::BALLOT_CORE_CANDIDATE_COUNT, cp_h3_gadget::bind_canonical_compressed_point,
    encode_point, Ciphertext, Point,
};
use ark_crypto_primitives::crh::sha256::constraints::Sha256Gadget;
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{BigInteger, PrimeField, ToConstraintField};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, groups::CurveVar, uint8::UInt8,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};
use vv_types::{sha256, CanonicalEncoder};

pub const STATE_BATCH_SIZE: usize = 2;
pub const BALLOT_PUBLIC_INPUTS: usize = 25;
pub const BALLOT_FIELD_VECTOR_BYTES: usize = BALLOT_PUBLIC_INPUTS * 32;
pub const STATE_PUBLIC_INPUTS: usize = 37;
pub const STATE_LAYOUT_VERSION: &str = "VV/JUBJUB/G2-A/STATE-B2-LAYOUT/v1";

const BATCH_LEAF_DOMAIN: &[u8] = b"VV/BATCH/LEAF/v2";
const BATCH_NODE_DOMAIN: &[u8] = b"VV/BATCH/NODE/v2";

#[derive(Clone, Debug)]
pub struct StateBallotRecord {
    pub election_id: [u8; 32],
    pub final_context: [u8; 32],
    pub region: u64,
    pub registration_root: [u8; 32],
    pub nullifier: [u8; 32],
    pub key_set_hash: [u8; 32],
    pub public_key: Point,
    pub ciphertexts: Vec<Ciphertext>,
}

#[derive(Clone)]
pub struct StateTransitionCircuit {
    pub election_id: [u8; 32],
    pub final_context: [u8; 32],
    pub region: u64,
    pub batch_index: u64,
    pub batch_size: u64,
    pub batch_root: [u8; 32],
    pub key_set_hash: [u8; 32],
    pub public_key: Point,
    pub old_state: Vec<Ciphertext>,
    pub new_state: Vec<Ciphertext>,
    pub records: Vec<StateBallotRecord>,
}

#[derive(Clone, Debug)]
pub struct StateTransitionReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub batch_size: usize,
    pub candidate_count: usize,
    pub ballot_public_inputs_per_record: usize,
    pub ballot_field_vector_bytes_per_record: usize,
    pub batch_hashes: usize,
    pub accumulator_components: usize,
    pub group_additions: usize,
}

fn append_bytes_as_fields(bytes: &[u8], fields: &mut Vec<Fq>) -> Option<()> {
    if bytes.len() != 32 {
        return None;
    }
    let packed: Vec<Fq> = bytes.to_field_elements()?;
    fields.extend(packed);
    Some(())
}

fn append_packed_bytes(bytes: &[u8], fields: &mut Vec<Fq>) -> Option<()> {
    let packed: Vec<Fq> = bytes.to_field_elements()?;
    fields.extend(packed);
    Some(())
}

pub fn ballot_public_inputs(record: &StateBallotRecord) -> Option<Vec<Fq>> {
    if record.ciphertexts.len() != BALLOT_CORE_CANDIDATE_COUNT {
        return None;
    }
    let mut fields = Vec::with_capacity(BALLOT_PUBLIC_INPUTS);
    append_bytes_as_fields(&record.election_id, &mut fields)?;
    append_bytes_as_fields(&record.final_context, &mut fields)?;
    fields.push(Fq::from(record.region));
    append_bytes_as_fields(&record.registration_root, &mut fields)?;
    append_bytes_as_fields(&record.nullifier, &mut fields)?;
    append_bytes_as_fields(&record.key_set_hash, &mut fields)?;
    append_bytes_as_fields(&encode_point(&record.public_key), &mut fields)?;
    for ciphertext in &record.ciphertexts {
        append_bytes_as_fields(&encode_point(&ciphertext.c0), &mut fields)?;
        append_bytes_as_fields(&encode_point(&ciphertext.c1), &mut fields)?;
    }
    (fields.len() == BALLOT_PUBLIC_INPUTS).then_some(fields)
}

fn field_to_fixed_be(value: &Fq) -> [u8; 32] {
    let bytes = value.into_bigint().to_bytes_be();
    let mut encoded = [0u8; 32];
    encoded[32 - bytes.len()..].copy_from_slice(&bytes);
    encoded
}

pub fn ballot_field_vector_bytes(record: &StateBallotRecord) -> Option<Vec<u8>> {
    let fields = ballot_public_inputs(record)?;
    let mut encoded = Vec::with_capacity(BALLOT_FIELD_VECTOR_BYTES);
    for field in &fields {
        encoded.extend_from_slice(&field_to_fixed_be(field));
    }
    Some(encoded)
}

fn batch_leaf(
    election_id: &[u8; 32],
    final_context: &[u8; 32],
    region: u64,
    batch_index: u64,
    record_index: usize,
    record: &StateBallotRecord,
) -> Option<[u8; 32]> {
    let field_bytes = ballot_field_vector_bytes(record)?;
    let mut encoded = CanonicalEncoder::new(BATCH_LEAF_DOMAIN);
    encoded.fixed(election_id);
    encoded.fixed(final_context);
    encoded.u64(region);
    encoded.u64(batch_index);
    encoded.u64(STATE_BATCH_SIZE as u64);
    encoded.u64(record_index as u64);
    encoded.u64(BALLOT_PUBLIC_INPUTS as u64);
    encoded.fixed(&field_bytes);
    Some(sha256(&encoded.finish()))
}

pub fn batch_root_v2(
    election_id: &[u8; 32],
    final_context: &[u8; 32],
    region: u64,
    batch_index: u64,
    records: &[StateBallotRecord],
) -> Option<[u8; 32]> {
    if records.len() != STATE_BATCH_SIZE {
        return None;
    }
    let left = batch_leaf(
        election_id,
        final_context,
        region,
        batch_index,
        0,
        &records[0],
    )?;
    let right = batch_leaf(
        election_id,
        final_context,
        region,
        batch_index,
        1,
        &records[1],
    )?;
    let mut encoded = CanonicalEncoder::new(BATCH_NODE_DOMAIN);
    encoded.fixed(&left);
    encoded.fixed(&right);
    Some(sha256(&encoded.finish()))
}

pub fn fold_state(
    old_state: &[Ciphertext],
    records: &[StateBallotRecord],
) -> Option<Vec<Ciphertext>> {
    if old_state.len() != BALLOT_CORE_CANDIDATE_COUNT
        || records.len() != STATE_BATCH_SIZE
        || records
            .iter()
            .any(|record| record.ciphertexts.len() != BALLOT_CORE_CANDIDATE_COUNT)
    {
        return None;
    }
    let mut state = old_state.to_vec();
    for record in records {
        for (accumulator, ciphertext) in state.iter_mut().zip(&record.ciphertexts) {
            *accumulator = accumulator.add(ciphertext);
        }
    }
    Some(state)
}

/// Packs the public statement exactly as `UInt8::new_input_vec` allocates it
/// in `generate_relation`. This order is frozen by the G2-A layout and reused
/// unchanged by G2-B verification.
pub fn state_public_inputs(circuit: &StateTransitionCircuit) -> Option<Vec<Fq>> {
    if !circuit.has_expected_shape() {
        return None;
    }
    let mut fields = Vec::with_capacity(STATE_PUBLIC_INPUTS);
    append_packed_bytes(&circuit.election_id, &mut fields)?;
    append_packed_bytes(&circuit.final_context, &mut fields)?;
    append_packed_bytes(&circuit.region.to_be_bytes(), &mut fields)?;
    append_packed_bytes(&circuit.batch_index.to_be_bytes(), &mut fields)?;
    append_packed_bytes(&circuit.batch_size.to_be_bytes(), &mut fields)?;
    append_packed_bytes(&circuit.batch_root, &mut fields)?;
    append_packed_bytes(&circuit.key_set_hash, &mut fields)?;
    append_packed_bytes(&encode_point(&circuit.public_key), &mut fields)?;
    for ciphertext in circuit.old_state.iter().chain(&circuit.new_state) {
        append_packed_bytes(&encode_point(&ciphertext.c0), &mut fields)?;
        append_packed_bytes(&encode_point(&ciphertext.c1), &mut fields)?;
    }
    (fields.len() == STATE_PUBLIC_INPUTS).then_some(fields)
}

fn constant_prefix(domain: &[u8]) -> Vec<UInt8<Fq>> {
    UInt8::constant_vec(&CanonicalEncoder::new(domain).finish())
}

fn constant_u64(value: u64) -> Vec<UInt8<Fq>> {
    UInt8::constant_vec(&value.to_be_bytes())
}

fn enforce_bytes_equal(left: &[UInt8<Fq>], right: &[UInt8<Fq>]) -> Result<(), SynthesisError> {
    if left.len() != right.len() {
        return Err(SynthesisError::Unsatisfiable);
    }
    for (left_byte, right_byte) in left.iter().zip(right) {
        left_byte.enforce_equal(right_byte)?;
    }
    Ok(())
}

fn append_32_bytes_as_field_vector(bytes: &[UInt8<Fq>], encoded: &mut Vec<UInt8<Fq>>) {
    debug_assert_eq!(bytes.len(), 32);
    encoded.push(UInt8::constant(0));
    encoded.extend(bytes[..31].iter().rev().cloned());
    encoded.extend(UInt8::constant_vec(&[0u8; 31]));
    encoded.push(bytes[31].clone());
}

fn append_u64_as_field(value: &[UInt8<Fq>], encoded: &mut Vec<UInt8<Fq>>) {
    debug_assert_eq!(value.len(), 8);
    encoded.extend(UInt8::constant_vec(&[0u8; 24]));
    encoded.extend_from_slice(value);
}

impl StateTransitionCircuit {
    fn has_expected_shape(&self) -> bool {
        self.batch_size == STATE_BATCH_SIZE as u64
            && self.records.len() == STATE_BATCH_SIZE
            && self.old_state.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self.new_state.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self
                .records
                .iter()
                .all(|record| record.ciphertexts.len() == BALLOT_CORE_CANDIDATE_COUNT)
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }

        // Public state statement in the paper's displayed order.
        let election_id_var = UInt8::new_input_vec(cs.clone(), &self.election_id)?;
        let final_context_var = UInt8::new_input_vec(cs.clone(), &self.final_context)?;
        let region_var = UInt8::new_input_vec(cs.clone(), &self.region.to_be_bytes())?;
        let batch_index_var = UInt8::new_input_vec(cs.clone(), &self.batch_index.to_be_bytes())?;
        let batch_size_var = UInt8::new_input_vec(cs.clone(), &self.batch_size.to_be_bytes())?;
        enforce_bytes_equal(
            &batch_size_var,
            &UInt8::constant_vec(&(STATE_BATCH_SIZE as u64).to_be_bytes()),
        )?;
        let batch_root_var = UInt8::new_input_vec(cs.clone(), &self.batch_root)?;
        let key_set_hash_var = UInt8::new_input_vec(cs.clone(), &self.key_set_hash)?;

        let public_key_var = EdwardsVar::new_witness(cs.clone(), || Ok(self.public_key))?;
        let public_key_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&self.public_key))?;
        bind_canonical_compressed_point(&public_key_var, &public_key_bytes)?;
        public_key_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;

        let mut old_vars = Vec::with_capacity(BALLOT_CORE_CANDIDATE_COUNT);
        for old in &self.old_state {
            let old_c0 = EdwardsVar::new_witness(cs.clone(), || Ok(old.c0))?;
            let old_c0_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&old.c0))?;
            bind_canonical_compressed_point(&old_c0, &old_c0_bytes)?;
            let old_c1 = EdwardsVar::new_witness(cs.clone(), || Ok(old.c1))?;
            let old_c1_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&old.c1))?;
            bind_canonical_compressed_point(&old_c1, &old_c1_bytes)?;
            old_vars.push((old_c0, old_c1));
        }
        let mut new_vars = Vec::with_capacity(BALLOT_CORE_CANDIDATE_COUNT);
        for new in &self.new_state {
            let new_c0 = EdwardsVar::new_witness(cs.clone(), || Ok(new.c0))?;
            let new_c0_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&new.c0))?;
            bind_canonical_compressed_point(&new_c0, &new_c0_bytes)?;
            let new_c1 = EdwardsVar::new_witness(cs.clone(), || Ok(new.c1))?;
            let new_c1_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&new.c1))?;
            bind_canonical_compressed_point(&new_c1, &new_c1_bytes)?;
            new_vars.push((new_c0, new_c1));
        }

        let mut leaves = Vec::with_capacity(STATE_BATCH_SIZE);
        let mut record_ciphertext_vars = Vec::with_capacity(STATE_BATCH_SIZE);
        for (record_index, record) in self.records.iter().enumerate() {
            let record_eid = UInt8::new_witness_vec(cs.clone(), &record.election_id)?;
            let record_context = UInt8::new_witness_vec(cs.clone(), &record.final_context)?;
            let record_region = UInt8::new_witness_vec(cs.clone(), &record.region.to_be_bytes())?;
            let record_root = UInt8::new_witness_vec(cs.clone(), &record.registration_root)?;
            let record_nullifier = UInt8::new_witness_vec(cs.clone(), &record.nullifier)?;
            let record_key_hash = UInt8::new_witness_vec(cs.clone(), &record.key_set_hash)?;
            enforce_bytes_equal(&record_eid, &election_id_var)?;
            enforce_bytes_equal(&record_context, &final_context_var)?;
            enforce_bytes_equal(&record_region, &region_var)?;
            enforce_bytes_equal(&record_key_hash, &key_set_hash_var)?;

            let record_public_key = EdwardsVar::new_witness(cs.clone(), || Ok(record.public_key))?;
            let record_public_key_bytes =
                UInt8::new_witness_vec(cs.clone(), &encode_point(&record.public_key))?;
            bind_canonical_compressed_point(&record_public_key, &record_public_key_bytes)?;
            record_public_key.enforce_equal(&public_key_var)?;
            enforce_bytes_equal(&record_public_key_bytes, &public_key_bytes)?;

            let mut ciphertext_bytes = Vec::with_capacity(BALLOT_CORE_CANDIDATE_COUNT * 2);
            let mut ciphertext_vars = Vec::with_capacity(BALLOT_CORE_CANDIDATE_COUNT);
            for ciphertext in &record.ciphertexts {
                let c0_var = EdwardsVar::new_witness(cs.clone(), || Ok(ciphertext.c0))?;
                let c0_bytes = UInt8::new_witness_vec(cs.clone(), &encode_point(&ciphertext.c0))?;
                bind_canonical_compressed_point(&c0_var, &c0_bytes)?;
                let c1_var = EdwardsVar::new_witness(cs.clone(), || Ok(ciphertext.c1))?;
                let c1_bytes = UInt8::new_witness_vec(cs.clone(), &encode_point(&ciphertext.c1))?;
                bind_canonical_compressed_point(&c1_var, &c1_bytes)?;
                ciphertext_bytes.push(c0_bytes);
                ciphertext_bytes.push(c1_bytes);
                ciphertext_vars.push((c0_var, c1_var));
            }
            record_ciphertext_vars.push(ciphertext_vars);

            let mut field_vector = Vec::with_capacity(BALLOT_FIELD_VECTOR_BYTES);
            append_32_bytes_as_field_vector(&record_eid, &mut field_vector);
            append_32_bytes_as_field_vector(&record_context, &mut field_vector);
            append_u64_as_field(&record_region, &mut field_vector);
            append_32_bytes_as_field_vector(&record_root, &mut field_vector);
            append_32_bytes_as_field_vector(&record_nullifier, &mut field_vector);
            append_32_bytes_as_field_vector(&record_key_hash, &mut field_vector);
            append_32_bytes_as_field_vector(&record_public_key_bytes, &mut field_vector);
            for point_bytes in &ciphertext_bytes {
                append_32_bytes_as_field_vector(point_bytes, &mut field_vector);
            }
            if field_vector.len() != BALLOT_FIELD_VECTOR_BYTES {
                return Err(SynthesisError::Unsatisfiable);
            }

            let mut leaf_input = constant_prefix(BATCH_LEAF_DOMAIN);
            leaf_input.extend_from_slice(&election_id_var);
            leaf_input.extend_from_slice(&final_context_var);
            leaf_input.extend_from_slice(&region_var);
            leaf_input.extend_from_slice(&batch_index_var);
            leaf_input.extend_from_slice(&batch_size_var);
            leaf_input.extend(constant_u64(record_index as u64));
            leaf_input.extend(constant_u64(BALLOT_PUBLIC_INPUTS as u64));
            leaf_input.extend(field_vector);
            leaves.push(Sha256Gadget::digest(&leaf_input)?.0);
        }

        let mut node_input = constant_prefix(BATCH_NODE_DOMAIN);
        node_input.extend_from_slice(&leaves[0]);
        node_input.extend_from_slice(&leaves[1]);
        let computed_root = Sha256Gadget::digest(&node_input)?.0;
        enforce_bytes_equal(&computed_root, &batch_root_var)?;

        let mut accumulators = old_vars;
        for record in record_ciphertext_vars {
            for (accumulator, ciphertext) in accumulators.iter_mut().zip(record) {
                accumulator.0 = accumulator.0.clone() + ciphertext.0;
                accumulator.1 = accumulator.1.clone() + ciphertext.1;
            }
        }
        for ((actual_c0, actual_c1), (expected_c0, expected_c1)) in
            accumulators.iter().zip(&new_vars)
        {
            actual_c0.enforce_equal(expected_c0)?;
            actual_c1.enforce_equal(expected_c1)?;
        }
        Ok(())
    }
}

impl ConstraintSynthesizer<Fq> for StateTransitionCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_state_transition_g2a(
    circuit: &StateTransitionCircuit,
) -> Result<StateTransitionReport, SynthesisError> {
    if !circuit.has_expected_shape() {
        return Ok(StateTransitionReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            batch_size: circuit.records.len(),
            candidate_count: circuit.old_state.len(),
            ballot_public_inputs_per_record: 0,
            ballot_field_vector_bytes_per_record: 0,
            batch_hashes: 0,
            accumulator_components: 0,
            group_additions: 0,
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.clone().generate_relation(cs.clone())?;
    Ok(StateTransitionReport {
        satisfied: cs.is_satisfied()?,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        batch_size: STATE_BATCH_SIZE,
        candidate_count: BALLOT_CORE_CANDIDATE_COUNT,
        ballot_public_inputs_per_record: BALLOT_PUBLIC_INPUTS,
        ballot_field_vector_bytes_per_record: BALLOT_FIELD_VECTOR_BYTES,
        batch_hashes: STATE_BATCH_SIZE + 1,
        accumulator_components: BALLOT_CORE_CANDIDATE_COUNT * 2,
        group_additions: STATE_BATCH_SIZE * BALLOT_CORE_CANDIDATE_COUNT * 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generator, Scalar};
    use ark_ec::Group;
    use ark_ff::PrimeField;

    fn record(tag: u8, region: u64) -> StateBallotRecord {
        let public_key = generator().mul_bigint(Scalar::from(7u64).into_bigint());
        let ciphertexts = (1u64..=3)
            .map(|value| Ciphertext {
                c0: generator().mul_bigint(Scalar::from(value + u64::from(tag)).into_bigint()),
                c1: generator().mul_bigint(Scalar::from(value + 10 + u64::from(tag)).into_bigint()),
            })
            .collect();
        StateBallotRecord {
            election_id: [1; 32],
            final_context: [2; 32],
            region,
            registration_root: [3; 32],
            nullifier: [tag; 32],
            key_set_hash: [4; 32],
            public_key,
            ciphertexts,
        }
    }

    #[test]
    fn field_vector_has_frozen_width_and_order_changes_root() {
        let first = record(5, 2);
        let second = record(6, 2);
        assert_eq!(
            ballot_field_vector_bytes(&first).unwrap().len(),
            BALLOT_FIELD_VECTOR_BYTES
        );
        let forward = batch_root_v2(&[1; 32], &[2; 32], 2, 1, &[first.clone(), second.clone()]);
        let reverse = batch_root_v2(&[1; 32], &[2; 32], 2, 1, &[second, first]);
        assert_ne!(forward, reverse);
    }

    #[test]
    fn state_public_input_packing_has_frozen_width_and_binds_root() {
        let records = vec![record(5, 2), record(6, 2)];
        let old_state = vec![Ciphertext::zero(); BALLOT_CORE_CANDIDATE_COUNT];
        let new_state = fold_state(&old_state, &records).unwrap();
        let batch_root = batch_root_v2(&[1; 32], &[2; 32], 2, 1, &records).unwrap();
        let circuit = StateTransitionCircuit {
            election_id: [1; 32],
            final_context: [2; 32],
            region: 2,
            batch_index: 1,
            batch_size: STATE_BATCH_SIZE as u64,
            batch_root,
            key_set_hash: [4; 32],
            public_key: records[0].public_key,
            old_state,
            new_state,
            records,
        };
        let inputs = state_public_inputs(&circuit).unwrap();
        assert_eq!(inputs.len(), STATE_PUBLIC_INPUTS);

        let mut changed = circuit;
        changed.batch_root[0] ^= 1;
        assert_ne!(inputs, state_public_inputs(&changed).unwrap());
    }
}
