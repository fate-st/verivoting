//! Public-only BLS12-381 Groth16 export in the EIP-2537 precompile encoding.

use ark_bls12_381::{Bls12_381, Fq, Fr, G1Affine, G2Affine};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{Proof, VerifyingKey};
use serde::Serialize;
use vv_types::hex;

fn fp_bytes(value: &Fq) -> Vec<u8> {
    let integer = value.into_bigint().to_bytes_be();
    let mut result = vec![0; 64 - integer.len()];
    result.extend(integer);
    result
}

fn g1_bytes(point: &G1Affine) -> Vec<u8> {
    if point.is_zero() {
        return vec![0; 128];
    }
    [fp_bytes(&point.x), fp_bytes(&point.y)].concat()
}

fn g2_bytes(point: &G2Affine) -> Vec<u8> {
    if point.is_zero() {
        return vec![0; 256];
    }
    [
        fp_bytes(&point.x.c0),
        fp_bytes(&point.x.c1),
        fp_bytes(&point.y.c0),
        fp_bytes(&point.y.c1),
    ]
    .concat()
}

fn scalar_hex(value: &Fr) -> String {
    let integer = value.into_bigint().to_bytes_be();
    let mut result = vec![0; 32 - integer.len()];
    result.extend(integer);
    format!("0x{}", hex(&result))
}

fn key_bytes(vk: &VerifyingKey<Bls12_381>) -> Vec<u8> {
    let mut key = g1_bytes(&vk.alpha_g1);
    key.extend(g2_bytes(&vk.beta_g2));
    key.extend(g2_bytes(&vk.gamma_g2));
    key.extend(g2_bytes(&vk.delta_g2));
    for point in &vk.gamma_abc_g1 {
        key.extend(g1_bytes(point));
    }
    key
}

fn proof_bytes(proof: &Proof<Bls12_381>) -> Vec<u8> {
    [
        g1_bytes(&(-proof.a)),
        g2_bytes(&proof.b),
        g1_bytes(&proof.c),
    ]
    .concat()
}

/// Public Groth16 wire material, without assigning any application relation or
/// mutation semantics. Callers must verify their own relation and input layout.
#[derive(Serialize)]
pub struct Eip2537PublicGroth16Proof {
    pub encoding: &'static str,
    pub curve: &'static str,
    pub public_input_count: usize,
    pub vk_eip2537_hex: String,
    pub proof_neg_a_eip2537_hex: String,
    pub public_inputs_be_hex: Vec<String>,
}

impl Eip2537PublicGroth16Proof {
    pub fn from_proof(
        vk: &VerifyingKey<Bls12_381>,
        proof: &Proof<Bls12_381>,
        inputs: &[Fr],
    ) -> Result<Self, &'static str> {
        if vk.gamma_abc_g1.len() != inputs.len() + 1 {
            return Err("verifying key and public-input count differ");
        }
        Ok(Self {
            encoding: "EIP-2537/Fp64BE/Fp2-c0-c1/G1-xy/G2-xy/proof-negA-B-C/v1",
            curve: "BLS12-381",
            public_input_count: inputs.len(),
            vk_eip2537_hex: format!("0x{}", hex(&key_bytes(vk))),
            proof_neg_a_eip2537_hex: format!("0x{}", hex(&proof_bytes(proof))),
            public_inputs_be_hex: inputs.iter().map(scalar_hex).collect(),
        })
    }
}

#[derive(Serialize)]
pub struct Eip2537Groth16Fixture {
    pub encoding: &'static str,
    pub curve: &'static str,
    pub relation: &'static str,
    pub public_input_count: usize,
    pub vk_eip2537_hex: String,
    pub proof_neg_a_eip2537_hex: String,
    pub public_inputs_be_hex: Vec<String>,
    pub changed_batch_root_public_inputs_be_hex: Vec<String>,
    pub private_witness_emitted: bool,
    pub production_setup: bool,
}

impl Eip2537Groth16Fixture {
    pub fn from_state_proof(
        vk: &VerifyingKey<Bls12_381>,
        proof: &Proof<Bls12_381>,
        inputs: &[Fr],
        changed_batch_root_inputs: &[Fr],
    ) -> Self {
        assert_eq!(vk.gamma_abc_g1.len(), inputs.len() + 1);
        assert_eq!(inputs.len(), changed_batch_root_inputs.len());
        Self {
            encoding: "EIP-2537/Fp64BE/Fp2-c0-c1/G1-xy/G2-xy/proof-negA-B-C/v1",
            curve: "BLS12-381",
            relation: "G2-B fixed-B2 three-candidate state transition",
            public_input_count: inputs.len(),
            vk_eip2537_hex: format!("0x{}", hex(&key_bytes(vk))),
            proof_neg_a_eip2537_hex: format!("0x{}", hex(&proof_bytes(proof))),
            public_inputs_be_hex: inputs.iter().map(scalar_hex).collect(),
            changed_batch_root_public_inputs_be_hex: changed_batch_root_inputs
                .iter()
                .map(scalar_hex)
                .collect(),
            private_witness_emitted: false,
            production_setup: false,
        }
    }
}

#[derive(Serialize)]
pub struct Eip2537DisclosureFixture {
    pub encoding: &'static str,
    pub curve: &'static str,
    pub relation: &'static str,
    pub public_input_count: usize,
    pub vk_eip2537_hex: String,
    pub proof_neg_a_eip2537_hex: String,
    pub public_inputs_be_hex: Vec<String>,
    pub changed_selector_public_inputs_be_hex: Vec<String>,
    pub supported_subsets: [[u32; 3]; 2],
    pub setup_subset: [u32; 3],
    pub proof_subset: [u32; 3],
    pub selector_public_input_index: usize,
    pub private_witness_emitted: bool,
    pub production_setup: bool,
}

impl Eip2537DisclosureFixture {
    pub fn from_two_subset_proof(
        vk: &VerifyingKey<Bls12_381>,
        proof: &Proof<Bls12_381>,
        inputs: &[Fr],
        changed_selector_inputs: &[Fr],
    ) -> Self {
        assert_eq!(inputs.len(), 60);
        assert_eq!(vk.gamma_abc_g1.len(), inputs.len() + 1);
        assert_eq!(inputs.len(), changed_selector_inputs.len());
        let changed_indices = inputs
            .iter()
            .zip(changed_selector_inputs)
            .enumerate()
            .filter_map(|(index, (valid, changed))| (valid != changed).then_some(index))
            .collect::<Vec<_>>();
        assert_eq!(changed_indices, [51]);
        Self {
            encoding: "EIP-2537/Fp64BE/Fp2-c0-c1/G1-xy/G2-xy/proof-negA-B-C/v1",
            curve: "BLS12-381",
            relation: "G4-B two-subset complete P3 disclosure",
            public_input_count: inputs.len(),
            vk_eip2537_hex: format!("0x{}", hex(&key_bytes(vk))),
            proof_neg_a_eip2537_hex: format!("0x{}", hex(&proof_bytes(proof))),
            public_inputs_be_hex: inputs.iter().map(scalar_hex).collect(),
            changed_selector_public_inputs_be_hex: changed_selector_inputs
                .iter()
                .map(scalar_hex)
                .collect(),
            supported_subsets: [[1, 2, 3], [1, 2, 4]],
            setup_subset: [1, 2, 3],
            proof_subset: [1, 2, 4],
            selector_public_input_index: 51,
            private_witness_emitted: false,
            production_setup: false,
        }
    }
}
