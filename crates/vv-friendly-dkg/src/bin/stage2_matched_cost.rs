//! One matched generation pilot. Historical protocols/keys/records are unchanged.
#![recursion_limit = "512"]
#[allow(dead_code)]
#[path = "common/native_batch.rs"]
mod helpers;
#[path = "common/g10b_intake.rs"]
mod intake;
#[path = "common/g10b_materials.rs"]
mod materials;
#[path = "common/g2e_receiver.rs"]
mod receiver;
#[path = "common/stage2_routes.rs"]
mod routes;
use ark_bls12_381::Bls12_381;
use ark_ec::Group;
use ark_ff::PrimeField;
use ark_groth16::{prepare_verifying_key, Groth16, Proof, ProvingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use bellperson::groth16::aggregate::{
    aggregate_proofs, setup_fake_srs, AggregateProof, AggregateVersion,
};
use helpers::{
    compressed, convert_inputs, convert_proof, convert_vk, peak_rss_mib, public_record, transcript,
    write_new, Result,
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::Instant,
};
use vv_friendly_crypto::ballot_identity_gadget::{
    ballot_address, ballot_cvote_public_inputs, ballot_nullifier, ballot_region, BallotCvoteCircuit,
};
use vv_friendly_crypto::state_gadget::{ballot_public_inputs, state_public_inputs};
use vv_friendly_crypto::{generator, random_nonzero_scalar, Ciphertext, Scalar};
use vv_friendly_dkg::registration::{
    registration_membership_path, verify_close, RegistrationAuthority, RegistrationEntry,
    RegistrationPolicy,
};
use vv_friendly_dkg::{run_dkg, AuthenticationKeypair, DkgConfig, FaultPlan};
use vv_types::{derive_final_context, hex, sha256};
const MANIFEST: &str = "configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json";
const MANIFEST_SHA: &str = "00c3869c4b10a6e48fa5972b2a98767966db2b2fac0de3692eb8cbbc13cafe5e";
const FIXTURE: &str = "fixtures/jubjub_state_g2e_multibatch_b4_public_v1.json";
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}
fn read_json(p: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(p)?)?)
}
fn decode(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err("odd hex".into());
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}
fn parse<T: CanonicalDeserialize + CanonicalSerialize>(b: &[u8]) -> Result<T> {
    let mut rest = b;
    let v = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || compressed(&v)? != b {
        return Err("noncanonical encoding".into());
    }
    Ok(v)
}
fn save_key(p: &Path, key: &ProvingKey<Bls12_381>) -> Result<Value> {
    use std::os::unix::fs::OpenOptionsExt;
    let t = Instant::now();
    let bytes = compressed(key)?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(p)?;
    file.write_all(&bytes)?;
    Ok(
        json!({"key_serialize_write_ms":ms(t),"key_bytes":bytes.len(),"key_sha256":hex(&sha256(&bytes)),"key_archive":"private temporary runtime only"}),
    )
}
fn check_cache_hash(bytes: &[u8], expected: &str) -> Result<()> {
    if hex(&sha256(bytes)) != expected {
        return Err("local proving-key cache hash mismatch".into());
    }
    Ok(())
}
fn load_key(p: &Path, expected: &str) -> Result<ProvingKey<Bls12_381>> {
    let bytes = fs::read(p)?;
    check_cache_hash(&bytes, expected)?;
    // ONLY this process pipeline's canonical, freshly generated local PK cache.
    // Its generation report authenticates the complete bytes, and the caller
    // separately checks the VK against the frozen public archive. This is NOT
    // the decoder for external keys, proofs, statements, or group witnesses.
    let mut rest = bytes.as_slice();
    let key = ProvingKey::<Bls12_381>::deserialize_compressed_unchecked(&mut rest)?;
    if !rest.is_empty() {
        return Err("trailing local PK bytes".into());
    }
    Ok(key)
}
fn fresh_rng() -> Result<ChaCha20Rng> {
    let mut seed = [0u8; 32];
    fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
    Ok(ChaCha20Rng::from_seed(seed))
}
// Exact G1-D0 witness reconstruction; not a new independent election/DKG deployment.
fn ballots(data: &materials::Loaded) -> Result<Vec<BallotCvoteCircuit<5>>> {
    let bytes = fs::read(MANIFEST)?;
    if hex(&sha256(&bytes)) != MANIFEST_SHA {
        return Err("frozen manifest changed".into());
    }
    let mut rng = ChaCha20Rng::seed_from_u64(20_261_028);
    let eid = sha256(b"VV/JUBJUB/G1-D0/FROZEN-D5-W32-REGISTRY/v1");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy = RegistrationPolicy::new(
        authority.public_key(),
        32,
        "highest-weight-smallest-id-on-tie",
    )?;
    let keys = (0..5)
        .map(|_| AuthenticationKeypair::generate(&mut rng))
        .collect::<Vec<_>>();
    let config = DkgConfig::new_with_registration_policy(
        eid,
        sha256(&bytes),
        keys.iter().map(|k| k.public).collect(),
        policy.digest(eid),
        2,
        3,
    )?;
    let policy_pre = config.policy_pre;
    let dkg = run_dkg(config, &keys, FaultPlan::default(), &mut rng)?;
    if !dkg.verify_certificate() {
        return Err("DKG certificate".into());
    }
    let key_hash = dkg.certificate.key_core_digest;
    let ctx = derive_final_context(dkg.config.prekey_context, key_hash);
    let mut secrets = Vec::new();
    let mut addresses = Vec::new();
    for _ in 0..32 {
        let mut found = false;
        for _ in 0..256 {
            let mut s = [0u8; 32];
            rng.fill_bytes(&mut s);
            let a = ballot_address(&s, &eid, &ctx, &key_hash);
            if ballot_region(&ctx, &a) == 4 && !addresses.contains(&a) {
                secrets.push(s);
                addresses.push(a);
                found = true;
                break;
            }
        }
        if !found {
            return Err("fixture sampling bound".into());
        }
    }
    let entries = addresses
        .iter()
        .map(|a| RegistrationEntry::new(*a, 1, &mut rng))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let cert = authority.close(entries.clone(), eid, ctx, policy_pre, &policy, &mut rng)?;
    let closed = verify_close(&cert, &policy, eid, ctx, policy_pre)?;
    if closed.w_registered() != 32 || cert.close.n_registered != 32 {
        return Err("registered weight/count".into());
    }
    let mut circuits = Vec::new();
    for i in 0..8 {
        let path = registration_membership_path(&entries, addresses[i])?;
        if path.root != cert.close.root_close || path.siblings.len() != 5 {
            return Err("registration membership".into());
        }
        let choices = (1..=3).map(|c| c == i % 3 + 1).collect::<Vec<_>>();
        let randomnesses = (0..3)
            .map(|_| random_nonzero_scalar(&mut rng))
            .collect::<Vec<_>>();
        let ciphertexts = randomnesses
            .iter()
            .zip(&choices)
            .map(|(r, c)| Ciphertext {
                c0: generator().mul_bigint(r.into_bigint()),
                c1: dkg.public_key.mul_bigint(r.into_bigint())
                    + generator().mul_bigint(Scalar::from(u64::from(*c)).into_bigint()),
            })
            .collect();
        let circuit = BallotCvoteCircuit::<5> {
            election_id: eid,
            final_context: ctx,
            region: 4,
            registration_root: cert.close.root_close,
            nullifier: ballot_nullifier(&secrets[i], &eid, &ctx, &key_hash),
            key_set_hash: key_hash,
            public_key: dkg.public_key,
            ciphertexts,
            registered_weight_bound: 32,
            voter_secret: secrets[i],
            address: addresses[i],
            weight: entries[i].weight,
            weight_opening: entries[i].opening(),
            weight_commitment: entries[i].commitment(),
            merkle_siblings: path.siblings,
            merkle_directions: path.directions,
            choices,
            randomnesses,
        };
        let original = &data.batches[i / 4].circuit.records[i % 4];
        let a = ballot_cvote_public_inputs(&circuit).ok_or("ballot inputs")?;
        if ballot_public_inputs(&public_record(&circuit)).as_ref() != Some(&a)
            || ballot_public_inputs(original).as_ref() != Some(&a)
        {
            return Err("reconstructed ballot differs from frozen 25 inputs".into());
        }
        circuits.push(circuit);
    }
    Ok(circuits)
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: stage2_matched_cost PHASE PUBLIC_OUTPUT_DIR PRIVATE_RUNTIME_DIR".into(),
        );
    }
    let phase = &args[0];
    let out = Path::new(&args[1]);
    let runtime = Path::new(&args[2]);
    let whole = Instant::now();
    let t = Instant::now();
    let mut data = materials::load()?;
    let archived_loader_ms = ms(t);
    let fixture: Value = read_json(Path::new(FIXTURE))?;
    let mut report = json!({"schema":"VV/STAGE2/MATCHED-COST-PILOT/v1","phase":phase,"archived_loader_ms":archived_loader_ms,
        "pins":data.pins,"setup_count":0,"ballot_proof_count":0,"state_proof_count":0,"aggregate_count":0,
        "same_archived_election":true,"new_evm":0,"new_p3":0,"production_setup":false});
    match phase.as_str() {
        "ballot-setup" => {
            let t = Instant::now();
            let c = ballots(&data)?;
            report["witness_rebuild_ms"] = json!(ms(t));
            let t = Instant::now();
            let key = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
                c[0].clone(),
                &mut ChaCha20Rng::seed_from_u64(0x4731_4430_5345_5455),
            )?;
            report["setup_ms"] = json!(ms(t));
            report["setup_count"] = json!(1);
            let mut wire = Vec::new();
            convert_vk(&key)?.write(&mut wire)?;
            if hex(&wire) != fixture["source_ballot_vk_hex"] {
                return Err("rebuilt ballot VK mismatch".into());
            }
            report["vk_sha256"] = json!(hex(&sha256(&compressed(&key.vk)?)));
            report["key_io"] = save_key(&runtime.join("ballot.pk"), &key)?;
        }
        "ballot-prove" => {
            let t = Instant::now();
            let c = ballots(&data)?;
            report["witness_rebuild_ms"] = json!(ms(t));
            let t = Instant::now();
            let setup = read_json(&out.join("ballot-setup.json"))?;
            let key = load_key(
                &runtime.join("ballot.pk"),
                setup["key_io"]["key_sha256"].as_str().ok_or("PK hash")?,
            )?;
            report["key_load_ms"] = json!(ms(t));
            let mut key_wire = Vec::new();
            convert_vk(&key)?.write(&mut key_wire)?;
            if hex(&key_wire) != fixture["source_ballot_vk_hex"] {
                return Err("ballot proving key VK mismatch".into());
            }
            let pvk = prepare_verifying_key(&key.vk);
            let mut rng = fresh_rng()?;
            let mut rows = Vec::new();
            for (i, circuit) in c.iter().enumerate() {
                let t = Instant::now();
                let p = Groth16::<Bls12_381>::create_random_proof_with_reduction(
                    circuit.clone(),
                    &key,
                    &mut rng,
                )?;
                let prove_ms = ms(t);
                let bytes = compressed(&p)?;
                let original = &fixture["batches"][i / 4]["source_ballot_proofs_hex"][i % 4];
                if json!(hex(&bytes)) == *original {
                    return Err("fresh ballot proof equals archived proof".into());
                }
                let t = Instant::now();
                if !Groth16::<Bls12_381>::verify_proof(
                    &pvk,
                    &p,
                    &ballot_cvote_public_inputs(circuit).ok_or("inputs")?,
                )? {
                    return Err("fresh ballot invalid".into());
                }
                rows.push(json!({"index":i,"prove_ms":prove_ms,"verify_ms":ms(t),"proof_hex":hex(&bytes),"sha256":hex(&sha256(&bytes)),"bytes":bytes.len(),"verified":true}));
                println!("fresh ballot {}/8: prove={prove_ms:.3} ms", i + 1);
            }
            report["ballot_proof_count"] = json!(8);
            report["proofs"] = json!(rows);
        }
        "state-setup" => {
            let t = Instant::now();
            let key = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
                data.batches[0].circuit.clone(),
                &mut ChaCha20Rng::seed_from_u64(0x4732_442D_5345_5455),
            )?;
            report["setup_ms"] = json!(ms(t));
            report["setup_count"] = json!(1);
            let bytes = compressed(&key.vk)?;
            if hex(&bytes) != fixture["state_vk_hex"] {
                return Err("rebuilt state VK mismatch".into());
            }
            report["vk_sha256"] = json!(hex(&sha256(&bytes)));
            report["key_io"] = save_key(&runtime.join("state.pk"), &key)?;
        }
        "state-prove" => {
            let t = Instant::now();
            let setup = read_json(&out.join("state-setup.json"))?;
            let key = load_key(
                &runtime.join("state.pk"),
                setup["key_io"]["key_sha256"].as_str().ok_or("PK hash")?,
            )?;
            report["key_load_ms"] = json!(ms(t));
            if hex(&compressed(&key.vk)?) != fixture["state_vk_hex"] {
                return Err("state proving key VK mismatch".into());
            }
            let mut rng = fresh_rng()?;
            let pvk = prepare_verifying_key(&key.vk);
            let mut rows = Vec::new();
            for (i, b) in data.batches.iter().enumerate() {
                let t = Instant::now();
                let p = Groth16::<Bls12_381>::create_random_proof_with_reduction(
                    b.circuit.clone(),
                    &key,
                    &mut rng,
                )?;
                let prove_ms = ms(t);
                let bytes = compressed(&p)?;
                if hex(&bytes) == fixture["batches"][i]["state_proof_hex"] {
                    return Err("fresh state proof equals archived proof".into());
                }
                let t = Instant::now();
                if !Groth16::<Bls12_381>::verify_proof(
                    &pvk,
                    &p,
                    &state_public_inputs(&b.circuit).ok_or("state inputs")?,
                )? {
                    return Err("fresh state invalid".into());
                }
                rows.push(json!({"batch_index":i+1,"prove_ms":prove_ms,"verify_ms":ms(t),"proof_hex":hex(&bytes),"sha256":hex(&sha256(&bytes)),"bytes":bytes.len(),"verified":true}));
                println!("fresh state {}/2: prove={prove_ms:.3} ms", i + 1);
            }
            report["state_proof_count"] = json!(2);
            report["proofs"] = json!(rows);
        }
        "aggregate-receive" => {
            let ballot = read_json(&out.join("ballot-prove.json"))?;
            let state = read_json(&out.join("state-prove.json"))?;
            let t = Instant::now();
            let srs = setup_fake_srs::<blstrs::Bls12, _>(
                &mut ChaCha20Rng::seed_from_u64(0x4731_4430_5352_5320),
                32,
            );
            report["research_srs_rebuild_ms"] = json!(ms(t));
            report["research_srs_hash"] = json!(hex(&srs.hash()));
            let (prover_srs, verifier_srs) = srs.specialize(4);
            let srs_fields = json!({"n":verifier_srs.n,"g":verifier_srs.g,"h":verifier_srs.h,
                "g_alpha":verifier_srs.g_alpha,"g_beta":verifier_srs.g_beta,
                "h_alpha":verifier_srs.h_alpha,"h_beta":verifier_srs.h_beta,"h_alpha_d":verifier_srs.h_alpha_d});
            if srs_fields != fixture["source_verifier_srs"] {
                return Err("SRS mismatch".into());
            }
            let mut aggregates = Vec::new();
            for (i, b) in data.batches.iter_mut().enumerate() {
                b.ballot_proofs = (i * 4..i * 4 + 4)
                    .map(|j| {
                        let p: Proof<Bls12_381> = parse(&decode(
                            ballot["proofs"][j]["proof_hex"]
                                .as_str()
                                .ok_or("fresh ballot")?,
                        )?)?;
                        convert_proof(&p)
                    })
                    .collect::<Result<Vec<_>>>()?;
                b.proof = parse(&decode(
                    state["proofs"][i]["proof_hex"]
                        .as_str()
                        .ok_or("fresh state")?,
                )?)?;
                let matrix = b
                    .circuit
                    .records
                    .iter()
                    .map(|r| convert_inputs(&ballot_public_inputs(r).ok_or("packing")?))
                    .collect::<Result<Vec<_>>>()?;
                let t = Instant::now();
                b.aggregate = aggregate_proofs::<blstrs::Bls12>(
                    &prover_srs,
                    &transcript(&matrix),
                    &b.ballot_proofs,
                    AggregateVersion::V2,
                )?;
                let aggregate_ms = ms(t);
                let mut wire = Vec::new();
                b.aggregate.write(&mut wire)?;
                let mut cursor = std::io::Cursor::new(&wire);
                let decoded = AggregateProof::<blstrs::Bls12>::read(&mut cursor)?;
                decoded.parsing_check()?;
                if cursor.position() as usize != wire.len() {
                    return Err("aggregate bytes".into());
                }
                b.aggregate = decoded;
                aggregates.push(json!({"batch_index":i+1,"aggregate_ms":aggregate_ms,"proof_hex":hex(&wire),"bytes":wire.len(),"sha256":hex(&sha256(&wire))}));
            }
            report["aggregate_count"] = json!(2);
            report["aggregates"] = json!(aggregates);
            report["receivers"] = routes::run_fresh(&data)?;
            report["new_ballot_proofs_consumed"] = json!(8);
            report["new_state_proofs_consumed"] = json!(2);
            report["historical_proofs_used_by_receivers"] = json!(0);
        }
        _ => return Err("unknown phase".into()),
    }
    report["status"] = json!("PASS");
    report["process_wall_ms"] = json!(ms(whole));
    report["process_peak_rss_mib"] = json!(peak_rss_mib());
    report["rss_scope"] = json!("fresh worker whole-process high-water; includes archived loader, key load/serialization where applicable; prove workers execute zero setup; NOT incremental algorithm-only RSS");
    write_new(&out.join(format!("{phase}.json")), &report)?;
    println!(
        "PASS {phase}: wall={:.3} ms peakRSS={:.3} MiB",
        ms(whole),
        peak_rss_mib()
    );
    Ok(())
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn changed_local_key_bytes_are_rejected_before_decoding() {
        let original = b"benchmark-only generated key cache";
        let expected = hex(&sha256(original));
        assert!(check_cache_hash(original, &expected).is_ok());
        assert!(check_cache_hash(b"changed cache", &expected).is_err());
    }
}
