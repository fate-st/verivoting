use vv_friendly_crypto::{
    state_gadget::{fold_state_for_batch, StateBallotRecord},
    Ciphertext,
};

/// Candidate-major independent oracle; never calls the production fold or Ciphertext::add.
pub fn reference(old: &[Ciphertext], records: &[StateBallotRecord]) -> Option<Vec<Ciphertext>> {
    if old.len() != 3 || records.iter().any(|r| r.ciphertexts.len() != 3) {
        return None;
    }
    let mut result = old.to_vec();
    for (j, output) in result.iter_mut().enumerate() {
        for record in records {
            output.c0 += record.ciphertexts[j].c0;
            output.c1 += record.ciphertexts[j].c1;
        }
    }
    Some(result)
}

pub fn actual(old: &[Ciphertext], records: &[StateBallotRecord]) -> Option<Vec<Ciphertext>> {
    match records.len() {
        2 => fold_state_for_batch::<2>(old, records),
        4 => fold_state_for_batch::<4>(old, records),
        8 => fold_state_for_batch::<8>(old, records),
        16 => fold_state_for_batch::<16>(old, records),
        32 => fold_state_for_batch::<32>(old, records),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vv_friendly_crypto::{generator, Scalar};
    fn records() -> Vec<StateBallotRecord> {
        (1..=32)
            .map(|i| StateBallotRecord {
                election_id: [1; 32],
                final_context: [2; 32],
                region: 4,
                registration_root: [3; 32],
                nullifier: [i; 32],
                key_set_hash: [4; 32],
                public_key: generator(),
                ciphertexts: (1..=3)
                    .map(|j| Ciphertext {
                        c0: generator() * Scalar::from(u64::from(i) * j),
                        c1: generator() * Scalar::from(u64::from(i) + j),
                    })
                    .collect(),
            })
            .collect()
    }
    #[test]
    fn all_candidates_match_reference_at_every_supported_boundary() {
        let records = records();
        let old = vec![
            Ciphertext {
                c0: generator(),
                c1: generator()
            };
            3
        ];
        for b in [2, 4, 8, 16, 32] {
            assert_eq!(actual(&old, &records[..b]), reference(&old, &records[..b]));
        }
    }
    #[test]
    fn one_changed_coordinate_has_exact_expected_delta() {
        let mut records = records();
        let old = vec![Ciphertext::zero(); 3];
        let before = actual(&old, &records).unwrap();
        records[15].ciphertexts[1].c1 += generator();
        let after = actual(&old, &records).unwrap();
        assert_eq!(after, reference(&old, &records).unwrap());
        assert_eq!(after[0], before[0]);
        assert_eq!(after[2], before[2]);
        assert_eq!(after[1].c0, before[1].c0);
        assert_eq!(after[1].c1, before[1].c1 + generator());
    }
    #[test]
    fn permutation_preserves_homomorphic_sum_not_ordered_batch_identity() {
        let mut records = records();
        let old = vec![Ciphertext::zero(); 3];
        let sum = reference(&old, &records);
        records.reverse();
        assert_eq!(actual(&old, &records), sum);
    }
    #[test]
    fn empty_and_malformed_inputs_have_explicit_boundaries() {
        let old = vec![Ciphertext::zero(); 3];
        assert_eq!(reference(&old, &[]), Some(old.clone()));
        assert!(actual(&old, &[]).is_none());
        assert!(actual(&old, &records()[..3]).is_none());
        assert!(reference(&old[..2], &records()[..2]).is_none());
        let mut records = records();
        records[0].ciphertexts.pop();
        assert!(reference(&old, &records).is_none());
        assert!(actual(&old, &records).is_none());
    }
}
