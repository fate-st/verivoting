//! Actual five-process CP/package delivery over private stdin/stdout pipes.
//! Research bootstrap is centralized; this is NOT independent distributed DKG.
use super::materials::{self, Result};
use ark_ec::Group;
use ark_ff::PrimeField;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};
use vv_friendly_crypto::{
    cp::{
        prove_decryption_share, transcript_and_challenge, verify_decryption_share,
        DecryptionSharePackage, GeneralizedCpContext, GeneralizedCpProof, VerifiedDecryptionShare,
    },
    cp_h4_gadget::H4ACircuit,
    decode_point, decode_scalar, derive_secondary_generator, encode_point, encode_scalar,
    generator,
    p3b_gadget::P3B1TrusteeCircuit,
    package_hash_to_field_v1, Ciphertext, Point,
};
use vv_friendly_dkg::package::{
    commitment_message, create_private_package, package_bytes, PackageCommitment,
    PACKAGE_SIGNATURE_DOMAIN,
};
use vv_friendly_dkg::registration::RegistrationAuthority;
use vv_friendly_dkg::{
    verify_signature_with_domain, AuthenticationKeypair, DkgOutput, SchnorrSignature,
};
use vv_types::hex;

pub fn context(c: &Value, id: u32) -> Result<GeneralizedCpContext> {
    Ok(GeneralizedCpContext {
        election_id: materials::fixed(c, "election_id_hex")?,
        policy_pre: materials::fixed(c, "policy_pre_hex")?,
        final_context: materials::fixed(c, "final_context_hex")?,
        registration_close_digest: materials::fixed(c, "registration_close_digest_hex")?,
        disclosure_digest: materials::fixed(c, "disclosure_digest_hex")?,
        target: materials::fixed(c, "cp_target_hex")?,
        trustee_id: id,
    })
}
fn scalar(v: &Value) -> Result<vv_friendly_crypto::Scalar> {
    let b = materials::decode(v.as_str().ok_or("scalar")?)?;
    Ok(decode_scalar(&b)?)
}
fn point(v: &Value, zero: bool) -> Result<Point> {
    Ok(decode_point(
        &materials::decode(v.as_str().ok_or("point")?)?,
        zero,
    )?)
}

pub fn worker() -> Result<()> {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;
    let q: Value = serde_json::from_slice(&input)?;
    let id = u32::try_from(q["trustee_id"].as_u64().ok_or("id")?)?;
    if !(1..=5).contains(&id) {
        return Err("worker id".into());
    }
    let c = context(&q["frozen_context"], id)?;
    let ct = materials::states(&q["frozen_context"]["frozen_ciphertexts"])?;
    // Replay ONLY the designated key's RNG checkpoint from the research bootstrap.
    let mut auth_rng = ChaCha20Rng::seed_from_u64(20_261_028);
    auth_rng.set_word_pos(
        q["authentication_rng_word_pos"]
            .as_str()
            .ok_or("auth rng offset")?
            .parse()?,
    );
    let auth = AuthenticationKeypair::generate(&mut auth_rng);
    if hex(&encode_point(&auth.public)) != q["authentication_public_key_hex"] {
        return Err("worker authentication anchor".into());
    }
    let vk = point(&q["verification_share_hex"], false)?;
    let mut rng = ChaCha20Rng::seed_from_u64(0x4731_3043_5000_0000 ^ u64::from(id));
    let cp = prove_decryption_share(&c, scalar(&q["secret_share_hex"])?, &vk, &ct, &mut rng)?;
    let (private, public) = create_private_package(c, vk, cp.clone(), &auth, &mut rng)?;
    let response = json!({"trustee_id":id,"process_id":std::process::id(),
        "cp":{"shares":cp.shares.iter().map(|p|hex(&encode_point(p))).collect::<Vec<_>>(),
        "a0":hex(&encode_point(&cp.proof.a0)),"coordinates":cp.proof.coordinates.iter().map(|p|hex(&encode_point(p))).collect::<Vec<_>>(),
        "response":hex(&encode_scalar(&cp.proof.response))},"opening":hex(&encode_scalar(&private.opening())),
        "public_record":{"trustee_id":id,"verification_share_hex":hex(&encode_point(&vk)),
        "authentication_public_key_hex":hex(&encode_point(&auth.public)),"package_commitment_hex":hex(&encode_point(&public.commitment.point)),
        "commitment_signature_hex":hex(&public.signature.canonical_bytes())}});
    std::io::stdout().write_all(&serde_json::to_vec(&response)?)?;
    Ok(())
}
pub struct Checked {
    pub trustee: P3B1TrusteeCircuit,
    pub verified: VerifiedDecryptionShare,
    pub row: Value,
}
pub fn check(
    context_value: &Value,
    ct: &[Ciphertext],
    wire: &Value,
    auth: &Point,
    vk: &Point,
) -> Result<Checked> {
    let id = u32::try_from(wire["trustee_id"].as_u64().ok_or("wire id")?)?;
    let row = &wire["public_record"];
    if row["trustee_id"] != id
        || point(&row["authentication_public_key_hex"], false)? != *auth
        || point(&row["verification_share_hex"], false)? != *vk
    {
        return Err("wire/certified roster identity".into());
    }
    let ctx = context(context_value, id)?;
    let cp = &wire["cp"];
    let package = DecryptionSharePackage {
        shares: cp["shares"]
            .as_array()
            .ok_or("shares")?
            .iter()
            .map(|p| point(p, true))
            .collect::<Result<Vec<_>>>()?,
        proof: GeneralizedCpProof {
            a0: point(&cp["a0"], false)?,
            coordinates: cp["coordinates"]
                .as_array()
                .ok_or("coords")?
                .iter()
                .map(|p| point(p, true))
                .collect::<Result<Vec<_>>>()?,
            response: scalar(&cp["response"])?,
        },
    };
    let opening = scalar(&wire["opening"])?;
    let commitment = PackageCommitment {
        point: point(&row["package_commitment_hex"], true)?,
    };
    let bytes = materials::decode(
        row["commitment_signature_hex"]
            .as_str()
            .ok_or("signature")?,
    )?;
    if bytes.len() != 66 {
        return Err("signature size".into());
    }
    let signature = SchnorrSignature {
        nonce: decode_point(&bytes[..32], false)?,
        response: decode_scalar(&bytes[32..64])?,
        rejection_counter: u16::from_be_bytes(bytes[64..].try_into()?),
    };
    if !verify_signature_with_domain(
        auth,
        PACKAGE_SIGNATURE_DOMAIN,
        &commitment_message(&ctx, &commitment),
        &signature,
    ) {
        return Err("package authentication".into());
    }
    let encoded = package_bytes(&ctx, vk, &package);
    let expected = generator().mul_bigint(package_hash_to_field_v1(&encoded).into_bigint())
        + derive_secondary_generator(b"PEDERSEN-H")?.mul_bigint(opening.into_bigint());
    if expected != commitment.point {
        return Err("package opening".into());
    }
    let verified = verify_decryption_share(&ctx, vk, ct, &package)?;
    let (transcript, _) = transcript_and_challenge(&ctx, vk, ct, &package)?;
    if transcript[..vv_friendly_crypto::p3a_gadget::P3A_PUBLIC_PREFIX_BYTES_PER_TRUSTEE]
        != materials::prefix(context_value, row, ct, id)?
    {
        return Err("CP public prefix".into());
    }
    Ok(Checked {
        trustee: P3B1TrusteeCircuit {
            cp: H4ACircuit {
                transcript,
                verification_share: *vk,
                ciphertext_c0: ct.iter().map(|c| c.c0).collect(),
                decryption_shares: package.shares,
                a0: package.proof.a0,
                proof_coordinates: package.proof.coordinates,
                response: package.proof.response,
            },
            package_commitment: commitment.point,
            opening,
        },
        verified,
        row: row.clone(),
    })
}
pub fn deliver(dkg: &DkgOutput, frozen: &Value) -> Result<(Vec<Value>, Value)> {
    let mut rng = ChaCha20Rng::seed_from_u64(20_261_028);
    let _authority = RegistrationAuthority::generate(&mut rng);
    let mut children = Vec::new();
    let mut evidence = Vec::new();
    for id in 1u32..=5 {
        let word_pos = rng.get_word_pos();
        let auth = AuthenticationKeypair::generate(&mut rng);
        if auth.public != dkg.config.authentication_public_keys[id as usize - 1] {
            return Err("bootstrap auth checkpoint".into());
        }
        let input = serde_json::to_vec(
            &json!({"trustee_id":id,"frozen_context":frozen,"secret_share_hex":hex(&encode_scalar(&dkg.secret_share(id)?)),
            "verification_share_hex":hex(&encode_point(dkg.verification_share(id)?)),"authentication_public_key_hex":hex(&encode_point(&auth.public)),
            "authentication_rng_word_pos":word_pos.to_string()}),
        )?;
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--trustee-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or("worker input")?
            .write_all(&input)?;
        evidence.push(
            json!({"trustee_id":id,"process_id":child.id(),"private_input_bytes":input.len()}),
        );
        children.push(child);
    }
    let mut wires = Vec::new();
    for (i, child) in children.into_iter().enumerate() {
        let result = child.wait_with_output()?;
        if !result.status.success() {
            return Err("trustee process failed".into());
        }
        let wire: Value = serde_json::from_slice(&result.stdout)?;
        if wire["process_id"] != evidence[i]["process_id"] || wire["trustee_id"] != i + 1 {
            return Err("worker response identity".into());
        }
        evidence[i]["private_response_bytes"] = json!(result.stdout.len());
        evidence[i]["exited_successfully"] = json!(true);
        wires.push(wire);
    }
    let unique = evidence
        .iter()
        .map(|e| e["process_id"].as_u64().unwrap())
        .collect::<std::collections::HashSet<_>>();
    if unique.len() != 5 || unique.contains(&u64::from(std::process::id())) {
        return Err("not five separate workers".into());
    }
    Ok((
        wires,
        json!({"workers":evidence,"unique_process_ids":5,"private_delivery":"OS stdin/stdout pipes, no private state files or command-line secrets",
        "centralized_research_dkg_bootstrap":true,"independent_distributed_dkg":false,"production_transport":false,
        "research_seeds_public_no_operational_secrecy":true,"private_payloads_archived":false}),
    ))
}
pub fn public_full_tally(
    p: &Value,
    anchors: &[(Point, Point)],
    frozen: &Value,
) -> Result<Vec<u64>> {
    if p["mode"] != "COUNTERFACTUAL_PUBLIC_DECRYPTION_BASELINE"
        || p["authenticated_winner_only_policy_authorizes_this_release"] != false
        || p["frozen_context"] != *frozen
    {
        return Err("Full Tally diagnostic scope/target".into());
    }
    let ct = materials::states(&frozen["frozen_ciphertexts"])?;
    let wires = p["released_packages"].as_array().ok_or("public packages")?;
    if wires.len() != 3 {
        return Err("threshold packages".into());
    }
    let subset = [1u32, 2, 3];
    let mut checked = Vec::new();
    for (wire, id) in wires.iter().zip(subset) {
        if wire["trustee_id"] != id {
            return Err("sorted exact subset".into());
        }
        checked.push(check(
            frozen,
            &ct,
            wire,
            &anchors[id as usize - 1].0,
            &anchors[id as usize - 1].1,
        )?);
    }
    let coeff = vv_friendly_dkg::lagrange_at_zero(&subset, 5, 3)?;
    let pk = anchors[..3]
        .iter()
        .zip(&coeff)
        .fold(Point::default(), |acc, ((_, vk), lambda)| {
            acc + vk.mul_bigint(lambda.into_bigint())
        });
    if pk != decode_point(&materials::fixed(frozen, "public_key_hex")?, false)? {
        return Err("interpolated certified public key".into());
    }
    let mut tally = Vec::new();
    for (j, ctj) in ct.iter().enumerate() {
        let decryption = checked
            .iter()
            .zip(&coeff)
            .fold(Point::default(), |acc, (p, l)| {
                acc + p.verified.shares()[j].mul_bigint(l.into_bigint())
            });
        tally.push(vv_friendly_crypto::bounded_discrete_log(
            &(ctj.c1 - decryption),
            32,
        )?);
    }
    if p["tally"] != json!(tally) || p["winner"] != vv_types::winner(&tally, 32)? {
        return Err("public tally/winner mismatch".into());
    }
    Ok(tally)
}
