//! G1-B1: complete five-condition weighted-ballot relation in R1CS.
//!
//! This gate extends G1-B0 with an exact HMAC-SHA-256 credential suite,
//! deterministic four-region assignment and a depth-two SHA-256 registration
//! path. It measures only R1CS witness synthesis; Groth16 and SnarkPack remain
//! later gates.

use crate::{
    ballot_gadget::BALLOT_CORE_CANDIDATE_COUNT,
    cp_h3_gadget::{bind_canonical_compressed_point, fixed_base_mul},
    derive_secondary_generator, encode_point,
    p3c1_gadget::less_than,
    Ciphertext, Point, Scalar,
};
use ark_crypto_primitives::crh::sha256::constraints::Sha256Gadget;
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{One, ToConstraintField, Zero};
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    fields::{fp::FpVar, nonnative::NonNativeFieldVar},
    groups::CurveVar,
    prelude::{CondSelectGadget, ToBitsGadget},
    uint8::UInt8,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};
use sha2::{Digest, Sha256};
use vv_types::{sha256, CanonicalEncoder};

pub const BALLOT_PRF_SUITE_ID: &str = "VV/JUBJUB/BALLOT-HMAC-SHA256/v1";
pub const BALLOT_REGION_COUNT: u64 = 4;
pub const BALLOT_REGISTRATION_DEPTH: usize = 2;

const ADDRESS_DOMAIN: &[u8] = b"VV/JUBJUB/BALLOT/ADDRESS-PRF/v1";
const NULLIFIER_DOMAIN: &[u8] = b"VV/JUBJUB/BALLOT/NULLIFIER-PRF/v1";
const REGION_DOMAIN: &[u8] = b"VV/JUBJUB/BALLOT/REGION-ASSIGN/v1";
const REGISTRATION_ENTRY_DOMAIN: &[u8] = b"VV/JUBJUB/REGISTER/ENTRY/v1";
const MERKLE_NODE_DOMAIN: &[u8] = b"VV/JUBJUB/MERKLE/NODE/v1";

#[derive(Clone)]
pub struct BallotCvoteCircuit {
    pub election_id: [u8; 32],
    pub final_context: [u8; 32],
    pub region: u64,
    pub registration_root: [u8; 32],
    pub nullifier: [u8; 32],
    pub key_set_hash: [u8; 32],
    pub public_key: Point,
    pub ciphertexts: Vec<Ciphertext>,
    pub registered_weight_bound: u64,
    pub voter_secret: [u8; 32],
    pub address: [u8; 32],
    pub weight: u64,
    pub weight_opening: Scalar,
    pub weight_commitment: Point,
    pub merkle_siblings: Vec<[u8; 32]>,
    /// `true` means the current node is the right child.
    pub merkle_directions: Vec<bool>,
    pub choices: Vec<bool>,
    pub randomnesses: Vec<Scalar>,
}

#[derive(Clone, Debug)]
pub struct BallotCvoteCircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub candidate_count: usize,
    pub prf_evaluations: usize,
    pub region_hashes: usize,
    pub merkle_hashes: usize,
    pub merkle_depth: usize,
    pub one_hot_relations: usize,
    pub weight_range_relations: usize,
    pub weight_commitment_opening_equations: usize,
    pub elgamal_group_equations: usize,
}

fn canonical_message(
    domain: &[u8],
    election_id: &[u8; 32],
    final_context: &[u8; 32],
    key_set_hash: &[u8; 32],
) -> Vec<u8> {
    let mut encoded = CanonicalEncoder::new(domain);
    encoded.fixed(election_id);
    encoded.fixed(final_context);
    encoded.fixed(key_set_hash);
    encoded.finish()
}

fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    key_block[..32].copy_from_slice(key);
    let mut inner_pad = [0u8; 64];
    let mut outer_pad = [0u8; 64];
    for index in 0..64 {
        inner_pad[index] = key_block[index] ^ 0x36;
        outer_pad[index] = key_block[index] ^ 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

pub fn ballot_address(
    voter_secret: &[u8; 32],
    election_id: &[u8; 32],
    final_context: &[u8; 32],
    key_set_hash: &[u8; 32],
) -> [u8; 32] {
    hmac_sha256(
        voter_secret,
        &canonical_message(ADDRESS_DOMAIN, election_id, final_context, key_set_hash),
    )
}

pub fn ballot_nullifier(
    voter_secret: &[u8; 32],
    election_id: &[u8; 32],
    final_context: &[u8; 32],
    key_set_hash: &[u8; 32],
) -> [u8; 32] {
    hmac_sha256(
        voter_secret,
        &canonical_message(NULLIFIER_DOMAIN, election_id, final_context, key_set_hash),
    )
}

pub fn ballot_region(final_context: &[u8; 32], address: &[u8; 32]) -> u64 {
    let mut encoded = CanonicalEncoder::new(REGION_DOMAIN);
    encoded.fixed(final_context);
    encoded.fixed(address);
    let digest = sha256(&encoded.finish());
    u64::from(digest[31] & 0x03) + 1
}

pub fn registration_leaf(address: &[u8; 32], commitment: &Point) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(REGISTRATION_ENTRY_DOMAIN);
    encoded.fixed(address);
    encoded.fixed(&encode_point(commitment));
    sha256(&encoded.finish())
}

pub fn registration_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(MERKLE_NODE_DOMAIN);
    encoded.fixed(left);
    encoded.fixed(right);
    sha256(&encoded.finish())
}

fn canonical_prefix_var(domain: &[u8]) -> Vec<UInt8<Fq>> {
    UInt8::constant_vec(&CanonicalEncoder::new(domain).finish())
}

fn hmac_sha256_var(
    key: &[UInt8<Fq>],
    message: &[UInt8<Fq>],
) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
    if key.len() != 32 {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut key_block = key.to_vec();
    key_block.extend(UInt8::constant_vec(&[0u8; 32]));
    let inner_pad = key_block
        .iter()
        .map(|byte| byte.xor(&UInt8::constant(0x36)))
        .collect::<Result<Vec<_>, _>>()?;
    let outer_pad = key_block
        .iter()
        .map(|byte| byte.xor(&UInt8::constant(0x5c)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut inner_input = inner_pad;
    inner_input.extend_from_slice(message);
    let inner_digest = Sha256Gadget::digest(&inner_input)?.0;
    let mut outer_input = outer_pad;
    outer_input.extend(inner_digest);
    Ok(Sha256Gadget::digest(&outer_input)?.0)
}

fn prf_message_var(
    domain: &[u8],
    election_id: &[UInt8<Fq>],
    final_context: &[UInt8<Fq>],
    key_set_hash: &[UInt8<Fq>],
) -> Vec<UInt8<Fq>> {
    let mut message = canonical_prefix_var(domain);
    message.extend_from_slice(election_id);
    message.extend_from_slice(final_context);
    message.extend_from_slice(key_set_hash);
    message
}

fn allocate_u64_bits(
    cs: ConstraintSystemRef<Fq>,
    value: u64,
) -> Result<Vec<Boolean<Fq>>, SynthesisError> {
    (0..64)
        .map(|bit| Boolean::new_witness(cs.clone(), || Ok(((value >> bit) & 1) == 1)))
        .collect()
}

fn equal_bits(a: &[Boolean<Fq>], b: &[Boolean<Fq>]) -> Result<Boolean<Fq>, SynthesisError> {
    if a.len() != b.len() {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut equal = Boolean::TRUE;
    for (a_bit, b_bit) in a.iter().zip(b) {
        equal = equal.and(&a_bit.xor(b_bit)?.not())?;
    }
    Ok(equal)
}

impl BallotCvoteCircuit {
    fn has_expected_shape(&self) -> bool {
        self.ciphertexts.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self.choices.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self.randomnesses.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self.merkle_siblings.len() == BALLOT_REGISTRATION_DEPTH
            && self.merkle_directions.len() == BALLOT_REGISTRATION_DEPTH
            && (1..=BALLOT_REGION_COUNT).contains(&self.region)
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }

        // Public ballot header in the paper's order.
        let election_id_var = UInt8::new_input_vec(cs.clone(), &self.election_id)?;
        let final_context_var = UInt8::new_input_vec(cs.clone(), &self.final_context)?;
        let region_var = FpVar::<Fq>::new_input(cs.clone(), || Ok(Fq::from(self.region)))?;
        let registration_root_var = UInt8::new_input_vec(cs.clone(), &self.registration_root)?;
        let nullifier_var = UInt8::new_input_vec(cs.clone(), &self.nullifier)?;
        let key_set_hash_var = UInt8::new_input_vec(cs.clone(), &self.key_set_hash)?;

        let voter_secret_var = UInt8::new_witness_vec(cs.clone(), &self.voter_secret)?;
        let address_var = UInt8::new_witness_vec(cs.clone(), &self.address)?;

        let address_message = prf_message_var(
            ADDRESS_DOMAIN,
            &election_id_var,
            &final_context_var,
            &key_set_hash_var,
        );
        hmac_sha256_var(&voter_secret_var, &address_message)?.enforce_equal(&address_var)?;

        let nullifier_message = prf_message_var(
            NULLIFIER_DOMAIN,
            &election_id_var,
            &final_context_var,
            &key_set_hash_var,
        );
        hmac_sha256_var(&voter_secret_var, &nullifier_message)?.enforce_equal(&nullifier_var)?;

        let mut region_message = canonical_prefix_var(REGION_DOMAIN);
        region_message.extend_from_slice(&final_context_var);
        region_message.extend_from_slice(&address_var);
        let region_digest = Sha256Gadget::digest(&region_message)?.0;
        let region_bits = region_digest[31].to_bits_le()?;
        let region_from_hash = FpVar::Constant(Fq::one())
            + region_bits[0].select(&FpVar::Constant(Fq::one()), &FpVar::Constant(Fq::zero()))?
            + region_bits[1].select(
                &FpVar::Constant(Fq::from(2u64)),
                &FpVar::Constant(Fq::zero()),
            )?;
        region_from_hash.enforce_equal(&region_var)?;

        let commitment_var = EdwardsVar::new_witness(cs.clone(), || Ok(self.weight_commitment))?;
        let commitment_bytes =
            UInt8::new_witness_vec(cs.clone(), &encode_point(&self.weight_commitment))?;
        bind_canonical_compressed_point(&commitment_var, &commitment_bytes)?;
        commitment_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;

        let mut leaf_input = canonical_prefix_var(REGISTRATION_ENTRY_DOMAIN);
        leaf_input.extend_from_slice(&address_var);
        leaf_input.extend_from_slice(&commitment_bytes);
        let mut current = Sha256Gadget::digest(&leaf_input)?.0;
        for ((sibling, direction), _) in self
            .merkle_siblings
            .iter()
            .zip(&self.merkle_directions)
            .zip(0..BALLOT_REGISTRATION_DEPTH)
        {
            let sibling_var = UInt8::new_witness_vec(cs.clone(), sibling)?;
            let direction_var = Boolean::new_witness(cs.clone(), || Ok(*direction))?;
            let left = current
                .iter()
                .zip(&sibling_var)
                .map(|(node, sibling)| UInt8::conditionally_select(&direction_var, sibling, node))
                .collect::<Result<Vec<_>, _>>()?;
            let right = current
                .iter()
                .zip(&sibling_var)
                .map(|(node, sibling)| UInt8::conditionally_select(&direction_var, node, sibling))
                .collect::<Result<Vec<_>, _>>()?;
            let mut node_input = canonical_prefix_var(MERKLE_NODE_DOMAIN);
            node_input.extend(left);
            node_input.extend(right);
            current = Sha256Gadget::digest(&node_input)?.0;
        }
        current.enforce_equal(&registration_root_var)?;

        let weight_bits = allocate_u64_bits(cs.clone(), self.weight)?;
        let bound_bits = (0..64)
            .map(|bit| Boolean::constant(((self.registered_weight_bound >> bit) & 1) == 1))
            .collect::<Vec<_>>();
        let mut weight_nonzero = Boolean::FALSE;
        for bit in &weight_bits {
            weight_nonzero = weight_nonzero.or(bit)?;
        }
        weight_nonzero.enforce_equal(&Boolean::TRUE)?;
        less_than(&weight_bits, &bound_bits)?
            .or(&equal_bits(&weight_bits, &bound_bits)?)?
            .enforce_equal(&Boolean::TRUE)?;

        let choice_vars = self
            .choices
            .iter()
            .map(|choice| Boolean::new_witness(cs.clone(), || Ok(*choice)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut choice_sum = FpVar::Constant(Fq::zero());
        for choice in &choice_vars {
            choice_sum +=
                choice.select(&FpVar::Constant(Fq::one()), &FpVar::Constant(Fq::zero()))?;
        }
        choice_sum.enforce_equal(&FpVar::Constant(Fq::one()))?;

        let weight_g = fixed_base_mul(&weight_bits)?;
        let opening_var =
            NonNativeFieldVar::<Scalar, Fq>::new_witness(cs.clone(), || Ok(self.weight_opening))?;
        let registration_h = derive_secondary_generator(b"REGISTRATION-PEDERSEN-H")
            .map_err(|_| SynthesisError::Unsatisfiable)?;
        let registration_h_var = EdwardsVar::new_constant(cs.clone(), registration_h)?;
        let opening_h = registration_h_var.scalar_mul_le(opening_var.to_bits_le()?.iter())?;
        commitment_var.enforce_equal(&(weight_g.clone() + opening_h))?;

        let public_key_var = EdwardsVar::new_witness(cs.clone(), || Ok(self.public_key))?;
        let public_key_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&self.public_key))?;
        bind_canonical_compressed_point(&public_key_var, &public_key_bytes)?;
        public_key_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;

        for ((ciphertext, randomness), choice) in self
            .ciphertexts
            .iter()
            .zip(&self.randomnesses)
            .zip(&choice_vars)
        {
            let randomness_var =
                NonNativeFieldVar::<Scalar, Fq>::new_witness(cs.clone(), || Ok(*randomness))?;
            let randomness_bits = randomness_var.to_bits_le()?;
            let expected_c0 = fixed_base_mul(&randomness_bits)?;
            let expected_shared = public_key_var.scalar_mul_le(randomness_bits.iter())?;
            let message_g = choice.select(&weight_g, &EdwardsVar::zero())?;

            let c0_var = EdwardsVar::new_witness(cs.clone(), || Ok(ciphertext.c0))?;
            let c0_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&ciphertext.c0))?;
            bind_canonical_compressed_point(&c0_var, &c0_bytes)?;
            c0_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;
            c0_var.enforce_equal(&expected_c0)?;

            let c1_var = EdwardsVar::new_witness(cs.clone(), || Ok(ciphertext.c1))?;
            let c1_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&ciphertext.c1))?;
            bind_canonical_compressed_point(&c1_var, &c1_bytes)?;
            c1_var.enforce_equal(&(expected_shared + message_g))?;
        }
        Ok(())
    }
}

impl ConstraintSynthesizer<Fq> for BallotCvoteCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_ballot_cvote_b1(
    circuit: &BallotCvoteCircuit,
) -> Result<BallotCvoteCircuitReport, SynthesisError> {
    if !circuit.has_expected_shape() {
        return Ok(BallotCvoteCircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            candidate_count: circuit.ciphertexts.len(),
            prf_evaluations: 0,
            region_hashes: 0,
            merkle_hashes: 0,
            merkle_depth: circuit.merkle_siblings.len(),
            one_hot_relations: 0,
            weight_range_relations: 0,
            weight_commitment_opening_equations: 0,
            elgamal_group_equations: 0,
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.clone().generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(BallotCvoteCircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        candidate_count: BALLOT_CORE_CANDIDATE_COUNT,
        prf_evaluations: 2,
        region_hashes: 1,
        merkle_hashes: 1 + BALLOT_REGISTRATION_DEPTH,
        merkle_depth: BALLOT_REGISTRATION_DEPTH,
        one_hot_relations: 1,
        weight_range_relations: 1,
        weight_commitment_opening_equations: 1,
        elgamal_group_equations: BALLOT_CORE_CANDIDATE_COUNT * 2,
    })
}

/// Pack the public instance in the exact order allocated by
/// `BallotCvoteCircuit::generate_relation`.
pub fn ballot_cvote_public_inputs(circuit: &BallotCvoteCircuit) -> Option<Vec<Fq>> {
    if !circuit.has_expected_shape() {
        return None;
    }
    let mut inputs = Vec::new();
    let election_fields: Vec<Fq> = circuit.election_id.as_slice().to_field_elements()?;
    inputs.extend(election_fields);
    let context_fields: Vec<Fq> = circuit.final_context.as_slice().to_field_elements()?;
    inputs.extend(context_fields);
    inputs.push(Fq::from(circuit.region));
    let root_fields: Vec<Fq> = circuit.registration_root.as_slice().to_field_elements()?;
    inputs.extend(root_fields);
    let nullifier_fields: Vec<Fq> = circuit.nullifier.as_slice().to_field_elements()?;
    inputs.extend(nullifier_fields);
    let key_set_fields: Vec<Fq> = circuit.key_set_hash.as_slice().to_field_elements()?;
    inputs.extend(key_set_fields);
    let public_key_bytes = encode_point(&circuit.public_key);
    let public_key_fields: Vec<Fq> = public_key_bytes.as_slice().to_field_elements()?;
    inputs.extend(public_key_fields);
    for ciphertext in &circuit.ciphertexts {
        let c0_bytes = encode_point(&ciphertext.c0);
        let c0_fields: Vec<Fq> = c0_bytes.as_slice().to_field_elements()?;
        inputs.extend(c0_fields);
        let c1_bytes = encode_point(&ciphertext.c1);
        let c1_fields: Vec<Fq> = c1_bytes.as_slice().to_field_elements()?;
        inputs.extend(c1_fields);
    }
    Some(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_domains_are_separated_and_region_is_bounded() {
        let secret = [7u8; 32];
        let election = [1u8; 32];
        let context = [2u8; 32];
        let key_set = [3u8; 32];
        assert_ne!(
            ballot_address(&secret, &election, &context, &key_set),
            ballot_nullifier(&secret, &election, &context, &key_set)
        );
        let address = ballot_address(&secret, &election, &context, &key_set);
        assert!((1..=BALLOT_REGION_COUNT).contains(&ballot_region(&context, &address)));
    }
}
