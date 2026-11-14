//! G0.9-H3: the adopted CP-H2F-v3 challenge joined to the first generalized
//! Chaum--Pedersen group equation.
//!
//! This is deliberately one Trustee and one equation. It is not the vector
//! equation, Groth16, or the complete winner-only disclosure circuit.

use crate::{hash_to_field_gadget::cp_hash_to_field_var, Point, Scalar};
use ark_ec::Group;
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{BigInteger, PrimeField};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::nonnative::NonNativeFieldVar,
    groups::CurveVar, prelude::ToBitsGadget, uint8::UInt8,
};
use ark_relations::r1cs::{ConstraintSystem, SynthesisError};

pub const H3_TRANSCRIPT_BYTES: usize = 684;
pub const H3_TRUSTEE_ID_OFFSET: usize = 304;
pub const H3_VERIFICATION_SHARE_OFFSET: usize = 308;
pub const H3_A0_OFFSET: usize = 548;
pub(crate) const COMPRESSED_POINT_BYTES: usize = 32;

#[derive(Clone, Debug)]
pub struct H3CircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub transcript_bytes: usize,
    pub h2f_input_bytes: usize,
    pub sha512_blocks: usize,
}

fn less_or_equal_constant(
    bits_le: &[Boolean<Fq>],
    constant_bits_le: &[bool],
) -> Result<Boolean<Fq>, SynthesisError> {
    if bits_le.len() != constant_bits_le.len() {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut equal = Boolean::TRUE;
    let mut less = Boolean::FALSE;
    for (bit, constant_bit) in bits_le.iter().zip(constant_bits_le).rev() {
        if *constant_bit {
            less = less.or(&equal.and(&bit.not())?)?;
            equal = equal.and(bit)?;
        } else {
            equal = equal.and(&bit.not())?;
        }
    }
    less.or(&equal)
}

pub(crate) fn bind_canonical_compressed_point(
    point: &EdwardsVar,
    encoded: &[UInt8<Fq>],
) -> Result<(), SynthesisError> {
    if encoded.len() != COMPRESSED_POINT_BYTES {
        return Err(SynthesisError::Unsatisfiable);
    }
    let encoded_bits = encoded
        .iter()
        .map(ToBitsGadget::to_bits_le)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let y_bits = point.y.to_bits_le()?;
    if y_bits.len() + 1 != encoded_bits.len() {
        return Err(SynthesisError::Unsatisfiable);
    }
    for (actual, encoded) in y_bits.iter().zip(&encoded_bits[..y_bits.len()]) {
        actual.enforce_equal(encoded)?;
    }

    // arkworks TE compression stores y in little-endian form and sets the
    // top bit iff x > -x, i.e. iff canonical x > (p-1)/2.
    let x_bits = point.x.to_bits_le()?;
    let mut half_bits = Fq::MODULUS_MINUS_ONE_DIV_TWO.to_bits_le();
    half_bits.resize(x_bits.len(), false);
    let x_is_positive = less_or_equal_constant(&x_bits, &half_bits)?;
    x_is_positive
        .not()
        .enforce_equal(&encoded_bits[encoded_bits.len() - 1])
}

pub(crate) fn fixed_base_mul(bits: &[Boolean<Fq>]) -> Result<EdwardsVar, SynthesisError> {
    let mut powers = Vec::with_capacity(bits.len());
    let mut current = Point::generator();
    for _ in bits {
        powers.push(current);
        current.double_in_place();
    }
    let mut result = EdwardsVar::zero();
    result.precomputed_base_scalar_mul_le(bits.iter().zip(&powers))?;
    Ok(result)
}

pub fn check_cp_first_equation_h3(
    transcript: &[u8],
    verification_share: Point,
    a0: Point,
    response: Scalar,
) -> Result<H3CircuitReport, SynthesisError> {
    if transcript.len() != H3_TRANSCRIPT_BYTES {
        return Ok(H3CircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            transcript_bytes: transcript.len(),
            h2f_input_bytes: 0,
            sha512_blocks: 0,
        });
    }

    let cs = ConstraintSystem::<Fq>::new_ref();
    let transcript_var = UInt8::new_input_vec(cs.clone(), transcript)?;
    let verification_share_var = EdwardsVar::new_witness(cs.clone(), || Ok(verification_share))?;
    let a0_var = EdwardsVar::new_witness(cs.clone(), || Ok(a0))?;
    verification_share_var
        .is_zero()?
        .enforce_equal(&Boolean::FALSE)?;
    a0_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;

    bind_canonical_compressed_point(
        &verification_share_var,
        &transcript_var
            [H3_VERIFICATION_SHARE_OFFSET..H3_VERIFICATION_SHARE_OFFSET + COMPRESSED_POINT_BYTES],
    )?;
    bind_canonical_compressed_point(
        &a0_var,
        &transcript_var[H3_A0_OFFSET..H3_A0_OFFSET + COMPRESSED_POINT_BYTES],
    )?;

    let (challenge_var, h2f_input_bytes, sha512_blocks) =
        cp_hash_to_field_var(b"GENERALIZED-CP", &transcript_var)?;
    let response_var = NonNativeFieldVar::<Scalar, Fq>::new_witness(cs.clone(), || Ok(response))?;
    let response_g = fixed_base_mul(&response_var.to_bits_le()?)?;
    let challenge_vk = verification_share_var.scalar_mul_le(challenge_var.to_bits_le()?.iter())?;
    response_g.enforce_equal(&(a0_var + challenge_vk))?;

    let satisfied = cs.is_satisfied()?;
    Ok(H3CircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        transcript_bytes: transcript.len(),
        h2f_input_bytes,
        sha512_blocks,
    })
}
