//! G1-B0: one weighted-ballot cryptographic core in R1CS.
//!
//! This is deliberately not the complete paper `C_vote`: it constrains the
//! registered-weight commitment opening, one-hot choice and three Weighted
//! ElGamal coordinates. Address/region derivation, voting nullifier and
//! registration-root membership remain later gates.

use crate::{
    cp_h3_gadget::{bind_canonical_compressed_point, fixed_base_mul},
    derive_secondary_generator, encode_point,
    p3c1_gadget::less_than,
    Ciphertext, Point, Scalar,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{Field, One, Zero};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar,
    fields::nonnative::NonNativeFieldVar, groups::CurveVar, prelude::ToBitsGadget, uint8::UInt8,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};

pub const BALLOT_CORE_CANDIDATE_COUNT: usize = 3;

#[derive(Clone)]
pub struct BallotCoreCircuit {
    pub public_key: Point,
    pub weight_commitment: Point,
    pub ciphertexts: Vec<Ciphertext>,
    pub registered_weight: u64,
    pub weight: u64,
    pub weight_opening: Scalar,
    pub choices: Vec<bool>,
    pub randomnesses: Vec<Scalar>,
}

#[derive(Clone, Debug)]
pub struct BallotCoreCircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub candidate_count: usize,
    pub one_hot_relations: usize,
    pub weight_range_relations: usize,
    pub weight_commitment_opening_equations: usize,
    pub elgamal_group_equations: usize,
    pub public_group_points: usize,
    pub private_scalar_values: usize,
}

fn allocate_u64_bits(
    cs: ConstraintSystemRef<Fq>,
    value: u64,
) -> Result<Vec<Boolean<Fq>>, SynthesisError> {
    (0..64)
        .map(|bit| Boolean::new_witness(cs.clone(), || Ok(((value >> bit) & 1) == 1)))
        .collect()
}

fn pack_bits(bits: &[Boolean<Fq>]) -> Result<FpVar<Fq>, SynthesisError> {
    let mut packed = FpVar::Constant(Fq::zero());
    let mut coefficient = Fq::one();
    for bit in bits {
        packed += bit.select(&FpVar::Constant(coefficient), &FpVar::Constant(Fq::zero()))?;
        coefficient.double_in_place();
    }
    Ok(packed)
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

impl BallotCoreCircuit {
    fn has_expected_shape(&self) -> bool {
        self.ciphertexts.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self.choices.len() == BALLOT_CORE_CANDIDATE_COUNT
            && self.randomnesses.len() == BALLOT_CORE_CANDIDATE_COUNT
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }

        let public_key_var = EdwardsVar::new_witness(cs.clone(), || Ok(self.public_key))?;
        let public_key_bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&self.public_key))?;
        bind_canonical_compressed_point(&public_key_var, &public_key_bytes)?;
        public_key_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;

        let commitment_var = EdwardsVar::new_witness(cs.clone(), || Ok(self.weight_commitment))?;
        let commitment_bytes =
            UInt8::new_input_vec(cs.clone(), &encode_point(&self.weight_commitment))?;
        bind_canonical_compressed_point(&commitment_var, &commitment_bytes)?;
        commitment_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;

        let public_bound =
            FpVar::<Fq>::new_input(cs.clone(), || Ok(Fq::from(self.registered_weight)))?;
        let bound_bits = allocate_u64_bits(cs.clone(), self.registered_weight)?;
        pack_bits(&bound_bits)?.enforce_equal(&public_bound)?;

        let weight_bits = allocate_u64_bits(cs.clone(), self.weight)?;
        let mut weight_nonzero = Boolean::FALSE;
        for bit in &weight_bits {
            weight_nonzero = weight_nonzero.or(bit)?;
        }
        weight_nonzero.enforce_equal(&Boolean::TRUE)?;
        let weight_below = less_than(&weight_bits, &bound_bits)?;
        let weight_equal = equal_bits(&weight_bits, &bound_bits)?;
        weight_below
            .or(&weight_equal)?
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

impl ConstraintSynthesizer<Fq> for BallotCoreCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_ballot_core_b0(
    circuit: &BallotCoreCircuit,
) -> Result<BallotCoreCircuitReport, SynthesisError> {
    if !circuit.has_expected_shape() {
        return Ok(BallotCoreCircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            candidate_count: circuit.ciphertexts.len(),
            one_hot_relations: 0,
            weight_range_relations: 0,
            weight_commitment_opening_equations: 0,
            elgamal_group_equations: 0,
            public_group_points: 0,
            private_scalar_values: 0,
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.clone().generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(BallotCoreCircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        candidate_count: BALLOT_CORE_CANDIDATE_COUNT,
        one_hot_relations: 1,
        weight_range_relations: 1,
        weight_commitment_opening_equations: 1,
        elgamal_group_equations: BALLOT_CORE_CANDIDATE_COUNT * 2,
        public_group_points: 2 + BALLOT_CORE_CANDIDATE_COUNT * 2,
        private_scalar_values: 2 + BALLOT_CORE_CANDIDATE_COUNT,
    })
}
