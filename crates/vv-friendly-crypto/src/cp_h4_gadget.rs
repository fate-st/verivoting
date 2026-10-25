//! G0.9-H4-A: complete one Trustee's generalized Chaum--Pedersen vector
//! relation under the single in-circuit CP-H2F-v3 challenge.
//!
//! This remains an R1CS feasibility gate. It does not generate Groth16 or
//! compose multiple Trustee packages into the winner-only disclosure proof.

use crate::{
    cp_h3_gadget::{
        bind_canonical_compressed_point, fixed_base_mul, COMPRESSED_POINT_BYTES, H3_A0_OFFSET,
        H3_TRANSCRIPT_BYTES, H3_VERIFICATION_SHARE_OFFSET,
    },
    hash_to_field_gadget::cp_hash_to_field_var,
    Point, Scalar,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::nonnative::NonNativeFieldVar,
    groups::CurveVar, prelude::ToBitsGadget, uint8::UInt8,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};

pub const H4_COORDINATE_COUNT: usize = 3;
pub const H4_CIPHERTEXT_C0_OFFSET: usize = 348;
pub const H4_DECRYPTION_SHARE_OFFSET: usize = 452;
pub const H4_PROOF_COORDINATE_OFFSET: usize = 588;

#[derive(Clone, Debug)]
pub struct H4ACircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub transcript_bytes: usize,
    pub h2f_input_bytes: usize,
    pub sha512_blocks: usize,
    pub cp_equations: usize,
    pub transcript_points_bound: usize,
}

#[derive(Clone)]
pub struct H4ACircuit {
    pub transcript: Vec<u8>,
    pub verification_share: Point,
    pub ciphertext_c0: Vec<Point>,
    pub decryption_shares: Vec<Point>,
    pub a0: Point,
    pub proof_coordinates: Vec<Point>,
    pub response: Scalar,
}

pub(crate) struct H4PrivateRelationOutput {
    pub h2f_input_bytes: usize,
    pub sha512_blocks: usize,
    pub response_bits: Vec<Boolean<Fq>>,
    pub decryption_share_vars: Vec<EdwardsVar>,
}

impl H4ACircuit {
    pub fn has_expected_shape(&self) -> bool {
        self.transcript.len() == H3_TRANSCRIPT_BYTES
            && self.ciphertext_c0.len() == H4_COORDINATE_COUNT
            && self.decryption_shares.len() == H4_COORDINATE_COUNT
            && self.proof_coordinates.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_transcript(
        &self,
        cs: ConstraintSystemRef<Fq>,
        transcript_var: &[UInt8<Fq>],
    ) -> Result<(usize, usize), SynthesisError> {
        let (h2f_input_bytes, sha512_blocks, _) =
            self.generate_relation_with_transcript_and_response(cs, transcript_var)?;
        Ok((h2f_input_bytes, sha512_blocks))
    }

    pub(crate) fn generate_relation_with_transcript_and_response(
        &self,
        cs: ConstraintSystemRef<Fq>,
        transcript_var: &[UInt8<Fq>],
    ) -> Result<(usize, usize, Vec<Boolean<Fq>>), SynthesisError> {
        let output = self.generate_relation_with_private_values(cs, transcript_var)?;
        Ok((
            output.h2f_input_bytes,
            output.sha512_blocks,
            output.response_bits,
        ))
    }

    pub(crate) fn generate_relation_with_private_values(
        &self,
        cs: ConstraintSystemRef<Fq>,
        transcript_var: &[UInt8<Fq>],
    ) -> Result<H4PrivateRelationOutput, SynthesisError> {
        if !self.has_expected_shape() || transcript_var.len() != H3_TRANSCRIPT_BYTES {
            return Err(SynthesisError::Unsatisfiable);
        }
        let verification_share_var =
            EdwardsVar::new_witness(cs.clone(), || Ok(self.verification_share))?;
        let a0_var = EdwardsVar::new_witness(cs.clone(), || Ok(self.a0))?;
        verification_share_var
            .is_zero()?
            .enforce_equal(&Boolean::FALSE)?;
        a0_var.is_zero()?.enforce_equal(&Boolean::FALSE)?;
        bind_canonical_compressed_point(
            &verification_share_var,
            &transcript_var[H3_VERIFICATION_SHARE_OFFSET
                ..H3_VERIFICATION_SHARE_OFFSET + COMPRESSED_POINT_BYTES],
        )?;
        bind_canonical_compressed_point(
            &a0_var,
            &transcript_var[H3_A0_OFFSET..H3_A0_OFFSET + COMPRESSED_POINT_BYTES],
        )?;

        let ciphertext_c0_vars = allocate_bound_vector(
            cs.clone(),
            transcript_var,
            &self.ciphertext_c0,
            H4_CIPHERTEXT_C0_OFFSET,
        )?;
        let decryption_share_vars = allocate_bound_vector(
            cs.clone(),
            transcript_var,
            &self.decryption_shares,
            H4_DECRYPTION_SHARE_OFFSET,
        )?;
        let proof_coordinate_vars = allocate_bound_vector(
            cs.clone(),
            transcript_var,
            &self.proof_coordinates,
            H4_PROOF_COORDINATE_OFFSET,
        )?;

        let (challenge_var, h2f_input_bytes, sha512_blocks) =
            cp_hash_to_field_var(b"GENERALIZED-CP", transcript_var)?;
        let response_var =
            NonNativeFieldVar::<Scalar, Fq>::new_witness(cs.clone(), || Ok(self.response))?;
        let response_bits = response_var.to_bits_le()?;
        let challenge_bits = challenge_var.to_bits_le()?;

        let response_g = fixed_base_mul(&response_bits)?;
        let challenge_vk = verification_share_var.scalar_mul_le(challenge_bits.iter())?;
        response_g.enforce_equal(&(a0_var + challenge_vk))?;

        for ((ciphertext, share), commitment) in ciphertext_c0_vars
            .iter()
            .zip(&decryption_share_vars)
            .zip(&proof_coordinate_vars)
        {
            let response_ciphertext = ciphertext.scalar_mul_le(response_bits.iter())?;
            let challenge_share = share.scalar_mul_le(challenge_bits.iter())?;
            response_ciphertext.enforce_equal(&(commitment.clone() + challenge_share))?;
        }
        Ok(H4PrivateRelationOutput {
            h2f_input_bytes,
            sha512_blocks,
            response_bits,
            decryption_share_vars,
        })
    }

    fn generate_relation(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<(usize, usize), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let transcript_var = UInt8::new_input_vec(cs.clone(), &self.transcript)?;
        self.generate_relation_with_transcript(cs, &transcript_var)
    }
}

impl ConstraintSynthesizer<Fq> for H4ACircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs).map(|_| ())
    }
}

fn empty_report(transcript_bytes: usize) -> H4ACircuitReport {
    H4ACircuitReport {
        satisfied: false,
        constraints: 0,
        public_inputs: 0,
        witness_variables: 0,
        transcript_bytes,
        h2f_input_bytes: 0,
        sha512_blocks: 0,
        cp_equations: 0,
        transcript_points_bound: 0,
    }
}

fn allocate_bound_vector(
    cs: ark_relations::r1cs::ConstraintSystemRef<Fq>,
    transcript: &[UInt8<Fq>],
    values: &[Point],
    start_offset: usize,
) -> Result<Vec<EdwardsVar>, SynthesisError> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let point = EdwardsVar::new_witness(cs.clone(), || Ok(*value))?;
            let offset = start_offset + index * COMPRESSED_POINT_BYTES;
            bind_canonical_compressed_point(
                &point,
                &transcript[offset..offset + COMPRESSED_POINT_BYTES],
            )?;
            Ok(point)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn check_complete_trustee_cp_h4a(
    transcript: &[u8],
    verification_share: Point,
    ciphertext_c0: &[Point],
    decryption_shares: &[Point],
    a0: Point,
    proof_coordinates: &[Point],
    response: Scalar,
) -> Result<H4ACircuitReport, SynthesisError> {
    let circuit = H4ACircuit {
        transcript: transcript.to_vec(),
        verification_share,
        ciphertext_c0: ciphertext_c0.to_vec(),
        decryption_shares: decryption_shares.to_vec(),
        a0,
        proof_coordinates: proof_coordinates.to_vec(),
        response,
    };
    if !circuit.has_expected_shape() {
        return Ok(empty_report(transcript.len()));
    }

    let cs = ConstraintSystem::<Fq>::new_ref();
    let (h2f_input_bytes, sha512_blocks) = circuit.generate_relation(cs.clone())?;

    let satisfied = cs.is_satisfied()?;
    Ok(H4ACircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        transcript_bytes: transcript.len(),
        h2f_input_bytes,
        sha512_blocks,
        cp_equations: 1 + H4_COORDINATE_COUNT,
        transcript_points_bound: 2 + 3 * H4_COORDINATE_COUNT,
    })
}
