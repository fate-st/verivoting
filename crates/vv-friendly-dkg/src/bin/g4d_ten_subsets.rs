//! G4-D: one P3-v2 relation/VK, ten legal subsets, the SAME accepted G2-E CT2.
#![recursion_limit = "512"]
use ark_bls12_381::{Bls12_381, Fr};
use ark_ec::Group;
use ark_ff::{PrimeField, ToConstraintField, Zero};
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, OptimizationGoal};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    env,
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use vv_friendly_crypto::ballot_identity_gadget::{ballot_address, ballot_nullifier, ballot_region};
use vv_friendly_crypto::cp::{
    derive_cp_target_with_registration, prove_decryption_share, transcript_and_challenge,
    verify_decryption_share, GeneralizedCpContext, VerifiedDecryptionShare,
};
use vv_friendly_crypto::cp_h4_gadget::H4ACircuit;
use vv_friendly_crypto::p3a_gadget::P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE;
use vv_friendly_crypto::p3b_gadget::P3B1TrusteeCircuit;
use vv_friendly_crypto::p3c1_gadget::P3_TALLY_RELATION_ID;
use vv_friendly_crypto::p3ten_gadget::{
    g4d_lagrange_coefficients, P3TenSubsetCircuit, G4D_INPUTS, G4D_LAYOUT, G4D_RELATION_ID,
    G4D_SELECTOR_INDEX, G4D_SUBSETS, G4D_WINNER_INDEX,
};
use vv_friendly_crypto::{
    decode_point, encode_point, encrypt_weighted_vote, fold_ballots, generator, Ciphertext, Point,
    Scalar, CP_H2F_SUITE_ID, PACKAGE_H2F_SUITE_ID, SUITE_ID,
};
use vv_friendly_dkg::package::{create_private_package, verify_private_package};
use vv_friendly_dkg::registration::{
    verify_close, RegistrationAuthority, RegistrationEntry, RegistrationPolicy,
    VerifiedRegistrationClose,
};
use vv_friendly_dkg::{
    lagrange_at_zero, run_dkg, threshold_decrypt_verified_shares_with_registration,
    AuthenticationKeypair, DkgConfig, DkgOutput, FaultPlan,
};
use vv_types::{derive_final_context, hex, sha256, CanonicalEncoder};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn decode(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}

fn fixed(value: &Value, name: &str) -> Result<[u8; 32]> {
    decode(
        value[name]
            .as_str()
            .ok_or_else(|| format!("missing {name}"))?,
    )?
    .try_into()
    .map_err(|_| format!("invalid {name} length").into())
}

fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

fn peak_rss_mib() -> f64 {
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

fn metadata(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        return Err(format!("{program} metadata failed").into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn write_new(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn ciphertexts(values: &[Ciphertext]) -> Vec<Value> {
    values
        .iter()
        .enumerate()
        .map(|(j, ct)| {
            json!({"candidate_id": j + 1,
        "c0_hex": hex(&encode_point(&ct.c0)), "c1_hex": hex(&encode_point(&ct.c1))})
        })
        .collect()
}

fn measure(circuit: &P3TenSubsetCircuit) -> Result<Value> {
    let started = Instant::now();
    let inputs = circuit.public_inputs()?;
    let cs = ConstraintSystem::<Fr>::new_ref();
    cs.set_optimization_goal(OptimizationGoal::Constraints);
    circuit.clone().generate_constraints(cs.clone())?;
    cs.finalize();
    let first_unsatisfied = cs.which_is_unsatisfied()?;
    let exact_packing = cs
        .borrow()
        .ok_or("missing constraint system")?
        .instance_assignment[1..]
        == inputs;
    if !exact_packing {
        return Err("actual state instance packing differs".into());
    }
    let synthesis_and_check_ms = started.elapsed().as_secs_f64() * 1000.0;
    let digest_started = Instant::now();
    let matrices = cs.to_matrices().ok_or("missing concrete R1CS matrices")?;
    let mut hasher = Sha256::new();
    hasher.update(b"VV/G4-D/ORDERED-R1CS-ABC/v1");
    for size in [
        matrices.num_instance_variables,
        matrices.num_witness_variables,
        matrices.num_constraints,
    ] {
        hasher.update((size as u64).to_be_bytes());
    }
    for (label, matrix) in [
        (b'A', &matrices.a),
        (b'B', &matrices.b),
        (b'C', &matrices.c),
    ] {
        hasher.update([label]);
        hasher.update((matrix.len() as u64).to_be_bytes());
        for row in matrix {
            hasher.update((row.len() as u64).to_be_bytes());
            for (coefficient, column) in row {
                let mut field = [0u8; 32];
                coefficient.serialize_compressed(&mut field[..])?;
                hasher.update(field);
                hasher.update((*column as u64).to_be_bytes());
            }
        }
    }
    let matrix_sha256 = hex(&hasher.finalize());
    Ok(
        json!({"matrix_sha256": matrix_sha256, "matrix_digest_encoding": "VV/G4-D/ORDERED-R1CS-ABC/v1; counts/row widths/column u64 BE; field coefficients canonical LE32; includes constant instance", "matrix_digest_ms": digest_started.elapsed().as_secs_f64()*1000.0, "satisfied": first_unsatisfied.is_none(), "first_unsatisfied_constraint": first_unsatisfied,
        "constraints": cs.num_constraints(), "public_inputs": cs.num_instance_variables() - 1,
        "witness_variables": cs.num_witness_variables(), "exact_public_input_packing": exact_packing,
        "synthesis_and_check_ms": synthesis_and_check_ms}),
    )
}

fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut rest = bytes;
    let value = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || compressed(&value)? != bytes {
        return Err("noncanonical Ark bytes".into());
    }
    Ok(value)
}

fn states(values: &Value) -> Result<Vec<Ciphertext>> {
    let values = values.as_array().ok_or("state vector")?;
    if values.len() != 3 {
        return Err("state width".into());
    }
    values
        .iter()
        .enumerate()
        .map(|(j, c)| {
            if c["candidate_id"] != j + 1 {
                return Err("state candidate order".into());
            }
            Ok(Ciphertext {
                c0: decode_point(&fixed(c, "c0_hex")?, true)?,
                c1: decode_point(&fixed(c, "c1_hex")?, true)?,
            })
        })
        .collect()
}

const CONFIG: &str = "configs/protocol_manifest.JUBJUB-G4D-P3-TEN-SUBSETS-W32.json";
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G4D-D5-W32-P3TEN-v1";
const SETUP_SEED: u64 = 0x4734_442D_5345_5455;
const PROOF_SEED: u64 = 0x4734_442D_5052_0000;
const PACKAGE_SEED: u64 = 0x4734_442D_5041_434B;
const RULE: &str = "highest-weight-smallest-id-on-tie";

fn source_hashes() -> Result<Vec<Value>> {
    [
        "crates/vv-friendly-crypto/src/p3ten_gadget.rs",
        "crates/vv-friendly-crypto/src/p3c1_gadget.rs",
        "crates/vv-friendly-crypto/src/p3d_gadget.rs",
        "crates/vv-friendly-crypto/src/p3b_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h4_gadget.rs",
        "crates/vv-friendly-crypto/src/cp_h3_gadget.rs",
        "crates/vv-friendly-crypto/src/hash_to_field_gadget.rs",
        "crates/vv-friendly-crypto/src/lib.rs",
        "crates/vv-friendly-dkg/src/bin/g4d_ten_subsets.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-dkg/src/package.rs",
        "crates/vv-friendly-dkg/src/registration.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "Cargo.toml",
        "Cargo.lock",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect()
}

struct Sources {
    config: Value,
    ballot_manifest: Vec<u8>,
    d2: Value,
    pool: Value,
    state: Value,
    pins: Vec<Value>,
}
fn load_sources() -> Result<Sources> {
    let bytes = fs::read(CONFIG)?;
    let config: Value = serde_json::from_slice(&bytes)?;
    if config["protocol_version"] != PROTOCOL
        || config["relation_id"] != G4D_RELATION_ID
        || config["tally_relation_id"] != P3_TALLY_RELATION_ID
        || config["input_layout"] != G4D_LAYOUT
        || config["subsets"] != json!(G4D_SUBSETS)
        || config["registered_weight"] != 32
        || config["cp_suite"] != CP_H2F_SUITE_ID
        || config["package_suite"] != PACKAGE_H2F_SUITE_ID
        || config["n"] != 5
        || config["t"] != 2
        || config["tau"] != 3
        || config["encryption_group"] != "Jubjub/ark-ed-on-bls12-381-0.4.0"
        || config["proof_curve"] != "BLS12-381/ark-groth16-0.4.0"
        || config["production_setup"] != false
    {
        return Err("wrong G4-D experiment descriptor".into());
    }
    let mut pins = vec![json!({"path":CONFIG,"sha256":hex(&sha256(&bytes))})];
    let mut pinned = |key: &str| -> Result<Vec<u8>> {
        let path = config[key].as_str().ok_or("pin path")?;
        let bytes = fs::read(path)?;
        let hash = hex(&sha256(&bytes));
        if config[format!("{key}_sha256")] != hash {
            return Err(format!("pin mismatch: {path}").into());
        }
        pins.push(json!({"path":path,"sha256":hash}));
        Ok(bytes)
    };
    let ballot_manifest = pinned("ballot_manifest")?;
    let d2: Value = serde_json::from_slice(&pinned("d2_raw")?)?;
    let pool: Value = serde_json::from_slice(&pinned("d2_fixture")?)?;
    let state_run: Value = serde_json::from_slice(&pinned("g2e_raw")?)?;
    let state: Value = serde_json::from_slice(&pinned("g2e_fixture")?)?;
    let _legacy = pinned("legacy_v2_descriptor")?;
    let _legacy_fixture = pinned("legacy_v2_fixture")?;
    if state_run["status"] != "PASS_TWO_CONSECUTIVE_B4_TRANSITIONS_AND_NATIVE_CHECKPOINT"
        || state_run["all_both_verifiers_pass"] != true
        || state_run["final_equals_independent_eight_ballot_fold"] != true
        || state_run["final_state"] != state["final_state"]
        || state["batches"].as_array().ok_or("state batches")?.len() != 2
        || state["batches"][1]["new_state"] != state["final_state"]
        || d2["source_proof_count"] != 32
    {
        return Err("not the accepted G2-E final target".into());
    }
    Ok(Sources {
        config,
        ballot_manifest,
        d2,
        pool,
        state,
        pins,
    })
}

fn rebuild_dkg(
    s: &Sources,
) -> Result<(
    DkgOutput,
    Vec<AuthenticationKeypair>,
    VerifiedRegistrationClose,
    Value,
)> {
    let started = Instant::now();
    let mut rng = ChaCha20Rng::seed_from_u64(20_261_028);
    let eid = sha256(b"VV/JUBJUB/G1-D0/FROZEN-D5-W32-REGISTRY/v1");
    let authority = RegistrationAuthority::generate(&mut rng);
    let policy = RegistrationPolicy::new(authority.public_key(), 32, RULE)?;
    let keys = (0..5)
        .map(|_| AuthenticationKeypair::generate(&mut rng))
        .collect::<Vec<_>>();
    let config = DkgConfig::new_with_registration_policy(
        eid,
        sha256(&s.ballot_manifest),
        keys.iter().map(|k| k.public).collect(),
        policy.digest(eid),
        2,
        3,
    )?;
    let policy_pre = config.policy_pre;
    let prekey = config.prekey_context;
    let session = config.session_id;
    let dkg = run_dkg(config, &keys, FaultPlan::default(), &mut rng)?;
    let ctx = derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);
    for (name, actual) in [
        ("election_id_hex", eid),
        ("pre_policy_hex", policy_pre),
        ("prekey_context_hex", prekey),
        ("key_core_digest_hex", dkg.certificate.key_core_digest),
        ("final_context_hex", ctx),
    ] {
        if s.d2[name] != hex(&actual) {
            return Err(format!("D2 DKG mismatch: {name}").into());
        }
    }
    if s.d2["public_key_hex"] != hex(&encode_point(&dkg.public_key))
        || !dkg.verify_certificate()
        || dkg.config.policy_pre != policy_pre
        || dkg.config.session_id != session
        || dkg.config.prekey_context != prekey
    {
        return Err("DKG pre-policy or certificate mismatch".into());
    }
    let mut secrets = Vec::new();
    let mut addresses = Vec::new();
    for _ in 0..32 {
        let mut accepted = None;
        for _ in 1..=256 {
            let mut secret = [0u8; 32];
            rng.fill_bytes(&mut secret);
            let address = ballot_address(&secret, &eid, &ctx, &dkg.certificate.key_core_digest);
            if ballot_region(&ctx, &address) == 4 && !addresses.contains(&address) {
                accepted = Some((secret, address));
                break;
            }
        }
        let (secret, address) = accepted.ok_or("same-region fixture sampling exhausted")?;
        secrets.push(secret);
        addresses.push(address);
    }
    let entries = addresses
        .iter()
        .map(|a| RegistrationEntry::new(*a, 1, &mut rng))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let certificate = authority.close(entries, eid, ctx, policy_pre, &policy, &mut rng)?;
    let registration = verify_close(&certificate, &policy, eid, ctx, policy_pre)?;
    if registration.w_registered() != 32
        || certificate.close.n_registered != 32
        || s.d2["registration_root_hex"] != hex(&certificate.close.root_close)
        || s.d2["registration_close_digest_hex"] != hex(&registration.registration_close_digest())
    {
        return Err("D2 close/root mismatch".into());
    }
    let records = s.pool["source_public_records"]
        .as_array()
        .ok_or("D2 records")?;
    if records.len() != 32 {
        return Err("D2 count".into());
    }
    for (i, (record, address)) in records.iter().zip(&addresses).enumerate() {
        if record["voter_index"] != i
            || record["address_hex"] != hex(address)
            || record["registration_root_hex"] != s.d2["registration_root_hex"]
        {
            return Err("reconstructed registry differs".into());
        }
    }
    let mut recreated_ballots = Vec::new();
    for i in 0..8 {
        let ballot = encrypt_weighted_vote(&dkg.public_key, 1, i % 3 + 1, 3, 32, &mut rng)?;
        if records[i]["nullifier_hex"]
            != hex(&ballot_nullifier(
                &secrets[i],
                &eid,
                &ctx,
                &dkg.certificate.key_core_digest,
            ))
            || records[i]["ciphertexts"] != json!(ciphertexts(&ballot))
        {
            return Err("D2 first-eight encryption/nullifier reconstruction differs".into());
        }
        recreated_ballots.push(ballot);
    }
    let ct2 = states(&s.state["final_state"])?;
    if fold_ballots(&recreated_ballots)? != ct2 {
        return Err("CT2 differs from regenerated D2 eight-ballot native sum".into());
    }
    let info = json!({"pre_policy_fixed_before_dkg":true,"certificate_verified":true,"registration_certificate_verified":true,
        "public_key_hex":hex(&encode_point(&dkg.public_key)),"key_core_digest_hex":hex(&dkg.certificate.key_core_digest),
        "election_id_hex":hex(&eid),"policy_pre_hex":hex(&policy_pre),"prekey_context_hex":hex(&prekey),"dkg_session_hex":hex(&session),
        "final_context_hex":hex(&ctx),"registration_root_hex":hex(&certificate.close.root_close),
        "registration_close_digest_hex":hex(&registration.registration_close_digest()),"disclosure_digest_hex":hex(&registration.disclosure_digest()),
        "registration_certificate_hex":hex(&certificate.canonical_bytes()),"registration_authority_hex":hex(&encode_point(&authority.public_key())),
        "all_32_addresses_match_d2":true,"first_eight_ciphertexts_and_nullifiers_match_d2":true,"ct2_matches_native_reconstruction":true,
        "registered_weight":32,"w_max":32,"n_registered":32,"rebuild_ms":started.elapsed().as_secs_f64()*1000.0});
    Ok((dkg, keys, registration, info))
}

fn prefix(context: &Value, roster: &Value, ct: &[Ciphertext], id: u32) -> Result<Vec<u8>> {
    let mut e = CanonicalEncoder::new(b"VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3");
    e.text(SUITE_ID);
    for key in [
        "election_id_hex",
        "policy_pre_hex",
        "final_context_hex",
        "registration_close_digest_hex",
        "disclosure_digest_hex",
        "cp_target_hex",
    ] {
        e.fixed(&fixed(context, key)?);
    }
    e.u32(id);
    e.fixed(&fixed(roster, "verification_share_hex")?);
    e.u64(ct.len() as u64);
    for c in ct {
        e.fixed(&encode_point(&c.c0));
    }
    e.u64(ct.len() as u64);
    let bytes = e.finish();
    if bytes.len() != P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE {
        return Err("public CP prefix length differs".into());
    }
    Ok(bytes)
}

fn packed(bytes: &[u8]) -> Result<Vec<Fr>> {
    bytes
        .to_field_elements()
        .ok_or_else(|| "invalid packed bytes".into())
}

fn expected_inputs(
    context: &Value,
    roster: &[Value],
    ct: &[Ciphertext],
    index: usize,
    winner: u64,
) -> Result<Vec<Fr>> {
    let subset = G4D_SUBSETS.get(index).ok_or("selector range")?;
    let mut out = Vec::new();
    for id in subset {
        let row = &roster[*id as usize - 1];
        if row["trustee_id"] != *id {
            return Err("roster order".into());
        }
        out.extend(packed(&prefix(context, row, ct, *id)?)?);
        out.extend(packed(&encode_point(&decode_point(
            &fixed(row, "package_commitment_hex")?,
            true,
        )?))?);
    }
    out.push(Fr::from(index as u64));
    for c in ct {
        out.extend(packed(&encode_point(&c.c1))?);
    }
    out.push(Fr::from(32u64));
    out.push(Fr::from(winner));
    if out.len() != G4D_INPUTS {
        return Err("P3 input arity".into());
    }
    Ok(out)
}

struct Materials {
    trustees: Vec<P3B1TrusteeCircuit>,
    verified: Vec<VerifiedDecryptionShare>,
    roster: Vec<Value>,
    context: Value,
}
fn build_materials(
    dkg: &DkgOutput,
    keys: &[AuthenticationKeypair],
    reg: &VerifiedRegistrationClose,
    ct: &[Ciphertext],
    mut context: Value,
) -> Result<Materials> {
    let final_context =
        derive_final_context(dkg.config.prekey_context, dkg.certificate.key_core_digest);
    let target = derive_cp_target_with_registration(
        dkg.config.election_id,
        dkg.config.policy_pre,
        final_context,
        reg.registration_close_digest(),
        reg.disclosure_digest(),
        ct,
    )?;
    context["cp_target_hex"] = json!(hex(&target));
    context["frozen_ciphertexts"] = json!(ciphertexts(ct));
    let mut rng = ChaCha20Rng::seed_from_u64(PACKAGE_SEED);
    let mut trustees = Vec::new();
    let mut verified = Vec::new();
    let mut roster = Vec::new();
    for id in 1u32..=5 {
        let cp_context = GeneralizedCpContext {
            election_id: dkg.config.election_id,
            policy_pre: dkg.config.policy_pre,
            final_context,
            registration_close_digest: reg.registration_close_digest(),
            disclosure_digest: reg.disclosure_digest(),
            target,
            trustee_id: id,
        };
        let verification_share = *dkg.verification_share(id)?;
        let package = prove_decryption_share(
            &cp_context,
            dkg.secret_share(id)?,
            &verification_share,
            ct,
            &mut rng,
        )?;
        verify_decryption_share(&cp_context, &verification_share, ct, &package)?;
        let (transcript, _) =
            transcript_and_challenge(&cp_context, &verification_share, ct, &package)?;
        let (private, public) = create_private_package(
            cp_context,
            verification_share,
            package.clone(),
            &keys[id as usize - 1],
            &mut rng,
        )?;
        let checked = verify_private_package(dkg, ct, &private, &public)?;
        verified.push(checked.verified_share().clone());
        let row = json!({"trustee_id":id,"verification_share_hex":hex(&encode_point(&verification_share)),
            "authentication_public_key_hex":hex(&encode_point(&keys[id as usize-1].public)),
            "package_commitment_hex":hex(&encode_point(&public.commitment.point)),"commitment_signature_hex":hex(&public.signature.canonical_bytes())});
        if transcript[..P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE] != prefix(&context, &row, ct, id)? {
            return Err("native CP transcript differs from fixed public target prefix".into());
        }
        roster.push(row);
        trustees.push(P3B1TrusteeCircuit {
            cp: H4ACircuit {
                transcript,
                verification_share,
                ciphertext_c0: ct.iter().map(|c| c.c0).collect(),
                decryption_shares: package.shares,
                a0: package.proof.a0,
                proof_coordinates: package.proof.coordinates,
                response: package.proof.response,
            },
            package_commitment: public.commitment.point,
            opening: private.opening(),
        });
    }
    Ok(Materials {
        trustees,
        verified,
        roster,
        context,
    })
}

fn build_case(
    index: usize,
    dkg: &DkgOutput,
    reg: &VerifiedRegistrationClose,
    ct: &[Ciphertext],
    m: &Materials,
) -> Result<P3TenSubsetCircuit> {
    let subset = G4D_SUBSETS[index];
    let coefficients = g4d_lagrange_coefficients(index).ok_or("subset coefficient table")?;
    if lagrange_at_zero(&subset, 5, 3)? != coefficients {
        return Err("independent native interpolation differs from circuit table".into());
    }
    let trustees = subset
        .iter()
        .map(|id| m.trustees[*id as usize - 1].clone())
        .collect::<Vec<_>>();
    let shares = subset
        .iter()
        .map(|id| m.verified[*id as usize - 1].clone())
        .collect::<Vec<_>>();
    let tallies = threshold_decrypt_verified_shares_with_registration(
        dkg,
        &subset,
        ct,
        &shares,
        reg.registration_close_digest(),
        reg.disclosure_digest(),
        32,
    )?;
    // Only a private fixture self-check: do not serialize the tally or shares.
    if tallies != [3, 3, 2] || vv_types::winner(&tallies, 32)? != 1 {
        return Err("same CT2 native decryption/winner mismatch".into());
    }
    let plaintext_groups = (0..3)
        .map(|j| {
            ct[j].c1
                - trustees
                    .iter()
                    .zip(coefficients)
                    .fold(Point::zero(), |sum, (t, c)| {
                        sum + t.cp.decryption_shares[j].mul_bigint(c.into_bigint())
                    })
        })
        .collect::<Vec<_>>();
    if !plaintext_groups
        .iter()
        .zip(&tallies)
        .all(|(p, t)| *p == generator().mul_bigint(Scalar::from(*t).into_bigint()))
    {
        return Err("tally/group mismatch".into());
    }
    let c = P3TenSubsetCircuit {
        trustees,
        ciphertext_c1: ct.iter().map(|c| c.c1).collect(),
        plaintext_groups,
        public_subset_index: index as u64,
        tallies,
        registered_weight: 32,
        public_winner: 1,
    };
    if c.public_inputs()? != expected_inputs(&m.context, &m.roster, ct, index, 1)? {
        return Err("public inputs differ from frozen CT2/roster".into());
    }
    Ok(c)
}

fn input_hex(x: &[Fr]) -> Result<Vec<String>> {
    x.iter().map(|v| Ok(hex(&compressed(v)?))).collect()
}
fn subset_name(index: usize) -> String {
    G4D_SUBSETS[index].iter().map(u32::to_string).collect()
}

fn public_replay(path: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        return Err("new replay output required".into());
    }
    let bytes = fs::read(path)?;
    let f: Value = serde_json::from_slice(&bytes)?;
    if f["schema"] != "VV/G4-D/P3-V2-TEN-SUBSETS/PUBLIC/v1"
        || f["relation_id"] != G4D_RELATION_ID
        || f["input_layout"] != G4D_LAYOUT
        || f["production_setup"] != false
    {
        return Err("wrong G4-D public schema".into());
    }
    let vk_bytes = decode(f["verifying_key_hex"].as_str().ok_or("VK")?)?;
    if f["verifying_key_sha256"] != hex(&sha256(&vk_bytes)) {
        return Err("VK digest".into());
    }
    let vk: ark_groth16::VerifyingKey<Bls12_381> = canonical(&vk_bytes)?;
    if vk.gamma_abc_g1.len() != G4D_INPUTS + 1 {
        return Err("VK arity".into());
    }
    let prepared = prepare_verifying_key(&vk);
    let sources = load_sources()?;
    let ct = states(&sources.state["final_state"])?;
    if f["public_context"]["frozen_ciphertexts"] != sources.state["final_state"] {
        return Err("public target differs from pinned CT2".into());
    }
    let roster = f["trustee_roster"].as_array().ok_or("roster")?;
    if roster.len() != 5 {
        return Err("five trustees".into());
    }
    let cases = f["cases"].as_array().ok_or("cases")?;
    if cases.len() != 10 {
        return Err("ten proofs required".into());
    }
    let mut measured = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        if c["subset"] != json!(G4D_SUBSETS[i])
            || c["selector"] != i
            || c["verifying_key_sha256"] != f["verifying_key_sha256"]
        {
            return Err("subset/key mismatch".into());
        }
        let expected = expected_inputs(&f["public_context"], roster, &ct, i, 1)?;
        if c["public_inputs_le_hex"] != json!(input_hex(&expected)?) {
            return Err("public case not bound to common CT2/roster".into());
        }
        let proof_bytes = decode(c["proof_hex"].as_str().ok_or("proof")?)?;
        if c["proof_sha256"] != hex(&sha256(&proof_bytes)) {
            return Err("proof digest".into());
        }
        let proof: ark_groth16::Proof<Bls12_381> = canonical(&proof_bytes)?;
        let t = Instant::now();
        if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &expected)? {
            return Err("public proof rejected".into());
        }
        measured.push(json!({"subset":G4D_SUBSETS[i],"verified":true,"verify_ms":t.elapsed().as_secs_f64()*1000.0}));
    }
    write_new(
        output,
        &json!({"gate":"G4-D-public-replay","status":"PASS_ALL_TEN_PUBLIC_P3_PROOFS_ONE_VK",
        "fixture":path,"fixture_sha256":hex(&sha256(&bytes)),"verifying_key_sha256":f["verifying_key_sha256"],
        "cases":measured,"setup_count":0,"prove_count":0,"evm_run":false,
        "scope":"public proof/key/statement replay, not a production registry or contract-state authentication check"}),
    )?;
    println!("G4-D PUBLIC REPLAY PASS: 10/10 proofs, one VK, same frozen CT2; zero setup/prove");
    Ok(())
}

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--verify-fixture") {
        if args.len() != 4 || args[2] != "--output" {
            return Err("use --verify-fixture PATH --output NEW_PATH".into());
        }
        return public_replay(Path::new(&args[1]), Path::new(&args[3]));
    }
    let mut output = PathBuf::from("experiments/g4_d_raw/ten_subsets_p3_v2.json");
    let mut fixture = PathBuf::from("fixtures/jubjub_g4d_ten_subsets_p3_v2_public.json");
    for pair in args.chunks(2) {
        if pair.len() != 2 {
            return Err("option value".into());
        }
        match pair[0].as_str() {
            "--output" => output = pair[1].clone().into(),
            "--fixture-output" => fixture = pair[1].clone().into(),
            _ => return Err("unknown option".into()),
        }
    }
    if output.exists() || fixture.exists() || output == fixture {
        return Err("distinct new outputs required".into());
    }
    let case_dir = output.parent().ok_or("output parent")?;
    for i in 0..10 {
        if case_dir
            .join(format!("subset-{}.json", subset_name(i)))
            .exists()
        {
            return Err("case archive already exists".into());
        }
    }
    let run_started = Instant::now();
    let started_utc = metadata("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])?;
    let git_commit = metadata("git", &["rev-parse", "HEAD"])?;
    let git_status = metadata("git", &["status", "--porcelain", "--untracked-files=all"])?;
    let source_snapshot = source_hashes()?;
    let sources = load_sources()?;
    let ct = states(&sources.state["final_state"])?;
    println!("G4-D [1/5] reconstruct SAME certified D5/W32 DKG/registration, compare exact first-eight ciphertexts to CT2");
    let (dkg, keys, registration, binding) = rebuild_dkg(&sources)?;
    let materials = build_materials(&dkg, &keys, &registration, &ct, binding)?;
    let mut circuits = Vec::new();
    let mut shapes = Vec::new();
    println!("G4-D [2/5] all TEN complete CP/package/tally-v2/winner R1CS, exact packing and full A/B/C fingerprints");
    for i in 0..10 {
        let c = build_case(i, &dkg, &registration, &ct, &materials)?;
        let shape = measure(&c)?;
        if shape["satisfied"] != true || shape["public_inputs"] != G4D_INPUTS {
            return Err("valid P3 relation unsatisfied".into());
        }
        if let Some(first) = shapes.first() {
            let first: &Value = first;
            if first["matrix_sha256"] != shape["matrix_sha256"] {
                return Err("subset changed complete R1CS matrices".into());
            }
        }
        println!(
            "  R1CS {}/10 subset={} PASS constraints={} matrix={}",
            i + 1,
            subset_name(i),
            shape["constraints"],
            shape["matrix_sha256"]
        );
        shapes.push(shape);
        circuits.push(c);
    }
    write_new(
        &case_dir.join("r1cs_shapes.json"),
        &json!({"all_ten_matrices_equal":true,"shapes":shapes}),
    )?;
    println!("G4-D [3/5] ONE fresh research setup, no historical P3/state/ballot key reuse");
    let started = Instant::now();
    let parameters = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuits[0].clone(),
        &mut ChaCha20Rng::seed_from_u64(SETUP_SEED),
    )?;
    let setup_ms = started.elapsed().as_secs_f64() * 1000.0;
    let rss_after_setup = peak_rss_mib();
    let vk_bytes = compressed(&parameters.vk)?;
    let vk_hash = hex(&sha256(&vk_bytes));
    let legacy: Value =
        serde_json::from_slice(&fs::read("fixtures/jubjub_g4c_partial_turnout_v2.json")?)?;
    if legacy["verifying_key_sha256"] == vk_hash || sources.state["state_vk_sha256"] == vk_hash {
        return Err("new relation reused an old VK".into());
    }
    let started = Instant::now();
    let prepared = prepare_verifying_key(&parameters.vk);
    let vk_preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
    if parameters.vk.gamma_abc_g1.len() != G4D_INPUTS + 1 {
        return Err("new VK arity mismatch".into());
    }
    println!("  Setup PASS {setup_ms:.3}ms; VK={vk_hash}; PK remains only in memory");
    println!("G4-D [4/5] generate TEN real proofs serially with the SAME PK/VK; archive each successful case immediately");
    let mut cases = Vec::new();
    let mut public_cases = Vec::new();
    let mut negatives = Value::Null;
    for (i, circuit) in circuits.iter().enumerate() {
        let inputs = circuit.public_inputs()?;
        let started = Instant::now();
        let proof = Groth16::<Bls12_381>::create_random_proof_with_reduction(
            circuit.clone(),
            &parameters,
            &mut ChaCha20Rng::seed_from_u64(PROOF_SEED + i as u64),
        )?;
        let prove_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        if !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &inputs)? {
            return Err("a real P3 proof failed under common VK".into());
        }
        let verify_ms = started.elapsed().as_secs_f64() * 1000.0;
        if i == 0 {
            let mut selector = inputs.clone();
            selector[G4D_SELECTOR_INDEX] = Fr::from(1u64);
            let t = Instant::now();
            let bad_selector = !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &selector)?;
            let selector_ms = t.elapsed().as_secs_f64() * 1000.0;
            let mut winner = inputs.clone();
            winner[G4D_WINNER_INDEX] = Fr::from(2u64);
            let t = Instant::now();
            let bad_winner = !Groth16::<Bls12_381>::verify_proof(&prepared, &proof, &winner)?;
            let winner_ms = t.elapsed().as_secs_f64() * 1000.0;
            if !bad_selector || !bad_winner {
                return Err("necessary P3 negative accepted".into());
            }
            negatives = json!({"changed_selector_only_rejected":bad_selector,"changed_winner_only_rejected":bad_winner,
                "selector_input_index":G4D_SELECTOR_INDEX,"winner_input_index":G4D_WINNER_INDEX,
                "changed_selector_verify_ms":selector_ms,"changed_winner_verify_ms":winner_ms,
                "invalid_selector_and_wrong_transcript_ids_rejected_by_unit_r1cs":true,"invalid_witness_proving_attempted":false});
        }
        let proof_bytes = compressed(&proof)?;
        let public = json!({"subset":G4D_SUBSETS[i],"selector":i,
            "verifying_key_sha256":vk_hash,"proof_hex":hex(&proof_bytes),"proof_sha256":hex(&sha256(&proof_bytes)),
            "public_inputs_le_hex":input_hex(&inputs)?});
        let case = json!({"subset":G4D_SUBSETS[i],"selector":i,"verifying_key_sha256":vk_hash,"shape":shapes[i],
            "native_cp_and_authenticated_packages_verified":true,"native_threshold_tally_matches_private_fixture":true,
            "all_public_inputs_match_same_frozen_ct2_and_roster":true,"proof_verified":true,"prove_ms":prove_ms,"verify_ms":verify_ms,
            "proof_bytes":proof_bytes.len(),"proof_sha256":hex(&sha256(&proof_bytes)),"public_input_bytes":inputs.len()*32,
            "peak_rss_mib_cumulative":peak_rss_mib(),"proof_seed_hex":format!("{:016x}",PROOF_SEED+i as u64),"public_material":public});
        write_new(
            &case_dir.join(format!("subset-{}.json", subset_name(i))),
            &case,
        )?;
        println!("  PROOF {}/10 subset={} PASS prove={prove_ms:.3}ms verify={verify_ms:.6}ms peakRSS={:.3}MiB",i+1,subset_name(i),peak_rss_mib());
        cases.push(case);
        public_cases.push(public);
    }
    if source_snapshot != source_hashes()? || git_commit != metadata("git", &["rev-parse", "HEAD"])?
    {
        return Err("source changed during run".into());
    }
    for pin in &sources.pins {
        if pin["sha256"] != hex(&sha256(&fs::read(pin["path"].as_str().ok_or("pin")?)?)) {
            return Err("input artifact changed during run".into());
        }
    }
    let public = json!({"schema":"VV/G4-D/P3-V2-TEN-SUBSETS/PUBLIC/v1","protocol_version":PROTOCOL,"relation_id":G4D_RELATION_ID,
        "tally_relation_id":P3_TALLY_RELATION_ID,"input_layout":G4D_LAYOUT,"public_input_count":G4D_INPUTS,
        "selector_input_index":G4D_SELECTOR_INDEX,"winner_input_index":G4D_WINNER_INDEX,"subsets":G4D_SUBSETS,
        "verifying_key_hex":hex(&vk_bytes),"verifying_key_sha256":vk_hash,"public_context":materials.context,
        "trustee_roster":materials.roster,"cases":public_cases,"public_winner":1,"production_setup":false,
        "private_tally_or_shares_emitted":false,"proving_key_persisted":false,"source_g2e_fixture":sources.config["g2e_fixture"],
        "source_g2e_fixture_sha256":sources.config["g2e_fixture_sha256"]});
    write_new(&fixture, &public)?;
    let result = json!({"gate":"G4-D","status":"PASS_TEN_P3_V2_SUBSETS_ONE_CIRCUIT_ONE_VK_FROZEN_W32_CT2",
        "started_utc":started_utc,"git_commit":git_commit,"git_dirty":!git_status.is_empty(),"git_status_at_start":git_status,
        "source_hashes":source_snapshot,"source_artifact_pins":sources.pins,"sources_unchanged_before_archival":true,
        "protocol_version":PROTOCOL,"relation_id":G4D_RELATION_ID,"tally_relation_id":P3_TALLY_RELATION_ID,"input_layout":G4D_LAYOUT,
        "subset_count":10,"subsets":G4D_SUBSETS,"one_setup_and_one_vk":true,"all_ten_matrices_equal":true,"matrix_sha256":shapes[0]["matrix_sha256"],
        "public_input_count":G4D_INPUTS,"selector_input_index":G4D_SELECTOR_INDEX,"winner_input_index":G4D_WINNER_INDEX,
        "public_winner":1,"public_context":materials.context,"trustee_roster":materials.roster,"registered_weight":32,"accepted_ballots":8,
        "native_packages_created":5,"native_packages_verified":5,"setup_count":1,"proof_count":10,"verify_count":10,
        "setup_ms":setup_ms,"setup_seed_hex":format!("{SETUP_SEED:016x}"),"package_seed_hex":format!("{PACKAGE_SEED:016x}"),
        "vk_preparation_ms":vk_preparation_ms,"verifying_key_sha256":vk_hash,"verifying_key_compressed_bytes":vk_bytes.len(),
        "proving_key_compressed_bytes":parameters.compressed_size(),"peak_rss_after_setup_mib":rss_after_setup,
        "peak_rss_mib":peak_rss_mib(),"cases":cases,"negative_tests":negatives,"new_ballot_proof_count":0,"new_state_proof_count":0,
        "source_g2e_fixture_sha256":sources.config["g2e_fixture_sha256"],"public_fixture":fixture,"public_fixture_sha256":hex(&sha256(&fs::read(&fixture)?)),
        "rustc_version":metadata(&env::var("RUSTC").unwrap_or_else(|_|"rustc".into()),&["--version"])?,
        "cpu":metadata("sysctl",&["-n","machdep.cpu.brand_string"]).unwrap_or_else(|_|"UNKNOWN".into()),"os":env::consts::OS,"arch":env::consts::ARCH,
        "build_profile":if cfg!(debug_assertions){"dev"}else{"release-opt-level-1"},"total_run_ms_excluding_compilation":run_started.elapsed().as_secs_f64()*1000.0,
        "production_setup":false,"proving_key_persisted":false,"private_tally_or_shares_emitted":false,
        "evm_run":false,"gas_total":null,"calldata_bytes":null,"full_settlement":false,"complete_native_receiver_to_winner_flow":false,
        "limitations":["one fixed W32 CT2 and ten functional subset assignments, not repeated performance statistics or a mathematical security proof",
            "one deterministic research Groth16 setup; prior ballots use fake aggregation SRS; not a production ceremony or real public-chain deployment",
            "P3 CP/package/tally/winner checked in-circuit; certified-roster, registration, common-context/frozen-target authentication remain host checks, not new contract logic",
            "CT2 is the pinned accepted G2-E output; no live receiver-to-freeze-to-one-time-winner state machine is developed in this gate",
            "exact SnarkPack EVM remains blocked; historical component Gas is not combined into total settlement costs"]});
    write_new(&output, &result)?;
    println!("G4-D [5/5] PASS: ten real P3 v2 proofs, one new circuit/VK, same frozen CT2; wrong selector/winner REJECT");
    println!(
        "archived {} and {}; peakRSS={:.3}MiB",
        output.display(),
        fixture.display(),
        peak_rss_mib()
    );
    Ok(())
}
