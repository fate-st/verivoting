//! Public-only export of this run's new worker commitments; original algorithms unchanged.
use ark_ec::{CurveGroup, Group};
use ark_ff::PrimeField;
use serde_json::{json, Value};
use std::{fs, io::Write, path::Path, process::Command};
use vv_friendly_crypto::cp::{derive_cp_target_with_registration, GeneralizedCpContext};
use vv_friendly_crypto::{
    decode_point, decode_scalar, encode_point, hash_to_scalar, Ciphertext, Point, Scalar, SUITE_ID,
};
use vv_friendly_dkg::package::{commitment_message, PackageCommitment, PACKAGE_SIGNATURE_DOMAIN};
use vv_friendly_dkg::{verify_signature_with_domain, SchnorrSignature};
use vv_types::{hex, sha256, CanonicalEncoder};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const G8A: &str = "fixtures/g8a_original_certificates_public_v1.json";
fn bytes(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return Err("odd hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| Ok(u8::from_str_radix(&s[i..i + 2], 16)?))
        .collect()
}
fn fixed(v: &Value, key: &str) -> Result<[u8; 32]> {
    bytes(v[key].as_str().ok_or("missing hex")?)?
        .try_into()
        .map_err(|_| "32 bytes required".into())
}
fn point(p: &Point) -> Value {
    let a = p.into_affine();
    json!({"x":a.x.into_bigint().to_string(),"y":a.y.into_bigint().to_string(),"encoded_hex":hex(&encode_point(p))})
}
fn write_new(p: &str, value: &Value) -> Result<()> {
    fs::create_dir_all(Path::new(p).parent().ok_or("parent")?)?;
    let mut f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(p)?;
    f.write_all(&serde_json::to_vec_pretty(value)?)?;
    f.write_all(b"\n")?;
    Ok(())
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: g10c_commitment_export PUBLIC_NATIVE NEW_FIXTURE NEW_REPORT".into());
    }
    let input_path = args[0].as_str();
    let output_path = args[1].as_str();
    let report_path = args[2].as_str();
    let input_hash = hex(&sha256(&fs::read(input_path)?));
    if Path::new(output_path).exists() || Path::new(report_path).exists() {
        return Err("new output paths required".into());
    }
    let pins = [
        (input_path, input_hash.as_str()),
        (
            G8A,
            "75e724ac0644702cbb864fafc8e54ffa0f64042bb08483038ad680db3f021b8e",
        ),
    ];
    for (p, h) in pins {
        if hex(&sha256(&fs::read(p)?)) != h {
            return Err(format!("pin {p}").into());
        }
    }
    let g6: Value = serde_json::from_slice(&fs::read(input_path)?)?;
    let a: Value = serde_json::from_slice(&fs::read(G8A)?)?;
    let c = &g6["public_context"];
    if a["public_context"] != *c {
        return Err("original certificate context".into());
    }
    let coeff = a["public_coefficients"]
        .as_array()
        .ok_or("coefficients")?
        .iter()
        .map(|p| Ok(decode_point(&fixed(p, "encoded_hex")?, true)?))
        .collect::<Result<Vec<_>>>()?;
    if coeff.len() != 3 {
        return Err("degree two".into());
    }
    let ciphertexts = c["frozen_ciphertexts"]
        .as_array()
        .ok_or("CT2")?
        .iter()
        .map(|p| {
            Ok(Ciphertext {
                c0: decode_point(&fixed(p, "c0_hex")?, true)?,
                c1: decode_point(&fixed(p, "c1_hex")?, true)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let target = derive_cp_target_with_registration(
        fixed(c, "election_id_hex")?,
        fixed(c, "policy_pre_hex")?,
        fixed(c, "final_context_hex")?,
        fixed(c, "registration_close_digest_hex")?,
        fixed(c, "disclosure_digest_hex")?,
        &ciphertexts,
    )?;
    if hex(&target) != c["cp_target_hex"].as_str().ok_or("target")? {
        return Err("CP target".into());
    }
    let mut records = Vec::new();
    for row in g6["trustee_roster"].as_array().ok_or("roster")? {
        let id = u32::try_from(row["trustee_id"].as_u64().ok_or("id")?)?;
        if !(1..=5).contains(&id) {
            return Err("trustee ID".into());
        }
        let key = decode_point(&fixed(row, "authentication_public_key_hex")?, false)?;
        let vk = decode_point(&fixed(row, "verification_share_hex")?, false)?;
        let i = Scalar::from(u64::from(id));
        if coeff[0]
            + coeff[1].mul_bigint(i.into_bigint())
            + coeff[2].mul_bigint((i * i).into_bigint())
            != vk
            || row["authentication_public_key_hex"]
                != a["authentication_keys"][(id - 1) as usize]["encoded_hex"]
            || row["verification_share_hex"]
                != a["verification_shares"][(id - 1) as usize]["encoded_hex"]
        {
            return Err("KeyCore membership".into());
        }
        let commitment = PackageCommitment {
            point: decode_point(&fixed(row, "package_commitment_hex")?, true)?,
        };
        let wire = bytes(
            row["commitment_signature_hex"]
                .as_str()
                .ok_or("signature")?,
        )?;
        if wire.len() != 66 {
            return Err("signature length".into());
        }
        let signature = SchnorrSignature {
            nonce: decode_point(&wire[..32], false)?,
            response: decode_scalar(&wire[32..64])?,
            rejection_counter: u16::from_be_bytes(wire[64..].try_into()?),
        };
        if signature.canonical_bytes() != wire {
            return Err("canonical signature".into());
        }
        let ctx = GeneralizedCpContext {
            election_id: fixed(c, "election_id_hex")?,
            policy_pre: fixed(c, "policy_pre_hex")?,
            final_context: fixed(c, "final_context_hex")?,
            registration_close_digest: fixed(c, "registration_close_digest_hex")?,
            disclosure_digest: fixed(c, "disclosure_digest_hex")?,
            target,
            trustee_id: id,
        };
        let message = commitment_message(&ctx, &commitment);
        if !verify_signature_with_domain(&key, PACKAGE_SIGNATURE_DOMAIN, &message, &signature) {
            return Err("original package signature".into());
        }
        let mut transcript = CanonicalEncoder::new(b"VV/JUBJUB/SCHNORR/TRANSCRIPT/v2");
        transcript.text(SUITE_ID);
        transcript.bytes(PACKAGE_SIGNATURE_DOMAIN);
        transcript.fixed(&encode_point(&key));
        transcript.fixed(&encode_point(&signature.nonce));
        transcript.bytes(&message);
        let (challenge, counter) = hash_to_scalar(b"SCHNORR-SIGNATURE", &transcript.finish())?;
        if counter != signature.rejection_counter {
            return Err("counter".into());
        }
        records.push(json!({"trustee_id":id,"authentication_key":point(&key),"verification_share":point(&vk),
            "commitment":point(&commitment.point),"signature":{"nonce":point(&signature.nonce),"response_scalar":signature.response.into_bigint().to_string(),
                "counter":counter,"signature_hex":hex(&wire)},"original_message_hex":hex(&message),"original_challenge_scalar":challenge.into_bigint().to_string(),
            "original_signature_bytes_match_g6":true,"verification_share_matches_keycore":true}));
    }
    if records.len() != 5 {
        return Err("five public records required".into());
    }
    let output = json!({"schema":"VV/G8-B/ORIGINAL-TRUSTEE-COMMITMENTS/PUBLIC/v1","public_context":c,
        "freeze_digest_hex":g6["freeze_digest_hex"],"records":records,"original_domain_hex":hex(PACKAGE_SIGNATURE_DOMAIN),
        "source_g6_fixture":input_path,"source_g6_fixture_sha256":hex(&sha256(&fs::read(input_path)?)),
        "source_g8a_fixture":G8A,"source_g8a_fixture_sha256":hex(&sha256(&fs::read(G8A)?)),
        "setup_count":0,"proof_generation_count":0,"new_cp_packages":0,"new_signatures":0,"dkg_reexecuted":false,"private_witness_emitted":false});
    let source_paths = [
        file!(),
        "crates/vv-friendly-dkg/src/package.rs",
        "crates/vv-friendly-dkg/src/lib.rs",
        "crates/vv-friendly-crypto/src/cp.rs",
        "crates/vv-friendly-crypto/src/lib.rs",
        "Cargo.lock",
    ];
    let source_hashes = source_paths
        .iter()
        .map(|p| Ok(json!({"path":p,"sha256":hex(&sha256(&fs::read(p)?))})))
        .collect::<Result<Vec<_>>>()?;
    write_new(output_path, &output)?;
    write_new(
        report_path,
        &json!({"gate":"G8-B-public-export","status":"PASS_FIVE_CURRENT_WORKER_COMMITMENTS_AND_CERTIFIED_SHARES",
        "git_commit":String::from_utf8(Command::new("git").args(["rev-parse","HEAD"]).output()?.stdout)?.trim(),
        "git_dirty":!Command::new("git").args(["status","--porcelain"]).output()?.stdout.is_empty(),
        "source_pins":pins.iter().map(|(p,h)|json!({"path":p,"sha256":h})).collect::<Vec<_>>(),"source_hashes":source_hashes,
        "public_signature_count_native_verified":5,"verification_share_polynomial_checks":5,"output_fixture":output_path,
        "output_fixture_sha256":hex(&sha256(&fs::read(output_path)?)),"setup_count":0,"proof_generation_count":0,
        "new_cp_packages":0,"new_signatures":0,"dkg_reexecuted":false,"private_witness_emitted":false}),
    )?;
    println!("G8-B public export PASS: five exact archived input_path signatures, five public polynomial evaluations; no DKG/CP/signing/Setup/Prove");
    Ok(())
}
