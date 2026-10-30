//! G4-D: one complete tally-v2 P3 relation for all ten sorted 3-of-5 subsets.
//! Public selector derives BOTH CP transcript IDs and canonical scalar coefficients.
use crate::{
    cp_h3_gadget::bind_canonical_compressed_point,
    encode_point,
    p3b_gadget::{P3B1Circuit, P3B1TrusteeCircuit},
    p3c1_gadget::bind_tallies_to_plaintexts,
    p3d_gadget::{enforce_winner, p3d0_bound_subset_public_inputs},
    Point, Scalar,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{BigInteger, Field, One, PrimeField};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar, groups::CurveVar,
    uint8::UInt8,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

pub const G4D_RELATION_ID: &str = "VV/JUBJUB/P3/BOUNDED-TALLY-v2/TEN-SUBSETS/v1";
pub const G4D_LAYOUT: &str = "VV/JUBJUB/G4-D/INPUT-LAYOUT/v1";
pub const G4D_INPUTS: usize = 60;
pub const G4D_SELECTOR_INDEX: usize = 51;
pub const G4D_WINNER_INDEX: usize = 59;
pub const G4D_SUBSETS: [[u32; 3]; 10] = [
    [1, 2, 3],
    [1, 2, 4],
    [1, 2, 5],
    [1, 3, 4],
    [1, 3, 5],
    [1, 4, 5],
    [2, 3, 4],
    [2, 3, 5],
    [2, 4, 5],
    [3, 4, 5],
];

#[derive(Clone)]
pub struct P3TenSubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub public_subset_index: u64,
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
    pub public_winner: u64,
}

/// Computes only the fixed public lookup table, never a witness-dependent coefficient.
pub fn g4d_lagrange_coefficients(index: usize) -> Option<[Scalar; 3]> {
    let ids = G4D_SUBSETS.get(index)?;
    let mut out = [Scalar::one(); 3];
    for (i, id) in ids.iter().enumerate() {
        for other in ids.iter().filter(|other| *other != id) {
            let numerator = -Scalar::from(u64::from(*other));
            let denominator = Scalar::from(u64::from(*id)) - Scalar::from(u64::from(*other));
            out[i] *= numerator * denominator.inverse()?;
        }
    }
    Some(out)
}

fn lookup_bit(
    flags: &[Boolean<Fq>],
    values: impl Iterator<Item = bool>,
) -> Result<Boolean<Fq>, SynthesisError> {
    let mut out = Boolean::FALSE;
    for (flag, value) in flags.iter().zip(values) {
        // value belongs to a constant table built for ALL selectors. No witness branch.
        if value {
            out = out.or(flag)?;
        }
    }
    Ok(out)
}

fn selection(
    cs: ConstraintSystemRef<Fq>,
    index: u64,
    ids: &[Vec<UInt8<Fq>>],
) -> Result<Vec<Vec<Boolean<Fq>>>, SynthesisError> {
    if ids.len() != 3 || ids.iter().any(|x| x.len() != 4) {
        return Err(SynthesisError::Unsatisfiable);
    }
    let selector = FpVar::new_input(cs, || Ok(Fq::from(index)))?;
    let flags = (0..10)
        .map(|i| selector.is_eq(&FpVar::Constant(Fq::from(i as u64))))
        .collect::<Result<Vec<_>, _>>()?;
    let mut legal = Boolean::FALSE;
    for f in &flags {
        legal = legal.or(f)?;
    }
    legal.enforce_equal(&Boolean::TRUE)?;
    for (slot, actual) in ids.iter().enumerate() {
        for (byte, a) in actual.iter().enumerate() {
            let bits = (0..8)
                .map(|bit| {
                    lookup_bit(
                        &flags,
                        G4D_SUBSETS
                            .iter()
                            .map(|s| ((s[slot].to_be_bytes()[byte] >> bit) & 1) == 1),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            a.enforce_equal(&UInt8::from_bits_le(&bits))?;
        }
    }
    let table = (0..10)
        .map(|i| g4d_lagrange_coefficients(i).expect("fixed distinct subset table"))
        .collect::<Vec<_>>();
    (0..3)
        .map(|slot| {
            let all_bits = table
                .iter()
                .map(|row| row[slot].into_bigint().to_bits_le())
                .collect::<Vec<_>>();
            (0..Scalar::MODULUS_BIT_SIZE as usize)
                .map(|bit| lookup_bit(&flags, all_bits.iter().map(|b| b[bit])))
                .collect()
        })
        .collect()
}

impl P3TenSubsetCircuit {
    pub fn public_inputs(&self) -> Result<Vec<Fq>, SynthesisError> {
        // The old encoder fixes byte-to-field packing only; the Boolean selector
        // is replaced by the NEW field selector. Old keys/layout semantics do not apply.
        let mut x = p3d0_bound_subset_public_inputs(
            &self.trustees,
            false,
            &self.ciphertext_c1,
            self.registered_weight,
            self.public_winner,
        )?;
        if x.len() != G4D_INPUTS {
            return Err(SynthesisError::Unsatisfiable);
        }
        x[G4D_SELECTOR_INDEX] = Fq::from(self.public_subset_index);
        Ok(x)
    }
}

impl ConstraintSynthesizer<Fq> for P3TenSubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if self.ciphertext_c1.len() != 3
            || self.plaintext_groups.len() != 3
            || self.tallies.len() != 3
        {
            return Err(SynthesisError::Unsatisfiable);
        }
        let base = P3B1Circuit {
            trustees: self.trustees,
        };
        let outputs = base.generate_relation_with_shares(cs.clone())?;
        if outputs.decryption_share_vars.len() != 3
            || outputs.decryption_share_vars.iter().any(|s| s.len() != 3)
        {
            return Err(SynthesisError::Unsatisfiable);
        }
        let coefficient_bits = selection(
            cs.clone(),
            self.public_subset_index,
            &outputs.trustee_id_bytes,
        )?;
        let mut plaintexts = Vec::new();
        for j in 0..3 {
            let scaled = outputs
                .decryption_share_vars
                .iter()
                .zip(&coefficient_bits)
                .map(|(shares, bits)| shares[j].scalar_mul_le(bits.iter()))
                .collect::<Result<Vec<_>, _>>()?;
            let combined = &scaled[0] + &scaled[1] + &scaled[2];
            let c1 = EdwardsVar::new_witness(cs.clone(), || Ok(self.ciphertext_c1[j]))?;
            let bytes = UInt8::new_input_vec(cs.clone(), &encode_point(&self.ciphertext_c1[j]))?;
            bind_canonical_compressed_point(&c1, &bytes)?;
            let plaintext = EdwardsVar::new_witness(cs.clone(), || Ok(self.plaintext_groups[j]))?;
            plaintext.enforce_equal(&(c1 - combined))?;
            plaintexts.push(plaintext);
        }
        let tallies = bind_tallies_to_plaintexts(
            cs.clone(),
            &plaintexts,
            &self.tallies,
            self.registered_weight,
        )?;
        enforce_winner(cs, &tallies.tally_bits, self.public_winner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ff::Zero;
    use ark_r1cs_std::R1CSVar;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn all_ten_constant_rows_interpolate_degree_zero_one_two() {
        for (k, ids) in G4D_SUBSETS.iter().enumerate() {
            assert!(ids.windows(2).all(|x| x[0] < x[1]));
            let c = g4d_lagrange_coefficients(k).unwrap();
            for degree in 0..3 {
                let value = ids.iter().zip(c).fold(Scalar::zero(), |sum, (id, c)| {
                    sum + c * Scalar::from(u64::from(*id)).pow([degree])
                });
                assert_eq!(
                    value,
                    if degree == 0 {
                        Scalar::one()
                    } else {
                        Scalar::zero()
                    }
                );
            }
        }
        assert!(g4d_lagrange_coefficients(10).is_none());
    }
    fn check(index: u64, ids: [u32; 3]) -> (bool, Vec<Vec<bool>>, usize) {
        let cs = ConstraintSystem::<Fq>::new_ref();
        let ids = ids
            .iter()
            .map(|id| UInt8::new_witness_vec(cs.clone(), &id.to_be_bytes()).unwrap())
            .collect::<Vec<_>>();
        let bits = selection(cs.clone(), index, &ids).unwrap();
        (
            cs.is_satisfied().unwrap(),
            bits.iter()
                .map(|b| b.iter().map(|b| b.value().unwrap()).collect())
                .collect(),
            cs.num_constraints(),
        )
    }
    #[test]
    fn dynamic_selector_binds_all_ids_and_exact_canonical_bits_without_shape_changes() {
        let mut count = None;
        for (index, ids) in G4D_SUBSETS.into_iter().enumerate() {
            let (satisfied, bits, n) = check(index as u64, ids);
            assert!(satisfied);
            assert_eq!(*count.get_or_insert(n), n);
            for (actual, expected) in bits.iter().zip(g4d_lagrange_coefficients(index).unwrap()) {
                assert_eq!(
                    *actual,
                    expected.into_bigint().to_bits_le()[..Scalar::MODULUS_BIT_SIZE as usize]
                );
            }
        }
        let (valid, _, n) = check(10, [1, 2, 3]);
        assert!(!valid);
        assert_eq!(Some(n), count);
        let (valid, _, n) = check(0, [1, 2, 4]);
        assert!(!valid);
        assert_eq!(Some(n), count);
    }
}
