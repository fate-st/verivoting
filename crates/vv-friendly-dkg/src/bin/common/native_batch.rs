//! Conversion/measurement helpers for new native batch runners. No group result is substituted into R1CS.
use ark_bls12_381::{Bls12_381, Fr, G1Affine, G2Affine};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{Proof, ProvingKey};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_serialize::CanonicalSerialize;
use bellperson::groth16::{Proof as BellProof, VerifyingKey as BellVk};
use blstrs::{Bls12, G1Affine as BellG1, G2Affine as BellG2, Scalar as BellScalar};
use serde_json::{json, Value};
use std::{error::Error, fs, io::Write, path::Path, process::Command, time::Instant};
use vv_friendly_crypto::ballot_identity_gadget::BallotCvoteCircuit;
use vv_friendly_crypto::state_gadget::StateBallotRecord;
pub(super) type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub(super) const TRANSCRIPT_DOMAIN: &[u8] = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1";

pub(super) fn metadata(program: &str, args: &[&str]) -> Result<String> {
    let result = Command::new(program).args(args).output()?;
    if !result.status.success() {
        return Err(format!("{program} metadata failed").into());
    }
    Ok(String::from_utf8(result.stdout)?.trim().to_owned())
}

pub(super) fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    #[cfg(target_os = "macos")]
    let divisor = 1024.0 * 1024.0;
    #[cfg(not(target_os = "macos"))]
    let divisor = 1024.0;
    usage.ru_maxrss as f64 / divisor
}

pub(super) fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn convert_g1(point: &G1Affine) -> Result<BellG1> {
    let bytes: [u8; 48] = compressed(point)?
        .try_into()
        .map_err(|_| "invalid G1 length")?;
    Option::<BellG1>::from(BellG1::from_compressed(&bytes))
        .ok_or_else(|| "invalid converted G1".into())
}

pub(super) fn convert_g2(point: &G2Affine) -> Result<BellG2> {
    let bytes: [u8; 96] = compressed(point)?
        .try_into()
        .map_err(|_| "invalid G2 length")?;
    Option::<BellG2>::from(BellG2::from_compressed(&bytes))
        .ok_or_else(|| "invalid converted G2".into())
}

pub(super) fn convert_inputs(inputs: &[Fr]) -> Result<Vec<BellScalar>> {
    inputs
        .iter()
        .map(|value| {
            let bytes = value.into_bigint().to_bytes_le();
            let mut fixed = [0u8; 32];
            if bytes.len() > fixed.len() {
                return Err("invalid scalar length".into());
            }
            fixed[..bytes.len()].copy_from_slice(&bytes);
            Option::<BellScalar>::from(BellScalar::from_bytes_le(&fixed))
                .ok_or_else(|| "noncanonical converted scalar".into())
        })
        .collect()
}

pub(super) fn convert_proof(proof: &Proof<Bls12_381>) -> Result<BellProof<Bls12>> {
    Ok(BellProof {
        a: convert_g1(&proof.a)?,
        b: convert_g2(&proof.b)?,
        c: convert_g1(&proof.c)?,
    })
}

pub(super) fn convert_vk(pk: &ProvingKey<Bls12_381>) -> Result<BellVk<Bls12>> {
    Ok(BellVk {
        alpha_g1: convert_g1(&pk.vk.alpha_g1)?,
        beta_g1: convert_g1(&pk.beta_g1)?,
        beta_g2: convert_g2(&pk.vk.beta_g2)?,
        gamma_g2: convert_g2(&pk.vk.gamma_g2)?,
        delta_g1: convert_g1(&pk.delta_g1)?,
        delta_g2: convert_g2(&pk.vk.delta_g2)?,
        ic: pk
            .vk
            .gamma_abc_g1
            .iter()
            .map(convert_g1)
            .collect::<Result<Vec<_>>>()?,
    })
}

pub(super) fn transcript(statements: &[Vec<BellScalar>]) -> Vec<u8> {
    let mut bytes = TRANSCRIPT_DOMAIN.to_vec();
    bytes.extend_from_slice(&(statements.len() as u32).to_be_bytes());
    for statement in statements {
        bytes.extend_from_slice(&(statement.len() as u32).to_be_bytes());
        for scalar in statement {
            bytes.extend_from_slice(&scalar.to_bytes_le());
        }
    }
    bytes
}

pub(super) fn public_record<const DEPTH: usize>(
    circuit: &BallotCvoteCircuit<DEPTH>,
) -> StateBallotRecord {
    StateBallotRecord {
        election_id: circuit.election_id,
        final_context: circuit.final_context,
        region: circuit.region,
        registration_root: circuit.registration_root,
        nullifier: circuit.nullifier,
        key_set_hash: circuit.key_set_hash,
        public_key: circuit.public_key,
        ciphertexts: circuit.ciphertexts.clone(),
    }
}

pub(super) fn measure_relation<const DEPTH: usize>(
    circuit: &BallotCvoteCircuit<DEPTH>,
    inputs: &[Fr],
) -> Result<Value> {
    let start = Instant::now();
    let cs = ConstraintSystem::<Fr>::new_ref();
    circuit.clone().generate_constraints(cs.clone())?;
    cs.finalize();
    let satisfied = cs.is_satisfied()?;
    let exact_inputs = cs
        .borrow()
        .ok_or("missing constraint system")?
        .instance_assignment[1..]
        == *inputs;
    if !satisfied || !exact_inputs {
        return Err("ballot R1CS or exact public input packing failed".into());
    }
    Ok(
        json!({"satisfied": satisfied, "constraints": cs.num_constraints(),
        "public_inputs": cs.num_instance_variables() - 1, "witness_variables": cs.num_witness_variables(),
        "exact_public_input_packing": exact_inputs, "synthesis_and_check_ms": start.elapsed().as_secs_f64() * 1000.0}),
    )
}

pub(super) fn write_new(path: &Path, data: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(data)?)?;
    file.write_all(b"\n")?;
    Ok(())
}
