//! G0.9-P3-D0: public winner linked to the private P3-C1 tally witnesses.
//!
//! The circuit enforces the highest-weight rule with the smallest candidate
//! identifier winning ties. It reuses the exact tally bits already bound to
//! the threshold-decrypted plaintext points. Groth16 is not run in this gate.

use crate::{
    cp_h4_gadget::H4_COORDINATE_COUNT,
    encode_point,
    p3a_gadget::P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE,
    p3b_gadget::P3B1TrusteeCircuit,
    p3c1_gadget::{less_than, P3C1BoundSubsetCircuit, P3C1Circuit, P3C1SubsetCircuit},
    Point, Scalar,
};
use ark_ed_on_bls12_381::Fq;
use ark_ff::ToConstraintField;
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};

#[derive(Clone)]
pub struct P3D0Circuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
    pub public_winner: u64,
}

#[derive(Clone)]
pub struct P3D0SubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub lagrange_coefficients: [Scalar; 3],
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
    pub public_winner: u64,
}

#[derive(Clone)]
pub struct P3D0BoundSubsetCircuit {
    pub trustees: Vec<P3B1TrusteeCircuit>,
    pub ciphertext_c1: Vec<Point>,
    pub plaintext_groups: Vec<Point>,
    pub public_second_subset: bool,
    pub tallies: Vec<u64>,
    pub registered_weight: u64,
    pub public_winner: u64,
}

#[derive(Clone, Debug)]
pub struct P3D0CircuitReport {
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
    pub winner_candidate_flags: usize,
    pub winner_pairwise_relations: usize,
    pub public_registered_weight_values: usize,
    pub public_winner_values: usize,
    pub private_tally_values: usize,
}

pub fn p3d0_public_inputs(
    trustees: &[P3B1TrusteeCircuit],
    ciphertext_c1: &[Point],
    registered_weight: u64,
    public_winner: u64,
) -> Result<Vec<Fq>, SynthesisError> {
    if trustees.len() != 3
        || trustees
            .iter()
            .any(|trustee| !trustee.cp.has_expected_shape())
        || ciphertext_c1.len() != H4_COORDINATE_COUNT
    {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut public_inputs = Vec::new();
    for trustee in trustees {
        let prefix = &trustee.cp.transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE];
        let prefix_fields: Vec<Fq> = prefix
            .to_field_elements()
            .ok_or(SynthesisError::Unsatisfiable)?;
        public_inputs.extend(prefix_fields);
        let commitment_fields: Vec<Fq> = encode_point(&trustee.package_commitment)
            .to_field_elements()
            .ok_or(SynthesisError::Unsatisfiable)?;
        public_inputs.extend(commitment_fields);
    }
    for point in ciphertext_c1 {
        let point_fields: Vec<Fq> = encode_point(point)
            .to_field_elements()
            .ok_or(SynthesisError::Unsatisfiable)?;
        public_inputs.extend(point_fields);
    }
    public_inputs.push(Fq::from(registered_weight));
    public_inputs.push(Fq::from(public_winner));
    Ok(public_inputs)
}

pub fn p3d0_bound_subset_public_inputs(
    trustees: &[P3B1TrusteeCircuit],
    public_second_subset: bool,
    ciphertext_c1: &[Point],
    registered_weight: u64,
    public_winner: u64,
) -> Result<Vec<Fq>, SynthesisError> {
    if trustees.len() != 3
        || trustees
            .iter()
            .any(|trustee| !trustee.cp.has_expected_shape())
        || ciphertext_c1.len() != H4_COORDINATE_COUNT
    {
        return Err(SynthesisError::Unsatisfiable);
    }
    let mut public_inputs = Vec::new();
    for trustee in trustees {
        let prefix = &trustee.cp.transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE];
        let prefix_fields: Vec<Fq> = prefix
            .to_field_elements()
            .ok_or(SynthesisError::Unsatisfiable)?;
        public_inputs.extend(prefix_fields);
        let commitment_fields: Vec<Fq> = encode_point(&trustee.package_commitment)
            .to_field_elements()
            .ok_or(SynthesisError::Unsatisfiable)?;
        public_inputs.extend(commitment_fields);
    }
    public_inputs.push(Fq::from(u64::from(public_second_subset)));
    for point in ciphertext_c1 {
        let point_fields: Vec<Fq> = encode_point(point)
            .to_field_elements()
            .ok_or(SynthesisError::Unsatisfiable)?;
        public_inputs.extend(point_fields);
    }
    public_inputs.push(Fq::from(registered_weight));
    public_inputs.push(Fq::from(public_winner));
    Ok(public_inputs)
}

pub(crate) fn enforce_winner(
    cs: ConstraintSystemRef<Fq>,
    tally_bits: &[Vec<Boolean<Fq>>],
    public_winner_value: u64,
) -> Result<(), SynthesisError> {
    let public_winner = FpVar::<Fq>::new_input(cs, || Ok(Fq::from(public_winner_value)))?;
    let winner_flags = (0..H4_COORDINATE_COUNT)
        .map(|index| public_winner.is_eq(&FpVar::Constant(Fq::from((index + 1) as u64))))
        .collect::<Result<Vec<_>, _>>()?;
    let mut legal_winner = Boolean::FALSE;
    for winner_flag in &winner_flags {
        legal_winner = legal_winner.or(winner_flag)?;
    }
    legal_winner.enforce_equal(&Boolean::TRUE)?;

    for (winner_index, winner_flag) in winner_flags.iter().enumerate() {
        for candidate_index in 0..H4_COORDINATE_COUNT {
            if candidate_index < winner_index {
                let earlier_is_smaller =
                    less_than(&tally_bits[candidate_index], &tally_bits[winner_index])?;
                winner_flag
                    .and(&earlier_is_smaller.not())?
                    .enforce_equal(&Boolean::FALSE)?;
            } else if candidate_index > winner_index {
                let later_is_larger =
                    less_than(&tally_bits[winner_index], &tally_bits[candidate_index])?;
                winner_flag
                    .and(&later_is_larger)?
                    .enforce_equal(&Boolean::FALSE)?;
            }
        }
    }
    Ok(())
}

impl P3D0Circuit {
    fn has_expected_shape(&self) -> bool {
        self.tallies.len() == H4_COORDINATE_COUNT
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let p3c1 = P3C1Circuit {
            trustees: self.trustees.clone(),
            ciphertext_c1: self.ciphertext_c1.clone(),
            plaintext_groups: self.plaintext_groups.clone(),
            tallies: self.tallies.clone(),
            registered_weight: self.registered_weight,
        };
        let p3c1_output = p3c1.generate_relation_with_tallies(cs.clone())?;

        enforce_winner(cs, &p3c1_output.tally_bits, self.public_winner)
    }
}

impl P3D0SubsetCircuit {
    fn has_expected_shape(&self) -> bool {
        self.tallies.len() == H4_COORDINATE_COUNT
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let p3c1 = P3C1SubsetCircuit {
            trustees: self.trustees.clone(),
            ciphertext_c1: self.ciphertext_c1.clone(),
            plaintext_groups: self.plaintext_groups.clone(),
            lagrange_coefficients: self.lagrange_coefficients,
            tallies: self.tallies.clone(),
            registered_weight: self.registered_weight,
        };
        let p3c1_output = p3c1.generate_relation_with_tallies(cs.clone())?;
        enforce_winner(cs, &p3c1_output.tally_bits, self.public_winner)
    }
}

impl ConstraintSynthesizer<Fq> for P3D0SubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

impl P3D0BoundSubsetCircuit {
    fn has_expected_shape(&self) -> bool {
        self.tallies.len() == H4_COORDINATE_COUNT
            && self.ciphertext_c1.len() == H4_COORDINATE_COUNT
            && self.plaintext_groups.len() == H4_COORDINATE_COUNT
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let p3c1 = P3C1BoundSubsetCircuit {
            trustees: self.trustees.clone(),
            ciphertext_c1: self.ciphertext_c1.clone(),
            plaintext_groups: self.plaintext_groups.clone(),
            public_second_subset: self.public_second_subset,
            tallies: self.tallies.clone(),
            registered_weight: self.registered_weight,
        };
        let p3c1_output = p3c1.generate_relation_with_tallies(cs.clone())?;
        enforce_winner(cs, &p3c1_output.tally_bits, self.public_winner)
    }
}

impl ConstraintSynthesizer<Fq> for P3D0BoundSubsetCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

impl ConstraintSynthesizer<Fq> for P3D0Circuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_winner_linkage_p3d0(
    trustees: &[P3B1TrusteeCircuit],
    ciphertext_c1: &[Point],
    plaintext_groups: &[Point],
    tallies: &[u64],
    registered_weight: u64,
    public_winner: u64,
) -> Result<P3D0CircuitReport, SynthesisError> {
    let circuit = P3D0Circuit {
        trustees: trustees.to_vec(),
        ciphertext_c1: ciphertext_c1.to_vec(),
        plaintext_groups: plaintext_groups.to_vec(),
        tallies: tallies.to_vec(),
        registered_weight,
        public_winner,
    };
    if !circuit.has_expected_shape() {
        return Ok(P3D0CircuitReport {
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
            winner_candidate_flags: 0,
            winner_pairwise_relations: 0,
            public_registered_weight_values: 0,
            public_winner_values: 0,
            private_tally_values: tallies.len(),
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3D0CircuitReport {
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
        winner_candidate_flags: H4_COORDINATE_COUNT,
        winner_pairwise_relations: H4_COORDINATE_COUNT * (H4_COORDINATE_COUNT - 1),
        public_registered_weight_values: 1,
        public_winner_values: 1,
        private_tally_values: H4_COORDINATE_COUNT,
    })
}

pub fn check_winner_linkage_p3d0_for_subset(
    trustees: &[P3B1TrusteeCircuit],
    ciphertext_c1: &[Point],
    plaintext_groups: &[Point],
    lagrange_coefficients: [Scalar; 3],
    tallies: &[u64],
    registered_weight: u64,
    public_winner: u64,
) -> Result<P3D0CircuitReport, SynthesisError> {
    let circuit = P3D0SubsetCircuit {
        trustees: trustees.to_vec(),
        ciphertext_c1: ciphertext_c1.to_vec(),
        plaintext_groups: plaintext_groups.to_vec(),
        lagrange_coefficients,
        tallies: tallies.to_vec(),
        registered_weight,
        public_winner,
    };
    if !circuit.has_expected_shape() {
        return Ok(P3D0CircuitReport {
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
            winner_candidate_flags: 0,
            winner_pairwise_relations: 0,
            public_registered_weight_values: 0,
            public_winner_values: 0,
            private_tally_values: tallies.len(),
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3D0CircuitReport {
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
        winner_candidate_flags: H4_COORDINATE_COUNT,
        winner_pairwise_relations: H4_COORDINATE_COUNT * (H4_COORDINATE_COUNT - 1),
        public_registered_weight_values: 1,
        public_winner_values: 1,
        private_tally_values: H4_COORDINATE_COUNT,
    })
}

pub fn check_winner_linkage_p3d0_for_bound_subset(
    trustees: &[P3B1TrusteeCircuit],
    ciphertext_c1: &[Point],
    plaintext_groups: &[Point],
    public_second_subset: bool,
    tallies: &[u64],
    registered_weight: u64,
    public_winner: u64,
) -> Result<P3D0CircuitReport, SynthesisError> {
    let circuit = P3D0BoundSubsetCircuit {
        trustees: trustees.to_vec(),
        ciphertext_c1: ciphertext_c1.to_vec(),
        plaintext_groups: plaintext_groups.to_vec(),
        public_second_subset,
        tallies: tallies.to_vec(),
        registered_weight,
        public_winner,
    };
    if !circuit.has_expected_shape() {
        return Ok(P3D0CircuitReport {
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
            winner_candidate_flags: 0,
            winner_pairwise_relations: 0,
            public_registered_weight_values: 0,
            public_winner_values: 0,
            private_tally_values: tallies.len(),
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3D0CircuitReport {
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
        winner_candidate_flags: H4_COORDINATE_COUNT,
        winner_pairwise_relations: H4_COORDINATE_COUNT * (H4_COORDINATE_COUNT - 1),
        public_registered_weight_values: 1,
        public_winner_values: 1,
        private_tally_values: H4_COORDINATE_COUNT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep this regression isolated from CP hashing: the full P3 experiment
    // separately proves that these same tally bits come from decryption.
    fn partial_turnout_winner_satisfied(public_winner: u64) -> bool {
        let cs = ConstraintSystem::<Fq>::new_ref();
        let tally_bits = [0u64, 2, 4]
            .iter()
            .map(|tally| {
                (0..64)
                    .map(|bit| {
                        Boolean::new_witness(cs.clone(), || Ok(((tally >> bit) & 1) == 1))
                            .expect("tally bit witness")
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        enforce_winner(cs.clone(), &tally_bits, public_winner).expect("winner synthesis");
        cs.is_satisfied().expect("winner satisfaction")
    }

    #[test]
    fn partial_turnout_keeps_the_correct_winner_and_rejects_a_wrong_winner() {
        assert!(partial_turnout_winner_satisfied(3));
        assert!(!partial_turnout_winner_satisfied(1));
    }
}
