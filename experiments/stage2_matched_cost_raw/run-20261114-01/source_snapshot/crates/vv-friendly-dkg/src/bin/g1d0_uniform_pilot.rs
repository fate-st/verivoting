//! G1-D0: same-depth, same-VK native B=2/4/8 pilot from one pool of eight real ballots.
//! New depth-5/weight-32 suite; old entry points, keys and archived measurements stay unchanged.
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
use vv_friendly_crypto::state_gadget::{
    ballot_public_inputs, fold_state_for_batch, StateBallotRecord,
};
use vv_friendly_crypto::{encode_point, generator, random_nonzero_scalar, Ciphertext, Scalar};
use vv_friendly_dkg::registration::{
    registration_membership_path, verify_close, RegistrationAuthority, RegistrationEntry,
    RegistrationPolicy,
};
use vv_friendly_dkg::{run_dkg, AuthenticationKeypair, DkgConfig, FaultPlan};
use vv_types::{derive_final_context, hex, sha256};
const DEPTH: usize = 5;
const REGISTERED: usize = 32;
const WEIGHT_BOUND: u64 = 32;
const BATCHES: [usize; 3] = [2, 4, 8];
const LAYOUT: &str = "VV/JUBJUB/EVALUATION/C-VOTE-D5-W32-LAYOUT/v1";
type BallotCvoteCircuit = vv_friendly_crypto::ballot_identity_gadget::BallotCvoteCircuit<DEPTH>;
const BATCH: usize = 8;
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-EVAL-D5-W32-v1";
const RULE: &str = "highest-weight-smallest-id-on-tie";
const SEED: u64 = 20_261_028;
const SETUP_SEED: u64 = 0x4731_4430_5345_5455;
const PROOF_SEED: u64 = 0x4731_4430_5052_0000;
const SRS_SEED: u64 = 0x4731_4430_5352_5320;
const VERIFY_SEED: u64 = 0x4731_4430_5645_5208;

fn encode_rows(rows: &[Vec<BellScalar>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().map(|x| hex(&x.to_bytes_le())).collect())
        .collect()
}

fn encode_ciphertexts(values: &[Ciphertext]) -> Vec<Value> {
    values
        .iter()
        .enumerate()
        .map(|(j, ct)| {
            json!({"candidate_id": j+1,
        "c0_hex": hex(&encode_point(&ct.c0)), "c1_hex": hex(&encode_point(&ct.c1))})
        })
        .collect()
}

fn direct_fold(old: &[Ciphertext], records: &[StateBallotRecord]) -> Result<Vec<Ciphertext>> {
    let folded = match records.len() {
        2 => fold_state_for_batch::<2>(old, records),
        4 => fold_state_for_batch::<4>(old, records),
        8 => fold_state_for_batch::<8>(old, records),
        _ => None,
    };
    folded.ok_or_else(|| "invalid pilot direct-fold shape".into())
}

fn main() -> Result<()> {
    let run_started = Instant::now();
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let mut manifest_path = PathBuf::from("configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json");
    let mut output = PathBuf::from("experiments/g1_d0_raw/uniform_depth5_native_pilot.json");
    let mut fixture_output = PathBuf::from("fixtures/snarkpack_g1d0_depth5_public_v1.json");
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
        return Err("G1-D0 needs two distinct, new output paths".into());
    }
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let base_path = manifest["base_manifest"]
        .as_str()
        .ok_or("missing base manifest")?;
    let base_sha = hex(&sha256(&fs::read(base_path)?));
    if manifest["protocol_version"] != PROTOCOL
        || manifest["base_manifest_sha256"] != base_sha
        || manifest["registration_count"] != REGISTERED
        || manifest["registration_tree_depth"] != DEPTH
        || manifest["region_count"] != BALLOT_REGION_COUNT
        || manifest["candidate_count"] != 3
        || manifest["registered_weight_bound"] != WEIGHT_BOUND
        || manifest["target_region"] != 4
        || manifest["n"] != 5
        || manifest["t"] != 2
        || manifest["tau"] != 3
        || manifest["q_cert"] != 3
        || manifest["planned_evaluation_batch_sizes"] != json!([2, 4, 8, 16, 32])
        || manifest["ballot_layout"] != LAYOUT
        || manifest["proof_curve"] != "BLS12-381/ark-groth16-0.4.0"
        || manifest["encryption_group"] != "Jubjub/ark-ed-on-bls12-381-0.4.0"
        || manifest["snarkpack_crate"] != "bellperson =0.27.0"
        || manifest["ballot_prf_suite"] != BALLOT_PRF_SUITE_ID
        || manifest["aggregate_version"] != "V2"
        || manifest["fixture_weights"] != json!(vec![1u64; REGISTERED])
        || manifest["fixture_choices"]
            != json!((0..REGISTERED).map(|i| i % 3 + 1).collect::<Vec<_>>())
        || manifest["production_setup"] != false
    {
        return Err("descriptor does not describe this uniform depth-5/weight-32 pilot".into());
    }
    println!("G1-D0 [1/7] freeze pre-policy and run one authenticated 5/3 DKG");
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let eid = sha256(b"VV/JUBJUB/G1-D0/FROZEN-D5-W32-REGISTRY/v1");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy = RegistrationPolicy::new(authority.public_key(), WEIGHT_BOUND, RULE)?;
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
    println!("G1-D0 [2/7] select 32 distinct PRF addresses in region 4 BEFORE registration close");
    let mut secrets = Vec::new();
    let mut addresses = Vec::new();
    let mut selection_attempts = Vec::new();
    for _ in 0..REGISTERED {
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
    let weights = [1u64; REGISTERED];
    let choices = (0..REGISTERED).map(|i| i % 3 + 1).collect::<Vec<_>>();
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
    if registration.w_registered() != WEIGHT_BOUND
        || certificate.close.n_registered != REGISTERED as u64
    {
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
    let distinct_voters = addresses.iter().collect::<HashSet<_>>().len() == REGISTERED;
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
        "G1-D0 [3/7] check all eight complete C_vote R1CS instances and exact 25-field packing"
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
    println!("G1-D0 [4/7] one fresh ballot Groth16 setup; no old proving-key reuse");
    let start = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuits[0].clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let groth16_setup_ms = start.elapsed().as_secs_f64() * 1000.0;
    let prepared = prepare_verifying_key(&parameters.vk);
    println!("G1-D0 [5/7] create and individually verify EIGHT fresh voter proofs serially");
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

    let vk_hash = hex(&sha256(&compressed(&parameters.vk)?));
    let mut vk_bytes = Vec::new();
    vk.write(&mut vk_bytes)?;
    let bell_vk_hash = hex(&sha256(&vk_bytes));
    let mut bell_source_proof_bytes = Vec::new();
    for (proof, original_bytes) in converted.iter().zip(&proof_bytes) {
        let mut bytes = Vec::new();
        proof.write(&mut bytes)?;
        if bytes != *original_bytes {
            return Err("source proof wire encodings differ".into());
        }
        bell_source_proof_bytes.push(bytes.len());
    }
    println!("G1-D0 [6/7] ONE size-32 research SRS; same-VK prefix B=2/4/8 comparisons");
    let start = Instant::now();
    let generic_srs =
        setup_fake_srs::<Bls12, _>(&mut ChaCha20Rng::seed_from_u64(SRS_SEED), REGISTERED);
    let aggregation_srs_ms = start.elapsed().as_secs_f64() * 1000.0;
    let shared_srs_hash = hex(&generic_srs.hash());
    let records = circuits.iter().map(public_record).collect::<Vec<_>>();
    let old_state = vec![Ciphertext::zero(); 3];
    // Prepare submitted ciphertext states outside the timed direct-fold check.
    let expected_states = BATCHES
        .iter()
        .map(|b| direct_fold(&old_state, &records[..*b]))
        .collect::<Result<Vec<_>>>()?;
    let mut cases = Vec::new();
    let mut public_cases = Vec::new();
    for (case_index, batch_size) in BATCHES.into_iter().enumerate() {
        let start = Instant::now();
        let (prover_srs, verifier_srs) = generic_srs.specialize(batch_size);
        let srs_specialize_ms = start.elapsed().as_secs_f64() * 1000.0;
        let bound_transcript = transcript(&statements[..batch_size]);
        let mut individual_verify_ms = Vec::new();
        for (proof, statement) in converted[..batch_size]
            .iter()
            .zip(&statements[..batch_size])
        {
            let start = Instant::now();
            if !verify_bellperson(&pvk, proof, statement)? {
                return Err("individual bellperson baseline failed".into());
            }
            individual_verify_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let individual_verify_total_ms = individual_verify_ms.iter().sum::<f64>();
        let start = Instant::now();
        let aggregate = aggregate_proofs::<Bls12>(
            &prover_srs,
            &bound_transcript,
            &converted[..batch_size],
            AggregateVersion::V2,
        )?;
        let aggregation_ms = start.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        let aggregate_verified = verify_aggregate_proof(
            &verifier_srs,
            &pvk,
            ChaCha20Rng::seed_from_u64(VERIFY_SEED + batch_size as u64),
            &statements[..batch_size],
            &aggregate,
            &bound_transcript,
            AggregateVersion::V2,
        )?;
        let aggregate_verify_ms = start.elapsed().as_secs_f64() * 1000.0;
        if !aggregate_verified {
            return Err("uniform aggregate verification failed".into());
        }
        let start = Instant::now();
        let folded = direct_fold(&old_state, &records[..batch_size])?;
        let direct_fold_matches = folded == expected_states[case_index];
        let direct_fold_check_ms = start.elapsed().as_secs_f64() * 1000.0;
        if !direct_fold_matches {
            return Err("native direct-fold check failed".into());
        }
        let mut aggregate_bytes = Vec::new();
        aggregate.write(&mut aggregate_bytes)?;
        let individual_proof_bytes = proof_bytes[..batch_size]
            .iter()
            .map(Vec::len)
            .sum::<usize>();
        let source_indices = (0..batch_size).collect::<Vec<_>>();
        let mut negative_report = Value::Null;
        let mut public_negative = Value::Null;
        if batch_size == BATCH {
            let mut changed_last = circuits[BATCH - 1].clone();
            changed_last.nullifier[0] ^= 1;
            let changed_inputs =
                ballot_cvote_public_inputs(&changed_last).ok_or("negative packing failed")?;
            let changed_fields = inputs[BATCH - 1]
                .iter()
                .zip(&changed_inputs)
                .enumerate()
                .filter_map(|(i, (a, b))| (a != b).then_some(i))
                .collect::<Vec<_>>();
            if changed_fields != [7] {
                return Err("negative must change only field 7".into());
            }
            let mut changed_statements = statements.clone();
            changed_statements[BATCH - 1] = convert_inputs(&changed_inputs)?;
            let changed_transcript = transcript(&changed_statements);
            let start = Instant::now();
            let rejected = !verify_aggregate_proof(
                &verifier_srs,
                &pvk,
                ChaCha20Rng::seed_from_u64(VERIFY_SEED + 100),
                &changed_statements,
                &aggregate,
                &changed_transcript,
                AggregateVersion::V2,
            )?;
            let verify_ms = start.elapsed().as_secs_f64() * 1000.0;
            if !rejected {
                return Err("changed eighth nullifier was accepted".into());
            }
            negative_report = json!({"kind": "changed_eighth_nullifier", "rejected": rejected,
                "changed_voter_index": 7, "changed_public_field_indices": changed_fields, "verify_ms": verify_ms});
            public_negative = json!({"public_inputs_le_hex": encode_rows(&changed_statements),
                "external_transcript_hex": hex(&changed_transcript)});
        }
        let srs_json = json!({"n": verifier_srs.n, "g": verifier_srs.g, "h": verifier_srs.h,
            "g_alpha": verifier_srs.g_alpha, "g_beta": verifier_srs.g_beta,
            "h_alpha": verifier_srs.h_alpha, "h_beta": verifier_srs.h_beta, "h_alpha_d": verifier_srs.h_alpha_d});
        let verifier_srs_hash = hex(&sha256(&serde_json::to_vec(&srs_json)?));
        cases.push(json!({"batch_size": batch_size, "source_voter_indices": source_indices,
            "ballot_layout": LAYOUT, "merkle_depth": DEPTH, "registered_weight_bound": WEIGHT_BOUND,
            "ballot_vk_sha256": vk_hash, "shared_srs_sha256": shared_srs_hash, "verifier_srs_sha256": verifier_srs_hash,
            "source_proofs_reused_from_one_pool": true, "individual_proofs_verified": true,
            "individual_verify_ms": individual_verify_ms, "individual_verify_total_ms": individual_verify_total_ms,
            "aggregation_ms": aggregation_ms, "aggregate_verified": aggregate_verified, "aggregate_verify_ms": aggregate_verify_ms,
            "individual_verify_ms_per_vote": individual_verify_total_ms / batch_size as f64,
            "aggregate_verify_ms_per_vote": aggregate_verify_ms / batch_size as f64,
            "single_sample_verify_speed_ratio": individual_verify_total_ms / aggregate_verify_ms,
            "individual_proof_bytes": individual_proof_bytes, "aggregate_proof_bytes": aggregate_bytes.len(),
            "single_sample_proof_size_ratio": aggregate_bytes.len() as f64 / individual_proof_bytes as f64,
            "aggregate_proof_sha256": hex(&sha256(&aggregate_bytes)), "public_input_matrix_bytes": batch_size * 25 * 32,
            "external_transcript_bytes": bound_transcript.len(),
            "individual_proof_plus_matrix_bytes": individual_proof_bytes + batch_size * 25 * 32,
            "aggregate_proof_plus_matrix_bytes": aggregate_bytes.len() + batch_size * 25 * 32,
            "payload_scope": "proof plus one canonical field matrix; transcript derived, cached VK/SRS and transaction/DA framing excluded",
            "native_direct_fold_check_ms": direct_fold_check_ms, "direct_fold_matches_submitted_state": direct_fold_matches,
            "direct_fold_group_additions": batch_size * 3 * 2, "srs_specialize_ms": srs_specialize_ms,
            "negative": negative_report, "state_prove_ms": null, "gas_total": null,
            "standalone_peak_rss_mib": null}));
        public_cases.push(json!({"batch_size": batch_size, "source_voter_indices": source_indices,
            "aggregate_proof_hex": hex(&aggregate_bytes), "aggregate_proof_sha256": hex(&sha256(&aggregate_bytes)),
            "public_inputs_le_hex": encode_rows(&statements[..batch_size]), "external_transcript_hex": hex(&bound_transcript),
            "groth16_vk_sha256": bell_vk_hash, "verifier_srs": srs_json,
            "old_state": encode_ciphertexts(&old_state), "new_state": encode_ciphertexts(&folded),
            "negative": public_negative}));
        println!("  B={batch_size}: individual={individual_verify_total_ms:.3}ms aggregate={aggregate_verify_ms:.3}ms aggregation={aggregation_ms:.3}ms fold={direct_fold_check_ms:.6}ms proof={}B / {individual_proof_bytes}B", aggregate_bytes.len());
    }
    println!("G1-D0 [7/7] archive the shared proof pool, three public aggregates and actual measurements");
    let source_public_records = circuits
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({
        "voter_index": i, "address_hex": hex(&c.address), "election_id_hex": hex(&eid),
        "final_context_hex": hex(&final_context), "region": c.region,
        "registration_root_hex": hex(&root), "nullifier_hex": hex(&c.nullifier),
        "key_set_hash_hex": hex(&key_set_hash), "public_key_hex": hex(&encode_point(&c.public_key)),
        "ciphertexts": encode_ciphertexts(&c.ciphertexts)})
        })
        .collect::<Vec<_>>();
    let fixture = json!({"schema": "VV/G1-D0/UNIFORM-DEPTH5-PUBLIC-FIXTURE/v1", "protocol_version": PROTOCOL,
        "experiment_manifest_sha256": hex(&sha256(&manifest_bytes)), "groth16_vk_ark_compressed_sha256": vk_hash,
        "ballot_layout": LAYOUT, "merkle_depth": DEPTH, "registered_weight_bound": WEIGHT_BOUND,
        "registry_count": REGISTERED, "source_proof_count": BATCH, "batch_sizes": BATCHES,
        "groth16_vk_hex": hex(&vk_bytes), "groth16_vk_encoding": "bellperson VerifyingKey::write, uncompressed",
        "groth16_vk_sha256": bell_vk_hash, "source_proofs_ark_compressed_hex": proof_bytes.iter().map(|p|hex(p)).collect::<Vec<_>>(),
        "source_public_records": source_public_records, "source_public_inputs_le_hex": encode_rows(&statements),
        "shared_aggregation_srs_sha256": shared_srs_hash, "cases": public_cases,
        "production_setup": false, "private_witness_emitted": false, "proving_key_persisted": false,
        "aggregation_prover_srs_persisted": false});
    let source_paths = [
        "crates/vv-friendly-dkg/src/bin/g1d0_uniform_pilot.rs",
        "crates/vv-friendly-dkg/src/bin/common/native_batch.rs",
        "crates/vv-friendly-crypto/src/ballot_identity_gadget.rs",
        "crates/vv-friendly-crypto/src/ballot_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h3_gadget.rs",
        "crates/vv-friendly-crypto/src/state_gadget.rs",
        "crates/vv-friendly-crypto/src/lib.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-dkg/src/registration.rs",
        "crates/vv-types/src/lib.rs",
        "Cargo.toml",
        "Cargo.lock",
    ];
    let source_hashes = source_paths
        .iter()
        .map(|path| Ok(json!({"path": path, "sha256": hex(&sha256(&fs::read(path)?))})))
        .collect::<Result<Vec<_>>>()?;
    if fs::read(&manifest_path)? != manifest_bytes {
        return Err("frozen suite changed during pilot".into());
    }
    let report = json!({"gate": "G1-D0", "status": "PASS_G1D0_FIXED_DEPTH5_W32_SAME_VK_NATIVE_B2_B4_B8_PILOT",
        "classification": "single-run-shared-eight-proof-pool-native-pilot-not-statistics-or-proposed-settlement",
        "started_utc": started_utc, "git_commit": metadata("git", &["rev-parse", "HEAD"])?,
        "git_dirty": !metadata("git", &["status", "--porcelain"])?.is_empty(),
        "rustc_version": metadata(&env::var("RUSTC").unwrap_or_else(|_| "rustc".into()), &["--version"])?,
        "build_profile": if cfg!(debug_assertions) {"dev-unoptimized"} else {"release-opt-level-1"},
        "compiler_config": {"opt_level": 1, "strip": env::var("CARGO_PROFILE_RELEASE_STRIP").ok(), "build_jobs": env::var("CARGO_BUILD_JOBS").ok()},
        "cpu": metadata("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|_| "UNKNOWN".into()),
        "ram_bytes": metadata("sysctl", &["-n", "hw.memsize"]).ok(), "os": env::consts::OS, "arch": env::consts::ARCH,
        "experiment_manifest": manifest_path, "manifest_sha256": hex(&sha256(&manifest_bytes)), "base_manifest_sha256": base_sha,
        "protocol_version": PROTOCOL, "ballot_layout": LAYOUT, "source_hashes": source_hashes,
        "seed": seed, "candidate_count": 3, "region_count": BALLOT_REGION_COUNT, "region": 4,
        "merkle_depth": DEPTH, "registry_count": REGISTERED, "registered_weight": registration.w_registered(),
        "registered_weight_bound": WEIGHT_BOUND, "active_voter_count": BATCH, "measured_batch_sizes": BATCHES,
        "dkg_ms": dkg_ms, "dkg_certificate_verified": true, "registration_certificate_verified": true, "pre_policy_fixed_before_dkg": true,
        "election_id_hex": hex(&eid), "pre_policy_hex": hex(&frozen_policy_pre), "prekey_context_hex": hex(&frozen_prekey),
        "dkg_session_id_hex": hex(&frozen_session), "key_core_digest_hex": hex(&key_set_hash), "final_context_hex": hex(&final_context),
        "registration_root_hex": hex(&root), "public_key_hex": hex(&encode_point(&dkg.public_key)),
        "registration_close_digest_hex": hex(&registration.registration_close_digest()),
        "fixture_selection_attempts_per_registered_voter": selection_attempts, "shapes": shapes,
        "distinct_registered_voters": distinct_voters, "distinct_active_nullifiers": distinct_nullifiers,
        "distinct_active_ciphertexts": distinct_ciphertexts, "source_proofs_distinct": distinct_proofs,
        "same_region": same_region, "source_proofs_verified_arkworks": true, "source_proofs_verified_bellperson": true,
        "source_proof_count": proofs.len(), "source_proof_pool_reuse": "prefixes [0,B); eight unique proofs total, not 14 generated proofs",
        "groth16_setup_count": 1, "groth16_setup_seed": SETUP_SEED, "groth16_setup_ms": groth16_setup_ms,
        "groth16_proof_seeds": (0..BATCH).map(|i| PROOF_SEED + i as u64).collect::<Vec<_>>(),
        "groth16_prove_ms": groth16_prove_ms, "groth16_prove_total_ms": groth16_prove_ms.iter().sum::<f64>(),
        "ark_individual_verify_ms": ark_individual_verify_ms, "conversion_and_bell_vk_prepare_ms": conversion_ms,
        "ballot_vk_ark_compressed_sha256": vk_hash, "ballot_vk_bellperson_wire_sha256": bell_vk_hash,
        "verifying_key_ark_compressed_bytes": parameters.vk.compressed_size(), "proving_key_compressed_bytes": parameters.compressed_size(),
        "source_proof_bytes_each": bell_source_proof_bytes, "public_inputs_per_proof": 25,
        "aggregation_srs_kind": "deterministic setup_fake_srs research-only", "aggregation_srs_seed": SRS_SEED,
        "aggregation_srs_max_size": REGISTERED, "aggregation_srs_setup_count": 1, "aggregation_srs_ms": aggregation_srs_ms,
        "shared_aggregation_srs_sha256": shared_srs_hash, "aggregate_version": "V2", "implementation": "bellperson =0.27.0 / blstrs =0.7.0",
        "cases": cases, "aggregate_positive_verification_count": BATCHES.len(), "aggregate_negative_verification_count": 1,
        "case_order": BATCHES, "samples_per_batch": 1, "independent_statistical_repetitions": false,
        "dedicated_warmup": false, "case_order_randomized": false, "per_case_memory_measured_in_separate_process": false,
        "peak_rss_mib": peak_rss_mib(), "total_run_ms_excluding_compilation": run_started.elapsed().as_secs_f64()*1000.0,
        "public_verifier_fixture": fixture_output, "production_setup": false, "proving_key_persisted": false,
        "private_witness_emitted": false, "aggregation_prover_srs_persisted": false,
        "state_proof_run": false, "p3_run": false, "evm_run": false, "calldata_bytes": null, "da_bytes": null, "gas_total": null,
        "limitations": ["one sample per B from one shared proof pool; no median/P95, confidence interval, crossover or production-SRS claim",
            "prefix proof reuse is explicit; per-point direct-fold times measure native arithmetic and equality only, not root/nullifier/registry/settlement checks",
            "public-input matrix includes encoded ciphertexts; proof-plus-matrix is a defined partial payload, not full DA or EVM calldata",
            "process peak RSS cannot be assigned to individual B points; source proving and setup are shared costs",
            "weight-32/depth-5 suite does not reuse weight-10 P3/state parameters or upgrade official manifests",
            "no new state proof, P3, SnarkPack EVM, full Proposed settlement or formal privacy proof"]});
    write_new(&fixture_output, &fixture)?;
    let mut report = report;
    report["public_verifier_fixture_sha256"] = json!(hex(&sha256(&fs::read(&fixture_output)?)));
    write_new(&output, &report)?;
    println!("G1-D0 PASS: same-key B=2/4/8 aggregates, native direct folds and one changed-eighth-nullifier rejection");
    println!(
        "wrote {}; peak RSS={:.3}MiB",
        output.display(),
        peak_rss_mib()
    );
    Ok(())
}
