//! Serial in-memory receiving -> frozen -> disclosed state machine. Not a contract.
use super::{materials, receiver};
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
use serde_json::{json, Value};
use vv_friendly_crypto::cp::{derive_cp_target_with_registration, GeneralizedCpContext};
use vv_friendly_crypto::p3ten_gadget::{G4D_INPUTS, G4D_SUBSETS};
use vv_friendly_crypto::state_gadget::{StateBallotRecord, StateTransitionCircuitForBatch};
use vv_friendly_crypto::{decode_point, decode_scalar, encode_point, Ciphertext, Point};
use vv_friendly_dkg::package::{commitment_message, PackageCommitment, PACKAGE_SIGNATURE_DOMAIN};
use vv_friendly_dkg::{verify_signature_with_domain, SchnorrSignature};
use vv_types::{hex, sha256, CanonicalEncoder};

pub(super) const RELEASE: &str = "WINNER_ONLY_V1";
const RULE: &str = "highest-weight-smallest-id-on-tie";
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Rejection {
    Frozen,
    Incomplete,
    Binding,
    Roster,
    Target,
    Key,
    Proof,
    Disclosed,
    Batch(receiver::Rejection),
}

#[cfg(test)]
mod tests {
    use super::*;
    use vv_friendly_crypto::state_gadget::{batch_root_v2_for_batch, fold_state_for_batch};
    use vv_friendly_crypto::{generator, Scalar};

    #[test]
    fn incomplete_freeze_rejects_and_live_state_can_only_be_frozen_once() {
        // Bookkeeping-only unit test; actual cryptographic evidence is the Release G6 run.
        let records = |start: u8| {
            (start..start + 4)
                .map(|id| StateBallotRecord {
                    election_id: [1; 32],
                    final_context: [2; 32],
                    region: 4,
                    registration_root: [3; 32],
                    key_set_hash: [4; 32],
                    nullifier: [id; 32],
                    public_key: generator(),
                    ciphertexts: vec![
                        Ciphertext {
                            c0: generator() * Scalar::from(u64::from(id)),
                            c1: generator()
                        };
                        3
                    ],
                })
                .collect::<Vec<_>>()
        };
        let circuit = |index: u64, old: Vec<Ciphertext>, records: Vec<StateBallotRecord>| {
            StateTransitionCircuitForBatch::<4> {
                election_id: [1; 32],
                final_context: [2; 32],
                region: 4,
                batch_index: index,
                batch_size: 4,
                batch_root: batch_root_v2_for_batch::<4>(&[1; 32], &[2; 32], 4, index, &records)
                    .unwrap(),
                key_set_hash: [4; 32],
                public_key: generator(),
                new_state: fold_state_for_batch::<4>(&old, &records).unwrap(),
                old_state: old,
                records,
            }
        };
        let first = circuit(1, vec![Ciphertext::zero(); 3], records(10));
        let second = circuit(2, first.new_state.clone(), records(20));
        let context = json!({"election_id_hex":hex(&[1;32]),"final_context_hex":hex(&[2;32]),
            "registration_root_hex":hex(&[3;32]),"key_core_digest_hex":hex(&[4;32]),
            "public_key_hex":hex(&encode_point(&generator())),"registered_weight":32,
            "policy_pre_hex":hex(&[5;32]),"dkg_session_hex":hex(&[6;32]),
            "registration_close_digest_hex":hex(&[7;32]),"disclosure_digest_hex":hex(&[8;32])});
        let mut flow = WinnerOnly::new(
            first.records[0].clone(),
            vec![first.batch_root, second.batch_root],
            context,
            vec![(generator(), generator()); 5],
            "00".repeat(32),
        )
        .unwrap();
        assert!(matches!(flow.freeze(), Err(Rejection::Incomplete)));
        flow.accept(&first, || Ok(true)).unwrap();
        assert!(matches!(flow.freeze(), Err(Rejection::Incomplete)));
        flow.accept(&second, || Ok(true)).unwrap();
        let frozen = flow.freeze().unwrap();
        assert_eq!(frozen.ciphertexts, second.new_state);
        let before = flow.snapshot();
        assert!(matches!(flow.freeze(), Err(Rejection::Frozen)));
        assert_eq!(
            flow.accept(&first, || panic!("must not run")),
            Err(Rejection::Frozen)
        );
        assert_eq!(flow.snapshot(), before);
        assert_eq!(before["winner_write_count"], 0);
    }
}
#[derive(Clone)]
pub(super) struct Frozen {
    pub(super) context: Value,
    pub(super) ciphertexts: Vec<Ciphertext>,
    pub(super) digest: [u8; 32],
}
pub(super) struct WinnerOnly {
    receiver: receiver::Receiver,
    context: Value,
    roots: Vec<[u8; 32]>,
    anchors: Vec<(Point, Point)>,
    vk_hash: String,
    frozen: Option<Frozen>,
    roster: Option<Vec<Value>>,
    winner: Option<u64>,
    writes: u64,
}

pub(super) fn context_for(context: &Value, ct: &[Ciphertext]) -> materials::Result<Value> {
    let mut out = json!({});
    for key in [
        "election_id_hex",
        "policy_pre_hex",
        "dkg_session_hex",
        "key_core_digest_hex",
        "public_key_hex",
        "prekey_context_hex",
        "final_context_hex",
        "registration_root_hex",
        "registration_close_digest_hex",
        "disclosure_digest_hex",
        "registration_certificate_hex",
        "registration_authority_hex",
        "registered_weight",
        "w_max",
        "n_registered",
    ] {
        out[key] = context[key].clone();
    }
    let target = derive_cp_target_with_registration(
        materials::fixed(context, "election_id_hex")?,
        materials::fixed(context, "policy_pre_hex")?,
        materials::fixed(context, "final_context_hex")?,
        materials::fixed(context, "registration_close_digest_hex")?,
        materials::fixed(context, "disclosure_digest_hex")?,
        ct,
    )?;
    out["cp_target_hex"] = json!(hex(&target));
    out["frozen_ciphertexts"] = json!(materials::ciphertexts(ct));
    out["release_policy"] = json!(RELEASE);
    out["winning_rule"] = json!(RULE);
    Ok(out)
}

impl WinnerOnly {
    pub(super) fn new(
        binding: StateBallotRecord,
        roots: Vec<[u8; 32]>,
        context: Value,
        anchors: Vec<(Point, Point)>,
        vk_hash: String,
    ) -> materials::Result<Self> {
        if roots.len() != 2
            || anchors.len() != 5
            || context["registered_weight"] != 32
            || binding.election_id != materials::fixed(&context, "election_id_hex")?
            || binding.final_context != materials::fixed(&context, "final_context_hex")?
            || binding.registration_root != materials::fixed(&context, "registration_root_hex")?
            || binding.key_set_hash != materials::fixed(&context, "key_core_digest_hex")?
            || encode_point(&binding.public_key) != materials::fixed(&context, "public_key_hex")?
        {
            return Err("receiver/certified DKG/registration binding differs".into());
        }
        Ok(Self {
            receiver: receiver::Receiver::new(binding, roots.clone()),
            context,
            roots,
            anchors,
            vk_hash,
            frozen: None,
            roster: None,
            winner: None,
            writes: 0,
        })
    }
    pub(super) fn accept(
        &mut self,
        c: &StateTransitionCircuitForBatch<4>,
        verify: impl FnOnce() -> materials::Result<bool>,
    ) -> Result<(), Rejection> {
        if self.frozen.is_some() {
            return Err(Rejection::Frozen);
        }
        self.receiver.apply(c, verify).map_err(Rejection::Batch)
    }
    pub(super) fn freeze(&mut self) -> Result<Frozen, Rejection> {
        if self.frozen.is_some() {
            return Err(Rejection::Frozen);
        }
        if self.receiver.next() != 3 || self.receiver.spent_count() != 8 {
            return Err(Rejection::Incomplete);
        }
        let ct = self.receiver.state().to_vec();
        let context = context_for(&self.context, &ct).map_err(|_| Rejection::Binding)?;
        let mut e = CanonicalEncoder::new(b"VV/G6/NATIVE-WINNER-ONLY/FROZEN/v1");
        for k in [
            "election_id_hex",
            "policy_pre_hex",
            "dkg_session_hex",
            "key_core_digest_hex",
            "public_key_hex",
            "final_context_hex",
            "registration_root_hex",
            "registration_close_digest_hex",
            "disclosure_digest_hex",
            "cp_target_hex",
        ] {
            e.fixed(&materials::fixed(&context, k).map_err(|_| Rejection::Binding)?);
        }
        e.text(RELEASE);
        e.text(RULE);
        e.text(&self.vk_hash);
        e.u64(32);
        e.u64(self.receiver.next());
        e.u64(self.receiver.spent_count() as u64);
        for root in &self.roots {
            e.fixed(root);
        }
        for (id, c) in ct.iter().enumerate() {
            e.u32((id + 1) as u32);
            e.fixed(&encode_point(&c.c0));
            e.fixed(&encode_point(&c.c1));
        }
        let frozen = Frozen {
            context,
            ciphertexts: ct,
            digest: sha256(&e.finish()),
        };
        self.frozen = Some(frozen.clone());
        Ok(frozen)
    }
    pub(super) fn install_roster(&mut self, roster: Vec<Value>) -> Result<(), Rejection> {
        if self.roster.is_some() || self.winner.is_some() {
            return Err(Rejection::Roster);
        }
        let f = self.frozen.as_ref().ok_or(Rejection::Incomplete)?;
        if roster.len() != 5 {
            return Err(Rejection::Roster);
        }
        let validate = || -> materials::Result<()> {
            for (i, row) in roster.iter().enumerate() {
                let (auth, vk) = self.anchors[i];
                if row["trustee_id"] != i + 1
                    || encode_point(&auth)
                        != materials::fixed(row, "authentication_public_key_hex")?
                    || encode_point(&vk) != materials::fixed(row, "verification_share_hex")?
                {
                    return Err("certified roster mismatch".into());
                }
                let context = GeneralizedCpContext {
                    election_id: materials::fixed(&f.context, "election_id_hex")?,
                    policy_pre: materials::fixed(&f.context, "policy_pre_hex")?,
                    final_context: materials::fixed(&f.context, "final_context_hex")?,
                    registration_close_digest: materials::fixed(
                        &f.context,
                        "registration_close_digest_hex",
                    )?,
                    disclosure_digest: materials::fixed(&f.context, "disclosure_digest_hex")?,
                    target: materials::fixed(&f.context, "cp_target_hex")?,
                    trustee_id: (i + 1) as u32,
                };
                let commitment = PackageCommitment {
                    point: decode_point(&materials::fixed(row, "package_commitment_hex")?, true)?,
                };
                let bytes = materials::decode(
                    row["commitment_signature_hex"]
                        .as_str()
                        .ok_or("signature")?,
                )?;
                if bytes.len() != 66 {
                    return Err("signature length".into());
                }
                let signature = SchnorrSignature {
                    nonce: decode_point(bytes[..32].try_into()?, false)?,
                    response: decode_scalar(&bytes[32..64])?,
                    rejection_counter: u16::from_be_bytes(bytes[64..].try_into()?),
                };
                if !verify_signature_with_domain(
                    &auth,
                    PACKAGE_SIGNATURE_DOMAIN,
                    &commitment_message(&context, &commitment),
                    &signature,
                ) {
                    return Err("package signature".into());
                }
            }
            Ok(())
        };
        validate().map_err(|_| Rejection::Roster)?;
        self.roster = Some(roster);
        Ok(())
    }
    pub(super) fn disclose(
        &mut self,
        winner: u64,
        inputs: &[Fr],
        proof: &Proof<Bls12_381>,
        vk: &VerifyingKey<Bls12_381>,
    ) -> Result<(), Rejection> {
        if self.winner.is_some() {
            return Err(Rejection::Disclosed);
        }
        let f = self.frozen.as_ref().ok_or(Rejection::Incomplete)?;
        let roster = self.roster.as_ref().ok_or(Rejection::Roster)?;
        if !(1..=3).contains(&winner) {
            return Err(Rejection::Target);
        }
        // All 60 public inputs are reconstructed from storage; no caller-supplied target is trusted.
        let expected = materials::expected_inputs(&f.context, roster, &f.ciphertexts, 0, winner)
            .map_err(|_| Rejection::Target)?;
        if inputs != expected {
            return Err(Rejection::Target);
        }
        if vk.gamma_abc_g1.len() != G4D_INPUTS + 1
            || hex(&sha256(
                &materials::compressed(vk).map_err(|_| Rejection::Key)?,
            )) != self.vk_hash
        {
            return Err(Rejection::Key);
        }
        if !Groth16::<Bls12_381>::verify_proof(&prepare_verifying_key(vk), proof, inputs)
            .map_err(|_| Rejection::Proof)?
        {
            return Err(Rejection::Proof);
        }
        self.winner = Some(winner);
        self.writes += 1;
        Ok(())
    }
    pub(super) fn snapshot(&self) -> Value {
        json!({"next_batch_index":self.receiver.next(),"spent_nullifier_count":self.receiver.spent_count(),"state":materials::ciphertexts(self.receiver.state()),"frozen":self.frozen.is_some(),"freeze_digest_hex":self.frozen.as_ref().map(|f|hex(&f.digest)),"public_winner":self.winner,"winner_write_count":self.writes,"release_policy":RELEASE,"selected_subset":G4D_SUBSETS[0]})
    }
}
