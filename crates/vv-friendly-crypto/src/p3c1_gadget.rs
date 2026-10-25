//! G0.9-P3-C1: bounded integer tallies bound to the P3-C0 plaintext points.
//!
//! This gate adds three private 64-bit tally witnesses, proves `M_j = t_j G`,
//! constrains every tally to the public registered-weight bound, and enforces
//! that the three tallies sum to that bound. Winner logic and Groth16 are absent.

use crate::{
    cp_h3_gadget::fixed_base_mul,
    cp_h4_gadget::H4_COORDINATE_COUNT,
    p3b_gadget::P3B1TrusteeCircuit,
    p3c_gadget::{P3C0BoundSubsetCircuit, P3C0Circuit, P3C0SubsetCircuit},
    Point, Scalar,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, Fq};
use ark_ff::{Field, One, Zero};
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};

#[derive(Clone)]
pub struct P3C1Circuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
}

#[derive(Clone)]
pub struct P3C1SubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub lagrange_coefficients: [Scalar; 3],
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
}

#[derive(Clone)]
pub struct P3C1BoundSubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub public_second_subset: bool,
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
}

#[derive(Clone, Debug)]
pub struct P3C1CircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub trustees: usize,
    pub cp_equations: usize,
    pub package_h2f_relations: usize,
    pub package_opening_equations: usize,
    pub lagrange_combination_equations: usize,
    pub tally_group_binding_equations: usize,
    pub tally_range_relations: usize,
    pub tally_sum_relations: usize,
    pub public_registered_weight_values: usize,
    pub private_tally_values: usize,
}

pub(crate) struct P3C1RelationOutput {
    pub tally_bits: Vec<Vec<Boolean<Fq>>>,
}

fn allocate_u64_bits(
    cs: ConstraintSystemRef<Fq>,
    value: u64,
) -> Result<Vec<Boolean<Fq>>, SynthesisError> {
    (0..64)
        .map(|bit| Boolean::new_witness(cs.clone(), || Ok(((value >> bit) & 1) == 1)))
        .collect()
}

fn pack_bits(bits: &[Boolean<Fq>]) -> Result<FpVar<Fq>, SynthesisError> {
    let mut packed = FpVar::Constant(Fq::zero());
    let mut coefficient = Fq::one();
    for bit in bits {
        let term = bit.select(&FpVar::Constant(coefficient), &FpVar::Constant(Fq::zero()))?;
        packed += term;
        coefficient.double_in_place();
    }
    Ok(packed)
}

pub(crate) fn less_than(
    a: &[Boolean<Fq>],
    b: &[Boolean<Fq>],
) -> Result<Boolean<Fq>, SynthesisError> {
    if a.len() != b.len() {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut lower = Boolean::FALSE;
    let mut equal_prefix = Boolean::TRUE;
    for (a_bit, b_bit) in a.iter().zip(b).rev() {
        let a_lower_here = a_bit.not().and(b_bit)?;
        lower = lower.or(&equal_prefix.and(&a_lower_here)?)?;
        equal_prefix = equal_prefix.and(&a_bit.xor(b_bit)?.not())?;
    }
    Ok(lower)
}

fn equal_bits(a: &[Boolean<Fq>], b: &[Boolean<Fq>]) -> Result<Boolean<Fq>, SynthesisError> {
    if a.len() != b.len() {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut equal = Boolean::TRUE;
    for (a_bit, b_bit) in a.iter().zip(b) {
        equal = equal.and(&a_bit.xor(b_bit)?.not())?;
    }
    Ok(equal)
}

fn bind_tallies_to_plaintexts(
    cs: ConstraintSystemRef<Fq>,
    plaintext_vars: &[EdwardsVar],
    tallies: &[u64],
    registered_weight: u64,
) -> Result<P3C1RelationOutput, SynthesisError> {
    let public_bound = FpVar::<Fq>::new_input(cs.clone(), || Ok(Fq::from(registered_weight)))?;
    let bound_bits = allocate_u64_bits(cs.clone(), registered_weight)?;
    pack_bits(&bound_bits)?.enforce_equal(&public_bound)?;

    let mut tally_sum = FpVar::Constant(Fq::zero());
    let mut tally_bits_output = Vec::with_capacity(H4_COORDINATE_COUNT);
    for (tally, plaintext_var) in tallies.iter().zip(plaintext_vars) {
        let tally_bits = allocate_u64_bits(cs.clone(), *tally)?;
        let tally_var = pack_bits(&tally_bits)?;
        tally_sum += &tally_var;

        let below = less_than(&tally_bits, &bound_bits)?;
        let equal = equal_bits(&tally_bits, &bound_bits)?;
        below.or(&equal)?.enforce_equal(&Boolean::TRUE)?;

        fixed_base_mul(&tally_bits)?.enforce_equal(plaintext_var)?;
        tally_bits_output.push(tally_bits);
    }
    tally_sum.enforce_equal(&public_bound)?;
    Ok(P3C1RelationOutput {
        tally_bits: tally_bits_output,
    })
}

impl P3C1Circuit {
    fn has_expected_shape(&self) -> bool {
        self.tallies.len() == H4_COORDINATE_COUNT
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_tallies(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3C1RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let p3c0 = P3C0Circuit {
            trustees: self.trustees.clone(),
            ciphertext_c1: self.ciphertext_c1.clone(),
            plaintext_groups: self.plaintext_groups.clone(),
        };
        let p3c0_output = p3c0.generate_relation_with_plaintexts(cs.clone())?;

        bind_tallies_to_plaintexts(
            cs,
            &p3c0_output.plaintext_vars,
            &self.tallies,
            self.registered_weight,
        )
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation_with_tallies(cs).map(|_| ())
    }
}

impl P3C1SubsetCircuit {
    fn has_expected_shape(&self) -> bool {
        self.tallies.len() == H4_COORDINATE_COUNT
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_tallies(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3C1RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let p3c0 = P3C0SubsetCircuit {
            trustees: self.trustees.clone(),
            ciphertext_c1: self.ciphertext_c1.clone(),
            plaintext_groups: self.plaintext_groups.clone(),
            lagrange_coefficients: self.lagrange_coefficients,
        };
        let p3c0_output = p3c0.generate_relation_with_plaintexts(cs.clone())?;
        bind_tallies_to_plaintexts(
            cs,
            &p3c0_output.plaintext_vars,
            &self.tallies,
            self.registered_weight,
        )
    }
}

impl ConstraintSynthesizer<Fq> for P3C1SubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation_with_tallies(cs).map(|_| ())
    }
}

impl P3C1BoundSubsetCircuit {
    fn has_expected_shape(&self) -> bool {
        self.tallies.len() == H4_COORDINATE_COUNT
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    pub(crate) fn generate_relation_with_tallies(
        &self,
        cs: ConstraintSystemRef<Fq>,
    ) -> Result<P3C1RelationOutput, SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let p3c0 = P3C0BoundSubsetCircuit {
            trustees: self.trustees.clone(),
            ciphertext_c1: self.ciphertext_c1.clone(),
            plaintext_groups: self.plaintext_groups.clone(),
            public_second_subset: self.public_second_subset,
        };
        let p3c0_output = p3c0.generate_relation_with_plaintexts(cs.clone())?;
        bind_tallies_to_plaintexts(
            cs,
            &p3c0_output.plaintext_vars,
            &self.tallies,
            self.registered_weight,
        )
    }
}

impl ConstraintSynthesizer<Fq> for P3C1BoundSubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation_with_tallies(cs).map(|_| ())
    }
}

impl ConstraintSynthesizer<Fq> for P3C1Circuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_bounded_tally_binding_p3c1(
    trustees: &[P3B1TrusteeCircuit],
    ciphertext_c1: &[Point],
    plaintext_groups: &[Point],
    tallies: &[u64],
    registered_weight: u64,
) -> Result<P3C1CircuitReport, SynthesisError> {
    let circuit = P3C1Circuit {
        trustees: trustees.to_vec(),
        ciphertext_c1: ciphertext_c1.to_vec(),
        plaintext_groups: plaintext_groups.to_vec(),
        tallies: tallies.to_vec(),
        registered_weight,
    };
    if !circuit.has_expected_shape() {
        return Ok(P3C1CircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            trustees: trustees.len(),
            cp_equations: 0,
            package_h2f_relations: 0,
            package_opening_equations: 0,
            lagrange_combination_equations: 0,
            tally_group_binding_equations: 0,
            tally_range_relations: 0,
            tally_sum_relations: 0,
            public_registered_weight_values: 0,
            private_tally_values: tallies.len(),
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3C1CircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        trustees: 3,
        cp_equations: 12,
        package_h2f_relations: 3,
        package_opening_equations: 3,
        lagrange_combination_equations: 3,
        tally_group_binding_equations: H4_COORDINATE_COUNT,
        tally_range_relations: H4_COORDINATE_COUNT,
        tally_sum_relations: 1,
        public_registered_weight_values: 1,
        private_tally_values: H4_COORDINATE_COUNT,
    })
}
