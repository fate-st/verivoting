//! Constrained pieces of the protocol's bounded SHA-512-to-Ristretto-scalar
//! conversion. The SHA-512 compression function itself is deliberately not
//! implemented in G0.5; callers must not describe this module as a complete
//! Fiat--Shamir gadget.

use ark_bls12_381::Fr;
use ark_r1cs_std::{
    alloc::{AllocVar, AllocationMode},
    boolean::Boolean,
    eq::EqGadget,
};
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};

pub const SCALAR_BITS: usize = 256;
pub const RISTRETTO_SCALAR_MODULUS_MINUS_ONE: [u64; 4] = [
    6_346_243_789_798_364_140,
    1_503_914_060_200_516_822,
    0,
    1_152_921_504_606_846_976,
];

pub fn allocate_scalar_bits(
    cs: ConstraintSystemRef<Fr>,
    canonical_le: [u8; 32],
    mode: AllocationMode,
) -> Result<Vec<Boolean<Fr>>, SynthesisError> {
    let bits = (0..SCALAR_BITS)
        .map(|index| {
            let value = ((canonical_le[index / 8] >> (index % 8)) & 1) == 1;
            Boolean::new_variable(cs.clone(), || Ok(value), mode)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Boolean::enforce_smaller_or_equal_than_le(&bits, RISTRETTO_SCALAR_MODULUS_MINUS_ONE)?;
    Ok(bits)
}

/// Enforce the post-SHA candidate branch used by `vv_crypto::hash_to_scalar`:
/// the high three bits are masked and the 253-bit candidate is canonical.
/// This does not constrain how SHA-512 produced the other bits.
pub fn enforce_accepted_rejection_candidate(bits: &[Boolean<Fr>]) -> Result<(), SynthesisError> {
    if bits.len() != SCALAR_BITS {
        return Err(SynthesisError::Unsatisfiable);
    }
    for bit in &bits[253..] {
        bit.enforce_equal(&Boolean::FALSE)?;
    }
    Boolean::enforce_smaller_or_equal_than_le(bits, RISTRETTO_SCALAR_MODULUS_MINUS_ONE)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    fn scalar_modulus_le() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        for (index, limb) in [
            6_346_243_789_798_364_141u64,
            1_503_914_060_200_516_822,
            0,
            1_152_921_504_606_846_976,
        ]
        .iter()
        .enumerate()
        {
            bytes[index * 8..(index + 1) * 8].copy_from_slice(&limb.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn canonical_scalar_range_is_constrained() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut valid = [0u8; 32];
        valid[0] = 7;
        allocate_scalar_bits(cs.clone(), valid, AllocationMode::Witness).unwrap();
        assert!(cs.is_satisfied().unwrap());

        let bad_cs = ConstraintSystem::<Fr>::new_ref();
        allocate_scalar_bits(bad_cs.clone(), scalar_modulus_le(), AllocationMode::Witness).unwrap();
        assert!(!bad_cs.is_satisfied().unwrap());
    }

    #[test]
    fn rejection_candidate_high_bits_are_constrained() {
        let cs = ConstraintSystem::<Fr>::new_ref();
        let mut candidate = [0u8; 32];
        candidate[31] = 0x20;
        let bits = (0..SCALAR_BITS)
            .map(|index| {
                Boolean::new_witness(cs.clone(), || {
                    Ok(((candidate[index / 8] >> (index % 8)) & 1) == 1)
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        enforce_accepted_rejection_candidate(&bits).unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }
}
