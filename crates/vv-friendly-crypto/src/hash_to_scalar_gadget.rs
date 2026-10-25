//! Isolated G0.9-H2-A R1CS for the exact Jubjub H2S-v2 mapping.
//!
//! The relation hashes every attempt from counter zero through the claimed
//! accepted counter, requires earlier candidates to be non-canonical, and
//! requires the accepted 252-bit candidate to be strictly below `Fr`.
//! Circuit size currently depends on the accepted counter; this module does
//! not claim one uniform 128-attempt production circuit.

use crate::{
    transcript_gadget::sha512_digest_var, HASH_REJECTION_MAX_ATTEMPTS, SCALAR_BYTES, SUITE_ID,
};
use ark_ed_on_bls12_381::{Fq, Fr as JubScalar};
use ark_ff::{BigInteger, PrimeField};
use ark_r1cs_std::{
    boolean::Boolean, eq::EqGadget, prelude::ToBitsGadget, select::CondSelectGadget, uint8::UInt8,
};
use ark_relations::r1cs::{ConstraintSystem, SynthesisError};

const CODEC_MAGIC: &[u8] = b"VV-CANONICAL-V1";
const H2S_DOMAIN: &[u8] = b"VV/JUBJUB/H2S/v2";
pub const H2B_FIXED_ATTEMPTS: u16 = 2;

#[derive(Clone, Debug)]
pub struct HashToScalarCircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub transcript_bytes: usize,
    pub h2s_input_bytes: usize,
    pub padded_blocks_per_attempt: usize,
    pub attempts_hashed: usize,
    pub accepted_counter: u16,
}

fn append_constant_bytes(output: &mut Vec<UInt8<Fq>>, value: &[u8]) {
    output.extend(UInt8::constant_vec(value));
}

fn append_constant_length_prefixed(output: &mut Vec<UInt8<Fq>>, value: &[u8]) {
    append_constant_bytes(output, &(value.len() as u64).to_be_bytes());
    append_constant_bytes(output, value);
}

fn h2s_input_var(
    operation_domain: &[u8],
    transcript: &[UInt8<Fq>],
    counter: u16,
) -> Vec<UInt8<Fq>> {
    let mut input = Vec::new();
    append_constant_length_prefixed(&mut input, CODEC_MAGIC);
    append_constant_length_prefixed(&mut input, H2S_DOMAIN);
    append_constant_length_prefixed(&mut input, SUITE_ID.as_bytes());
    append_constant_length_prefixed(&mut input, operation_domain);
    append_constant_bytes(&mut input, &(transcript.len() as u64).to_be_bytes());
    input.extend_from_slice(transcript);
    append_constant_bytes(&mut input, &counter.to_be_bytes());
    input
}

fn masked_candidate(digest: &[UInt8<Fq>]) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
    debug_assert_eq!(digest.len(), 64);
    let mut candidate = digest[..SCALAR_BYTES].to_vec();
    let last_bits = candidate[SCALAR_BYTES - 1].to_bits_le()?;
    let mut masked_last = last_bits[..4].to_vec();
    masked_last.extend(std::iter::repeat_n(Boolean::FALSE, 4));
    candidate[SCALAR_BYTES - 1] = UInt8::from_bits_le(&masked_last);
    Ok(candidate)
}

fn less_than_scalar_modulus(candidate: &[UInt8<Fq>]) -> Result<Boolean<Fq>, SynthesisError> {
    debug_assert_eq!(candidate.len(), SCALAR_BYTES);
    let mut candidate_bits = Vec::with_capacity(SCALAR_BYTES * 8);
    for byte in candidate {
        candidate_bits.extend(byte.to_bits_le()?);
    }
    let modulus_bits = JubScalar::MODULUS.to_bits_le();
    debug_assert_eq!(candidate_bits.len(), modulus_bits.len());

    let mut equal_prefix = Boolean::TRUE;
    let mut less = Boolean::FALSE;
    for (candidate_bit, modulus_bit) in candidate_bits.iter().zip(&modulus_bits).rev() {
        if *modulus_bit {
            let first_lower_bit = equal_prefix.and(&candidate_bit.not())?;
            less = less.or(&first_lower_bit)?;
            equal_prefix = equal_prefix.and(candidate_bit)?;
        } else {
            equal_prefix = equal_prefix.and(&candidate_bit.not())?;
        }
    }
    Ok(less)
}

pub fn check_hash_to_scalar_first_valid(
    operation_domain: &[u8],
    transcript: &[u8],
    expected_scalar_le: &[u8],
    expected_counter: u16,
) -> Result<HashToScalarCircuitReport, SynthesisError> {
    if expected_scalar_le.len() != SCALAR_BYTES || expected_counter >= HASH_REJECTION_MAX_ATTEMPTS {
        return Ok(HashToScalarCircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            transcript_bytes: transcript.len(),
            h2s_input_bytes: 0,
            padded_blocks_per_attempt: 0,
            attempts_hashed: 0,
            accepted_counter: expected_counter,
        });
    }

    let cs = ConstraintSystem::<Fq>::new_ref();
    let transcript_var = UInt8::new_witness_vec(cs.clone(), transcript)?;
    let mut public_output = expected_counter.to_be_bytes().to_vec();
    public_output.extend_from_slice(expected_scalar_le);
    let public_output_var = UInt8::new_input_vec(cs.clone(), &public_output)?;
    public_output_var[0].enforce_equal(&UInt8::constant(expected_counter.to_be_bytes()[0]))?;
    public_output_var[1].enforce_equal(&UInt8::constant(expected_counter.to_be_bytes()[1]))?;

    let mut h2s_input_bytes = 0;
    let mut accepted_candidate = None;
    for counter in 0..=expected_counter {
        let input = h2s_input_var(operation_domain, &transcript_var, counter);
        h2s_input_bytes = input.len();
        let digest = sha512_digest_var(&input)?;
        let candidate = masked_candidate(&digest)?;
        let is_canonical = less_than_scalar_modulus(&candidate)?;
        if counter < expected_counter {
            is_canonical.enforce_equal(&Boolean::FALSE)?;
        } else {
            is_canonical.enforce_equal(&Boolean::TRUE)?;
            accepted_candidate = Some(candidate);
        }
    }

    for (candidate, expected) in accepted_candidate
        .expect("the accepted attempt is always synthesized")
        .iter()
        .zip(&public_output_var[2..])
    {
        candidate.enforce_equal(expected)?;
    }

    let satisfied = cs.is_satisfied()?;
    Ok(HashToScalarCircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        transcript_bytes: transcript.len(),
        h2s_input_bytes,
        padded_blocks_per_attempt: (h2s_input_bytes + 1 + 16).div_ceil(128),
        attempts_hashed: expected_counter as usize + 1,
        accepted_counter: expected_counter,
    })
}

/// Check one fixed-shape two-attempt H2S-v2 relation.
///
/// Both SHA-512 attempts are always synthesized. The circuit derives whether
/// counter 0 or 1 is first valid and constrains the public counter and scalar
/// to that result. This is an H2-B feasibility circuit, not the Suite v2
/// 128-attempt production relation.
pub fn check_hash_to_scalar_fixed_two(
    operation_domain: &[u8],
    transcript: &[u8],
    expected_scalar_le: &[u8],
    expected_counter: u16,
) -> Result<HashToScalarCircuitReport, SynthesisError> {
    if expected_scalar_le.len() != SCALAR_BYTES || expected_counter >= H2B_FIXED_ATTEMPTS {
        return Ok(HashToScalarCircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            transcript_bytes: transcript.len(),
            h2s_input_bytes: 0,
            padded_blocks_per_attempt: 0,
            attempts_hashed: 0,
            accepted_counter: expected_counter,
        });
    }

    let cs = ConstraintSystem::<Fq>::new_ref();
    let transcript_var = UInt8::new_witness_vec(cs.clone(), transcript)?;
    let mut public_output = expected_counter.to_be_bytes().to_vec();
    public_output.extend_from_slice(expected_scalar_le);
    let public_output_var = UInt8::new_input_vec(cs.clone(), &public_output)?;

    let mut candidates = Vec::with_capacity(H2B_FIXED_ATTEMPTS as usize);
    let mut validities = Vec::with_capacity(H2B_FIXED_ATTEMPTS as usize);
    let mut h2s_input_bytes = 0;
    for counter in 0..H2B_FIXED_ATTEMPTS {
        let input = h2s_input_var(operation_domain, &transcript_var, counter);
        h2s_input_bytes = input.len();
        let digest = sha512_digest_var(&input)?;
        let candidate = masked_candidate(&digest)?;
        validities.push(less_than_scalar_modulus(&candidate)?);
        candidates.push(candidate);
    }

    let first_is_valid = &validities[0];
    first_is_valid
        .or(&validities[1])?
        .enforce_equal(&Boolean::TRUE)?;

    public_output_var[0].enforce_equal(&UInt8::constant(0))?;
    let selected_counter =
        UInt8::conditionally_select(first_is_valid, &UInt8::constant(0), &UInt8::constant(1))?;
    public_output_var[1].enforce_equal(&selected_counter)?;

    for ((first, second), expected) in candidates[0]
        .iter()
        .zip(&candidates[1])
        .zip(&public_output_var[2..])
    {
        UInt8::conditionally_select(first_is_valid, first, second)?.enforce_equal(expected)?;
    }

    let satisfied = cs.is_satisfied()?;
    Ok(HashToScalarCircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        transcript_bytes: transcript.len(),
        h2s_input_bytes,
        padded_blocks_per_attempt: (h2s_input_bytes + 1 + 16).div_ceil(128),
        attempts_hashed: H2B_FIXED_ATTEMPTS as usize,
        accepted_counter: expected_counter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_scalar, hash_to_scalar};

    fn vector_with_counter(target: u16) -> (Vec<u8>, [u8; SCALAR_BYTES]) {
        (0u16..2048)
            .find_map(|case| {
                let transcript = case.to_be_bytes().to_vec();
                let (scalar, counter) = hash_to_scalar(b"H2A-TEST", &transcript).ok()?;
                (counter == target).then_some((transcript, encode_scalar(&scalar)))
            })
            .expect("a deterministic H2-A test vector exists")
    }

    #[test]
    fn counter_zero_and_one_enforce_first_valid_candidate() {
        for counter in [0, 1] {
            let (transcript, scalar) = vector_with_counter(counter);
            assert!(
                check_hash_to_scalar_first_valid(b"H2A-TEST", &transcript, &scalar, counter)
                    .unwrap()
                    .satisfied
            );
        }

        let (transcript, scalar) = vector_with_counter(0);
        assert!(
            !check_hash_to_scalar_first_valid(b"H2A-TEST", &transcript, &scalar, 1)
                .unwrap()
                .satisfied
        );
    }

    #[test]
    fn fixed_two_attempt_shape_accepts_counter_zero_and_one() {
        let (transcript_zero, scalar_zero) = vector_with_counter(0);
        let report_zero =
            check_hash_to_scalar_fixed_two(b"H2A-TEST", &transcript_zero, &scalar_zero, 0).unwrap();
        assert!(report_zero.satisfied);

        let (transcript_one, scalar_one) = vector_with_counter(1);
        let report_one =
            check_hash_to_scalar_fixed_two(b"H2A-TEST", &transcript_one, &scalar_one, 1).unwrap();
        assert!(report_one.satisfied);
        assert_eq!(report_zero.constraints, report_one.constraints);
        assert_eq!(report_zero.public_inputs, report_one.public_inputs);
        assert_eq!(report_zero.witness_variables, report_one.witness_variables);

        assert!(
            !check_hash_to_scalar_fixed_two(b"H2A-TEST", &transcript_zero, &scalar_zero, 1)
                .unwrap()
                .satisfied
        );
    }
}
