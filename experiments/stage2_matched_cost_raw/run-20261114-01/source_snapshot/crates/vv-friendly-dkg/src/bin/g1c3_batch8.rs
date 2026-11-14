//! Eight distinct, registered same-region ballots; one real SnarkPack V2 batch.
//! New depth-3 relation/key; archived B=2/B=4 entry points and results stay unchanged.
#![recursion_limit = "512"]
#[path = "common/native_batch.rs"]
mod native_batch;
use ark_bls12_381::Bls12_381;
use ark_ec::Group;
use ark_ff::PrimeField;
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_serialize::CanonicalSerialize;
use bellperson::groth16::{
    aggregate::{aggregate_proofs, setup_fake_srs, verify_aggregate_proof, AggregateVersion},
    prepare_verifying_key as prepare_bellperson_vk, verify_proof as verify_bellperson,
};
use blstrs::{Bls12, Scalar as BellScalar};
use native_batch::*;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde_json::{json, Value};
use std::{collections::HashSet, env, fs, path::PathBuf, time::Instant};
use vv_friendly_crypto::ballot_identity_gadget::{
    ballot_address, ballot_cvote_public_inputs, ballot_nullifier, ballot_region,
    BALLOT_PRF_SUITE_ID, BALLOT_REGION_COUNT,
};
use vv_friendly_crypto::state_gadget::ballot_public_inputs;
use vv_friendly_crypto::{encode_point, generator, random_nonzero_scalar, Ciphertext, Scalar};
use vv_friendly_dkg::registration::{
    registration_membership_path, verify_close, RegistrationAuthority, RegistrationEntry,
    RegistrationPolicy,
};
use vv_friendly_dkg::{run_dkg, AuthenticationKeypair, DkgConfig, FaultPlan};
use vv_types::{derive_final_context, hex, sha256};
const DEPTH: usize = 3;
type BallotCvoteCircuit = vv_friendly_crypto::ballot_identity_gadget::BallotCvoteCircuit<DEPTH>;
const BATCH: usize = 8;
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G1C3-B8-v1";
const RULE: &str = "highest-weight-smallest-id-on-tie";
const SEED: u64 = 20_261_028;
const SETUP_SEED: u64 = 0x4731_4333_5345_5455;
const PROOF_SEED: u64 = 0x4731_4333_5052_0000;
const SRS_SEED: u64 = 0x4731_4333_5352_5308;
const VERIFY_SEED: u64 = 0x4731_4333_5645_5208;

fn main() -> Result<()> {
    let run_started = Instant::now();
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-G1C3-B8.json");
    let mut output = PathBuf::from("experiments/g1_c3_raw/snarkpack_distinct_voters_batch_8.json");
    let mut fixture_output = PathBuf::from("fixtures/snarkpack_g1c3_batch8_public_v1.json");
    let mut seed = SEED;
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match flag.as_str() {
            "--manifest" => manifest_path = value.into(),
            "--output" => output = value.into(),
            "--fixture-output" => fixture_output = value.into(),
            "--seed" => seed = value.parse()?,
            _ => return Err(format!("unknown option {flag}").into()),
        }
    }
    if output.exists() || fixture_output.exists() || output == fixture_output {
        return Err("G1-C3 needs two distinct, new output paths".into());
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let base_path = manifest["base_manifest"]
        .as_str()
        .ok_or("missing base manifest")?;
    let base_sha = hex(&sha256(&fs::read(base_path)?));
    if manifest["protocol_version"] != PROTOCOL
        || manifest["base_manifest_sha256"] != base_sha
        || manifest["batch_size"] != BATCH
        || manifest["registration_tree_depth"] != DEPTH
        || manifest["region_count"] != BALLOT_REGION_COUNT
        || manifest["candidate_count"] != 3
        || manifest["registered_weight_bound"] != 10
        || manifest["target_region"] != 4
        || manifest["n"] != 5
        || manifest["t"] != 2
        || manifest["tau"] != 3
        || manifest["ballot_layout"] != "VV/JUBJUB/G1-C3/C-VOTE-DEPTH3-LAYOUT/v1"
        || manifest["proof_curve"] != "BLS12-381/ark-groth16-0.4.0"
        || manifest["encryption_group"] != "Jubjub/ark-ed-on-bls12-381-0.4.0"
        || manifest["snarkpack_crate"] != "bellperson =0.27.0"
        || manifest["ballot_prf_suite"] != BALLOT_PRF_SUITE_ID
        || manifest["aggregate_version"] != "V2"
        || manifest["fixture_weights"] != json!([1, 1, 1, 1, 1, 1, 1, 3])
        || manifest["fixture_choices"] != json!([1, 2, 1, 3, 2, 3, 1, 2])
        || manifest["production_setup"] != false
    {
        return Err("descriptor does not describe this isolated native B=8 experiment".into());
    }
    println!("G1-C3 [1/8] freeze pre-policy and run one authenticated 5/3 DKG");
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let eid = sha256(b"VV/JUBJUB/G1-C3/EIGHT-DISTINCT-SAME-REGION/v1");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy = RegistrationPolicy::new(authority.public_key(), 10, RULE)?;
    let authentication_keys = (0..5)
        .map(|_| AuthenticationKeypair::generate(&mut rng))
        .collect::<Vec<_>>();
    let config = DkgConfig::new_with_registration_policy(
        eid,
        sha256(&manifest_bytes),
        authentication_keys.iter().map(|key| key.public).collect(),
        policy.digest(eid),
        2,
        3,
    )?;
    let frozen_policy_pre = config.policy_pre;
    let frozen_prekey = config.prekey_context;
    let frozen_session = config.session_id;
    let start = Instant::now();
    let dkg = run_dkg(config, &authentication_keys, FaultPlan::default(), &mut rng)?;
    let dkg_ms = start.elapsed().as_secs_f64() * 1000.0;
    if !dkg.verify_certificate()
        || dkg.public_coefficients.first() != Some(&dkg.public_key)
        || dkg.config.policy_pre != frozen_policy_pre
        || dkg.config.prekey_context != frozen_prekey
        || dkg.config.session_id != frozen_session
    {
        return Err("DKG certificate or pre-policy freeze failed".into());
    }
    let key_set_hash = dkg.certificate.key_core_digest;
    let final_context = derive_final_context(dkg.config.prekey_context, key_set_hash);
    println!(
        "G1-C3 [2/8] select eight distinct PRF addresses in region 4 BEFORE registration close"
    );
    let mut secrets = Vec::new();
    let mut addresses = Vec::new();
    let mut selection_attempts = Vec::new();
    for _ in 0..BATCH {
        let mut accepted = false;
        // Fixture sampling bound only, not a change to any protocol hash suite.
        for attempt in 1..=256 {
            let mut secret = [0u8; 32];
            rng.fill_bytes(&mut secret);
            let address = ballot_address(&secret, &eid, &final_context, &key_set_hash);
            if ballot_region(&final_context, &address) == 4 && !addresses.contains(&address) {
                secrets.push(secret);
                addresses.push(address);
                selection_attempts.push(attempt);
                accepted = true;
                break;
            }
        }
        if !accepted {
            return Err("same-region fixture sampling exhausted its fixed bound".into());
        }
    }
    let weights = [1u64, 1, 1, 1, 1, 1, 1, 3];
    let choices = [1usize, 2, 1, 3, 2, 3, 1, 2];
    let entries = addresses
        .iter()
        .zip(weights)
        .map(|(address, weight)| RegistrationEntry::new(*address, weight, &mut rng))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let certificate = authority.close(
        entries.clone(),
        eid,
        final_context,
        frozen_policy_pre,
        &policy,
        &mut rng,
    )?;
    let registration = verify_close(&certificate, &policy, eid, final_context, frozen_policy_pre)?;
    if registration.w_registered() != 10 {
        return Err("wrong registered weight".into());
    }
    let root = certificate.close.root_close;
    let mut circuits = Vec::new();
    for i in 0..BATCH {
        let path = registration_membership_path(&entries, addresses[i])?;
        let (siblings, directions, leaf_root) = (path.siblings, path.directions, path.root);
        if leaf_root != root || siblings.len() != DEPTH {
            return Err("registration path mismatch".into());
        }
        let one_hot = (1..=3)
            .map(|candidate| candidate == choices[i])
            .collect::<Vec<_>>();
        let randomnesses = (0..3)
            .map(|_| random_nonzero_scalar(&mut rng))
            .collect::<Vec<_>>();
        let ciphertexts = randomnesses
            .iter()
            .zip(&one_hot)
            .map(|(randomness, choice)| Ciphertext {
                c0: generator().mul_bigint(randomness.into_bigint()),
                c1: dkg.public_key.mul_bigint(randomness.into_bigint())
                    + generator().mul_bigint(
                        Scalar::from(if *choice { weights[i] } else { 0 }).into_bigint(),
                    ),
            })
            .collect();
        circuits.push(BallotCvoteCircuit {
            election_id: eid,
            final_context,
            region: ballot_region(&final_context, &addresses[i]),
            registration_root: root,
            nullifier: ballot_nullifier(&secrets[i], &eid, &final_context, &key_set_hash),
            key_set_hash,
            public_key: dkg.public_key,
            ciphertexts,
            registered_weight_bound: policy.w_max,
            voter_secret: secrets[i],
            address: addresses[i],
            weight: entries[i].weight,
            weight_opening: entries[i].opening(),
            weight_commitment: entries[i].commitment(),
            merkle_siblings: siblings,
            merkle_directions: directions,
            choices: one_hot,
            randomnesses,
        });
    }
    let distinct_voters = addresses.iter().collect::<HashSet<_>>().len() == BATCH;
    let distinct_nullifiers = circuits
        .iter()
        .map(|c| c.nullifier)
        .collect::<HashSet<_>>()
        .len()
        == BATCH;
    let same_region = circuits.iter().all(|c| c.region == 4);
    let distinct_ciphertexts = circuits
        .iter()
        .map(|c| {
            c.ciphertexts
                .iter()
                .flat_map(|ct| [encode_point(&ct.c0), encode_point(&ct.c1)].concat())
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>()
        .len()
        == BATCH;
    if !distinct_voters || !distinct_nullifiers || !same_region || !distinct_ciphertexts {
        return Err("B=8 source fixture is not distinct and same-region".into());
    }
    println!(
        "G1-C3 [3/8] check all eight complete C_vote R1CS instances and exact 25-field packing"
    );
    let mut inputs = Vec::new();
    let mut shapes = Vec::new();
    for circuit in &circuits {
        let packed = ballot_cvote_public_inputs(circuit).ok_or("invalid ballot packing")?;
        if packed.len() != 25
            || ballot_public_inputs(&public_record(circuit)).as_ref() != Some(&packed)
        {
            return Err("ballot and public-record packing differ".into());
        }
        let shape = measure_relation(circuit, &packed)?;
        if let Some(first) = shapes.first() {
            let first: &Value = first;
            for key in ["constraints", "public_inputs", "witness_variables"] {
                if first[key] != shape[key] {
                    return Err("source circuit shapes differ".into());
                }
            }
        }
        shapes.push(shape);
        inputs.push(packed);
    }
    println!("G1-C3 [4/8] one fresh ballot Groth16 setup; no old proving-key reuse");
    let start = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuits[0].clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let groth16_setup_ms = start.elapsed().as_secs_f64() * 1000.0;
    let prepared = prepare_verifying_key(&parameters.vk);
    println!("G1-C3 [5/8] create and individually verify EIGHT fresh voter proofs serially");
    let mut proofs = Vec::new();
    let mut proof_bytes = Vec::new();
    let mut groth16_prove_ms = Vec::new();
    let mut ark_individual_verify_ms = Vec::new();
    for (i, (circuit, public_inputs)) in circuits.iter().zip(&inputs).enumerate() {
        let start = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            circuit.clone(),
            &parameters,
            &mut ChaCha20Rng::seed_from_u64(PROOF_SEED + i as u64),
        )?;
        let prove_ms = start.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, public_inputs)? {
            return Err("source Groth16 proof failed".into());
        }
        ark_individual_verify_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        proof_bytes.push(compressed(&proof)?);
        proofs.push(proof);
        groth16_prove_ms.push(prove_ms);
        println!(
            "  voter {}: real proof verified; prove={prove_ms:.3}ms",
            i + 1
        );
    }
    let distinct_proofs = proof_bytes.iter().collect::<HashSet<_>>().len() == BATCH;
    if !distinct_proofs {
        return Err("source proofs were duplicated".into());
    }
    let start = Instant::now();
    let vk = convert_vk(&parameters)?;
    let pvk = prepare_bellperson_vk(&vk);
    let converted = proofs
        .iter()
        .map(convert_proof)
        .collect::<Result<Vec<_>>>()?;
    let statements = inputs
        .iter()
        .map(|row| convert_inputs(row))
        .collect::<Result<Vec<_>>>()?;
    let conversion_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut individual_verify_ms = Vec::new();
    for (proof, statement) in converted.iter().zip(&statements) {
        let start = Instant::now();
        if !verify_bellperson(&pvk, proof, statement)? {
            return Err("converted source proof failed".into());
        }
        individual_verify_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    println!("G1-C3 [6/8] size-8 FUNCTIONAL fake SRS; aggregate eight real V2 proofs");
    let start = Instant::now();
    let generic_srs = setup_fake_srs::<Bls12, _>(&mut ChaCha20Rng::seed_from_u64(SRS_SEED), BATCH);
    let (prover_srs, verifier_srs) = generic_srs.specialize(BATCH);
    let aggregation_srs_ms = start.elapsed().as_secs_f64() * 1000.0;
    let bound_transcript = transcript(&statements);
    let start = Instant::now();
    let aggregate = aggregate_proofs::<Bls12>(
        &prover_srs,
        &bound_transcript,
        &converted,
        AggregateVersion::V2,
    )?;
    let aggregation_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let aggregate_verified = verify_aggregate_proof(
        &verifier_srs,
        &pvk,
        ChaCha20Rng::seed_from_u64(VERIFY_SEED),
        &statements,
        &aggregate,
        &bound_transcript,
        AggregateVersion::V2,
    )?;
    let aggregate_verify_ms = start.elapsed().as_secs_f64() * 1000.0;
    if !aggregate_verified {
        return Err("B=8 aggregate failed".into());
    }
    println!("G1-C3 [7/8] only negative: change LAST voter's nullifier and verify the complete ordered batch");
    let mut changed_last = circuits[BATCH - 1].clone();
    changed_last.nullifier[0] ^= 1;
    let changed_last_inputs =
        ballot_cvote_public_inputs(&changed_last).ok_or("changed ballot packing failed")?;
    let changed_field_indices = inputs[BATCH - 1]
        .iter()
        .zip(&changed_last_inputs)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect::<Vec<_>>();
    if changed_field_indices != [7] {
        return Err("nullifier mutation must change exactly public field 7".into());
    }
    let mut changed_statements = statements.clone();
    changed_statements[BATCH - 1] = convert_inputs(&changed_last_inputs)?;
    let changed_transcript = transcript(&changed_statements);
    let start = Instant::now();
    let changed_last_nullifier_rejected = !verify_aggregate_proof(
        &verifier_srs,
        &pvk,
        ChaCha20Rng::seed_from_u64(VERIFY_SEED + 1),
        &changed_statements,
        &aggregate,
        &changed_transcript,
        AggregateVersion::V2,
    )?;
    let changed_last_nullifier_verify_ms = start.elapsed().as_secs_f64() * 1000.0;
    if !changed_last_nullifier_rejected {
        return Err("changed last nullifier was accepted".into());
    }
    println!("G1-C3 [8/8] archive public verifier material and actual measurements");
    let mut aggregate_bytes = Vec::new();
    aggregate.write(&mut aggregate_bytes)?;
    let mut vk_bytes = Vec::new();
    vk.write(&mut vk_bytes)?;
    let source_public_records = circuits.iter().enumerate().map(|(i, c)| json!({
        "voter_index": i, "address_hex": hex(&c.address), "election_id_hex": hex(&eid),
        "final_context_hex": hex(&final_context), "region": c.region,
        "registration_root_hex": hex(&root), "nullifier_hex": hex(&c.nullifier),
        "key_set_hash_hex": hex(&key_set_hash), "public_key_hex": hex(&encode_point(&c.public_key)),
        "ciphertexts": c.ciphertexts.iter().enumerate().map(|(j, ct)| json!({
            "candidate_id": j + 1, "c0_hex": hex(&encode_point(&ct.c0)), "c1_hex": hex(&encode_point(&ct.c1))
        })).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let encode_rows = |rows: &[Vec<BellScalar>]| {
        rows.iter()
            .map(|row| {
                row.iter()
                    .map(|x| hex(&x.to_bytes_le()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let fixture = json!({
        "schema": "VV/G1-C3/SNARKPACK-B8-PUBLIC-FIXTURE/v1", "protocol_version": PROTOCOL,
        "ballot_layout": "VV/JUBJUB/G1-C3/C-VOTE-DEPTH3-LAYOUT/v1", "merkle_depth": DEPTH,
        "aggregate_version": "V2", "batch_size": BATCH, "public_inputs_per_proof": 25,
        "aggregate_proof_hex": hex(&aggregate_bytes), "aggregate_proof_sha256": hex(&sha256(&aggregate_bytes)),
        "groth16_vk_hex": hex(&vk_bytes), "groth16_vk_encoding": "bellperson VerifyingKey::write, uncompressed",
        "source_proofs_ark_compressed_hex": proof_bytes.iter().map(|p| hex(p)).collect::<Vec<_>>(),
        "public_inputs_le_hex": encode_rows(&statements),
        "changed_last_nullifier_public_inputs_le_hex": encode_rows(&changed_statements),
        "external_transcript_hex": hex(&bound_transcript), "changed_external_transcript_hex": hex(&changed_transcript),
        "verifier_srs": {"n": verifier_srs.n, "g": verifier_srs.g, "h": verifier_srs.h,
            "g_alpha": verifier_srs.g_alpha, "g_beta": verifier_srs.g_beta,
            "h_alpha": verifier_srs.h_alpha, "h_beta": verifier_srs.h_beta, "h_alpha_d": verifier_srs.h_alpha_d},
        "source_public_records": source_public_records, "production_setup": false,
        "private_witness_emitted": false, "proving_key_persisted": false, "aggregation_prover_srs_persisted": false,
    });
    if fs::read(&manifest_path)? != manifest_bytes {
        return Err("descriptor changed during experiment".into());
    }
    let source_paths = [
        "crates/vv-friendly-dkg/src/bin/g1c3_batch8.rs",
        "crates/vv-friendly-dkg/src/bin/common/native_batch.rs",
        "crates/vv-friendly-crypto/src/ballot_identity_gadget.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-dkg/src/registration.rs",
        "Cargo.lock",
    ];
    let source_hashes = source_paths
        .iter()
        .map(|p| Ok(json!({"path": p, "sha256": hex(&sha256(&fs::read(p)?))})))
        .collect::<Result<Vec<_>>>()?;
    let legacy_paths = [
        "crates/vv-friendly-dkg/src/bin/ballot_cvote_b1.rs",
        "experiments/g1_c1_raw/snarkpack_distinct_voters_batch_2.json",
        "crates/vv-friendly-dkg/src/bin/g1c2_batch4.rs",
        "configs/protocol_manifest.JUBJUB-G1C2-B4.json",
        "experiments/g1_c2_raw/snarkpack_distinct_voters_batch_4.json",
        "fixtures/snarkpack_g1c2_batch4_public_v1.json",
    ];
    let legacy_hashes = legacy_paths
        .iter()
        .map(|p| Ok(json!({"path": p, "sha256": hex(&sha256(&fs::read(p)?))})))
        .collect::<Result<Vec<_>>>()?;
    let report = json!({
        "gate": "G1-C3", "status": "PASS_G1C3_REAL_SNARKPACK_V2_EIGHT_DISTINCT_SAME_REGION_VOTERS_NONPRODUCTION_SRS",
        "classification": "one-authenticated-dkg-one-ballot-setup-eight-real-proofs-native-b8-aggregation-only",
        "git_commit": metadata("git", &["rev-parse", "HEAD"])?,
        "git_dirty": !metadata("git", &["status", "--porcelain", "--untracked-files=normal"])?.is_empty(),
        "build_profile": if cfg!(debug_assertions) {"dev-unoptimized"} else {"release-opt-level-1"},
        "rustc_version": metadata(&env::var("RUSTC").unwrap_or_else(|_| "rustc".into()), &["--version"])?,
        "cpu": metadata("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|_| "UNKNOWN".into()),
        "os": env::consts::OS, "arch": env::consts::ARCH,
        "experiment_manifest": manifest_path, "manifest_sha256": hex(&sha256(&manifest_bytes)),
        "ballot_layout": "VV/JUBJUB/G1-C3/C-VOTE-DEPTH3-LAYOUT/v1",
        "new_depth3_groth16_key": true, "historical_baseline_commit": "69e9d5a235d6a333fc36bba95bb756cf94ba782c",
        "base_manifest_sha256": base_sha, "protocol_version": PROTOCOL, "seed": seed,
        "source_hashes": source_hashes, "preserved_b2_b4_entries_and_artifacts": legacy_hashes,
        "batch_size": BATCH, "distinct_voters": distinct_voters, "same_region": same_region,
        "distinct_nullifiers": distinct_nullifiers, "distinct_ciphertexts": distinct_ciphertexts,
        "source_proofs_distinct": distinct_proofs, "source_proofs_verified": true, "converted_source_proofs_verified": true,
        "source_proof_count": proofs.len(), "aggregate_verified": aggregate_verified,
        "changed_last_nullifier_rejected": changed_last_nullifier_rejected,
        "changed_voter_index": BATCH - 1, "changed_public_field_indices": changed_field_indices,
        "changed_last_nullifier_verify_ms": changed_last_nullifier_verify_ms,
        "dkg_ms": dkg_ms, "dkg_certificate_verified": true, "pre_policy_fixed_before_dkg": true,
        "election_id_hex": hex(&eid), "policy_pre_hex": hex(&frozen_policy_pre),
        "prekey_context_hex": hex(&frozen_prekey), "dkg_session_id_hex": hex(&frozen_session),
        "dkg_key_core_digest_hex": hex(&key_set_hash), "public_key_hex": hex(&encode_point(&dkg.public_key)),
        "final_context_hex": hex(&final_context), "registration_root_hex": hex(&root),
        "registration_close_digest_hex": hex(&registration.registration_close_digest()),
        "registration_certificate_verified": true, "registered_weight": registration.w_registered(),
        "registration_leaves": BATCH, "merkle_depth": DEPTH,
        "candidate_count": 3, "region_count": BALLOT_REGION_COUNT, "region": 4,
        "fixture_selection_attempts_per_voter": selection_attempts, "shapes": shapes,
        "public_inputs_per_proof": 25, "public_input_matrix_bytes": BATCH * 25 * 32,
        "same_setup_parameters_used": true, "groth16_setup_count": 1,
        "groth16_setup_seed": SETUP_SEED, "groth16_setup_ms": groth16_setup_ms,
        "groth16_proof_seeds": (0..BATCH).map(|i| PROOF_SEED + i as u64).collect::<Vec<_>>(),
        "groth16_prove_ms": groth16_prove_ms, "groth16_prove_total_ms": groth16_prove_ms.iter().sum::<f64>(),
        "ark_individual_verify_ms": ark_individual_verify_ms,
        "individual_verify_ms": individual_verify_ms, "individual_verify_total_ms": individual_verify_ms.iter().sum::<f64>(),
        "conversion_ms": conversion_ms, "aggregation_srs_kind": "deterministic setup_fake_srs functional only",
        "aggregation_srs_seed": SRS_SEED, "aggregation_srs_ms": aggregation_srs_ms, "aggregation_srs_size": BATCH,
        "aggregation_ms": aggregation_ms, "aggregate_verify_ms": aggregate_verify_ms,
        "aggregate_version": "V2", "implementation": "bellperson =0.27.0 / blstrs =0.7.0",
        "transcript_domain": String::from_utf8_lossy(TRANSCRIPT_DOMAIN), "transcript_bytes": bound_transcript.len(),
        "individual_proof_bytes": proof_bytes.iter().map(Vec::len).collect::<Vec<_>>(),
        "total_individual_proof_bytes": proof_bytes.iter().map(Vec::len).sum::<usize>(),
        "aggregate_proof_bytes": aggregate_bytes.len(), "aggregate_proof_sha256": hex(&sha256(&aggregate_bytes)),
        "verifying_key_ark_compressed_bytes": parameters.vk.compressed_size(),
        "verifying_key_ark_sha256": hex(&sha256(&compressed(&parameters.vk)?)),
        "proving_key_compressed_bytes": parameters.compressed_size(),
        "peak_rss_mib": peak_rss_mib(), "total_run_ms_excluding_compilation": run_started.elapsed().as_secs_f64() * 1000.0,
        "source_public_records": source_public_records, "public_verifier_fixture": fixture_output,
        "production_setup": false, "private_witness_emitted": false, "proving_key_persisted": false,
        "aggregation_prover_srs_persisted": false, "state_proof_b8_run": false, "evm_run": false,
        "aggregate_positive_verification_count": 1, "aggregate_negative_verification_count": 1,
        "limitations": ["functional B8 bridge, not statistical performance or a speedup/compression claim",
            "deterministic public research seeds and fake aggregation SRS are not production setup or operational privacy",
            "DKG and RA certificates are checked natively, not proven inside C_vote",
            "fixed three candidates, four regions and eight registration leaves; no B16 or larger tree",
            "no B8 state proof, complete settlement, EVM SnarkPack or full Proposed Gas measurement"],
    });
    write_new(&fixture_output, &fixture)?;
    let mut report = report;
    report["public_verifier_fixture_sha256"] = json!(hex(&sha256(&fs::read(&fixture_output)?)));
    write_new(&output, &report)?;
    println!("G1-C3 PASS: eight distinct source proofs and B=8 aggregate verified; changed last nullifier rejected");
    println!(
        "  aggregate={} B; aggregate verify={aggregate_verify_ms:.3}ms; peak RSS={:.3}MiB",
        aggregate_bytes.len(),
        peak_rss_mib()
    );
    println!("wrote {}", output.display());
    Ok(())
}
