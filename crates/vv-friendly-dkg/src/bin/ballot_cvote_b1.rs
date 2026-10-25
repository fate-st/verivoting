use ark_bls12_381::{Bls12_381, Fr as ArkScalar, G1Affine as ArkG1Affine, G2Affine as ArkG2Affine};
use ark_ec::Group;
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{prepare_verifying_key, Groth16, ProvingKey as ArkProvingKey};
use ark_serialize::CanonicalSerialize;
use bellperson::groth16::{
    aggregate::{aggregate_proofs, setup_fake_srs, verify_aggregate_proof, AggregateVersion},
    prepare_verifying_key as prepare_bellperson_verifying_key,
    verify_proof as verify_bellperson_proof, Proof as BellpersonProof,
    VerifyingKey as BellpersonVerifyingKey,
};
use blstrs::{
    Bls12 as BellpersonBls12, G1Affine as BellpersonG1Affine, G2Affine as BellpersonG2Affine,
    Scalar as BellpersonScalar,
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, fs, path::PathBuf, process::Command, time::Instant};
use vv_friendly_crypto::ballot_gadget::BALLOT_CORE_CANDIDATE_COUNT;
use vv_friendly_crypto::ballot_identity_gadget::{
    ballot_address, ballot_cvote_public_inputs, ballot_nullifier, ballot_region,
    check_ballot_cvote_b1, registration_leaf, registration_node, BallotCvoteCircuit,
    BALLOT_PRF_SUITE_ID, BALLOT_REGION_COUNT, BALLOT_REGISTRATION_DEPTH,
};
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs as state_ballot_public_inputs, StateBallotRecord,
};
use vv_friendly_crypto::{
    encode_point, generator, random_nonzero_scalar, Ciphertext, Point, Scalar, SUITE_ID,
};
use vv_friendly_dkg::registration::{
    verify_close, RegistrationAuthority, RegistrationEntry, RegistrationPolicy,
};
use vv_types::{derive_final_context, derive_prekey_context, hex, sha256, CanonicalEncoder};

const DEFAULT_SEED: u64 = 20_261_028;
const LAYOUT_VERSION: &str = "VV/JUBJUB/G1-B1/C-VOTE-LAYOUT/v1";
const RULE_ID: &str = "highest-weight-smallest-id-on-tie";
const SETUP_SEED: u64 = 0x4731_4232_5345_5455;
const PROOF_SEED: u64 = 0x4731_4232_5052_4F56;
const SECOND_PROOF_SEED: u64 = 0x4731_4330_5052_4F32;
const SNARKPACK_SRS_SEED: u64 = 0x4731_4330_5352_5302;
const SNARKPACK_VERIFY_SEED: u64 = 0x4731_4330_5645_5202;
const SNARKPACK_TRANSCRIPT_DOMAIN: &[u8] = b"VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1";

#[derive(Serialize)]
struct PublicCiphertext {
    candidate_id: usize,
    c0_hex: String,
    c1_hex: String,
}

#[derive(Serialize)]
struct SnarkpackPublicRecord {
    voter_index: usize,
    candidate_choice: usize,
    election_id_hex: String,
    final_context_hex: String,
    region: u64,
    registration_root_hex: String,
    nullifier_hex: String,
    key_set_hash_hex: String,
    public_key_hex: String,
    ciphertexts: Vec<PublicCiphertext>,
}

#[derive(Serialize)]
struct PublicLayoutFixture {
    layout_version: &'static str,
    protocol_version: String,
    suite_id: &'static str,
    ballot_prf_suite: &'static str,
    seed: u64,
    public_inputs_in_order: Vec<&'static str>,
    private_witnesses: Vec<&'static str>,
    election_id_hex: String,
    final_context_hex: String,
    region: u64,
    registration_root_hex: String,
    nullifier_hex: String,
    key_set_hash_hex: String,
    public_key_hex: String,
    ciphertexts: Vec<PublicCiphertext>,
    registered_weight_bound_constant: u64,
    registration_leaf_count: usize,
    registration_depth: usize,
    private_voter_secret_emitted: bool,
    private_address_emitted: bool,
    private_weight_emitted: bool,
    private_commitment_emitted: bool,
    private_merkle_path_emitted: bool,
    private_choice_emitted: bool,
    private_randomness_emitted: bool,
}

#[derive(Serialize)]
struct BallotCvoteResult {
    gate: &'static str,
    status: &'static str,
    classification: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    prototype_manifest: String,
    protocol_version: String,
    manifest_capability: String,
    manifest_matches_g1b1: bool,
    layout_version: &'static str,
    layout_file: String,
    seed: u64,
    candidate_count: usize,
    region_count: u64,
    registration_leaf_count: usize,
    registration_depth: usize,
    native_registration_root_matches_certificate: bool,
    native_relations_match: bool,
    five_conditions_in_r1cs: usize,
    prf_evaluations_in_r1cs: usize,
    region_hashes_in_r1cs: usize,
    merkle_hashes_in_r1cs: usize,
    one_hot_relations_in_r1cs: usize,
    weight_range_relations_in_r1cs: usize,
    weight_commitment_opening_equations_in_r1cs: usize,
    elgamal_group_equations_in_r1cs: usize,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    peak_rss_mib: f64,
    valid_relation_satisfied: bool,
    changed_context_rejected: bool,
    full_c_vote_r1cs: bool,
    groth16_run: bool,
    snarkpack_run: bool,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct BallotGroth16Result {
    gate: &'static str,
    status: &'static str,
    classification: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    prototype_manifest: String,
    protocol_version: String,
    manifest_capability: String,
    manifest_matches_g1b2: bool,
    r1cs_baseline_file: String,
    layout_version: &'static str,
    layout_file: String,
    constraints: usize,
    public_inputs: usize,
    witness_variables: usize,
    witness_ms: f64,
    setup_seed: u64,
    proof_seed: u64,
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    changed_context_verify_ms: f64,
    peak_rss_after_setup_mib: f64,
    peak_rss_after_prove_mib: f64,
    peak_rss_after_verify_mib: f64,
    proving_key_compressed_bytes: usize,
    verifying_key_compressed_bytes: usize,
    proof_bytes: usize,
    valid_proof_verified: bool,
    changed_public_context_rejected: bool,
    proving_key_persisted: bool,
    private_witness_emitted: bool,
    full_c_vote: bool,
    groth16_run: bool,
    snarkpack_run: bool,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
struct SnarkpackResult {
    gate: &'static str,
    status: &'static str,
    classification: &'static str,
    git_commit: String,
    git_dirty: String,
    build_profile: &'static str,
    rustc_version: String,
    os: &'static str,
    arch: &'static str,
    cpu: String,
    prototype_manifest: String,
    protocol_version: String,
    manifest_capability: String,
    manifest_matches_g1c0: bool,
    manifest_matches_g1c1: bool,
    implementation: &'static str,
    implementation_version: &'static str,
    curve: &'static str,
    source_proof_system: &'static str,
    aggregate_version: &'static str,
    batch_size: usize,
    public_inputs_per_proof: usize,
    transcript_domain: &'static str,
    transcript_bytes: usize,
    groth16_setup_seed: u64,
    groth16_setup_ms: f64,
    first_proof_seed: u64,
    first_prove_ms: f64,
    first_verify_ms: f64,
    second_proof_seed: u64,
    aggregation_srs_seed: u64,
    aggregation_srs_kind: &'static str,
    second_prove_ms: f64,
    conversion_ms: f64,
    aggregation_srs_ms: f64,
    aggregation_ms: f64,
    aggregate_verify_ms: f64,
    changed_public_context_verify_ms: f64,
    individual_verify_ms: f64,
    individual_proof_bytes: usize,
    total_individual_proof_bytes: usize,
    aggregate_proof_bytes: usize,
    peak_rss_mib: f64,
    source_proofs_distinct: bool,
    source_registered_voter_indices: Vec<usize>,
    source_candidate_choices: Vec<usize>,
    source_public_records: Vec<SnarkpackPublicRecord>,
    state_record_serialization_matches_snarkpack_inputs: bool,
    registered_voter_statements_distinct: bool,
    nullifiers_distinct: bool,
    ciphertexts_distinct: bool,
    same_election_context_registration_root: bool,
    same_region: bool,
    converted_individual_proofs_verified: bool,
    aggregate_proof_verified: bool,
    changed_public_context_rejected: bool,
    changed_second_nullifier_verify_ms: Option<f64>,
    changed_second_nullifier_rejected: Option<bool>,
    aggregation_srs_persisted: bool,
    source_proofs_persisted: bool,
    full_c_vote: bool,
    real_snarkpack_code_path: bool,
    production_setup: bool,
    limitations: Vec<&'static str>,
}

struct Groth16Measurement {
    setup_ms: f64,
    prove_ms: f64,
    verify_ms: f64,
    changed_context_verify_ms: f64,
    peak_rss_after_setup_mib: f64,
    peak_rss_after_prove_mib: f64,
    peak_rss_after_verify_mib: f64,
    proving_key_compressed_bytes: usize,
    verifying_key_compressed_bytes: usize,
    proof_bytes: usize,
    valid_proof_verified: bool,
    changed_public_context_rejected: bool,
}

struct MembershipPath {
    siblings: Vec<[u8; 32]>,
    directions: Vec<bool>,
    root: [u8; 32],
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn ark_g1_to_bellperson(point: &ArkG1Affine) -> Result<BellpersonG1Affine, Box<dyn Error>> {
    let mut encoded = Vec::new();
    point.serialize_compressed(&mut encoded)?;
    let encoded: [u8; 48] = encoded
        .try_into()
        .map_err(|_| "arkworks G1 encoding is not 48 bytes")?;
    Option::<BellpersonG1Affine>::from(BellpersonG1Affine::from_compressed(&encoded))
        .ok_or_else(|| "arkworks G1 encoding is not accepted by blstrs".into())
}

fn ark_g2_to_bellperson(point: &ArkG2Affine) -> Result<BellpersonG2Affine, Box<dyn Error>> {
    let mut encoded = Vec::new();
    point.serialize_compressed(&mut encoded)?;
    let encoded: [u8; 96] = encoded
        .try_into()
        .map_err(|_| "arkworks G2 encoding is not 96 bytes")?;
    Option::<BellpersonG2Affine>::from(BellpersonG2Affine::from_compressed(&encoded))
        .ok_or_else(|| "arkworks G2 encoding is not accepted by blstrs".into())
}

fn ark_scalar_to_bellperson(value: &ArkScalar) -> Result<BellpersonScalar, Box<dyn Error>> {
    let bytes = value.into_bigint().to_bytes_le();
    let mut canonical = [0u8; 32];
    if bytes.len() > canonical.len() {
        return Err("arkworks scalar encoding exceeds 32 bytes".into());
    }
    canonical[..bytes.len()].copy_from_slice(&bytes);
    Option::<BellpersonScalar>::from(BellpersonScalar::from_bytes_le(&canonical))
        .ok_or_else(|| "arkworks scalar encoding is not accepted by blstrs".into())
}

fn ark_proof_to_bellperson(
    proof: &ark_groth16::Proof<Bls12_381>,
) -> Result<BellpersonProof<BellpersonBls12>, Box<dyn Error>> {
    Ok(BellpersonProof {
        a: ark_g1_to_bellperson(&proof.a)?,
        b: ark_g2_to_bellperson(&proof.b)?,
        c: ark_g1_to_bellperson(&proof.c)?,
    })
}

fn ark_verifying_key_to_bellperson(
    parameters: &ArkProvingKey<Bls12_381>,
) -> Result<BellpersonVerifyingKey<BellpersonBls12>, Box<dyn Error>> {
    Ok(BellpersonVerifyingKey {
        alpha_g1: ark_g1_to_bellperson(&parameters.vk.alpha_g1)?,
        beta_g1: ark_g1_to_bellperson(&parameters.beta_g1)?,
        beta_g2: ark_g2_to_bellperson(&parameters.vk.beta_g2)?,
        gamma_g2: ark_g2_to_bellperson(&parameters.vk.gamma_g2)?,
        delta_g1: ark_g1_to_bellperson(&parameters.delta_g1)?,
        delta_g2: ark_g2_to_bellperson(&parameters.vk.delta_g2)?,
        ic: parameters
            .vk
            .gamma_abc_g1
            .iter()
            .map(ark_g1_to_bellperson)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn ark_public_inputs_to_bellperson(
    inputs: &[ArkScalar],
) -> Result<Vec<BellpersonScalar>, Box<dyn Error>> {
    inputs.iter().map(ark_scalar_to_bellperson).collect()
}

fn snarkpack_transcript(statements: &[Vec<BellpersonScalar>]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(SNARKPACK_TRANSCRIPT_DOMAIN);
    encoded.extend_from_slice(&(statements.len() as u32).to_be_bytes());
    for statement in statements {
        encoded.extend_from_slice(&(statement.len() as u32).to_be_bytes());
        for scalar in statement {
            encoded.extend_from_slice(&scalar.to_bytes_le());
        }
    }
    encoded
}

#[cfg(target_os = "macos")]
fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    usage.ru_maxrss as f64 / (1024.0 * 1024.0)
}

#[cfg(not(target_os = "macos"))]
fn peak_rss_mib() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    usage.ru_maxrss as f64 / 1024.0
}

fn key_set_hash(public_key: &Point) -> [u8; 32] {
    let mut encoded = CanonicalEncoder::new(b"VV/JUBJUB/G1-B1/KEY-SET/v1");
    encoded.fixed(&encode_point(public_key));
    sha256(&encoded.finish())
}

fn membership_path(
    entries: &[RegistrationEntry],
    target_address: [u8; 32],
) -> Result<MembershipPath, Box<dyn Error>> {
    let mut sorted = entries.to_vec();
    sorted.sort_by_key(|entry| entry.credential_id);
    let mut index = sorted
        .iter()
        .position(|entry| entry.credential_id == target_address)
        .ok_or("target address is absent from registration entries")?;
    let mut level = sorted
        .iter()
        .map(|entry| registration_leaf(&entry.credential_id, &entry.commitment()))
        .collect::<Vec<_>>();
    let mut siblings = Vec::new();
    let mut directions = Vec::new();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().ok_or("empty Merkle level")?);
        }
        let current_is_right = index % 2 == 1;
        let sibling_index = if current_is_right {
            index - 1
        } else {
            index + 1
        };
        siblings.push(level[sibling_index]);
        directions.push(current_is_right);
        level = level
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| registration_node(&pair[0], &pair[1]))
            .collect();
        index /= 2;
    }
    Ok(MembershipPath {
        siblings,
        directions,
        root: level[0],
    })
}

fn snarkpack_public_record(
    circuit: &BallotCvoteCircuit,
    voter_index: usize,
    candidate_choice: usize,
) -> SnarkpackPublicRecord {
    SnarkpackPublicRecord {
        voter_index,
        candidate_choice,
        election_id_hex: hex(&circuit.election_id),
        final_context_hex: hex(&circuit.final_context),
        region: circuit.region,
        registration_root_hex: hex(&circuit.registration_root),
        nullifier_hex: hex(&circuit.nullifier),
        key_set_hash_hex: hex(&circuit.key_set_hash),
        public_key_hex: hex(&encode_point(&circuit.public_key)),
        ciphertexts: circuit
            .ciphertexts
            .iter()
            .enumerate()
            .map(|(index, ciphertext)| PublicCiphertext {
                candidate_id: index + 1,
                c0_hex: hex(&encode_point(&ciphertext.c0)),
                c1_hex: hex(&encode_point(&ciphertext.c1)),
            })
            .collect(),
    }
}

fn state_ballot_record(circuit: &BallotCvoteCircuit) -> StateBallotRecord {
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-PROTOTYPE.json");
    let mut output_path = PathBuf::from("experiments/g1_b1_raw/full_c_vote_r1cs.json");
    let mut layout_path = PathBuf::from("fixtures/jubjub_ballot_cvote_b1_public_layout_v1.json");
    let mut groth16_output_path = None;
    let mut snarkpack_output_path = None;
    let mut snarkpack_distinct_output_path = None;
    let mut seed = DEFAULT_SEED;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--manifest" => manifest_path = PathBuf::from(value),
            "--output" => output_path = PathBuf::from(value),
            "--layout-output" => layout_path = PathBuf::from(value),
            "--groth16-output" => groth16_output_path = Some(PathBuf::from(value)),
            "--snarkpack-output" => snarkpack_output_path = Some(PathBuf::from(value)),
            "--snarkpack-distinct-output" => {
                snarkpack_distinct_output_path = Some(PathBuf::from(value))
            }
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    if snarkpack_output_path.is_some() && snarkpack_distinct_output_path.is_some() {
        return Err("choose either G1-C0 or G1-C1 SnarkPack output, not both".into());
    }

    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let protocol_version = manifest["protocol_version"]
        .as_str()
        .ok_or("prototype manifest lacks protocol_version")?
        .to_owned();
    let manifest_capability = manifest["capabilities"]["g1b1_full_c_vote_r1cs"]
        .as_str()
        .ok_or("prototype manifest lacks G1-B1 capability")?
        .to_owned();
    let manifest_g1b2_capability = manifest["capabilities"]["g1b2_ballot_groth16"]
        .as_str()
        .ok_or("prototype manifest lacks G1-B2 capability")?
        .to_owned();
    let manifest_g1c0_capability = manifest["capabilities"]["g1c0_snarkpack_compatibility"]
        .as_str()
        .ok_or("prototype manifest lacks G1-C0 capability")?
        .to_owned();
    let manifest_g1c1_capability = manifest["capabilities"]["g1c1_distinct_voter_snarkpack"]
        .as_str()
        .ok_or("prototype manifest lacks G1-C1 capability")?
        .to_owned();
    let manifest_matches_g1b1 = (protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1B1-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1B2-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C0-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C1-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2A-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2B-"))
        && manifest_capability
            == "READY_R1CS_FIVE_CONDITION_HMAC_SHA256_DEPTH2_FOUR_REGION_NO_GROTH16"
        && manifest["ballot_prf_suite"]
            .as_str()
            .is_some_and(|suite| suite.starts_with(BALLOT_PRF_SUITE_ID));
    if !manifest_matches_g1b1 {
        return Err("prototype manifest does not describe G1-B1".into());
    }
    let manifest_matches_g1b2 = (protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1B2-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C0-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C1-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2A-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2B-"))
        && manifest_g1b2_capability == "READY_REAL_GROTH16_FIVE_CONDITION_FIXED_DEPTH2_FOUR_REGION";
    if (groth16_output_path.is_some()
        || snarkpack_output_path.is_some()
        || snarkpack_distinct_output_path.is_some())
        && !manifest_matches_g1b2
    {
        return Err("prototype manifest does not describe G1-B2".into());
    }
    let manifest_matches_g1c0 = (protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C0-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C1-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2A-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2B-"))
        && manifest_g1c0_capability == "READY_REAL_SNARKPACK_V2_MIN_BATCH_2_NONPRODUCTION_SRS";
    if snarkpack_output_path.is_some() && !manifest_matches_g1c0 {
        return Err("prototype manifest does not describe G1-C0".into());
    }
    let manifest_matches_g1c1 = (protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G1C1-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2A-")
        || protocol_version.starts_with("VERIVOTING-JUBJUB-PROTOTYPE-G2B-"))
        && manifest_g1c1_capability
            == "READY_REAL_SNARKPACK_V2_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS";
    if snarkpack_distinct_output_path.is_some() && !manifest_matches_g1c1 {
        return Err("prototype manifest does not describe G1-C1".into());
    }

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let public_key_secret = random_nonzero_scalar(&mut rng);
    let public_key = generator().mul_bigint(public_key_secret.into_bigint());
    let election_id = sha256(b"VV/JUBJUB/G1-B1/FOUR-VOTER");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy = RegistrationPolicy::new(authority.public_key(), 10, RULE_ID)?;
    let policy_pre = policy.digest(election_id);
    let prekey_context = derive_prekey_context(election_id, policy_pre, sha256(&manifest_bytes));
    let key_set_hash = key_set_hash(&public_key);
    let final_context = derive_final_context(prekey_context, key_set_hash);

    let weights = [3u64, 2, 1, 4];
    let mut voter_secrets = Vec::new();
    let mut addresses = Vec::new();
    let mut entries = Vec::new();
    for weight in weights {
        let mut voter_secret = [0u8; 32];
        rng.fill_bytes(&mut voter_secret);
        let address = ballot_address(&voter_secret, &election_id, &final_context, &key_set_hash);
        entries.push(RegistrationEntry::new(address, weight, &mut rng)?);
        voter_secrets.push(voter_secret);
        addresses.push(address);
    }
    let certificate = authority.close(
        entries.clone(),
        election_id,
        final_context,
        policy_pre,
        &policy,
        &mut rng,
    )?;
    let registration = verify_close(
        &certificate,
        &policy,
        election_id,
        final_context,
        policy_pre,
    )?;

    let source_voter_indices = if snarkpack_distinct_output_path.is_some() {
        let regions = addresses
            .iter()
            .map(|address| ballot_region(&final_context, address))
            .collect::<Vec<_>>();
        let mut pair = None;
        'outer: for first in 0..regions.len() {
            for second in first + 1..regions.len() {
                if regions[first] == regions[second] {
                    pair = Some([first, second]);
                    break 'outer;
                }
            }
        }
        pair.ok_or("G1-C1 fixture has no two distinct voters in one region")?
    } else {
        [0, 0]
    };
    let target_index = source_voter_indices[0];
    let target_entry = &entries[target_index];
    let target_secret = voter_secrets[target_index];
    let target_address = addresses[target_index];
    let membership = membership_path(&entries, target_address)?;
    let merkle_siblings = membership.siblings;
    let merkle_directions = membership.directions;
    let computed_root = membership.root;
    let registration_root_matches = computed_root == registration.certificate().close.root_close;
    if !registration_root_matches
        || merkle_siblings.len() != BALLOT_REGISTRATION_DEPTH
        || entries.len() != 1usize << BALLOT_REGISTRATION_DEPTH
    {
        return Err("G1-B1 registration fixture does not match the fixed tree".into());
    }

    let nullifier = ballot_nullifier(&target_secret, &election_id, &final_context, &key_set_hash);
    let region = ballot_region(&final_context, &target_address);
    let choices = vec![true, false, false];
    let randomnesses = (0..BALLOT_CORE_CANDIDATE_COUNT)
        .map(|_| random_nonzero_scalar(&mut rng))
        .collect::<Vec<_>>();
    let ciphertexts = randomnesses
        .iter()
        .zip(&choices)
        .map(|(randomness, choice)| {
            let message = if *choice { target_entry.weight } else { 0 };
            Ciphertext {
                c0: generator().mul_bigint(randomness.into_bigint()),
                c1: public_key.mul_bigint(randomness.into_bigint())
                    + generator().mul_bigint(Scalar::from(message).into_bigint()),
            }
        })
        .collect::<Vec<_>>();

    let native_relations_match = registration_root_matches
        && ballot_address(&target_secret, &election_id, &final_context, &key_set_hash)
            == target_address
        && ballot_nullifier(&target_secret, &election_id, &final_context, &key_set_hash)
            == nullifier
        && ballot_region(&final_context, &target_address) == region
        && choices.iter().filter(|choice| **choice).count() == 1;
    if !native_relations_match {
        return Err("native G1-B1 fixture is inconsistent".into());
    }

    let circuit = BallotCvoteCircuit {
        election_id,
        final_context,
        region,
        registration_root: computed_root,
        nullifier,
        key_set_hash,
        public_key,
        ciphertexts: ciphertexts.clone(),
        registered_weight_bound: policy.w_max,
        voter_secret: target_secret,
        address: target_address,
        weight: target_entry.weight,
        weight_opening: target_entry.opening(),
        weight_commitment: target_entry.commitment(),
        merkle_siblings,
        merkle_directions,
        choices,
        randomnesses,
    };
    let started = Instant::now();
    let valid = check_ballot_cvote_b1(&circuit)?;
    let witness_ms = started.elapsed().as_secs_f64() * 1000.0;
    let r1cs_peak_rss_mib = peak_rss_mib();

    let mut changed_context = circuit.clone();
    changed_context.final_context[0] ^= 1;
    let changed_context_rejected = !check_ballot_cvote_b1(&changed_context)?.satisfied;
    if !valid.satisfied || !changed_context_rejected {
        return Err("G1-B1 R1CS acceptance boundary failed".into());
    }

    let groth16_measurement = if groth16_output_path.is_some()
        || snarkpack_output_path.is_some()
        || snarkpack_distinct_output_path.is_some()
    {
        let public_inputs =
            ballot_cvote_public_inputs(&circuit).ok_or("failed to pack G1-B2 public inputs")?;
        if public_inputs.len() != valid.public_inputs {
            return Err("G1-B2 public input packing differs from G1-B1 R1CS".into());
        }

        let mut setup_rng = ChaCha20Rng::seed_from_u64(SETUP_SEED);
        let setup_started = Instant::now();
        let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            circuit.clone(),
            &mut setup_rng,
        )?;
        let setup_ms = setup_started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_setup_mib = peak_rss_mib();

        let mut proof_rng = ChaCha20Rng::seed_from_u64(PROOF_SEED);
        let prove_started = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            circuit.clone(),
            &parameters,
            &mut proof_rng,
        )?;
        let prove_ms = prove_started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_prove_mib = peak_rss_mib();

        let prepared = prepare_verifying_key(&parameters.vk);
        let verify_started = Instant::now();
        let valid_proof_verified =
            Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &public_inputs)?;
        let verify_ms = verify_started.elapsed().as_secs_f64() * 1000.0;

        let changed_public_inputs = ballot_cvote_public_inputs(&changed_context)
            .ok_or("failed to pack changed G1-B2 public inputs")?;
        let changed_verify_started = Instant::now();
        let changed_public_context_rejected =
            !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &changed_public_inputs)?;
        let changed_context_verify_ms = changed_verify_started.elapsed().as_secs_f64() * 1000.0;
        let peak_rss_after_verify_mib = peak_rss_mib();
        if !valid_proof_verified || !changed_public_context_rejected {
            return Err("G1-B2 Groth16 acceptance boundary failed".into());
        }

        let mut proof_encoding = Vec::new();
        proof.serialize_compressed(&mut proof_encoding)?;

        if let Some(snarkpack_output_path) = snarkpack_output_path
            .as_ref()
            .or(snarkpack_distinct_output_path.as_ref())
        {
            let distinct_voter_gate = snarkpack_distinct_output_path.is_some();
            let second_circuit = if distinct_voter_gate {
                let second_index = source_voter_indices[1];
                let second_entry = &entries[second_index];
                let second_secret = voter_secrets[second_index];
                let second_address = addresses[second_index];
                let second_membership = membership_path(&entries, second_address)?;
                if second_membership.root != computed_root {
                    return Err("G1-C1 second voter has a different registration root".into());
                }
                let second_choices = vec![false, true, false];
                let second_randomnesses = (0..BALLOT_CORE_CANDIDATE_COUNT)
                    .map(|_| random_nonzero_scalar(&mut rng))
                    .collect::<Vec<_>>();
                let second_ciphertexts = second_randomnesses
                    .iter()
                    .zip(&second_choices)
                    .map(|(randomness, choice)| {
                        let message = if *choice { second_entry.weight } else { 0 };
                        Ciphertext {
                            c0: generator().mul_bigint(randomness.into_bigint()),
                            c1: public_key.mul_bigint(randomness.into_bigint())
                                + generator().mul_bigint(Scalar::from(message).into_bigint()),
                        }
                    })
                    .collect::<Vec<_>>();
                BallotCvoteCircuit {
                    election_id,
                    final_context,
                    region: ballot_region(&final_context, &second_address),
                    registration_root: second_membership.root,
                    nullifier: ballot_nullifier(
                        &second_secret,
                        &election_id,
                        &final_context,
                        &key_set_hash,
                    ),
                    key_set_hash,
                    public_key,
                    ciphertexts: second_ciphertexts,
                    registered_weight_bound: policy.w_max,
                    voter_secret: second_secret,
                    address: second_address,
                    weight: second_entry.weight,
                    weight_opening: second_entry.opening(),
                    weight_commitment: second_entry.commitment(),
                    merkle_siblings: second_membership.siblings,
                    merkle_directions: second_membership.directions,
                    choices: second_choices,
                    randomnesses: second_randomnesses,
                }
            } else {
                circuit.clone()
            };
            if !check_ballot_cvote_b1(&second_circuit)?.satisfied {
                return Err("second SnarkPack source relation is unsatisfied".into());
            }
            let second_public_inputs = ballot_cvote_public_inputs(&second_circuit)
                .ok_or("failed to pack second SnarkPack public inputs")?;
            let mut second_proof_rng = ChaCha20Rng::seed_from_u64(SECOND_PROOF_SEED);
            let second_prove_started = Instant::now();
            let second_proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
                second_circuit.clone(),
                &parameters,
                &mut second_proof_rng,
            )?;
            let second_prove_ms = second_prove_started.elapsed().as_secs_f64() * 1000.0;
            if !Groth16::<Bls12_381>::verify_proof(&prepared, &second_proof, &second_public_inputs)?
            {
                return Err("second SnarkPack source proof did not verify in arkworks".into());
            }
            let mut second_proof_encoding = Vec::new();
            second_proof.serialize_compressed(&mut second_proof_encoding)?;
            let source_proofs_distinct = proof_encoding != second_proof_encoding;
            if !source_proofs_distinct {
                return Err("SnarkPack source proofs are unexpectedly identical".into());
            }

            let conversion_started = Instant::now();
            let bellperson_vk = ark_verifying_key_to_bellperson(&parameters)?;
            let bellperson_pvk = prepare_bellperson_verifying_key(&bellperson_vk);
            let bellperson_proofs = vec![
                ark_proof_to_bellperson(&proof)?,
                ark_proof_to_bellperson(&second_proof)?,
            ];
            let bellperson_public_inputs = ark_public_inputs_to_bellperson(&public_inputs)?;
            let second_bellperson_public_inputs =
                ark_public_inputs_to_bellperson(&second_public_inputs)?;
            let statements = vec![
                bellperson_public_inputs.clone(),
                second_bellperson_public_inputs.clone(),
            ];
            let conversion_ms = conversion_started.elapsed().as_secs_f64() * 1000.0;

            let individual_verify_started = Instant::now();
            let converted_individual_proofs_verified = bellperson_proofs
                .iter()
                .zip(&statements)
                .map(|(converted_proof, statement)| {
                    verify_bellperson_proof(&bellperson_pvk, converted_proof, statement)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .all(|verified| verified);
            let individual_verify_ms = individual_verify_started.elapsed().as_secs_f64() * 1000.0;
            if !converted_individual_proofs_verified {
                return Err("arkworks-to-bellperson proof conversion failed verification".into());
            }

            let mut srs_rng = ChaCha20Rng::seed_from_u64(SNARKPACK_SRS_SEED);
            let aggregation_srs_started = Instant::now();
            let generic_srs = setup_fake_srs::<BellpersonBls12, _>(&mut srs_rng, 2);
            let (aggregation_prover_srs, aggregation_verifier_srs) = generic_srs.specialize(2);
            let aggregation_srs_ms = aggregation_srs_started.elapsed().as_secs_f64() * 1000.0;

            let transcript = snarkpack_transcript(&statements);
            let aggregation_started = Instant::now();
            let aggregate_proof = aggregate_proofs::<BellpersonBls12>(
                &aggregation_prover_srs,
                &transcript,
                &bellperson_proofs,
                AggregateVersion::V2,
            )?;
            let aggregation_ms = aggregation_started.elapsed().as_secs_f64() * 1000.0;

            let mut aggregate_verify_rng = ChaCha20Rng::seed_from_u64(SNARKPACK_VERIFY_SEED);
            let aggregate_verify_started = Instant::now();
            let aggregate_proof_verified = verify_aggregate_proof(
                &aggregation_verifier_srs,
                &bellperson_pvk,
                &mut aggregate_verify_rng,
                &statements,
                &aggregate_proof,
                &transcript,
                AggregateVersion::V2,
            )?;
            let aggregate_verify_ms = aggregate_verify_started.elapsed().as_secs_f64() * 1000.0;

            let changed_statement = ark_public_inputs_to_bellperson(&changed_public_inputs)?;
            let changed_statements = vec![changed_statement, second_bellperson_public_inputs];
            let changed_transcript = snarkpack_transcript(&changed_statements);
            let mut changed_verify_rng =
                ChaCha20Rng::seed_from_u64(SNARKPACK_VERIFY_SEED.wrapping_add(1));
            let changed_verify_started = Instant::now();
            let changed_public_context_rejected = !verify_aggregate_proof(
                &aggregation_verifier_srs,
                &bellperson_pvk,
                &mut changed_verify_rng,
                &changed_statements,
                &aggregate_proof,
                &changed_transcript,
                AggregateVersion::V2,
            )?;
            let changed_public_context_verify_ms =
                changed_verify_started.elapsed().as_secs_f64() * 1000.0;
            if !aggregate_proof_verified || !changed_public_context_rejected {
                return Err("SnarkPack acceptance boundary failed".into());
            }

            let (changed_second_nullifier_verify_ms, changed_second_nullifier_rejected) =
                if distinct_voter_gate {
                    let mut changed_second_circuit = second_circuit.clone();
                    changed_second_circuit.nullifier[0] ^= 1;
                    let changed_second_inputs = ballot_cvote_public_inputs(&changed_second_circuit)
                        .ok_or("failed to pack changed G1-C1 second-voter inputs")?;
                    let changed_second_statement =
                        ark_public_inputs_to_bellperson(&changed_second_inputs)?;
                    let changed_second_statements =
                        vec![bellperson_public_inputs.clone(), changed_second_statement];
                    let changed_second_transcript =
                        snarkpack_transcript(&changed_second_statements);
                    let mut changed_second_rng =
                        ChaCha20Rng::seed_from_u64(SNARKPACK_VERIFY_SEED.wrapping_add(2));
                    let changed_second_started = Instant::now();
                    let rejected = !verify_aggregate_proof(
                        &aggregation_verifier_srs,
                        &bellperson_pvk,
                        &mut changed_second_rng,
                        &changed_second_statements,
                        &aggregate_proof,
                        &changed_second_transcript,
                        AggregateVersion::V2,
                    )?;
                    let elapsed = changed_second_started.elapsed().as_secs_f64() * 1000.0;
                    if !rejected {
                        return Err("G1-C1 changed second-voter nullifier was accepted".into());
                    }
                    (Some(elapsed), Some(rejected))
                } else {
                    (None, None)
                };

            let registered_voter_statements_distinct = public_inputs != second_public_inputs;
            let state_record_serialization_matches_snarkpack_inputs =
                state_ballot_public_inputs(&state_ballot_record(&circuit))
                    .is_some_and(|inputs| inputs == public_inputs)
                    && state_ballot_public_inputs(&state_ballot_record(&second_circuit))
                        .is_some_and(|inputs| inputs == second_public_inputs);
            let nullifiers_distinct = circuit.nullifier != second_circuit.nullifier;
            let ciphertexts_distinct = circuit.ciphertexts != second_circuit.ciphertexts;
            let same_election_context_registration_root = circuit.election_id
                == second_circuit.election_id
                && circuit.final_context == second_circuit.final_context
                && circuit.registration_root == second_circuit.registration_root;
            let same_region = circuit.region == second_circuit.region;
            if distinct_voter_gate
                && (!registered_voter_statements_distinct
                    || !nullifiers_distinct
                    || !ciphertexts_distinct
                    || !same_election_context_registration_root
                    || !same_region
                    || !state_record_serialization_matches_snarkpack_inputs)
            {
                return Err("G1-C1 distinct-voter fixture boundary failed".into());
            }

            let mut aggregate_encoding = Vec::new();
            aggregate_proof.write(&mut aggregate_encoding)?;
            let snarkpack_report = SnarkpackResult {
                gate: if distinct_voter_gate { "G1-C1" } else { "G1-C0" },
                status: if distinct_voter_gate {
                    "PASS_G1C1_REAL_SNARKPACK_V2_DISTINCT_VOTERS_BATCH_2_NONPRODUCTION_SRS"
                } else {
                    "PASS_G1C0_REAL_SNARKPACK_V2_MIN_BATCH_2_NONPRODUCTION_SRS"
                },
                classification: if distinct_voter_gate {
                    "real-bellperson-snarkpack-v2-over-two-distinct-registered-voter-g1b2-statements-with-reproducible-nonproduction-aggregation-srs"
                } else {
                    "real-bellperson-snarkpack-v2-over-two-converted-g1b2-arkworks-groth16-proofs-with-reproducible-nonproduction-aggregation-srs"
                },
                git_commit: command_output("git", &["rev-parse", "HEAD"])
                    .unwrap_or_else(|| "UNKNOWN".to_owned()),
                git_dirty: command_output(
                    "git",
                    &["status", "--porcelain", "--untracked-files=normal"],
                )
                .map_or_else(|| "UNKNOWN".to_owned(), |value| (!value.is_empty()).to_string()),
                build_profile: if cfg!(debug_assertions) {
                    "dev-unoptimized"
                } else {
                    "release-opt-level-1"
                },
                rustc_version: command_output(
                    &env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()),
                    &["--version"],
                )
                .unwrap_or_else(|| "UNKNOWN".to_owned()),
                os: env::consts::OS,
                arch: env::consts::ARCH,
                cpu: if cfg!(target_os = "macos") {
                    command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
                        .unwrap_or_else(|| "UNKNOWN".to_owned())
                } else {
                    "UNKNOWN".to_owned()
                },
                prototype_manifest: manifest_path.display().to_string(),
                protocol_version: protocol_version.clone(),
                manifest_capability: if distinct_voter_gate {
                    manifest_g1c1_capability.clone()
                } else {
                    manifest_g1c0_capability.clone()
                },
                manifest_matches_g1c0,
                manifest_matches_g1c1,
                implementation: "filecoin-project/bellperson",
                implementation_version: "0.27.0",
                curve: "BLS12-381",
                source_proof_system: "ark-groth16 0.4.0",
                aggregate_version: "V2",
                batch_size: bellperson_proofs.len(),
                public_inputs_per_proof: statements[0].len(),
                transcript_domain: "VV/JUBJUB/G1-C0/SNARKPACK-PUBLIC-INPUTS/v1",
                transcript_bytes: transcript.len(),
                groth16_setup_seed: SETUP_SEED,
                groth16_setup_ms: setup_ms,
                first_proof_seed: PROOF_SEED,
                first_prove_ms: prove_ms,
                first_verify_ms: verify_ms,
                second_proof_seed: SECOND_PROOF_SEED,
                aggregation_srs_seed: SNARKPACK_SRS_SEED,
                aggregation_srs_kind: "bellperson setup_fake_srs deterministic functional experiment only",
                second_prove_ms,
                conversion_ms,
                aggregation_srs_ms,
                aggregation_ms,
                aggregate_verify_ms,
                changed_public_context_verify_ms,
                individual_verify_ms,
                individual_proof_bytes: proof_encoding.len(),
                total_individual_proof_bytes: proof_encoding.len()
                    + second_proof_encoding.len(),
                aggregate_proof_bytes: aggregate_encoding.len(),
                peak_rss_mib: peak_rss_mib(),
                source_proofs_distinct,
                source_registered_voter_indices: if distinct_voter_gate {
                    source_voter_indices.to_vec()
                } else {
                    vec![target_index, target_index]
                },
                source_candidate_choices: if distinct_voter_gate {
                    vec![1, 2]
                } else {
                    vec![1, 1]
                },
                source_public_records: vec![
                    snarkpack_public_record(&circuit, target_index, 1),
                    snarkpack_public_record(
                        &second_circuit,
                        if distinct_voter_gate {
                            source_voter_indices[1]
                        } else {
                            target_index
                        },
                        if distinct_voter_gate { 2 } else { 1 },
                    ),
                ],
                state_record_serialization_matches_snarkpack_inputs,
                registered_voter_statements_distinct,
                nullifiers_distinct,
                ciphertexts_distinct,
                same_election_context_registration_root,
                same_region,
                converted_individual_proofs_verified,
                aggregate_proof_verified,
                changed_public_context_rejected,
                changed_second_nullifier_verify_ms,
                changed_second_nullifier_rejected,
                aggregation_srs_persisted: false,
                source_proofs_persisted: false,
                full_c_vote: true,
                real_snarkpack_code_path: true,
                production_setup: false,
                limitations: vec![
                    "the minimum supported power-of-two batch of two proofs is tested; this is not a scale result",
                    if distinct_voter_gate {
                        "the two source proofs cover distinct registered voters but do not establish large-batch performance"
                    } else {
                        "both source proofs prove the same valid G1-B2 public statement with independent Groth16 randomness"
                    },
                    "the deterministic bellperson setup_fake_srs path is suitable only for functional reproducibility, not production trust",
                    "the arkworks proving key, source proofs, private witness and aggregation SRS are not persisted",
                ],
            };
            if let Some(parent) = snarkpack_output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(
                snarkpack_output_path,
                serde_json::to_vec_pretty(&snarkpack_report)?,
            )?;
            println!(
                "{}: implementation=bellperson-{} version={} batch={} public-inputs/proof={} second-prove={:.3}ms conversion={:.3}ms srs={:.3}ms aggregate={:.3}ms verify={:.3}ms individual-verify={:.3}ms source-proofs={}B aggregate-proof={}B peak-rss={:.3}MiB valid={} changed-context-rejected={} distinct-statements={} distinct-nullifiers={} distinct-ciphertexts={} changed-second-nullifier-rejected={:?} production-setup=false",
                snarkpack_report.gate,
                snarkpack_report.implementation_version,
                snarkpack_report.aggregate_version,
                snarkpack_report.batch_size,
                snarkpack_report.public_inputs_per_proof,
                snarkpack_report.second_prove_ms,
                snarkpack_report.conversion_ms,
                snarkpack_report.aggregation_srs_ms,
                snarkpack_report.aggregation_ms,
                snarkpack_report.aggregate_verify_ms,
                snarkpack_report.individual_verify_ms,
                snarkpack_report.total_individual_proof_bytes,
                snarkpack_report.aggregate_proof_bytes,
                snarkpack_report.peak_rss_mib,
                snarkpack_report.aggregate_proof_verified,
                snarkpack_report.changed_public_context_rejected,
                snarkpack_report.registered_voter_statements_distinct,
                snarkpack_report.nullifiers_distinct,
                snarkpack_report.ciphertexts_distinct,
                snarkpack_report.changed_second_nullifier_rejected,
            );
            println!("wrote {}", snarkpack_output_path.display());
        }

        Some(Groth16Measurement {
            setup_ms,
            prove_ms,
            verify_ms,
            changed_context_verify_ms,
            peak_rss_after_setup_mib,
            peak_rss_after_prove_mib,
            peak_rss_after_verify_mib,
            proving_key_compressed_bytes: parameters.compressed_size(),
            verifying_key_compressed_bytes: parameters.vk.compressed_size(),
            proof_bytes: proof_encoding.len(),
            valid_proof_verified,
            changed_public_context_rejected,
        })
    } else {
        None
    };

    let layout = PublicLayoutFixture {
        layout_version: LAYOUT_VERSION,
        protocol_version: protocol_version.clone(),
        suite_id: SUITE_ID,
        ballot_prf_suite: BALLOT_PRF_SUITE_ID,
        seed,
        public_inputs_in_order: vec![
            "election identifier",
            "final election context",
            "deterministic region identifier",
            "registration closing root",
            "election-scoped voting nullifier",
            "certified key-set hash",
            "canonical certified election public key",
            "three canonical Weighted ElGamal ciphertext pairs",
        ],
        private_witnesses: vec![
            "32-byte voter secret",
            "PRF-derived address",
            "registered weight and commitment opening",
            "weight commitment",
            "depth-two registration Merkle path",
            "three one-hot choice bits",
            "three encryption randomness scalars",
        ],
        election_id_hex: hex(&election_id),
        final_context_hex: hex(&final_context),
        region,
        registration_root_hex: hex(&computed_root),
        nullifier_hex: hex(&nullifier),
        key_set_hash_hex: hex(&key_set_hash),
        public_key_hex: hex(&encode_point(&public_key)),
        ciphertexts: ciphertexts
            .iter()
            .enumerate()
            .map(|(index, ciphertext)| PublicCiphertext {
                candidate_id: index + 1,
                c0_hex: hex(&encode_point(&ciphertext.c0)),
                c1_hex: hex(&encode_point(&ciphertext.c1)),
            })
            .collect(),
        registered_weight_bound_constant: policy.w_max,
        registration_leaf_count: entries.len(),
        registration_depth: BALLOT_REGISTRATION_DEPTH,
        private_voter_secret_emitted: false,
        private_address_emitted: false,
        private_weight_emitted: false,
        private_commitment_emitted: false,
        private_merkle_path_emitted: false,
        private_choice_emitted: false,
        private_randomness_emitted: false,
    };
    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&layout_path, serde_json::to_vec_pretty(&layout)?)?;

    let report = BallotCvoteResult {
        gate: "G1-B1",
        status: "PASS_G1B1_FULL_C_VOTE_R1CS_FIXED_DEPTH2_FOUR_REGION",
        classification: "complete-five-condition-jubjub-ballot-r1cs-fixed-four-leaf-registration-no-groth16-or-snarkpack",
        git_commit: command_output("git", &["rev-parse", "HEAD"])
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
        git_dirty: command_output(
            "git",
            &["status", "--porcelain", "--untracked-files=normal"],
        )
        .map_or_else(|| "UNKNOWN".to_owned(), |value| (!value.is_empty()).to_string()),
        build_profile: if cfg!(debug_assertions) {
            "dev-unoptimized"
        } else {
            "release-opt-level-1"
        },
        rustc_version: command_output(
            &env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()),
            &["--version"],
        )
        .unwrap_or_else(|| "UNKNOWN".to_owned()),
        os: env::consts::OS,
        arch: env::consts::ARCH,
        cpu: if cfg!(target_os = "macos") {
            command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
                .unwrap_or_else(|| "UNKNOWN".to_owned())
        } else {
            "UNKNOWN".to_owned()
        },
        prototype_manifest: manifest_path.display().to_string(),
        protocol_version: protocol_version.clone(),
        manifest_capability: manifest_capability.clone(),
        manifest_matches_g1b1,
        layout_version: LAYOUT_VERSION,
        layout_file: layout_path.display().to_string(),
        seed,
        candidate_count: valid.candidate_count,
        region_count: BALLOT_REGION_COUNT,
        registration_leaf_count: entries.len(),
        registration_depth: valid.merkle_depth,
        native_registration_root_matches_certificate: registration_root_matches,
        native_relations_match,
        five_conditions_in_r1cs: 5,
        prf_evaluations_in_r1cs: valid.prf_evaluations,
        region_hashes_in_r1cs: valid.region_hashes,
        merkle_hashes_in_r1cs: valid.merkle_hashes,
        one_hot_relations_in_r1cs: valid.one_hot_relations,
        weight_range_relations_in_r1cs: valid.weight_range_relations,
        weight_commitment_opening_equations_in_r1cs: valid
            .weight_commitment_opening_equations,
        elgamal_group_equations_in_r1cs: valid.elgamal_group_equations,
        constraints: valid.constraints,
        public_inputs: valid.public_inputs,
        witness_variables: valid.witness_variables,
        witness_ms,
        peak_rss_mib: r1cs_peak_rss_mib,
        valid_relation_satisfied: valid.satisfied,
        changed_context_rejected,
        full_c_vote_r1cs: true,
        groth16_run: false,
        snarkpack_run: false,
        limitations: vec![
            "G1-B1 fixes exactly four registration leaves, Merkle depth two and four regions",
            "RA certificate and immutable election-registry comparisons remain external contract checks",
            "Groth16 and SnarkPack are not run in G1-B1",
        ],
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "G1-B1: five-conditions={} candidates={} regions={} leaves={} depth={} prfs={} region-hashes={} merkle-hashes={} elgamal-equations={} constraints={} public-inputs={} witness-vars={} witness={:.3}ms peak-rss={:.3}MiB valid={} changed-context-rejected={} groth16=false snarkpack=false",
        report.five_conditions_in_r1cs,
        report.candidate_count,
        report.region_count,
        report.registration_leaf_count,
        report.registration_depth,
        report.prf_evaluations_in_r1cs,
        report.region_hashes_in_r1cs,
        report.merkle_hashes_in_r1cs,
        report.elgamal_group_equations_in_r1cs,
        report.constraints,
        report.public_inputs,
        report.witness_variables,
        report.witness_ms,
        report.peak_rss_mib,
        report.valid_relation_satisfied,
        report.changed_context_rejected,
    );
    println!("wrote {}", output_path.display());
    println!("wrote {}", layout_path.display());

    if let (Some(groth16_output_path), Some(measurement)) =
        (groth16_output_path, groth16_measurement)
    {
        let groth16_report = BallotGroth16Result {
            gate: "G1-B2",
            status: "PASS_G1B2_REAL_GROTH16_FULL_C_VOTE_FIXED_DEPTH2_FOUR_REGION",
            classification: "real-groth16-over-five-condition-fixed-four-leaf-four-region-c-vote-no-snarkpack",
            git_commit: command_output("git", &["rev-parse", "HEAD"])
                .unwrap_or_else(|| "UNKNOWN".to_owned()),
            git_dirty: command_output(
                "git",
                &["status", "--porcelain", "--untracked-files=normal"],
            )
            .map_or_else(|| "UNKNOWN".to_owned(), |value| (!value.is_empty()).to_string()),
            build_profile: if cfg!(debug_assertions) {
                "dev-unoptimized"
            } else {
                "release-opt-level-1"
            },
            rustc_version: command_output(
                &env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()),
                &["--version"],
            )
            .unwrap_or_else(|| "UNKNOWN".to_owned()),
            os: env::consts::OS,
            arch: env::consts::ARCH,
            cpu: if cfg!(target_os = "macos") {
                command_output("sysctl", &["-n", "machdep.cpu.brand_string"])
                    .unwrap_or_else(|| "UNKNOWN".to_owned())
            } else {
                "UNKNOWN".to_owned()
            },
            prototype_manifest: manifest_path.display().to_string(),
            protocol_version,
            manifest_capability: manifest_g1b2_capability,
            manifest_matches_g1b2,
            r1cs_baseline_file: output_path.display().to_string(),
            layout_version: LAYOUT_VERSION,
            layout_file: layout_path.display().to_string(),
            constraints: valid.constraints,
            public_inputs: valid.public_inputs,
            witness_variables: valid.witness_variables,
            witness_ms,
            setup_seed: SETUP_SEED,
            proof_seed: PROOF_SEED,
            setup_ms: measurement.setup_ms,
            prove_ms: measurement.prove_ms,
            verify_ms: measurement.verify_ms,
            changed_context_verify_ms: measurement.changed_context_verify_ms,
            peak_rss_after_setup_mib: measurement.peak_rss_after_setup_mib,
            peak_rss_after_prove_mib: measurement.peak_rss_after_prove_mib,
            peak_rss_after_verify_mib: measurement.peak_rss_after_verify_mib,
            proving_key_compressed_bytes: measurement.proving_key_compressed_bytes,
            verifying_key_compressed_bytes: measurement.verifying_key_compressed_bytes,
            proof_bytes: measurement.proof_bytes,
            valid_proof_verified: measurement.valid_proof_verified,
            changed_public_context_rejected: measurement.changed_public_context_rejected,
            proving_key_persisted: false,
            private_witness_emitted: false,
            full_c_vote: true,
            groth16_run: true,
            snarkpack_run: false,
            limitations: vec![
                "the proof covers the fixed four-leaf, depth-two and four-region experiment suite",
                "the deterministic setup seed is for reproducibility and is not a production ceremony",
                "the proving key and private witness are not persisted",
                "SnarkPack is not run in G1-B2",
            ],
        };
        if let Some(parent) = groth16_output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &groth16_output_path,
            serde_json::to_vec_pretty(&groth16_report)?,
        )?;
        println!(
            "G1-B2: constraints={} public-inputs={} setup={:.3}ms prove={:.3}ms verify={:.3}ms proof={}B pk={}B vk={}B peak-rss={:.3}MiB valid={} changed-context-rejected={} snarkpack=false",
            groth16_report.constraints,
            groth16_report.public_inputs,
            groth16_report.setup_ms,
            groth16_report.prove_ms,
            groth16_report.verify_ms,
            groth16_report.proof_bytes,
            groth16_report.proving_key_compressed_bytes,
            groth16_report.verifying_key_compressed_bytes,
            groth16_report.peak_rss_after_verify_mib,
            groth16_report.valid_proof_verified,
            groth16_report.changed_public_context_rejected,
        );
        println!("wrote {}", groth16_output_path.display());
    }
    Ok(())
}
