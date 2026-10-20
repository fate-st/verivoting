//! G0.5 feasibility circuit for one real Ristretto255 Chaum--Pedersen group
//! equation inside BLS12-381 Groth16.
//!
//! This module constrains `z*G = A + d*VK` using non-native arithmetic in the
//! Curve25519 base field and Ristretto quotient equality. G0.5 deliberately
//! exposes `d` as a public input: compressed-point decoding and SHA-512
//! Fiat--Shamir recomputation remain explicit blockers for full P3.

#![allow(non_local_definitions)]

use crate::hash_to_scalar_gadget::allocate_scalar_bits;
use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::{BigInteger, Field, Fp256, MontBackend, MontConfig, One, PrimeField, Zero};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
    eq::EqGadget,
    fields::{nonnative::NonNativeFieldVar, FieldVar},
    select::CondSelectGadget,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

#[derive(MontConfig)]
#[modulus = "57896044618658097711785492504343953926634992332820282019728792003956564819949"]
#[generator = "2"]
pub struct Curve25519BaseConfig;

pub type Curve25519Base = Fp256<MontBackend<Curve25519BaseConfig, 4>>;
type FqVar = NonNativeFieldVar<Curve25519Base, Fr>;

#[derive(Debug, Error)]
pub enum CpGadgetError {
    #[error("non-canonical or invalid Ristretto encoding: {0}")]
    InvalidEncoding(&'static str),
    #[error("R1CS/Groth16 error: {0}")]
    Synthesis(#[from] SynthesisError),
    #[error("serialization error: {0}")]
    Serialization(#[from] ark_serialize::SerializationError),
}

#[derive(Clone, Copy, Debug)]
pub struct EdwardsAffineValue {
    pub x: Curve25519Base,
    pub y: Curve25519Base,
}

impl EdwardsAffineValue {
    pub fn x_decimal(&self) -> String {
        self.x.into_bigint().to_string()
    }

    pub fn y_decimal(&self) -> String {
        self.y.into_bigint().to_string()
    }
}

#[derive(Clone)]
struct ExtendedPointVar {
    x: FqVar,
    y: FqVar,
    z: FqVar,
    t: FqVar,
}

impl ExtendedPointVar {
    fn identity() -> Self {
        Self {
            x: FqVar::zero(),
            y: FqVar::one(),
            z: FqVar::one(),
            t: FqVar::zero(),
        }
    }

    fn constant(value: EdwardsAffineValue) -> Self {
        let x = FqVar::constant(value.x);
        let y = FqVar::constant(value.y);
        Self {
            t: &x * &y,
            x,
            y,
            z: FqVar::one(),
        }
    }

    fn new_public(
        cs: ConstraintSystemRef<Fr>,
        value: EdwardsAffineValue,
    ) -> Result<Self, SynthesisError> {
        let x = FqVar::new_input(cs.clone(), || Ok(value.x))?;
        let y = FqVar::new_input(cs, || Ok(value.y))?;
        Ok(Self {
            t: &x * &y,
            x,
            y,
            z: FqVar::one(),
        })
    }

    fn new_witness(
        cs: ConstraintSystemRef<Fr>,
        value: EdwardsAffineValue,
    ) -> Result<Self, SynthesisError> {
        let x = FqVar::new_witness(cs.clone(), || Ok(value.x))?;
        let y = FqVar::new_witness(cs, || Ok(value.y))?;
        Ok(Self {
            t: &x * &y,
            x,
            y,
            z: FqVar::one(),
        })
    }

    fn negate(&self) -> Result<Self, SynthesisError> {
        Ok(Self {
            x: self.x.negate()?,
            y: self.y.clone(),
            z: self.z.clone(),
            t: self.t.negate()?,
        })
    }

    fn select(
        condition: &Boolean<Fr>,
        when_true: &Self,
        when_false: &Self,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            x: FqVar::conditionally_select(condition, &when_true.x, &when_false.x)?,
            y: FqVar::conditionally_select(condition, &when_true.y, &when_false.y)?,
            z: FqVar::conditionally_select(condition, &when_true.z, &when_false.z)?,
            t: FqVar::conditionally_select(condition, &when_true.t, &when_false.t)?,
        })
    }
}

fn edwards_d() -> Curve25519Base {
    -Curve25519Base::from(121_665u64) * Curve25519Base::from(121_666u64).inverse().unwrap()
}

fn add_points(left: &ExtendedPointVar, right: &ExtendedPointVar) -> ExtendedPointVar {
    // Complete extended-coordinate addition for a=-1 twisted Edwards curves.
    let a = (&left.y - &left.x) * (&right.y - &right.x);
    let b = (&left.y + &left.x) * (&right.y + &right.x);
    let c = (&left.t * &right.t) * (edwards_d() + edwards_d());
    let d = (&left.z * &right.z) * Curve25519Base::from(2u64);
    let e = &b - &a;
    let f = &d - &c;
    let g = &d + &c;
    let h = &b + &a;
    ExtendedPointVar {
        x: &e * &f,
        y: &g * &h,
        t: &e * &h,
        z: &f * &g,
    }
}

fn enforce_affine_curve(point: &ExtendedPointVar) -> Result<(), SynthesisError> {
    let x2 = &point.x * &point.x;
    let y2 = &point.y * &point.y;
    let left = &y2 - &x2;
    let right = FqVar::one() + (&x2 * &y2) * edwards_d();
    left.enforce_equal(&right)?;
    let identity = point.x.is_zero()?.and(&point.y.is_eq(&FqVar::one())?)?;
    identity.enforce_equal(&Boolean::FALSE)?;
    Ok(())
}

fn enforce_ristretto_quotient_equal(
    left: &ExtendedPointVar,
    right: &ExtendedPointVar,
) -> Result<(), SynthesisError> {
    // Ristretto equality on Edwards representatives:
    // X1*Y2 == Y1*X2 OR X1*X2 == Y1*Y2.
    let first = (&left.x * &right.y).is_eq(&(&left.y * &right.x))?;
    let second = (&left.x * &right.x).is_eq(&(&left.y * &right.y))?;
    first.or(&second)?.enforce_equal(&Boolean::TRUE)
}

fn double_scalar_mul_g_minus_vk(
    generator: &ExtendedPointVar,
    verification_key: &ExtendedPointVar,
    response_bits: &[Boolean<Fr>],
    challenge_bits: &[Boolean<Fr>],
    scalar_bits: usize,
) -> Result<ExtendedPointVar, SynthesisError> {
    if response_bits.len() != 256 || challenge_bits.len() != 256 || scalar_bits > 256 {
        return Err(SynthesisError::Unsatisfiable);
    }
    let identity = ExtendedPointVar::identity();
    let negative_vk = verification_key.negate()?;
    let g_minus_vk = add_points(generator, &negative_vk);
    let mut accumulator = identity.clone();
    for index in (0..scalar_bits).rev() {
        let response_bit = &response_bits[index];
        let challenge_bit = &challenge_bits[index];
        let doubled = add_points(&accumulator, &accumulator);
        let when_challenge_zero = ExtendedPointVar::select(response_bit, generator, &identity)?;
        let when_challenge_one = ExtendedPointVar::select(response_bit, &g_minus_vk, &negative_vk)?;
        let selected =
            ExtendedPointVar::select(challenge_bit, &when_challenge_one, &when_challenge_zero)?;
        accumulator = add_points(&doubled, &selected);
    }
    Ok(accumulator)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CpScalarMulVersion {
    Baseline256,
    Canonical253,
}

impl CpScalarMulVersion {
    pub fn label(self) -> &'static str {
        match self {
            Self::Baseline256 => "baseline_256_bit_joint",
            Self::Canonical253 => "canonical_253_bit_joint",
        }
    }

    fn scalar_bits(self) -> usize {
        match self {
            Self::Baseline256 => 256,
            Self::Canonical253 => 253,
        }
    }
}

#[derive(Clone)]
pub struct CpEquationCircuit {
    pub generator: EdwardsAffineValue,
    pub commitment: EdwardsAffineValue,
    pub verification_key: EdwardsAffineValue,
    pub challenge_le: [u8; 32],
    pub response_le: [u8; 32],
    pub scalar_mul_version: CpScalarMulVersion,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CpConstraintBreakdown {
    pub input_and_scalar_range: usize,
    pub curve_membership: usize,
    pub nonnative_double_scalar_mul: usize,
    pub ristretto_quotient_equality: usize,
    pub total: usize,
}

impl CpEquationCircuit {
    fn synthesize_with_breakdown(
        &self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<CpConstraintBreakdown, SynthesisError> {
        let mut breakdown = CpConstraintBreakdown::default();
        let start = cs.num_constraints();
        let commitment = ExtendedPointVar::new_public(cs.clone(), self.commitment)?;
        let verification_key = ExtendedPointVar::new_public(cs.clone(), self.verification_key)?;
        let challenge_bits =
            allocate_scalar_bits(cs.clone(), self.challenge_le, AllocationMode::Input)?;
        let response_bits =
            allocate_scalar_bits(cs.clone(), self.response_le, AllocationMode::Witness)?;
        breakdown.input_and_scalar_range = cs.num_constraints() - start;

        let phase = cs.num_constraints();
        enforce_affine_curve(&commitment)?;
        enforce_affine_curve(&verification_key)?;
        breakdown.curve_membership = cs.num_constraints() - phase;

        let phase = cs.num_constraints();
        let result = double_scalar_mul_g_minus_vk(
            &ExtendedPointVar::constant(self.generator),
            &verification_key,
            &response_bits,
            &challenge_bits,
            self.scalar_mul_version.scalar_bits(),
        )?;
        breakdown.nonnative_double_scalar_mul = cs.num_constraints() - phase;

        let phase = cs.num_constraints();
        enforce_ristretto_quotient_equal(&result, &commitment)?;
        breakdown.ristretto_quotient_equality = cs.num_constraints() - phase;
        breakdown.total = cs.num_constraints() - start;
        Ok(breakdown)
    }
}

impl ConstraintSynthesizer<Fr> for CpEquationCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        self.synthesize_with_breakdown(cs).map(|_| ())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpFeasibilityMetrics {
    pub classification: String,
    pub equation: String,
    pub implementation: String,
    pub constraints: usize,
    pub public_inputs: usize,
    pub breakdown: CpConstraintBreakdown,
    pub setup_ms: f64,
    pub prove_ms: f64,
    pub verify_ms: f64,
    pub proof_bytes: usize,
    pub proving_key_bytes: usize,
    pub verifying_key_bytes: usize,
    pub valid_proof_verified: bool,
    pub changed_public_challenge_rejected: bool,
    pub altered_response_relation_unsatisfied: bool,
    pub compressed_encoding_bound_in_circuit: bool,
    pub sha512_recomputed_in_circuit: bool,
    pub full_p3: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpPreflightMetrics {
    pub classification: String,
    pub equation: String,
    pub implementation: String,
    pub constraints: usize,
    pub public_inputs: usize,
    pub breakdown: CpConstraintBreakdown,
    pub valid_relation_satisfied: bool,
    pub synthesis_witness_ms: f64,
    pub groth16_setup_run: bool,
    pub groth16_prove_run: bool,
    pub groth16_verify_run: bool,
}

pub fn preflight_cp_equation(
    circuit: CpEquationCircuit,
) -> Result<CpPreflightMetrics, CpGadgetError> {
    let cs = ConstraintSystem::<Fr>::new_ref();
    let implementation = circuit.scalar_mul_version.label().to_owned();
    let started = Instant::now();
    let breakdown = circuit.synthesize_with_breakdown(cs.clone())?;
    let valid_relation_satisfied = cs.is_satisfied()?;
    let synthesis_witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(CpPreflightMetrics {
        classification: "real-r1cs-single-cp-equation-preflight-no-groth16-proof".to_owned(),
        equation: "z*G = A0 + d*VK (Ristretto quotient equality)".to_owned(),
        implementation,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        breakdown,
        valid_relation_satisfied,
        synthesis_witness_ms,
        groth16_setup_run: false,
        groth16_prove_run: false,
        groth16_verify_run: false,
    })
}

pub fn run_cp_equation_groth16(
    circuit: CpEquationCircuit,
    seed: u64,
) -> Result<CpFeasibilityMetrics, CpGadgetError> {
    let cs = ConstraintSystem::<Fr>::new_ref();
    let implementation = circuit.scalar_mul_version.label().to_owned();
    let breakdown = circuit.synthesize_with_breakdown(cs.clone())?;
    if !cs.is_satisfied()? {
        return Err(CpGadgetError::Synthesis(SynthesisError::Unsatisfiable));
    }
    let constraints = cs.num_constraints();
    let public_inputs = cs.num_instance_variables().saturating_sub(1);
    let public_assignment = {
        let borrowed = cs.borrow().ok_or(SynthesisError::AssignmentMissing)?;
        borrowed.instance_assignment[1..].to_vec()
    };

    let mut setup_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x0043_505F_4730_35);
    let setup_started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut setup_rng,
    )?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;

    let mut proof_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x0043_505F_5052_4F);
    let prove_started = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit.clone(),
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
    *last = if last.is_zero() {
        Fr::one()
    } else {
        Fr::zero()
    };
    let changed_public_challenge_rejected =
        !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed_public)?;

    let mut altered = circuit;
    altered.response_le[0] ^= 1;
    let bad_cs = ConstraintSystem::<Fr>::new_ref();
    altered.generate_constraints(bad_cs.clone())?;
    let altered_response_relation_unsatisfied = !bad_cs.is_satisfied()?;

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;
    let mut pk_encoding = Vec::new();
    parameters.serialize_compressed(&mut pk_encoding)?;
    let mut vk_encoding = Vec::new();
    parameters.vk.serialize_compressed(&mut vk_encoding)?;

    Ok(CpFeasibilityMetrics {
        classification: "real-groth16-single-cp-equation-feasibility-not-p3".to_owned(),
        equation: "z*G = A0 + d*VK (Ristretto quotient equality)".to_owned(),
        implementation,
        constraints,
        public_inputs,
        breakdown,
        setup_ms,
        prove_ms,
        verify_ms,
        proof_bytes: proof_encoding.len(),
        proving_key_bytes: pk_encoding.len(),
        verifying_key_bytes: vk_encoding.len(),
        valid_proof_verified,
        changed_public_challenge_rejected,
        altered_response_relation_unsatisfied,
        compressed_encoding_bound_in_circuit: false,
        sha512_recomputed_in_circuit: false,
        full_p3: false,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpPrimitiveCosts {
    pub nonnative_field_mul_with_output_check: usize,
    pub extended_point_add_with_output_check: usize,
    pub ristretto_quotient_equality: usize,
}

pub fn profile_cp_primitives(
    left: EdwardsAffineValue,
    right: EdwardsAffineValue,
) -> Result<CpPrimitiveCosts, CpGadgetError> {
    let field_cs = ConstraintSystem::<Fr>::new_ref();
    let left_field = FqVar::new_witness(field_cs.clone(), || Ok(left.x))?;
    let right_field = FqVar::new_witness(field_cs.clone(), || Ok(right.x))?;
    let field_start = field_cs.num_constraints();
    let field_product = &left_field * &right_field;
    field_product.enforce_equal(&FqVar::constant(left.x * right.x))?;
    let nonnative_field_mul_with_output_check = field_cs.num_constraints() - field_start;
    if !field_cs.is_satisfied()? {
        return Err(CpGadgetError::Synthesis(SynthesisError::Unsatisfiable));
    }

    let add_cs = ConstraintSystem::<Fr>::new_ref();
    let left_var = ExtendedPointVar::new_witness(add_cs.clone(), left)?;
    let right_var = ExtendedPointVar::new_witness(add_cs.clone(), right)?;
    let add_start = add_cs.num_constraints();
    let sum = add_points(&left_var, &right_var);
    let expected = add_value(left, right);
    sum.x.enforce_equal(&FqVar::constant(expected[0]))?;
    sum.y.enforce_equal(&FqVar::constant(expected[1]))?;
    sum.z.enforce_equal(&FqVar::constant(expected[2]))?;
    sum.t.enforce_equal(&FqVar::constant(expected[3]))?;
    let extended_point_add_with_output_check = add_cs.num_constraints() - add_start;
    if !add_cs.is_satisfied()? {
        return Err(CpGadgetError::Synthesis(SynthesisError::Unsatisfiable));
    }

    let equality_cs = ConstraintSystem::<Fr>::new_ref();
    let equality_left = ExtendedPointVar::new_witness(equality_cs.clone(), left)?;
    let equality_right = ExtendedPointVar::new_witness(equality_cs.clone(), left)?;
    let equality_start = equality_cs.num_constraints();
    enforce_ristretto_quotient_equal(&equality_left, &equality_right)?;
    let ristretto_quotient_equality = equality_cs.num_constraints() - equality_start;
    if !equality_cs.is_satisfied()? {
        return Err(CpGadgetError::Synthesis(SynthesisError::Unsatisfiable));
    }

    Ok(CpPrimitiveCosts {
        nonnative_field_mul_with_output_check,
        extended_point_add_with_output_check,
        ristretto_quotient_equality,
    })
}

fn add_value(left: EdwardsAffineValue, right: EdwardsAffineValue) -> [Curve25519Base; 4] {
    let left_t = left.x * left.y;
    let right_t = right.x * right.y;
    let a = (left.y - left.x) * (right.y - right.x);
    let b = (left.y + left.x) * (right.y + right.x);
    let c = left_t * right_t * (edwards_d() + edwards_d());
    let d = Curve25519Base::from(2u64);
    let e = b - a;
    let f = d - c;
    let g = d + c;
    let h = b + a;
    [e * f, g * h, f * g, e * h]
}

pub fn decompress_ristretto(compressed_le: [u8; 32]) -> Result<EdwardsAffineValue, CpGadgetError> {
    let s = Curve25519Base::from_le_bytes_mod_order(&compressed_le);
    if field_bytes_le(s) != compressed_le {
        return Err(CpGadgetError::InvalidEncoding(
            "non-canonical field element",
        ));
    }
    if is_negative(s) {
        return Err(CpGadgetError::InvalidEncoding("negative s encoding"));
    }
    let one = Curve25519Base::one();
    let ss = s.square();
    let u1 = one - ss;
    let u2 = one + ss;
    let u2_square = u2.square();
    let v = (-edwards_d()) * u1.square() - u2_square;
    let inverse = (v * u2_square)
        .inverse()
        .ok_or(CpGadgetError::InvalidEncoding(
            "zero inverse-square-root input",
        ))?;
    let mut inverse_sqrt = inverse.sqrt().ok_or(CpGadgetError::InvalidEncoding(
        "nonsquare inverse-square-root input",
    ))?;
    let mut dx = inverse_sqrt * u2;
    let mut x = (s + s) * dx;
    if is_negative(x) {
        inverse_sqrt = -inverse_sqrt;
        dx = inverse_sqrt * u2;
        x = (s + s) * dx;
    }
    let dy = inverse_sqrt * dx * v;
    let y = u1 * dy;
    let t = x * y;
    if is_negative(t) || y.is_zero() {
        return Err(CpGadgetError::InvalidEncoding(
            "invalid Ristretto representative",
        ));
    }
    let value = EdwardsAffineValue { x, y };
    if !is_on_curve(value) {
        return Err(CpGadgetError::InvalidEncoding(
            "representative is off curve",
        ));
    }
    Ok(value)
}

fn is_on_curve(point: EdwardsAffineValue) -> bool {
    let x2 = point.x.square();
    let y2 = point.y.square();
    -x2 + y2 == Curve25519Base::one() + edwards_d() * x2 * y2
}

fn is_negative(value: Curve25519Base) -> bool {
    field_bytes_le(value)[0] & 1 == 1
}

fn field_bytes_le(value: Curve25519Base) -> [u8; 32] {
    let bytes = value.into_bigint().to_bytes_le();
    let mut output = [0u8; 32];
    output[..bytes.len()].copy_from_slice(&bytes);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use curve25519_dalek::{constants::RISTRETTO_BASEPOINT_POINT, scalar::Scalar};

    #[test]
    fn dalek_points_decompress_to_valid_edwards_representatives() {
        for point in [
            RISTRETTO_BASEPOINT_POINT,
            Scalar::from(23u64) * RISTRETTO_BASEPOINT_POINT,
            Scalar::from(91u64) * RISTRETTO_BASEPOINT_POINT,
        ] {
            let value = decompress_ristretto(point.compress().to_bytes()).unwrap();
            assert!(is_on_curve(value));
        }
    }

    #[test]
    fn real_dalek_equation_satisfies_non_native_r1cs_and_mutation_fails() {
        let secret = Scalar::from(23u64);
        let nonce = Scalar::from(91u64);
        let challenge = Scalar::from(17u64);
        let response = nonce + challenge * secret;
        let circuit = CpEquationCircuit {
            generator: decompress_ristretto(RISTRETTO_BASEPOINT_POINT.compress().to_bytes())
                .unwrap(),
            commitment: decompress_ristretto(
                (nonce * RISTRETTO_BASEPOINT_POINT).compress().to_bytes(),
            )
            .unwrap(),
            verification_key: decompress_ristretto(
                (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes(),
            )
            .unwrap(),
            challenge_le: challenge.to_bytes(),
            response_le: response.to_bytes(),
            scalar_mul_version: CpScalarMulVersion::Canonical253,
        };
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.clone().generate_constraints(cs.clone()).unwrap();
        println!(
            "G0.5 single-equation constraints={} public_inputs={}",
            cs.num_constraints(),
            cs.num_instance_variables().saturating_sub(1)
        );
        assert!(cs.is_satisfied().unwrap());

        let mut bad = circuit;
        bad.response_le[0] ^= 1;
        let bad_cs = ConstraintSystem::<Fr>::new_ref();
        bad.generate_constraints(bad_cs.clone()).unwrap();
        assert!(!bad_cs.is_satisfied().unwrap());
    }
}
