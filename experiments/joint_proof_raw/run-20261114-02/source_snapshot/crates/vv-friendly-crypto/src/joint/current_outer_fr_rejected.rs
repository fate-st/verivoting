//! Expected compile failure, isolated from the original workspace.
//! Identical inner proof curve/gadget; require the actual existing Fr domain.
mod inner_groth16;

use ark_bls12_381::{Bls12_381, Fr};
use ark_crypto_primitives::snark::constraints::SNARKGadget;
use ark_groth16::Groth16;
use inner_groth16::NativeVerifier;

fn requires_current_outer_verifier<V: SNARKGadget<Fr, Fr, Groth16<Bls12_381>>>() {}

fn main() {
    requires_current_outer_verifier::<NativeVerifier>();
}
