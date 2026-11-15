//! J1 compilation-only diagnostic for the existing arkworks 0.4 verifier.
//!
//! NOT an Fr-based recursive verifier, joint circuit or production API.
//! This module is compiled only by the isolated tools/j1-inner-verifier-probe
//! package. The original crypto library, ballot/state relations and lockfile
//! remain unchanged. The Fq entry below must not be treated as an Fr circuit.

use ark_bls12_381::{Bls12_381, Config, Fq, Fr};
use ark_crypto_primitives::snark::{constraints::SNARKGadget, BooleanInputVar};
use ark_groth16::{constraints::Groth16VerifierGadget, Groth16, Proof, VerifyingKey};
use ark_r1cs_std::{alloc::AllocVar, boolean::Boolean, eq::EqGadget, pairing::bls12::PairingVar};
use ark_relations::r1cs::{ConstraintSystem, ConstraintSystemRef, SynthesisError};

pub type NativeVerifier = Groth16VerifierGadget<Bls12_381, PairingVar<Config>>;
pub type NativeConstraintEntry = fn(
    &VerifyingKey<Bls12_381>,
    &Proof<Bls12_381>,
    &[Fr],
) -> Result<ConstraintSystemRef<Fq>, SynthesisError>;

/// A fully typed *Fq* constraint body, not executed in the fixed-Fr J1 gate.
/// VK is constant, proof variables use checked witness allocation, and the
/// pairing-verifier output would be enforced true INSIDE R1CS if invoked.
/// No host-language verification result or boolean witness is used here.
pub fn constrain_in_native_base_field(
    vk: &VerifyingKey<Bls12_381>,
    proof: &Proof<Bls12_381>,
    inputs: &[Fr],
) -> Result<ConstraintSystemRef<Fq>, SynthesisError> {
    let cs = ConstraintSystem::<Fq>::new_ref();
    let vk_var =
        <NativeVerifier as SNARKGadget<Fr, Fq, Groth16<Bls12_381>>>::VerifyingKeyVar::new_constant(
            cs.clone(),
            vk.clone(),
        )?;
    let proof_var =
        <NativeVerifier as SNARKGadget<Fr, Fq, Groth16<Bls12_381>>>::ProofVar::new_witness(
            cs.clone(),
            || Ok(proof.clone()),
        )?;
    let input_var = BooleanInputVar::<Fr, Fq>::new_input(cs.clone(), || Ok(inputs.to_vec()))?;
    NativeVerifier::verify(&vk_var, &input_var, &proof_var)?
        .enforce_equal(&Boolean::constant(true))?;
    Ok(cs)
}
