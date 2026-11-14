//! G0.9-P3-C0: fixed Trustee 1--3 threshold-share combination.
//!
//! This gate extends P3-B1 with the 3-of-5 Lagrange coefficients `(3,-3,1)`
//! and binds three private plaintext group elements to the public aggregate
//! ciphertext `C1` vector. Discrete-log decoding and winner logic are absent.

use crate::{
    cp_h3_gadget::bind_canonical_compressed_point,
    cp_h4_gadget::H4_COORDINATE_COUNT,
    encode_point,
    p3a_gadget::P3A_TRUSTEE_COUNT,
    p3b_gadget::{P3B1Circuit, P3B1TrusteeCircuit},
    Point, Scalar,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{BigInteger, Field, PrimeField};
use ark_r1cs_std::{
    alloc::AllocVar, boolean::Boolean, eq::EqGadget, groups::CurveVar, select::CondSelectGadget,
    uint8::UInt8,
};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};

pub const P3C0_LAGRANGE_SUBSET: [u32; 3] = [1, 2, 3];
pub const P3C0_LAGRANGE_COEFFICIENTS: [&str; 3] = ["3", "-3", "1"];
pub const G4B_BASELINE_SUBSET: [u32; 3] = [1, 2, 3];
pub const G4B_SECOND_SUBSET: [u32; 3] = [1, 2, 4];

#[derive(Clone)]
pub struct P3C0Circuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
}

/// G4-A feasibility variant of P3-C0.
///
/// The three Lagrange coefficients are circuit parameters.  Their fixed-width
/// bit representations are allocated and constrained before using the same
/// scalar-multiplication path for every legal subset.  They are deliberately
/// not public inputs in G4-A: binding a verifier-selected subset and its
/// coefficients is the separate G4-B gate.
#[derive(Clone)]
pub struct P3C0SubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub lagrange_coefficients: [Scalar; P3A_TRUSTEE_COUNT],
}

/// G4-B two-subset relation. One public selector is bound to the Trustee IDs
/// already present in the three public CP transcript prefixes. The same
/// selector derives the exact Lagrange coefficients inside the circuit.
#[derive(Clone)]
pub struct P3C0BoundSubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub public_second_subset: bool,
}

#[derive(Clone, Debug)]
pub struct P3C0CircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub trustees: usize,
    pub cp_equations: usize,
    pub package_h2f_relations: usize,
    pub package_opening_equations: usize,
    pub lagrange_combination_equations: usize,
    pub public_ciphertext_c1_points: usize,
    pub private_plaintext_group_points: usize,
}

pub(crate) struct P3C0RelationOutput {
    pub plaintext_vars: Vec<EdwardsVar>,
}

impl P3C0Circuit {
    pub fn has_expected_shape(&self) -> bool {
        self.trustees.len() == P3A_TRUSTEE_COUNT
            && self
                .trustees
                .iter()
                .all(|trustee| trustee.cp.has_expected_shape())
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_plaintexts(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3C0RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let base = P3B1Circuit {
            trustees: self.trustees.clone(),
        };
        let base_output = base.generate_relation_with_shares(cs.clone())?;
        if base_output.decryption_share_vars.len() != P3A_TRUSTEE_COUNT
            || base_output
                .decryption_share_vars
                .iter()
                .any(|shares| shares.len() != H4_COORDINATE_COUNT)
        {
            return Err(SynthesisError::Unsatisfiable);
        }

        let mut plaintext_vars = Vec::with_capacity(H4_COORDINATE_COUNT);
        for coordinate in 0..H4_COORDINATE_COUNT {
            let share_1 = &base_output.decryption_share_vars[0][coordinate];
            let share_2 = &base_output.decryption_share_vars[1][coordinate];
            let share_3 = &base_output.decryption_share_vars[2][coordinate];
            let three_share_1 = share_1.double()? + share_1;
            let three_share_2 = share_2.double()? + share_2;
            let combined_share = three_share_1 - &three_share_2 + share_3;

            let ciphertext_c1_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(self.ciphertext_c1[coordinate]))?;
            let ciphertext_c1_bytes =
                UInt8::new_input_vec(cs.clone(), &encode_point(&self.ciphertext_c1[coordinate]))?;
            bind_canonical_compressed_point(&ciphertext_c1_var, &ciphertext_c1_bytes)?;

            let plaintext_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(self.plaintext_groups[coordinate]))?;
            plaintext_var.enforce_equal(&(ciphertext_c1_var - combined_share))?;
            plaintext_vars.push(plaintext_var);
        }
        Ok(P3C0RelationOutput { plaintext_vars })
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation_with_plaintexts(cs).map(|_| ())
    }
}

fn constrained_coefficient_bits(
    cs: ConstraintSystemRef<Fq>,
    coefficient: Scalar,
) -> Result<Vec<Boolean<Fq>>, SynthesisError> {
    let mut values = coefficient.into_bigint().to_bits_le();
    values.resize(Scalar::MODULUS_BIT_SIZE as usize, false);
    values.truncate(Scalar::MODULUS_BIT_SIZE as usize);
    values
        .into_iter()
        .map(|value| {
            let bit = Boolean::new_witness(cs.clone(), || Ok(value))?;
            bit.enforce_equal(&Boolean::constant(value))?;
            Ok(bit)
        })
        .collect()
}

pub fn g4b_selected_subset(public_second_subset: bool) -> [u32; 3] {
    if public_second_subset {
        G4B_SECOND_SUBSET
    } else {
        G4B_BASELINE_SUBSET
    }
}

pub fn g4b_lagrange_coefficients(public_second_subset: bool) -> [Scalar; 3] {
    if public_second_subset {
        let inverse_three = Scalar::from(3u64)
            .inverse()
            .expect("three is invertible in the Jubjub scalar field");
        [
            Scalar::from(8u64) * inverse_three,
            -Scalar::from(2u64),
            inverse_three,
        ]
    } else {
        [Scalar::from(3u64), -Scalar::from(3u64), Scalar::from(1u64)]
    }
}

fn canonical_scalar_bits(coefficient: Scalar) -> Vec<bool> {
    let mut values = coefficient.into_bigint().to_bits_le();
    values.resize(Scalar::MODULUS_BIT_SIZE as usize, false);
    values.truncate(Scalar::MODULUS_BIT_SIZE as usize);
    values
}

fn selected_coefficient_bits(
    selector: &Boolean<Fq>,
) -> Result<Vec<Vec<Boolean<Fq>>>, SynthesisError> {
    g4b_lagrange_coefficients(false)
        .into_iter()
        .zip(g4b_lagrange_coefficients(true))
        .map(|(baseline, second)| {
            canonical_scalar_bits(baseline)
                .into_iter()
                .zip(canonical_scalar_bits(second))
                .map(|(baseline_bit, second_bit)| {
                    Boolean::conditionally_select(
                        selector,
                        &Boolean::constant(second_bit),
                        &Boolean::constant(baseline_bit),
                    )
                })
                .collect()
        })
        .collect()
}

fn enforce_selected_trustee_ids(
    trustee_id_bytes: &[Vec<UInt8<Fq>>],
    selector: &Boolean<Fq>,
) -> Result<(), SynthesisError> {
    if trustee_id_bytes.len() != P3A_TRUSTEE_COUNT
        || trustee_id_bytes.iter().any(|bytes| bytes.len() != 4)
    {
        return Err(SynthesisError::Unsatisfiable);
    }
    for (actual, expected) in trustee_id_bytes[..2]
        .iter()
        .zip([1u32.to_be_bytes(), 2u32.to_be_bytes()])
    {
        for (actual_byte, expected_byte) in actual.iter().zip(expected) {
            actual_byte.enforce_equal(&UInt8::constant(expected_byte))?;
        }
    }
    for byte in &trustee_id_bytes[2][..3] {
        byte.enforce_equal(&UInt8::constant(0))?;
    }
    let selected_last_byte =
        UInt8::conditionally_select(selector, &UInt8::constant(4), &UInt8::constant(3))?;
    trustee_id_bytes[2][3].enforce_equal(&selected_last_byte)
}

impl P3C0SubsetCircuit {
    pub fn has_expected_shape(&self) -> bool {
        self.trustees.len() == P3A_TRUSTEE_COUNT
            && self
                .trustees
                .iter()
                .all(|trustee| trustee.cp.has_expected_shape())
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_plaintexts(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3C0RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let base = P3B1Circuit {
            trustees: self.trustees.clone(),
        };
        let base_output = base.generate_relation_with_shares(cs.clone())?;
        if base_output.decryption_share_vars.len() != P3A_TRUSTEE_COUNT
            || base_output
                .decryption_share_vars
                .iter()
                .any(|shares| shares.len() != H4_COORDINATE_COUNT)
        {
            return Err(SynthesisError::Unsatisfiable);
        }

        let coefficient_bits = self
            .lagrange_coefficients
            .iter()
            .map(|coefficient| constrained_coefficient_bits(cs.clone(), *coefficient))
            .collect::<Result<Vec<_>, _>>()?;

        let mut plaintext_vars = Vec::with_capacity(H4_COORDINATE_COUNT);
        for coordinate in 0..H4_COORDINATE_COUNT {
            let mut scaled_shares = Vec::with_capacity(P3A_TRUSTEE_COUNT);
            for (shares, bits) in base_output
                .decryption_share_vars
                .iter()
                .zip(&coefficient_bits)
            {
                scaled_shares.push(shares[coordinate].scalar_mul_le(bits.iter())?);
            }
            let combined_share = &scaled_shares[0] + &scaled_shares[1] + &scaled_shares[2];

            let ciphertext_c1_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(self.ciphertext_c1[coordinate]))?;
            let ciphertext_c1_bytes =
                UInt8::new_input_vec(cs.clone(), &encode_point(&self.ciphertext_c1[coordinate]))?;
            bind_canonical_compressed_point(&ciphertext_c1_var, &ciphertext_c1_bytes)?;

            let plaintext_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(self.plaintext_groups[coordinate]))?;
            plaintext_var.enforce_equal(&(ciphertext_c1_var - combined_share))?;
            plaintext_vars.push(plaintext_var);
        }
        Ok(P3C0RelationOutput { plaintext_vars })
    }
}

impl ConstraintSynthesizer<Fq> for P3C0SubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation_with_plaintexts(cs).map(|_| ())
    }
}

impl P3C0BoundSubsetCircuit {
    pub fn has_expected_shape(&self) -> bool {
        self.trustees.len() == P3A_TRUSTEE_COUNT
            && self
                .trustees
                .iter()
                .all(|trustee| trustee.cp.has_expected_shape())
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_plaintexts(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3C0RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let base = P3B1Circuit {
            trustees: self.trustees.clone(),
        };
        let base_output = base.generate_relation_with_shares(cs.clone())?;
        if base_output.decryption_share_vars.len() != P3A_TRUSTEE_COUNT
            || base_output
                .decryption_share_vars
                .iter()
                .any(|shares| shares.len() != H4_COORDINATE_COUNT)
        {
            return Err(SynthesisError::Unsatisfiable);
        }

        let selector = Boolean::new_input(cs.clone(), || Ok(self.public_second_subset))?;
        enforce_selected_trustee_ids(&base_output.trustee_id_bytes, &selector)?;
        let coefficient_bits = selected_coefficient_bits(&selector)?;

        let mut plaintext_vars = Vec::with_capacity(H4_COORDINATE_COUNT);
        for coordinate in 0..H4_COORDINATE_COUNT {
            let mut scaled_shares = Vec::with_capacity(P3A_TRUSTEE_COUNT);
            for (shares, bits) in base_output
                .decryption_share_vars
                .iter()
                .zip(&coefficient_bits)
            {
                scaled_shares.push(shares[coordinate].scalar_mul_le(bits.iter())?);
            }
            let combined_share = &scaled_shares[0] + &scaled_shares[1] + &scaled_shares[2];

            let ciphertext_c1_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(self.ciphertext_c1[coordinate]))?;
            let ciphertext_c1_bytes =
                UInt8::new_input_vec(cs.clone(), &encode_point(&self.ciphertext_c1[coordinate]))?;
            bind_canonical_compressed_point(&ciphertext_c1_var, &ciphertext_c1_bytes)?;

            let plaintext_var =
                EdwardsVar::new_witness(cs.clone(), || Ok(self.plaintext_groups[coordinate]))?;
            plaintext_var.enforce_equal(&(ciphertext_c1_var - combined_share))?;
            plaintext_vars.push(plaintext_var);
        }
        Ok(P3C0RelationOutput { plaintext_vars })
    }
}

impl ConstraintSynthesizer<Fq> for P3C0BoundSubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation_with_plaintexts(cs).map(|_| ())
    }
}

impl ConstraintSynthesizer<Fq> for P3C0Circuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_fixed_threshold_combination_p3c0(
    trustees: &[P3B1TrusteeCircuit],
    ciphertext_c1: &[Point],
    plaintext_groups: &[Point],
) -> Result<P3C0CircuitReport, SynthesisError> {
    let circuit = P3C0Circuit {
        trustees: trustees.to_vec(),
        ciphertext_c1: ciphertext_c1.to_vec(),
        plaintext_groups: plaintext_groups.to_vec(),
    };
    if !circuit.has_expected_shape() {
        return Ok(P3C0CircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            trustees: trustees.len(),
            cp_equations: 0,
            package_h2f_relations: 0,
            package_opening_equations: 0,
            lagrange_combination_equations: 0,
            public_ciphertext_c1_points: ciphertext_c1.len(),
            private_plaintext_group_points: plaintext_groups.len(),
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3C0CircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        trustees: P3A_TRUSTEE_COUNT,
        cp_equations: P3A_TRUSTEE_COUNT * 4,
        package_h2f_relations: P3A_TRUSTEE_COUNT,
        package_opening_equations: P3A_TRUSTEE_COUNT,
        lagrange_combination_equations: H4_COORDINATE_COUNT,
        public_ciphertext_c1_points: H4_COORDINATE_COUNT,
        private_plaintext_group_points: H4_COORDINATE_COUNT,
    })
}
