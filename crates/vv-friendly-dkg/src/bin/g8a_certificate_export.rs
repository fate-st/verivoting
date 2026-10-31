//! Original G6 DKG/registration certificate reconstruction and PUBLIC-only export.
//! No Groth16 setup/proving and no re-signing under a different scheme.
#[allow(dead_code)]
#[path = "common/g6_materials.rs"]
mod materials;
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use materials::{fixed, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha512};
use std::{fs, io::Write, path::Path, process::Command};
use vv_friendly_crypto::{
    encode_point, encode_scalar, hash_to_scalar, hash_to_scalar_input, Point, SUITE_ID,
};
use vv_friendly_dkg::registration::CLOSE_SIGNATURE_DOMAIN;
use vv_friendly_dkg::{verify_signature_with_domain, SchnorrSignature};
use vv_types::{hex, sha256, CanonicalEncoder};
const OUTPUT: &str = "fixtures/g8a_original_certificates_public_v1.json";
const REPORT: &str = "experiments/g8a_raw/public_export.json";
fn point(p: &Point) -> Value {
    let a = p.into_affine();
    json!({"x":a.x.into_bigint().to_string(),"y":a.y.into_bigint().to_string(),"encoded_hex":hex(&encode_point(p))})
}
fn signature(
    public: &Point,
    domain: &[u8],
    message: &[u8; 32],
    sig: &SchnorrSignature,
) -> Result<Value> {
    if !verify_signature_with_domain(public, domain, message, sig) {
        return Err("original signature invalid".into());
    }
    let mut t = CanonicalEncoder::new(b"VV/JUBJUB/SCHNORR/TRANSCRIPT/v2");
    t.text(SUITE_ID);
    t.bytes(domain);
    t.fixed(&encode_point(public));
    t.fixed(&encode_point(&sig.nonce));
    t.bytes(message);
    let transcript = t.finish();
    let (challenge, counter) = hash_to_scalar(b"SCHNORR-SIGNATURE", &transcript)?;
    if counter != sig.rejection_counter {
        return Err("signature counter".into());
    }
    let preimages = (0..=counter)
        .map(|i| {
            let b = hash_to_scalar_input(b"SCHNORR-SIGNATURE", &transcript, i);
            json!({"counter":i,"input_hex":hex(&b),"sha512_hex":hex(&Sha512::digest(&b))})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"public_key":point(public),"domain_hex":hex(domain),"message_hex":hex(message),
        "signature_hex":hex(&sig.canonical_bytes()),"nonce":point(&sig.nonce),"response_scalar":sig.response.into_bigint().to_string(),
        "counter":counter,"challenge_scalar":challenge.into_bigint().to_string(),"challenge_le_hex":hex(&encode_scalar(&challenge)),"hash_attempts":preimages}),
    )
}
fn write_new(p: &str, v: &Value) -> Result<()> {
    fs::create_dir_all(Path::new(p).parent().ok_or("parent")?)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)?;
    file.write_all(&serde_json::to_vec_pretty(v)?)?;
    file.write_all(b"\n")?;
    Ok(())
}
fn main() -> Result<()> {
    if Path::new(OUTPUT).exists() || Path::new(REPORT).exists() {
        return Err("new output paths required".into());
    }
    let sources = materials::load_sources()?;
    let (dkg, _keys, registration, info) = materials::rebuild_dkg(&sources)?;
    let g6path = "fixtures/jubjub_g6_native_winner_only_public_v1.json";
    let g6bytes = fs::read(g6path)?;
    if hex(&sha256(&g6bytes)) != "ca58fb6cd23258e1edbbdcd96ad7a5d5f93ca20b7af0ab7f8b330f7c02cbef78"
    {
        return Err("G6 pin".into());
    }
    let g6: Value = serde_json::from_slice(&g6bytes)?;
    let context = &g6["public_context"];
    for k in [
        "election_id_hex",
        "policy_pre_hex",
        "prekey_context_hex",
        "dkg_session_hex",
        "key_core_digest_hex",
        "final_context_hex",
        "registration_root_hex",
        "registration_close_digest_hex",
        "disclosure_digest_hex",
        "registration_certificate_hex",
    ] {
        if info[k] != context[k] {
            return Err(format!("G6 original context differs {k}").into());
        }
    }
    if dkg.config.q_cert != dkg.config.n - dkg.config.t || dkg.config.tau != dkg.config.t + 1 {
        return Err("threshold roles".into());
    }
    let mut signatures = Vec::new();
    for (id, sig) in &dkg.certificate.signatures {
        let mut row = signature(
            &dkg.config.authentication_public_keys[*id as usize - 1],
            b"DKG-CERT-SIGNATURE",
            &dkg.certificate.key_core_digest,
            sig,
        )?;
        row["trustee_id"] = json!(id);
        signatures.push(row);
    }
    let cert = registration.certificate();
    let close = &cert.close;
    if hex(&cert.canonical_bytes()) != context["registration_certificate_hex"] {
        return Err("not original registration certificate".into());
    }
    let authority =
        vv_friendly_crypto::decode_point(&fixed(context, "registration_authority_hex")?, false)?;
    let mut m = CanonicalEncoder::new(b"VV/JUBJUB/REGISTER/CLOSE/v1");
    m.bytes(&close.canonical_bytes());
    let message = sha256(&m.finish());
    let reg = signature(
        &authority,
        CLOSE_SIGNATURE_DOMAIN,
        &message,
        &cert.signature,
    )?;
    let cfg = &dkg.config;
    let mut prefix = CanonicalEncoder::new(b"VV/JUBJUB/KEYCORE/v2");
    prefix.text(SUITE_ID);
    for b in [
        cfg.election_id,
        cfg.policy_pre,
        cfg.manifest_digest,
        cfg.prekey_context,
        cfg.session_id,
    ] {
        prefix.fixed(&b);
    }
    for n in [
        cfg.n,
        cfg.t,
        cfg.tau,
        cfg.q_cert,
        cfg.authentication_public_keys.len(),
    ] {
        prefix.u64(n as u64);
    }
    for (i, key) in cfg.authentication_public_keys.iter().enumerate() {
        prefix.u32((i + 1) as u32);
        prefix.fixed(&encode_point(key));
    }
    prefix.u64(dkg.qual.len() as u64);
    for id in &dkg.qual {
        prefix.u32(*id);
    }
    for p in &dkg.public_coefficients {
        prefix.fixed(&encode_point(p));
    }
    prefix.fixed(&dkg.transcript.public_digest);
    let keycore = prefix.finish();
    if sha256(&keycore) != dkg.certificate.key_core_digest {
        return Err("keycore bytes".into());
    }
    let output = json!({"schema":"VV/G8-A/ORIGINAL-CERTIFICATES/PUBLIC/v1","public_context":context,
        "manifest_digest_hex":hex(&cfg.manifest_digest),"authentication_keys":cfg.authentication_public_keys.iter().map(point).collect::<Vec<_>>(),
        "registration_authority":point(&authority),"public_coefficients":dkg.public_coefficients.iter().map(point).collect::<Vec<_>>(),
        "verification_shares":dkg.verification_shares.iter().map(point).collect::<Vec<_>>(),"qual":dkg.qual,
        "n":cfg.n,"t":cfg.t,"tau":cfg.tau,"q_cert":cfg.q_cert,"transcript_digest_hex":hex(&dkg.transcript.public_digest),
        "keycore_bytes_hex":hex(&keycore),"dkg_certificate_signatures":signatures,"registration_signature":reg,
        "close":{"election_id_hex":hex(&close.election_id),"final_context_hex":hex(&close.final_context),"policy_pre_hex":hex(&close.policy_pre),
        "root_hex":hex(&close.root_close),"n_registered":close.n_registered,"w_registered":close.w_registered,"w_max":close.w_max,
        "version":close.version,"canonical_registry_digest_hex":hex(&close.canonical_registry_digest),"canonical_bytes_hex":hex(&close.canonical_bytes())},
        "generator":point(&vv_friendly_crypto::generator()),"public_signature_count":signatures.len()+1,
        "original_registration_bytes_match_g6":true,"source_g6_fixture":g6path,"source_g6_fixture_sha256":hex(&sha256(&g6bytes)),
        "groth16_setup_count":0,"groth16_proof_generation_count":0,"signatures_resigned":false,
        "private_witness_emitted":false,"production_setup":false});
    // Reconstructed public material only. Native DKG used the original research seed,
    // but shares/keys/registry openings are never serialized by this exporter.
    let mut pins = sources.pins;
    pins.push(json!({"path":g6path,"sha256":hex(&sha256(&g6bytes))}));
    let source_hashes = [
        "crates/vv-friendly-dkg/src/bin/g8a_certificate_export.rs",
        "crates/vv-friendly-dkg/src/bin/common/g6_materials.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-dkg/src/registration.rs",
        "crates/vv-friendly-crypto/src/lib.rs",
        "crates/vv-friendly-dkg/Cargo.toml",
        "Cargo.lock",
    ]
    .iter()
    .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
    .collect::<Result<Vec<_>>>()?;
    write_new(OUTPUT, &output)?;
    let git = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    write_new(
        REPORT,
        &json!({"gate":"G8-A-public-export","status":"PASS_ORIGINAL_DKG_AND_CLOSE_CERTIFICATE_NATIVE_RECONSTRUCTION",
        "git_commit":String::from_utf8(git.stdout)?.trim(),"git_dirty":!Command::new("git").args(["status","--porcelain"]).output()?.stdout.is_empty(),
        "public_signatures_native_verified":6,"q_cert":cfg.q_cert,"tau":cfg.tau,"native_dkg_reconstruction_ms":info["rebuild_ms"],
        "original_registration_bytes_match_g6":true,"keycore_matches_g6":true,"signatures_resigned":false,
        "output_fixture":OUTPUT,"output_fixture_sha256":hex(&sha256(&fs::read(OUTPUT)?)),"source_pins":pins,"source_hashes":source_hashes,
        "groth16_setup_count":0,"groth16_proof_generation_count":0,"private_witness_emitted":false}),
    )?;
    println!("G8-A export PASS: original deterministic DKG/close reconstructed; six native signatures verified; no new SNARK setup/prove or replacement signatures");
    Ok(())
}
