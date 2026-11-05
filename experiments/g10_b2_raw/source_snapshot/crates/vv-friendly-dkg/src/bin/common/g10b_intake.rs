//! Single-record intake against the ORIGINAL ordered B4 SHA-256 commitment.
//! Paths are sender-prepared; verification, packing, fold and writes are online.
use std::collections::HashSet;
use vv_friendly_crypto::{
    state_gadget::{ballot_field_vector_bytes, StateBallotRecord},
    Ciphertext,
};
use vv_types::{sha256, CanonicalEncoder};

fn leaf(r: &StateBallotRecord, batch: u64, position: usize) -> Option<[u8; 32]> {
    let mut e = CanonicalEncoder::new(b"VV/BATCH/LEAF/v2");
    e.fixed(&r.election_id);
    e.fixed(&r.final_context);
    e.u64(r.region);
    e.u64(batch);
    e.u64(4);
    e.u64(position as u64);
    e.u64(25);
    e.fixed(&ballot_field_vector_bytes(r)?);
    Some(sha256(&e.finish()))
}
fn node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut e = CanonicalEncoder::new(b"VV/BATCH/NODE/v2");
    e.fixed(left);
    e.fixed(right);
    sha256(&e.finish())
}
pub fn membership_path(
    records: &[StateBallotRecord],
    batch: u64,
    position: usize,
) -> Option<[[u8; 32]; 2]> {
    if records.len() != 4 || position >= 4 {
        return None;
    }
    let leaves = records
        .iter()
        .enumerate()
        .map(|(i, r)| leaf(r, batch, i))
        .collect::<Option<Vec<_>>>()?;
    let other = if position < 2 {
        node(&leaves[2], &leaves[3])
    } else {
        node(&leaves[0], &leaves[1])
    };
    Some([leaves[position ^ 1], other])
}
pub fn verify_membership(
    record: &StateBallotRecord,
    batch: u64,
    position: usize,
    path: &[[u8; 32]; 2],
    root: &[u8; 32],
) -> bool {
    if position >= 4 {
        return false;
    }
    let Some(mut current) = leaf(record, batch, position) else {
        return false;
    };
    for (level, sibling) in path.iter().enumerate() {
        current = if (position >> level) & 1 == 0 {
            node(&current, sibling)
        } else {
            node(sibling, &current)
        };
    }
    &current == root
}

#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    Sequence,
    OldState,
    Binding,
    Membership,
    Nullifier,
    Proof,
}

#[derive(Clone)]
pub struct IndividualReceiver {
    binding: StateBallotRecord,
    roots: Vec<[u8; 32]>,
    state: Vec<Ciphertext>,
    spent: HashSet<[u8; 32]>,
    cursor: usize,
}
impl IndividualReceiver {
    pub fn new(binding: StateBallotRecord, roots: Vec<[u8; 32]>) -> Self {
        Self {
            binding,
            roots,
            state: vec![Ciphertext::zero(); 3],
            spent: HashSet::new(),
            cursor: 0,
        }
    }
    pub fn state(&self) -> &[Ciphertext] {
        &self.state
    }
    pub fn spent(&self) -> &HashSet<[u8; 32]> {
        &self.spent
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn apply(
        &mut self,
        record: &StateBallotRecord,
        batch: u64,
        position: usize,
        old: &[Ciphertext],
        path: &[[u8; 32]; 2],
        verify: impl FnOnce() -> bool,
    ) -> Result<(), Rejection> {
        if batch != (self.cursor / 4 + 1) as u64
            || position != self.cursor % 4
            || self.cursor >= self.roots.len() * 4
        {
            return Err(Rejection::Sequence);
        }
        if old != self.state {
            return Err(Rejection::OldState);
        }
        let b = &self.binding;
        if record.election_id != b.election_id
            || record.final_context != b.final_context
            || record.region != b.region
            || record.registration_root != b.registration_root
            || record.key_set_hash != b.key_set_hash
            || record.public_key != b.public_key
            || record.ciphertexts.len() != 3
        {
            return Err(Rejection::Binding);
        }
        if !verify_membership(record, batch, position, path, &self.roots[self.cursor / 4]) {
            return Err(Rejection::Membership);
        }
        if self.spent.contains(&record.nullifier) {
            return Err(Rejection::Nullifier);
        }
        if !verify() {
            return Err(Rejection::Proof);
        }
        // Derive the new state internally, never take an unchecked caller result.
        let mut next = self.state.clone();
        for (sum, ct) in next.iter_mut().zip(&record.ciphertexts) {
            *sum = sum.add(ct);
        }
        self.state = next;
        self.spent.insert(record.nullifier);
        self.cursor += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vv_friendly_crypto::state_gadget::batch_root_v2_for_batch;
    use vv_friendly_crypto::{generator, Scalar};
    fn records() -> Vec<StateBallotRecord> {
        (1u8..=4)
            .map(|i| StateBallotRecord {
                election_id: [1; 32],
                final_context: [2; 32],
                region: 1,
                registration_root: [3; 32],
                key_set_hash: [4; 32],
                nullifier: [i; 32],
                public_key: generator(),
                ciphertexts: vec![
                    Ciphertext {
                        c0: generator() * Scalar::from(u64::from(i)),
                        c1: generator()
                    };
                    3
                ],
            })
            .collect()
    }
    #[test]
    fn paths_match_original_root_at_all_positions_and_bind_order() {
        let records = records();
        let root = batch_root_v2_for_batch::<4>(&[1; 32], &[2; 32], 1, 1, &records).unwrap();
        for (i, r) in records.iter().enumerate() {
            let p = membership_path(&records, 1, i).unwrap();
            assert!(verify_membership(r, 1, i, &p, &root));
            assert!(!verify_membership(r, 1, i ^ 1, &p, &root));
            assert!(!verify_membership(r, 2, i, &p, &root));
        }
    }
    #[test]
    fn individual_writes_are_atomic_and_ordered() {
        let records = records();
        let root = batch_root_v2_for_batch::<4>(&[1; 32], &[2; 32], 1, 1, &records).unwrap();
        let mut r = IndividualReceiver::new(records[0].clone(), vec![root]);
        let path = membership_path(&records, 1, 0).unwrap();
        let old = r.state().to_vec();
        assert_eq!(
            r.apply(&records[0], 1, 0, &old, &path, || false),
            Err(Rejection::Proof)
        );
        assert_eq!((r.cursor(), r.spent().len()), (0, 0));
        assert_eq!(r.state(), old);
        let mut bad = path;
        bad[0][0] ^= 1;
        assert_eq!(
            r.apply(&records[0], 1, 0, &old, &bad, || panic!(
                "must reject before verifier"
            )),
            Err(Rejection::Membership)
        );
        for (i, record) in records.iter().enumerate() {
            let old = r.state().to_vec();
            r.apply(
                record,
                1,
                i,
                &old,
                &membership_path(&records, 1, i).unwrap(),
                || true,
            )
            .unwrap();
            assert_eq!((r.cursor(), r.spent().len()), (i + 1, i + 1));
        }
        assert_eq!(
            r.apply(&records[0], 1, 0, &old, &path, || true),
            Err(Rejection::Sequence)
        );
    }
}
