//! Isolated G0.6 curve-decision experiment using Jubjub over the BLS12-381
//! scalar field. This module is not wired into the VeriVoting manifest or
//! native protocol; it exists only to measure circuit costs before any curve
//! migration decision.

use ark_bls12_381::Bls12_381;
use ark_ec::Group;
use ark_ed_on_bls12_381::{
    constraints::EdwardsVar, EdwardsProjective, Fq as ConstraintField, Fr as JubScalar,
};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
    eq::EqGadget,
    fields::fp::FpVar,
    groups::CurveVar,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::cp_gadget::CpGadgetError;

const SCALAR_BITS: usize = 256;

#[derive(Clone)]
struct FriendlyCpCircuit {
    commitment: EdwardsProjective,
    verification_key: EdwardsProjective,
    challenge: JubScalar,
    response: JubScalar,
}

#[derive(Clone)]
struct FriendlyElgamalCircuit {
    public_key: EdwardsProjective,
    c0: EdwardsProjective,
    c1: EdwardsProjective,
    randomness: JubScalar,
    message: JubScalar,
}

#[derive(Clone)]
struct FriendlyStateAddCircuit {
    old: EdwardsProjective,
    ballot: EdwardsProjective,
    new: EdwardsProjective,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendlyCircuitMetrics {
    pub component: String,
    pub constraints: usize,
    pub public_inputs: usize,
    pub synthesis_witness_ms: f64,
    pub setup_ms: f64,
    pub prove_ms: f64,
    pub verify_ms: f64,
    pub proof_bytes: usize,
    pub valid_relation_satisfied: bool,
    pub valid_proof_verified: bool,
    pub changed_public_input_rejected: bool,
    pub invalid_relation_rejected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendlyCurveReport {
    pub classification: String,
    pub experiment_group: String,
    pub proof_curve: String,
    pub manifest_or_native_protocol_changed: bool,
    pub public_points_checked_on_curve: bool,
    pub public_points_checked_prime_order: bool,
    pub canonical_scalar_range_constrained: bool,
    pub cp_challenge_recomputed_in_circuit: bool,
    pub cp_wrong_commitment_rejected: bool,
    pub cp_wrong_challenge_rejected: bool,
    pub cp_wrong_response_rejected: bool,
    pub cp: FriendlyCircuitMetrics,
    pub elgamal_coordinate: FriendlyCircuitMetrics,
    pub state_point_addition: FriendlyCircuitMetrics,
    pub full_p3: bool,
    pub limitations: Vec<String>,
}

fn scalar_modulus_minus_one() -> Vec<u64> {
    let mut limbs = JubScalar::MODULUS.as_ref().to_vec();
    for limb in &mut limbs {
        let (value, borrowed) = limb.overflowing_sub(1);
        *limb = value;
        if !borrowed {
            break;
        }
    }
    limbs
}

fn allocate_scalar_bits(
    cs: ConstraintSystemRef<ConstraintField>,
    value: JubScalar,
    mode: AllocationMode,
) -> Result<Vec<Boolean<ConstraintField>>, SynthesisError> {
    let native_bits = value.into_bigint().to_bits_le();
    let bit_mode = if matches!(mode, AllocationMode::Input) {
        AllocationMode::Witness
    } else {
        mode
    };
    let bits = (0..SCALAR_BITS)
        .map(|index| {
            Boolean::new_variable(
                cs.clone(),
                || Ok(native_bits.get(index).copied().unwrap_or(false)),
                bit_mode,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Boolean::enforce_smaller_or_equal_than_le(&bits, scalar_modulus_minus_one())?;
    if matches!(mode, AllocationMode::Input) {
        let public_value =
            ConstraintField::from_le_bytes_mod_order(&value.into_bigint().to_bytes_le());
        let public = FpVar::new_input(cs, || Ok(public_value))?;
        let mut packed = FpVar::Constant(ConstraintField::from(0u64));
        let mut coefficient = ConstraintField::from(1u64);
        for bit in &bits {
            packed += bit.select(
                &FpVar::Constant(coefficient),
                &FpVar::Constant(ConstraintField::from(0u64)),
            )?;
            coefficient += coefficient;
        }
        packed.enforce_equal(&public)?;
    }
    Ok(bits)
}

fn fixed_base_mul(bits: &[Boolean<ConstraintField>]) -> Result<EdwardsVar, SynthesisError> {
    let mut powers = Vec::with_capacity(bits.len());
    let mut current = EdwardsProjective::generator();
    for _ in bits {
        powers.push(current);
        current.double_in_place();
    }
    let mut result = EdwardsVar::zero();
    result.precomputed_base_scalar_mul_le(bits.iter().zip(&powers))?;
    Ok(result)
}

fn allocate_public_point(
    cs: ConstraintSystemRef<ConstraintField>,
    value: EdwardsProjective,
    require_nonzero: bool,
) -> Result<EdwardsVar, SynthesisError> {
    let point = EdwardsVar::new_input(cs, || Ok(value))?;
    point.enforce_prime_order()?;
    if require_nonzero {
        point.is_zero()?.enforce_equal(&Boolean::constant(false))?;
    }
    Ok(point)
}

impl ConstraintSynthesizer<ConstraintField> for FriendlyCpCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<ConstraintField>,
    ) -> Result<(), SynthesisError> {
        let commitment = allocate_public_point(cs.clone(), self.commitment, true)?;
        let verification_key = allocate_public_point(cs.clone(), self.verification_key, true)?;
        let challenge = allocate_scalar_bits(cs.clone(), self.challenge, AllocationMode::Input)?;
        let response = allocate_scalar_bits(cs, self.response, AllocationMode::Witness)?;

        let response_g = fixed_base_mul(&response)?;
        let challenge_vk = verification_key.scalar_mul_le(challenge.iter())?;
        response_g.enforce_equal(&(commitment + challenge_vk))
    }
}

impl ConstraintSynthesizer<ConstraintField> for FriendlyElgamalCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<ConstraintField>,
    ) -> Result<(), SynthesisError> {
        let public_key = allocate_public_point(cs.clone(), self.public_key, true)?;
        let c0 = allocate_public_point(cs.clone(), self.c0, false)?;
        let c1 = allocate_public_point(cs.clone(), self.c1, false)?;
        let randomness =
            allocate_scalar_bits(cs.clone(), self.randomness, AllocationMode::Witness)?;
        let message = allocate_scalar_bits(cs, self.message, AllocationMode::Witness)?;

        let randomness_g = fixed_base_mul(&randomness)?;
        randomness_g.enforce_equal(&c0)?;
        let randomness_pk = public_key.scalar_mul_le(randomness.iter())?;
        let message_g = fixed_base_mul(&message)?;
        (randomness_pk + message_g).enforce_equal(&c1)
    }
}

impl ConstraintSynthesizer<ConstraintField> for FriendlyStateAddCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<ConstraintField>,
    ) -> Result<(), SynthesisError> {
        let old = allocate_public_point(cs.clone(), self.old, false)?;
        let ballot = allocate_public_point(cs.clone(), self.ballot, false)?;
        let new = allocate_public_point(cs, self.new, false)?;
        (old + ballot).enforce_equal(&new)
    }
}

fn run_groth16<C>(
    name: &str,
    circuit: C,
    invalid: C,
    seed: u64,
) -> Result<FriendlyCircuitMetrics, CpGadgetError>
where
    C: ConstraintSynthesizer<ConstraintField> + Clone,
{
    let cs = ConstraintSystem::<ConstraintField>::new_ref();
    let synthesis_started = Instant::now();
    circuit.clone().generate_constraints(cs.clone())?;
    let valid_relation_satisfied = cs.is_satisfied()?;
    let synthesis_witness_ms = synthesis_started.elapsed().as_secs_f64() * 1000.0;
    if !valid_relation_satisfied {
        return Err(CpGadgetError::Synthesis(SynthesisError::Unsatisfiable));
    }
    let constraints = cs.num_constraints();
    let public_inputs = cs.num_instance_variables().saturating_sub(1);
    let public_assignment = {
        let borrowed = cs.borrow().ok_or(SynthesisError::AssignmentMissing)?;
        borrowed.instance_assignment[1..].to_vec()
    };

    let mut setup_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x4652_4945_4E44_4C59);
    let setup_started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut setup_rng,
    )?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;

    let mut proof_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x4355_5256_455F_4350);
    let prove_started = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit,
        &parameters,
        &mut proof_rng,
    )?;
    let prove_ms = prove_started.elapsed().as_secs_f64() * 1000.0;
    let prepared = prepare_verifying_key(&parameters.vk);
    let verify_started = Instant::now();
    let valid_proof_verified =
        Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &public_assignment)?;
    let verify_ms = verify_started.elapsed().as_secs_f64() * 1000.0;

    let mut changed_public = public_assignment.clone();
    let last = changed_public
        .last_mut()
        .ok_or(SynthesisError::AssignmentMissing)?;
    *last += ConstraintField::from(1u64);
    let changed_public_input_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed_public)?;

    let invalid_cs = ConstraintSystem::<ConstraintField>::new_ref();
    invalid.generate_constraints(invalid_cs.clone())?;
    let invalid_relation_rejected = !invalid_cs.is_satisfied()?;

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;
    Ok(FriendlyCircuitMetrics {
        component: name.to_owned(),
        constraints,
        public_inputs,
        synthesis_witness_ms,
        setup_ms,
        prove_ms,
        verify_ms,
        proof_bytes: proof_encoding.len(),
        valid_relation_satisfied,
        valid_proof_verified,
        changed_public_input_rejected,
        invalid_relation_rejected,
    })
}

pub fn run_friendly_curve_experiment(seed: u64) -> Result<FriendlyCurveReport, CpGadgetError> {
    let generator = EdwardsProjective::generator();
    let secret = JubScalar::from(23u64);
    let nonce = JubScalar::from(91u64);
    let challenge = JubScalar::from(17u64);
    let response = nonce + challenge * secret;
    let verification_key = generator.mul_bigint(secret.into_bigint());
    let commitment = generator.mul_bigint(nonce.into_bigint());
    let cp = FriendlyCpCircuit {
        commitment,
        verification_key,
        challenge,
        response,
    };
    let mut wrong_commitment = cp.clone();
    wrong_commitment.commitment += generator;
    let mut wrong_challenge = cp.clone();
    wrong_challenge.challenge += JubScalar::from(1u64);
    let mut wrong_response = cp.clone();
    wrong_response.response += JubScalar::from(1u64);
    let cp_wrong_commitment_rejected = relation_is_unsatisfied(wrong_commitment.clone())?;
    let cp_wrong_challenge_rejected = relation_is_unsatisfied(wrong_challenge)?;
    let cp_wrong_response_rejected = relation_is_unsatisfied(wrong_response)?;
    let cp_metrics = run_groth16("single_cp_equation", cp, wrong_commitment, seed ^ 0x4350)?;

    let randomness = JubScalar::from(31u64);
    let message = JubScalar::from(3u64);
    let c0 = generator.mul_bigint(randomness.into_bigint());
    let c1 = verification_key.mul_bigint(randomness.into_bigint())
        + generator.mul_bigint(message.into_bigint());
    let elgamal = FriendlyElgamalCircuit {
        public_key: verification_key,
        c0,
        c1,
        randomness,
        message,
    };
    let mut wrong_elgamal = elgamal.clone();
    wrong_elgamal.c1 += generator;
    let elgamal_metrics = run_groth16(
        "one_elgamal_coordinate",
        elgamal,
        wrong_elgamal,
        seed ^ 0x454C_4741,
    )?;

    let old = generator.mul_bigint(JubScalar::from(5u64).into_bigint());
    let state = FriendlyStateAddCircuit {
        old,
        ballot: c1,
        new: old + c1,
    };
    let mut wrong_state = state.clone();
    wrong_state.new += generator;
    let state_metrics = run_groth16(
        "one_homomorphic_point_addition",
        state,
        wrong_state,
        seed ^ 0x5354_4154,
    )?;

    Ok(FriendlyCurveReport {
        classification: "isolated-snark-friendly-curve-decision-experiment-not-protocol-migration"
            .to_owned(),
        experiment_group: "Jubjub/ark-ed-on-bls12-381-0.4.0".to_owned(),
        proof_curve: "BLS12-381/ark-groth16-0.4.0".to_owned(),
        manifest_or_native_protocol_changed: false,
        public_points_checked_on_curve: true,
        public_points_checked_prime_order: true,
        canonical_scalar_range_constrained: true,
        cp_challenge_recomputed_in_circuit: false,
        cp_wrong_commitment_rejected,
        cp_wrong_challenge_rejected,
        cp_wrong_response_rejected,
        cp: cp_metrics,
        elgamal_coordinate: elgamal_metrics,
        state_point_addition: state_metrics,
        full_p3: false,
        limitations: vec![
            "challenge is a canonical public scalar, not an in-circuit Fiat-Shamir hash".to_owned(),
            "ElGamal row is one coordinate relation, not the weighted ballot circuit".to_owned(),
            "state row is one group-component addition, not Pi_state".to_owned(),
            "Jubjub is not wired into DKG, commitments, ballots, or the protocol manifest"
                .to_owned(),
        ],
    })
}

fn relation_is_unsatisfied<C>(circuit: C) -> Result<bool, CpGadgetError>
where
    C: ConstraintSynthesizer<ConstraintField>,
{
    let cs = ConstraintSystem::<ConstraintField>::new_ref();
    circuit.generate_constraints(cs.clone())?;
    Ok(!cs.is_satisfied()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_curve_relations_prove_and_reject_mutations() {
        let report = run_friendly_curve_experiment(20261020).unwrap();
        assert!(report.cp.valid_proof_verified);
        assert!(report.elgamal_coordinate.valid_proof_verified);
        assert!(report.state_point_addition.valid_proof_verified);
        assert!(report.cp_wrong_commitment_rejected);
        assert!(report.cp_wrong_challenge_rejected);
        assert!(report.cp_wrong_response_rejected);
        assert!(!report.full_p3);
    }
}
