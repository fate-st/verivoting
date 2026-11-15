//! Positive compile control only: available verifier over BLS12-381 Fq.
mod inner_groth16;

use ark_bls12_381::{Bls12_381, Fq, Fr};
use ark_crypto_primitives::snark::constraints::SNARKGadget;
use ark_ff::PrimeField;
use ark_groth16::Groth16;
use inner_groth16::{constrain_in_native_base_field, NativeConstraintEntry, NativeVerifier};

fn requires_base_field_verifier<V: SNARKGadget<Fr, Fq, Groth16<Bls12_381>>>() {}

fn main() {
    requires_base_field_verifier::<NativeVerifier>();
    // Keep the full constraint body typechecked, but DO NOT invoke it under
    // a different field and call that a success for the current protocol.
    let _typed_entry: NativeConstraintEntry = constrain_in_native_base_field;
    println!(
        "{{\"status\":\"PASS_NATIVE_FQ_COMPILE_ONLY\",\"inner_input_field_bits\":{},\"native_gadget_constraint_field_bits\":{},\"current_outer_constraint_field_bits\":{},\"native_fq_modulus\":\"{}\",\"current_outer_fr_modulus\":\"{}\",\"r1cs_synthesized\":false,\"verification_executed\":false}}",
        Fr::MODULUS_BIT_SIZE,
        Fq::MODULUS_BIT_SIZE,
        Fr::MODULUS_BIT_SIZE,
        Fq::MODULUS,
        Fr::MODULUS,
    );
}
