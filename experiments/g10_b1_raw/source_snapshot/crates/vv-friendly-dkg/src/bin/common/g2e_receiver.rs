//! Native, serial check-then-write checkpoint. Not a contract, durable store or intake authority.
use std::collections::HashSet;
use vv_friendly_crypto::{
    state_gadget::{
        batch_root_v2_for_batch, state_public_inputs, StateBallotRecord,
        StateTransitionCircuitForBatch,
    },
    Ciphertext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    NextIndex,
    OldState,
    ContextOrRegistry,
    AcceptedRoot,
    SpentNullifier,
    InvalidShape,
    ProofVerification,
}

#[derive(Clone)]
pub struct Receiver {
    binding: StateBallotRecord,
    accepted_roots: Vec<[u8; 32]>,
    state: Vec<Ciphertext>,
    next: u64,
    spent: HashSet<[u8; 32]>,
}

impl Receiver {
    pub fn new(binding: StateBallotRecord, accepted_roots: Vec<[u8; 32]>) -> Self {
        Self {
            binding,
            accepted_roots,
            state: vec![Ciphertext::zero(); 3],
            next: 1,
            spent: HashSet::new(),
        }
    }
    pub fn state(&self) -> &[Ciphertext] {
        &self.state
    }
    pub fn next(&self) -> u64 {
        self.next
    }
    pub fn spent_count(&self) -> usize {
        self.spent.len()
    }

    pub fn preflight<const B: usize>(
        &self,
        c: &StateTransitionCircuitForBatch<B>,
    ) -> Result<(), Rejection> {
        if c.batch_index != self.next {
            return Err(Rejection::NextIndex);
        }
        if c.old_state != self.state {
            return Err(Rejection::OldState);
        }
        let b = &self.binding;
        if c.election_id != b.election_id
            || c.final_context != b.final_context
            || c.region != b.region
            || c.key_set_hash != b.key_set_hash
            || c.public_key != b.public_key
            || c.records.iter().any(|r| {
                r.election_id != b.election_id
                    || r.final_context != b.final_context
                    || r.region != b.region
                    || r.registration_root != b.registration_root
                    || r.key_set_hash != b.key_set_hash
                    || r.public_key != b.public_key
            })
        {
            return Err(Rejection::ContextOrRegistry);
        }
        if state_public_inputs(c).is_none() {
            return Err(Rejection::InvalidShape);
        }
        let root = batch_root_v2_for_batch::<B>(
            &c.election_id,
            &c.final_context,
            c.region,
            c.batch_index,
            &c.records,
        )
        .ok_or(Rejection::InvalidShape)?;
        if root != c.batch_root || self.accepted_roots.get((self.next - 1) as usize) != Some(&root)
        {
            return Err(Rejection::AcceptedRoot);
        }
        let mut pending = HashSet::new();
        if c.records
            .iter()
            .any(|r| self.spent.contains(&r.nullifier) || !pending.insert(r.nullifier))
        {
            return Err(Rejection::SpentNullifier);
        }
        Ok(())
    }

    /// The closure must execute BOTH actual verifiers on c's exact ordered records.
    /// No state or nullifier is written until preflight and both verifiers succeed.
    pub fn apply<const B: usize>(
        &mut self,
        c: &StateTransitionCircuitForBatch<B>,
        verify: impl FnOnce() -> Result<bool, Box<dyn std::error::Error>>,
    ) -> Result<(), Rejection> {
        self.preflight(c)?;
        if !verify().map_err(|_| Rejection::ProofVerification)? {
            return Err(Rejection::ProofVerification);
        }
        let next = self.next.checked_add(1).ok_or(Rejection::NextIndex)?;
        self.state.clone_from(&c.new_state);
        self.spent.extend(c.records.iter().map(|r| r.nullifier));
        self.next = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vv_friendly_crypto::state_gadget::fold_state_for_batch;
    use vv_friendly_crypto::{generator, Scalar};
    fn records(start: u8) -> Vec<StateBallotRecord> {
        (start..start + 4)
            .map(|i| StateBallotRecord {
                election_id: [1; 32],
                final_context: [2; 32],
                region: 4,
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
    fn circuit(
        index: u64,
        old: Vec<Ciphertext>,
        records: Vec<StateBallotRecord>,
    ) -> StateTransitionCircuitForBatch<4> {
        StateTransitionCircuitForBatch {
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
    }
    #[test]
    fn rejected_verification_is_atomic_then_continuity_and_replay_checks_hold() {
        let a = circuit(1, vec![Ciphertext::zero(); 3], records(10));
        let b = circuit(2, a.new_state.clone(), records(20));
        let mut r = Receiver::new(a.records[0].clone(), vec![a.batch_root, b.batch_root]);
        assert_eq!(r.apply(&a, || Ok(false)), Err(Rejection::ProofVerification));
        assert_eq!((r.next(), r.spent_count()), (1, 0));
        assert_eq!(r.state(), a.old_state);
        assert_eq!(r.preflight(&b), Err(Rejection::NextIndex));
        // Unit tests exercise checkpoint bookkeeping only, not cryptographic evidence.
        r.apply(&a, || Ok(true)).unwrap();
        assert_eq!(r.preflight(&a), Err(Rejection::NextIndex));
        let mut reset = b.clone();
        reset.old_state = vec![Ciphertext::zero(); 3];
        assert_eq!(r.preflight(&reset), Err(Rejection::OldState));
        r.apply(&b, || Ok(true)).unwrap();
        assert_eq!((r.next(), r.spent_count()), (3, 8));
        assert_eq!(r.state(), b.new_state);
    }
    #[test]
    fn across_batch_nullifier_reuse_rejects_even_with_a_different_committed_batch_root() {
        let a = circuit(1, vec![Ciphertext::zero(); 3], records(10));
        let mut repeated = records(20);
        repeated[0].nullifier = a.records[0].nullifier;
        let b = circuit(2, a.new_state.clone(), repeated);
        let mut r = Receiver::new(a.records[0].clone(), vec![a.batch_root, b.batch_root]);
        r.apply(&a, || Ok(true)).unwrap();
        assert_eq!(r.preflight(&b), Err(Rejection::SpentNullifier));
        assert_eq!((r.next(), r.spent_count()), (2, 4));
    }
}
