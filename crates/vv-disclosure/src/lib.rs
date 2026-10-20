use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::{Field, One, Zero};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, SynthesisError,
};
use ark_serialize::CanonicalSerialize;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use vv_types::winner;

#[derive(Debug, Error)]
pub enum DisclosureError {
    #[error("invalid P1 input: {0}")]
    InvalidInput(String),
    #[error("R1CS/Groth16 error: {0}")]
    Synthesis(#[from] SynthesisError),
    #[error("serialization error: {0}")]
    Serialization(#[from] ark_serialize::SerializationError),
    #[error("winner helper error: {0}")]
    Winner(#[from] vv_types::TypesError),
}

/// Real Groth16 circuit for P1 only. It proves bounded integer scores and the
/// smallest-ID deterministic winner. It is deliberately not labelled P3.
#[derive(Clone)]
pub struct WinnerP1Circuit {
    pub scores: Vec<u64>,
    pub w_registered: u64,
    pub public_winner: u64,
}

fn allocate_u64_bits(
    cs: ConstraintSystemRef<Fr>,
    value: u64,
) -> Result<Vec<Boolean<Fr>>, SynthesisError> {
    (0..64)
        .map(|bit| Boolean::new_witness(cs.clone(), || Ok(((value >> bit) & 1) == 1)))
        .collect()
}

fn pack_bits(bits: &[Boolean<Fr>]) -> Result<FpVar<Fr>, SynthesisError> {
    let mut packed = FpVar::Constant(Fr::zero());
    let mut coefficient = Fr::one();
    for bit in bits {
        let term = bit.select(&FpVar::Constant(coefficient), &FpVar::Constant(Fr::zero()))?;
        packed += term;
        coefficient.double_in_place();
    }
    Ok(packed)
}

fn less_than(a: &[Boolean<Fr>], b: &[Boolean<Fr>]) -> Result<Boolean<Fr>, SynthesisError> {
    let mut lower = Boolean::constant(false);
    let mut equal_prefix = Boolean::constant(true);
    for (a_bit, b_bit) in a.iter().zip(b).rev() {
        let a_lower_here = a_bit.not().and(b_bit)?;
        lower = lower.or(&equal_prefix.and(&a_lower_here)?)?;
        let different = a_bit.xor(b_bit)?;
        equal_prefix = equal_prefix.and(&different.not())?;
    }
    Ok(lower)
}

fn equal_bits(a: &[Boolean<Fr>], b: &[Boolean<Fr>]) -> Result<Boolean<Fr>, SynthesisError> {
    let mut equal = Boolean::constant(true);
    for (a_bit, b_bit) in a.iter().zip(b) {
        equal = equal.and(&a_bit.xor(b_bit)?.not())?;
    }
    Ok(equal)
}

impl ConstraintSynthesizer<Fr> for WinnerP1Circuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        if self.scores.is_empty() {
            return Err(SynthesisError::Unsatisfiable);
        }
        let public_bound = FpVar::<Fr>::new_input(cs.clone(), || Ok(Fr::from(self.w_registered)))?;
        let public_winner =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(Fr::from(self.public_winner)))?;
        let bound_bits = allocate_u64_bits(cs.clone(), self.w_registered)?;
        pack_bits(&bound_bits)?.enforce_equal(&public_bound)?;

        let mut score_bits = Vec::with_capacity(self.scores.len());
        for score in self.scores {
            let bits = allocate_u64_bits(cs.clone(), score)?;
            let below = less_than(&bits, &bound_bits)?;
            let equal = equal_bits(&bits, &bound_bits)?;
            below.or(&equal)?.enforce_equal(&Boolean::constant(true))?;
            score_bits.push(bits);
        }

        let winner_flags: Vec<_> = (0..score_bits.len())
            .map(|index| public_winner.is_eq(&FpVar::Constant(Fr::from((index + 1) as u64))))
            .collect::<Result<_, _>>()?;
        let mut legal_winner = Boolean::constant(false);
        for flag in &winner_flags {
            legal_winner = legal_winner.or(flag)?;
        }
        legal_winner.enforce_equal(&Boolean::constant(true))?;

        for (winner_index, winner_flag) in winner_flags.iter().enumerate() {
            for candidate_index in 0..score_bits.len() {
                if candidate_index < winner_index {
                    // Earlier identifiers must be strictly smaller.
                    let required =
                        less_than(&score_bits[candidate_index], &score_bits[winner_index])?;
                    winner_flag
                        .and(&required.not())?
                        .enforce_equal(&Boolean::constant(false))?;
                } else if candidate_index > winner_index {
                    // Later identifiers may tie but cannot be larger.
                    let violation =
                        less_than(&score_bits[winner_index], &score_bits[candidate_index])?;
                    winner_flag
                        .and(&violation)?
                        .enforce_equal(&Boolean::constant(false))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct P1Metrics {
    pub classification: String,
    pub candidate_count: usize,
    pub constraints: usize,
    pub public_inputs: usize,
    pub setup_ms: f64,
    pub prove_ms: f64,
    pub verify_ms: f64,
    pub proof_bytes: usize,
    pub proving_key_bytes: usize,
    pub verifying_key_bytes: usize,
    pub verified: bool,
    pub wrong_public_winner_rejected: bool,
}

pub fn run_p1_groth16(
    scores: &[u64],
    w_registered: u64,
    seed: u64,
) -> Result<P1Metrics, DisclosureError> {
    let expected_winner = winner(scores, w_registered)? as u64;
    let circuit = WinnerP1Circuit {
        scores: scores.to_vec(),
        w_registered,
        public_winner: expected_winner,
    };
    let cs = ConstraintSystem::<Fr>::new_ref();
    circuit.clone().generate_constraints(cs.clone())?;
    if !cs.is_satisfied()? {
        return Err(DisclosureError::InvalidInput(
            "valid P1 witness did not satisfy its circuit".to_owned(),
        ));
    }
    let constraints = cs.num_constraints();
    let public_inputs = cs.num_instance_variables().saturating_sub(1);

    let mut setup_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x0053_4554_5550);
    let setup_start = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit.clone(),
        &mut setup_rng,
    )?;
    let setup_ms = setup_start.elapsed().as_secs_f64() * 1000.0;

    let mut proof_rng = ChaCha20Rng::seed_from_u64(seed ^ 0x0050_524f_5645);
    let prove_start = Instant::now();
    let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
        circuit,
        &parameters,
        &mut proof_rng,
    )?;
    let prove_ms = prove_start.elapsed().as_secs_f64() * 1000.0;

    let prepared = prepare_verifying_key(&parameters.vk);
    let public = [Fr::from(w_registered), Fr::from(expected_winner)];
    let verify_start = Instant::now();
    let verified = Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &public)?;
    let verify_ms = verify_start.elapsed().as_secs_f64() * 1000.0;
    let wrong_winner = expected_winner % scores.len() as u64 + 1;
    let wrong_public_winner_rejected = !Groth16::<Bls12_381>::verify_proof(
        &prepared,
        &proof,
        &[Fr::from(w_registered), Fr::from(wrong_winner)],
    )?;

    let mut proof_encoding = Vec::new();
    proof.serialize_compressed(&mut proof_encoding)?;
    let mut pk_encoding = Vec::new();
    parameters.serialize_compressed(&mut pk_encoding)?;
    let mut vk_encoding = Vec::new();
    parameters.vk.serialize_compressed(&mut vk_encoding)?;
    Ok(P1Metrics {
        classification: "real-crypto-p1-groth16-not-p3".to_owned(),
        candidate_count: scores.len(),
        constraints,
        public_inputs,
        setup_ms,
        prove_ms,
        verify_ms,
        proof_bytes: proof_encoding.len(),
        proving_key_bytes: pk_encoding.len(),
        verifying_key_bytes: vk_encoding.len(),
        verified,
        wrong_public_winner_rejected,
    })
}

pub fn wrong_winner_circuit_is_unsatisfied(
    scores: &[u64],
    w_registered: u64,
    wrong_winner: u64,
) -> Result<bool, DisclosureError> {
    let circuit = WinnerP1Circuit {
        scores: scores.to_vec(),
        w_registered,
        public_winner: wrong_winner,
    };
    let cs = ConstraintSystem::<Fr>::new_ref();
    circuit.generate_constraints(cs.clone())?;
    Ok(!cs.is_satisfied()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_p1_groth16_accepts_tie_rule_and_rejects_changed_public_winner() {
        let metrics = run_p1_groth16(&[4, 2, 4], 10, 37).unwrap();
        assert!(metrics.verified);
        assert!(metrics.wrong_public_winner_rejected);
        assert!(metrics.constraints > 0);
        assert!(metrics.proof_bytes > 0);
    }

    #[test]
    fn wrong_winners_make_relation_unsatisfied() {
        assert!(wrong_winner_circuit_is_unsatisfied(&[4, 2, 4], 10, 3).unwrap());
        assert!(wrong_winner_circuit_is_unsatisfied(&[3, 5, 1], 10, 1).unwrap());
    }

    #[test]
    fn out_of_range_score_is_unsatisfied() {
        let circuit = WinnerP1Circuit {
            scores: vec![11, 0, 0],
            w_registered: 10,
            public_winner: 1,
        };
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }
}
