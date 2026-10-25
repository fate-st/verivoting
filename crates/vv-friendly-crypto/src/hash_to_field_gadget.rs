//! Fixed-shape wide SHA-512-to-Jubjub-field circuits.
//!
//! H2-C measured the versioned candidate mapping. H2-D adopted the same
//! construction for generalized Chaum--Pedersen under `CP_H2F_SUITE_ID`.

use crate::{
    transcript_gadget::sha512_digest_var, CP_H2F_SUITE_ID, PACKAGE_H2F_SUITE_ID, SCALAR_BYTES,
    WIDE_H2F_CANDIDATE_SUITE_ID,
};
use ark_ed_on_bls12_381::{Fq, Fr as JubScalar};
use ark_r1cs_std::{
    alloc::AllocVar,
    eq::EqGadget,
    fields::{nonnative::NonNativeFieldVar, FieldVar},
    prelude::ToBitsGadget,
    uint8::UInt8,
    R1CSVar,
};
use ark_relations::r1cs::{ConstraintSystem, SynthesisError};

const CODEC_MAGIC: &[u8] = b"VV-CANONICAL-V1";
const H2F_DOMAIN: &[u8] = b"VV/JUBJUB/H2F/v3";

pub(crate) type JubScalarVar = NonNativeFieldVar<JubScalar, Fq>;

#[derive(Clone, Debug)]
pub struct WideHashToFieldCircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub transcript_bytes: usize,
    pub h2f_input_bytes: usize,
    pub padded_blocks: usize,
}

fn append_constant_bytes(output: &mut Vec<UInt8<Fq>>, value: &[u8]) {
    output.extend(UInt8::constant_vec(value));
}

fn append_constant_length_prefixed(output: &mut Vec<UInt8<Fq>>, value: &[u8]) {
    append_constant_bytes(output, &(value.len() as u64).to_be_bytes());
    append_constant_bytes(output, value);
}

fn h2f_input_var(
    suite_id: &str,
    operation_domain: &[u8],
    transcript: &[UInt8<Fq>],
) -> Vec<UInt8<Fq>> {
    let mut input = Vec::new();
    append_constant_length_prefixed(&mut input, CODEC_MAGIC);
    append_constant_length_prefixed(&mut input, H2F_DOMAIN);
    append_constant_length_prefixed(&mut input, suite_id.as_bytes());
    append_constant_length_prefixed(&mut input, operation_domain);
    append_constant_bytes(&mut input, &(transcript.len() as u64).to_be_bytes());
    input.extend_from_slice(transcript);
    input
}

fn reduce_digest_be_mod_jubjub(digest: &[UInt8<Fq>]) -> Result<JubScalarVar, SynthesisError> {
    debug_assert_eq!(digest.len(), 64);
    let cs = digest[0].cs();
    let radix = JubScalar::from(256u64);
    let mut accumulator = JubScalarVar::zero();

    for digest_byte in digest {
        let byte_var = JubScalarVar::new_witness(cs.clone(), || {
            digest_byte
                .value()
                .map(|value| JubScalar::from(value as u64))
        })?;
        let byte_var_bits = byte_var.to_bits_le()?;
        let digest_bits = digest_byte.to_bits_le()?;
        for (bound, source) in byte_var_bits[..8].iter().zip(&digest_bits) {
            bound.enforce_equal(source)?;
        }
        for high_bit in &byte_var_bits[8..] {
            high_bit.enforce_equal(&ark_r1cs_std::boolean::Boolean::FALSE)?;
        }
        accumulator = &accumulator * radix + &byte_var;
    }

    Ok(accumulator)
}

pub(crate) fn cp_hash_to_field_var(
    operation_domain: &[u8],
    transcript: &[UInt8<Fq>],
) -> Result<(JubScalarVar, usize, usize), SynthesisError> {
    let input = h2f_input_var(CP_H2F_SUITE_ID, operation_domain, transcript);
    let input_bytes = input.len();
    let digest = sha512_digest_var(&input)?;
    let reduced = reduce_digest_be_mod_jubjub(&digest)?;
    Ok((reduced, input_bytes, (input_bytes + 1 + 16).div_ceil(128)))
}

pub(crate) fn package_hash_to_field_var(
    transcript: &[UInt8<Fq>],
) -> Result<(JubScalarVar, usize, usize), SynthesisError> {
    let input = h2f_input_var(PACKAGE_H2F_SUITE_ID, b"PRIVATE-PACKAGE", transcript);
    let input_bytes = input.len();
    let digest = sha512_digest_var(&input)?;
    let reduced = reduce_digest_be_mod_jubjub(&digest)?;
    Ok((reduced, input_bytes, (input_bytes + 1 + 16).div_ceil(128)))
}

fn check_hash_to_field_wide_for_suite(
    suite_id: &str,
    operation_domain: &[u8],
    transcript: &[u8],
    expected_scalar_le: &[u8],
) -> Result<WideHashToFieldCircuitReport, SynthesisError> {
    if expected_scalar_le.len() != SCALAR_BYTES {
        return Ok(WideHashToFieldCircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            transcript_bytes: transcript.len(),
            h2f_input_bytes: 0,
            padded_blocks: 0,
        });
    }

    let cs = ConstraintSystem::<Fq>::new_ref();
    let transcript_var = UInt8::new_witness_vec(cs.clone(), transcript)?;
    let expected_scalar_var = UInt8::new_input_vec(cs.clone(), expected_scalar_le)?;
    let input = h2f_input_var(suite_id, operation_domain, &transcript_var);
    let h2f_input_bytes = input.len();
    let digest = sha512_digest_var(&input)?;
    let reduced = reduce_digest_be_mod_jubjub(&digest)?;

    let mut reduced_bits = reduced.to_bits_le()?;
    reduced_bits.resize(SCALAR_BYTES * 8, ark_r1cs_std::boolean::Boolean::FALSE);
    for (actual, expected) in reduced_bits
        .chunks(8)
        .map(UInt8::from_bits_le)
        .zip(&expected_scalar_var)
    {
        actual.enforce_equal(expected)?;
    }

    let satisfied = cs.is_satisfied()?;
    Ok(WideHashToFieldCircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        transcript_bytes: transcript.len(),
        h2f_input_bytes,
        padded_blocks: (h2f_input_bytes + 1 + 16).div_ceil(128),
    })
}

pub fn check_hash_to_field_wide_v3(
    operation_domain: &[u8],
    transcript: &[u8],
    expected_scalar_le: &[u8],
) -> Result<WideHashToFieldCircuitReport, SynthesisError> {
    check_hash_to_field_wide_for_suite(
        WIDE_H2F_CANDIDATE_SUITE_ID,
        operation_domain,
        transcript,
        expected_scalar_le,
    )
}

pub fn check_cp_hash_to_field_v3(
    operation_domain: &[u8],
    transcript: &[u8],
    expected_scalar_le: &[u8],
) -> Result<WideHashToFieldCircuitReport, SynthesisError> {
    check_hash_to_field_wide_for_suite(
        CP_H2F_SUITE_ID,
        operation_domain,
        transcript,
        expected_scalar_le,
    )
}

pub fn check_package_hash_to_field_v1(
    transcript: &[u8],
    expected_scalar_le: &[u8],
) -> Result<WideHashToFieldCircuitReport, SynthesisError> {
    check_hash_to_field_wide_for_suite(
        PACKAGE_H2F_SUITE_ID,
        b"PRIVATE-PACKAGE",
        transcript,
        expected_scalar_le,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_scalar, hash_to_field_wide_v3, package_hash_to_field_v1};

    #[test]
    fn wide_mapping_matches_native_and_has_fixed_shape() {
        let first = b"H2C-WIDE-FIXED-TRANSCRIPT-000";
        let second = b"H2C-WIDE-FIXED-TRANSCRIPT-001";
        let first_scalar = encode_scalar(&hash_to_field_wide_v3(b"H2C-TEST", first));
        let second_scalar = encode_scalar(&hash_to_field_wide_v3(b"H2C-TEST", second));

        let first_report = check_hash_to_field_wide_v3(b"H2C-TEST", first, &first_scalar).unwrap();
        let second_report =
            check_hash_to_field_wide_v3(b"H2C-TEST", second, &second_scalar).unwrap();
        assert!(first_report.satisfied);
        assert!(second_report.satisfied);
        assert_eq!(first_report.constraints, second_report.constraints);
        assert_eq!(first_report.public_inputs, second_report.public_inputs);
        assert_eq!(
            first_report.witness_variables,
            second_report.witness_variables
        );

        assert!(
            !check_hash_to_field_wide_v3(b"H2C-TEST", second, &first_scalar)
                .unwrap()
                .satisfied
        );
    }

    #[test]
    fn package_mapping_matches_native() {
        let package = b"canonical-private-package";
        let scalar = encode_scalar(&package_hash_to_field_v1(package));
        let report = check_package_hash_to_field_v1(package, &scalar).unwrap();
        assert!(report.satisfied);
        assert!(
            !check_package_hash_to_field_v1(b"changed-private-package", &scalar)
                .unwrap()
                .satisfied
        );
    }
}
