//! G0.9-P3-B1: bind three selected private CP packages to their public
//! Pedersen commitments.
//!
//! This relation extends P3-A only with fixed-shape Package-H2F-v1 and the
//! three commitment-opening equations. Threshold interpolation, tally and
//! winner logic deliberately remain outside this gate.

use crate::{
    cp_h3_gadget::{
        bind_canonical_compressed_point, fixed_base_mul, H3_TRANSCRIPT_BYTES, H3_TRUSTEE_ID_OFFSET,
    },
    cp_h4_gadget::{H4ACircuit, H4_DECRYPTION_SHARE_OFFSET},
    derive_secondary_generator, encode_point,
    hash_to_field_gadget::package_hash_to_field_var,
    p3a_gadget::{P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE, P3A_TRUSTEE_COUNT},
    Point, Scalar, SUITE_ID,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::nonnative::NonNativeFieldVar,
    groups::CurveVar, prelude::ToBitsGadget, uint8::UInt8,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};
use vv_types::CanonicalEncoder;

pub const P3B1_PACKAGE_BYTES_PER_TRUSTEE: usize = 602;
const CP_CONTEXT_START: usize = 112;
const CP_VERIFICATION_SHARE_END: usize = 340;
const CP_SHARE_COUNT_START: usize = H4_DECRYPTION_SHARE_OFFSET - 8;

#[derive(Clone)]
pub struct P3B1TrusteeCircuit {
    pub cp: H4ACircuit,
    pub package_commitment: Point,
    pub opening: Scalar,
}

#[derive(Clone)]
pub struct P3B1Circuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
}

#[derive(Clone, Debug)]
pub struct P3B1CircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub trustees: usize,
    pub cp_equations: usize,
    pub package_h2f_relations: usize,
    pub package_opening_equations: usize,
    pub package_bytes_per_trustee: usize,
    pub package_h2f_input_bytes: usize,
    pub package_sha512_blocks: usize,
}

pub(crate) struct P3B1RelationOutput {
    pub package_h2f_input_bytes: usize,
    pub package_sha512_blocks: usize,
    pub decryption_share_vars: Vec<Vec<EdwardsVar>>,
    pub trustee_id_bytes: Vec<Vec<UInt8<Fq>>>,
}

fn package_prefix() -> Vec<u8> {
    let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/PRIVATE-PACKAGE/v5");
    encoded.text(SUITE_ID);
    encoded.finish()
}

fn response_bytes(response_bits: &[Boolean<Fq>]) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
    if response_bits.len() > 256 {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut padded = response_bits.to_vec();
    padded.resize(256, Boolean::FALSE);
    Ok(padded.chunks(8).map(UInt8::from_bits_le).collect())
}

fn package_bytes_var(
    transcript: &[UInt8<Fq>],
    response_bits: &[Boolean<Fq>],
) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
    if transcript.len() != H3_TRANSCRIPT_BYTES {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut package = UInt8::constant_vec(&package_prefix());
    package.extend_from_slice(&transcript[CP_CONTEXT_START..CP_VERIFICATION_SHARE_END]);
    package.extend_from_slice(&transcript[CP_SHARE_COUNT_START..H3_TRANSCRIPT_BYTES]);
    package.extend(response_bytes(response_bits)?);
    if package.len() != P3B1_PACKAGE_BYTES_PER_TRUSTEE {
        return Err(SynthesisError::Unsatisfiable);
    }
    Ok(package)
}

impl P3B1Circuit {
    pub fn has_expected_shape(&self) -> bool {
        self.trustees.len() == P3A_TRUSTEE_COUNT
            && self
                .trustees
                .iter()
                .all(|trustee| trustee.cp.has_expected_shape())
    }

    pub(crate) fn generate_relation_with_shares(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3B1RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let secondary_generator =
            derive_secondary_generator(b"PEDERSEN-H").map_err(|_| SynthesisError::Unsatisfiable)?;
        let mut package_h2f_input_bytes = 0;
        let mut package_sha512_blocks = 0;
        let mut all_decryption_share_vars = Vec::with_capacity(P3A_TRUSTEE_COUNT);
        let mut all_trustee_id_bytes = Vec::with_capacity(P3A_TRUSTEE_COUNT);

        for trustee in &self.trustees {
            let mut transcript_var = UInt8::new_input_vec(
                cs.clone(),
                &trustee.cp.transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE],
            )?;
            transcript_var.extend(UInt8::new_witness_vec(
                cs.clone(),
                &trustee.cp.transcript[P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE..],
            )?);
            all_trustee_id_bytes
                .push(transcript_var[H3_TRUSTEE_ID_OFFSET..H3_TRUSTEE_ID_OFFSET + 4].to_vec());
            let private_values = trustee
                .cp
                .generate_relation_with_private_values(cs.clone(), &transcript_var)?;
            all_decryption_share_vars.push(private_values.decryption_share_vars);

            let package_var = package_bytes_var(&transcript_var, &private_values.response_bits)?;
            let (package_scalar, h2f_input_bytes, sha512_blocks) =
                package_hash_to_field_var(&package_var)?;
            package_h2f_input_bytes = h2f_input_bytes;
            package_sha512_blocks = sha512_blocks;

            let opening_var =
                NonNativeFieldVar::<Scalar, Fq>::new_witness(cs.clone(), || Ok(trustee.opening))?;
            let message_g = fixed_base_mul(&package_scalar.to_bits_le()?)?;
            let secondary_var = EdwardsVar::new_constant(cs.clone(), secondary_generator)?;
            let opening_h = secondary_var.scalar_mul_le(opening_var.to_bits_le()?.iter())?;

            let commitment_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(trustee.package_commitment))?;
            let commitment_bytes =
                UInt8::new_input_vec(cs.clone(), &encode_point(&trustee.package_commitment))?;
            bind_canonical_compressed_point(&commitment_var, &commitment_bytes)?;
            commitment_var.enforce_equal(&(message_g + opening_h))?;
        }
        Ok(P3B1RelationOutput {
            package_h2f_input_bytes,
            package_sha512_blocks,
            decryption_share_vars: all_decryption_share_vars,
            trustee_id_bytes: all_trustee_id_bytes,
        })
    }

    fn generate_relation(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<(usize, usize), SynthesisError> {
        let output = self.generate_relation_with_shares(cs)?;
        Ok((output.package_h2f_input_bytes, output.package_sha512_blocks))
    }
}

impl ConstraintSynthesizer<Fq> for P3B1Circuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs).map(|_| ())
    }
}

pub fn check_three_package_openings_p3b1(
    trustees: &[P3B1TrusteeCircuit],
) -> Result<P3B1CircuitReport, SynthesisError> {
    let circuit = P3B1Circuit {
        trustees: trustees.to_vec(),
    };
    if !circuit.has_expected_shape() {
        return Ok(P3B1CircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            trustees: trustees.len(),
            cp_equations: 0,
            package_h2f_relations: 0,
            package_opening_equations: 0,
            package_bytes_per_trustee: P3B1_PACKAGE_BYTES_PER_TRUSTEE,
            package_h2f_input_bytes: 0,
            package_sha512_blocks: 0,
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    let (package_h2f_input_bytes, package_sha512_blocks) = circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3B1CircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        trustees: P3A_TRUSTEE_COUNT,
        cp_equations: P3A_TRUSTEE_COUNT * 4,
        package_h2f_relations: P3A_TRUSTEE_COUNT,
        package_opening_equations: P3A_TRUSTEE_COUNT,
        package_bytes_per_trustee: P3B1_PACKAGE_BYTES_PER_TRUSTEE,
        package_h2f_input_bytes,
        package_sha512_blocks,
    })
}
