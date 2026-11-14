//! G0.9-P3-A: three selected Trustee CP relations with a P3-oriented
//! public/private transcript boundary.
//!
//! This gate deliberately stops before private-package commitment openings,
//! Lagrange interpolation, tally reconstruction and winner logic.

use crate::{
    cp_h3_gadget::H3_TRANSCRIPT_BYTES,
    cp_h4_gadget::{H4ACircuit, H4_DECRYPTION_SHARE_OFFSET},
};
use ark_ed_on_bls12_381::Fq;
use ark_r1cs_std::uint8::UInt8;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};

pub const P3A_TRUSTEE_COUNT: usize = 3;
pub const P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE: usize = H4_DECRYPTION_SHARE_OFFSET;
pub const P3A_PRIVATE_TRANSCRIPT_BYTES_PER_TRUSTEE: usize =
    H3_TRANSCRIPT_BYTES - P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE;

#[derive(Clone, Debug)]
pub struct P3ACircuitReport {
    pub satisfied: bool,
    pub constraints: usize,
    pub public_inputs: usize,
    pub witness_variables: usize,
    pub trustees: usize,
    pub cp_equations: usize,
    pub public_prefix_bytes_per_trustee: usize,
    pub private_transcript_bytes_per_trustee: usize,
}

#[derive(Clone)]
pub struct P3ACircuit {
    pub trustees: Vec<H4ACircuit>,
}

impl P3ACircuit {
    pub fn has_expected_shape(&self) -> bool {
        self.trustees.len() == P3A_TRUSTEE_COUNT
            && self.trustees.iter().all(H4ACircuit::has_expected_shape)
    }

    fn generate_relation(&self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        if !self.has_expected_shape() {
            return Err(SynthesisError::Unsatisfiable);
        }
        for trustee in &self.trustees {
            let mut transcript_var = UInt8::new_input_vec(
                cs.clone(),
                &trustee.transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE],
            )?;
            transcript_var.extend(UInt8::new_witness_vec(
                cs.clone(),
                &trustee.transcript[P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE..],
            )?);
            trustee.generate_relation_with_transcript(cs.clone(), &transcript_var)?;
        }
        Ok(())
    }
}

impl ConstraintSynthesizer<Fq> for P3ACircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> Result<(), SynthesisError> {
        self.generate_relation(cs)
    }
}

pub fn check_three_trustee_cp_p3a(
    trustees: &[H4ACircuit],
) -> Result<P3ACircuitReport, SynthesisError> {
    let circuit = P3ACircuit {
        trustees: trustees.to_vec(),
    };
    if !circuit.has_expected_shape() {
        return Ok(P3ACircuitReport {
            satisfied: false,
            constraints: 0,
            public_inputs: 0,
            witness_variables: 0,
            trustees: trustees.len(),
            cp_equations: 0,
            public_prefix_bytes_per_trustee: P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE,
            private_transcript_bytes_per_trustee: P3A_PRIVATE_TRANSCRIPT_BYTES_PER_TRUSTEE,
        });
    }
    let cs = ConstraintSystem::<Fq>::new_ref();
    circuit.generate_relation(cs.clone())?;
    let satisfied = cs.is_satisfied()?;
    Ok(P3ACircuitReport {
        satisfied,
        constraints: cs.num_constraints(),
        public_inputs: cs.num_instance_variables().saturating_sub(1),
        witness_variables: cs.num_witness_variables(),
        trustees: P3A_TRUSTEE_COUNT,
        cp_equations: P3A_TRUSTEE_COUNT * 4,
        public_prefix_bytes_per_trustee: P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE,
        private_transcript_bytes_per_trustee: P3A_PRIVATE_TRANSCRIPT_BYTES_PER_TRUSTEE,
    })
}
