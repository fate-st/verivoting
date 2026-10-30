//! G6-only adapters copied from the pinned G4-D runner; old runners/relations stay unchanged.
use ark_bls12_381::Fr;
use ark_ec::Group;
use ark_ff::{PrimeField, ToConstraintField, Zero};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem, OptimizationGoal};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, time::Instant};
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
    G4D_SUBSETS,
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
pub(super) type Result<T> = std::result::Result<T, Box<dyn Error>>;
const CONFIG: &str = "configs/protocol_manifest.JUBJUB-G4D-P3-TEN-SUBSETS-W32.json";
const PROTOCOL: &str = "VERIVOTING-JUBJUB-EXPERIMENT-G4D-D5-W32-P3TEN-v1";
// Fresh G6 packages; not reused G4-D private witnesses.
pub(super) const PACKAGE_SEED: u64 = 0x4736_2D50_4143_4B31;
const RULE: &str = "highest-weight-smallest-id-on-tie";
pub(super) fn decode(value: &str) -> Result<Vec<u8>> {
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

pub(super) fn fixed(value: &Value, name: &str) -> Result<[u8; 32]> {
    decode(
        value[name]
            .as_str()
            .ok_or_else(|| format!("missing {name}"))?,
    )?
    .try_into()
    .map_err(|_| format!("invalid {name} length").into())
}

pub(super) fn compressed<T: CanonicalSerialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    value.serialize_compressed(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn ciphertexts(values: &[Ciphertext]) -> Vec<Value> {
    values
        .iter()
        .enumerate()
        .map(|(j, ct)| {
            json!({"candidate_id": j + 1,
        "c0_hex": hex(&encode_point(&ct.c0)), "c1_hex": hex(&encode_point(&ct.c1))})
        })
        .collect()
}

pub(super) fn measure(circuit: &P3TenSubsetCircuit) -> Result<Value> {
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

pub(super) fn canonical<T: CanonicalDeserialize + CanonicalSerialize>(bytes: &[u8]) -> Result<T> {
    let mut rest = bytes;
    let value = T::deserialize_compressed(&mut rest)?;
    if !rest.is_empty() || compressed(&value)? != bytes {
        return Err("noncanonical Ark bytes".into());
    }
    Ok(value)
}

pub(super) fn states(values: &Value) -> Result<Vec<Ciphertext>> {
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

pub(super) struct Sources {
    pub(super) config: Value,
    pub(super) ballot_manifest: Vec<u8>,
    pub(super) d2: Value,
    pub(super) pool: Value,
    pub(super) state: Value,
    pub(super) pins: Vec<Value>,
}
pub(super) fn load_sources() -> Result<Sources> {
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

pub(super) fn rebuild_dkg(
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

pub(super) fn prefix(
    context: &Value,
    roster: &Value,
    ct: &[Ciphertext],
    id: u32,
) -> Result<Vec<u8>> {
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

pub(super) fn packed(bytes: &[u8]) -> Result<Vec<Fr>> {
    bytes
        .to_field_elements()
        .ok_or_else(|| "invalid packed bytes".into())
}

pub(super) fn expected_inputs(
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

pub(super) struct Materials {
    pub(super) trustees: Vec<P3B1TrusteeCircuit>,
    pub(super) verified: Vec<VerifiedDecryptionShare>,
    pub(super) roster: Vec<Value>,
    pub(super) context: Value,
}
pub(super) fn build_materials(
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

pub(super) fn build_case(
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
    let winner = vv_types::winner(&tallies, reg.w_registered())? as u64;
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
        public_winner: winner,
    };
    if c.public_inputs()? != expected_inputs(&m.context, &m.roster, ct, index, winner)? {
        return Err("public inputs differ from frozen CT2/roster".into());
    }
    Ok(c)
}

pub(super) fn input_hex(x: &[Fr]) -> Result<Vec<String>> {
    x.iter().map(|v| Ok(hex(&compressed(v)?))).collect()
}
