//! Unchanged G10-B1 route algorithms, isolated for the new proof-generation pilot.
use super::{helpers, intake, materials, receiver};
use ark_bls12_381::Bls12_381;
use ark_groth16::Groth16;
use bellperson::groth16::{
    aggregate::{verify_aggregate_proof, AggregateVersion},
    verify_proof as verify_ballot,
};
use helpers::Result;
use materials::{Batch, Loaded};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use serde_json::{json, Value};
use std::time::Instant;
use vv_friendly_crypto::{
    generator,
    state_gadget::{
        ballot_public_inputs, state_public_inputs, StateTransitionCircuitForBatch as Circuit,
    },
    Ciphertext,
};

const SEED: u64 = 0x4731_3042_314E_4154;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Individual,
    TransactionBatch,
    Direct,
    State,
}
const ROUTES: [Route; 4] = [
    Route::Individual,
    Route::TransactionBatch,
    Route::Direct,
    Route::State,
];
impl Route {
    fn name(self) -> &'static str {
        match self {
            Self::Individual => "individual_groth16_per_vote",
            Self::TransactionBatch => "transaction_batch_individual_groth16",
            Self::Direct => "snarkpack_direct_fold",
            Self::State => "snarkpack_state_proof",
        }
    }
}
#[derive(Default)]
struct Parts {
    packing_ms: f64,
    validity_ms: f64,
    extra_ms: f64,
    validity_calls: usize,
    validity_successes: usize,
    extra_called: bool,
    extra_ok: bool,
    checkpoint_writes: usize,
}
fn milliseconds(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}
fn direct(
    old: &[Ciphertext],
    records: &[vv_friendly_crypto::state_gadget::StateBallotRecord],
) -> Vec<Ciphertext> {
    let mut new = old.to_vec();
    for record in records {
        for (sum, ct) in new.iter_mut().zip(&record.ciphertexts) {
            *sum = sum.add(ct);
        }
    }
    new
}
fn receiver(data: &Loaded) -> receiver::Receiver {
    receiver::Receiver::new(
        data.batches[0].circuit.records[0].clone(),
        data.batches.iter().map(|b| b.circuit.batch_root).collect(),
    )
}
fn single_receiver(data: &Loaded) -> intake::IndividualReceiver {
    intake::IndividualReceiver::new(
        data.batches[0].circuit.records[0].clone(),
        data.batches.iter().map(|b| b.circuit.batch_root).collect(),
    )
}
fn individual_verify(
    data: &Loaded,
    b: &Batch,
    record: &vv_friendly_crypto::state_gadget::StateBallotRecord,
    proof_index: usize,
    m: &mut Parts,
) -> Result<bool> {
    let t = Instant::now();
    let inputs = helpers::convert_inputs(&ballot_public_inputs(record).ok_or("ballot packing")?)?;
    m.packing_ms += milliseconds(t);
    let t = Instant::now();
    m.validity_calls += 1;
    let valid = verify_ballot(&data.ballot_key, &b.ballot_proofs[proof_index], &inputs)?;
    m.validity_successes += usize::from(valid);
    m.validity_ms += milliseconds(t);
    Ok(valid)
}
fn batch_verify(
    data: &Loaded,
    b: &Batch,
    c: &Circuit<4>,
    route: Route,
    nonce: u64,
    m: &mut Parts,
) -> Result<bool> {
    if route == Route::TransactionBatch {
        for j in 0..4 {
            if !individual_verify(data, b, &c.records[j], j, m)? {
                return Ok(false);
            }
        }
    } else {
        let t = Instant::now();
        let matrix = c
            .records
            .iter()
            .map(|r| helpers::convert_inputs(&ballot_public_inputs(r).ok_or("ballot packing")?))
            .collect::<Result<Vec<_>>>()?;
        let transcript = helpers::transcript(&matrix);
        m.packing_ms += milliseconds(t);
        let t = Instant::now();
        m.validity_calls += 1;
        let valid = verify_aggregate_proof(
            &data.srs,
            &data.ballot_key,
            ChaCha20Rng::seed_from_u64(SEED ^ nonce ^ c.batch_index),
            &matrix,
            &b.aggregate,
            &transcript,
            AggregateVersion::V2,
        )?;
        m.validity_ms += milliseconds(t);
        m.validity_successes += usize::from(valid);
        if !valid {
            return Ok(false);
        }
    }
    let t = Instant::now();
    m.extra_called = true;
    let valid = if route == Route::State {
        Groth16::<Bls12_381>::verify_proof(
            &data.state_key,
            &b.proof,
            &state_public_inputs(c).ok_or("state packing")?,
        )?
    } else {
        direct(&c.old_state, &c.records) == c.new_state
    };
    m.extra_ms += milliseconds(t);
    m.extra_ok = valid;
    Ok(valid)
}
fn flow(data: &Loaded, route: Route, nonce: u64) -> Result<Parts> {
    let mut m = Parts::default();
    if route == Route::Individual {
        let mut r = single_receiver(data);
        for b in &data.batches {
            for (j, record) in b.circuit.records.iter().enumerate() {
                let old = r.state().to_vec();
                let mut error = None;
                r.apply(record, b.circuit.batch_index, j, &old, &b.paths[j], || {
                    match individual_verify(data, b, record, j, &mut m) {
                        Ok(ok) => ok,
                        Err(e) => {
                            error = Some(e);
                            false
                        }
                    }
                })
                .map_err(|e| format!("individual receive {e:?}; verifier error {error:?}"))?;
                m.checkpoint_writes += 1;
            }
            if r.state() != b.circuit.new_state {
                return Err("individual CT1/CT2 mismatch".into());
            }
        }
        if r.cursor() != 8 || r.spent().len() != 8 {
            return Err("individual final checkpoint".into());
        }
        let expected = data
            .batches
            .iter()
            .flat_map(|b| b.circuit.records.iter().map(|r| r.nullifier))
            .collect::<std::collections::HashSet<_>>();
        if *r.spent() != expected {
            return Err("individual spent set".into());
        }
    } else {
        let mut r = receiver(data);
        for b in &data.batches {
            r.apply(&b.circuit, || {
                batch_verify(data, b, &b.circuit, route, nonce, &mut m)
            })
            .map_err(|e| format!("batch receive {e:?}"))?;
            m.checkpoint_writes += 1;
            if r.state() != b.circuit.new_state
                || r.spent_count() != b.circuit.batch_index as usize * 4
            {
                return Err("batch CT1/CT2/checkpoint".into());
            }
        }
        if r.next() != 3 || r.spent_count() != 8 {
            return Err("batch final checkpoint".into());
        }
    }
    let expected_calls = if matches!(route, Route::Individual | Route::TransactionBatch) {
        8
    } else {
        2
    };
    if m.validity_calls != expected_calls
        || m.checkpoint_writes != if route == Route::Individual { 8 } else { 2 }
    {
        return Err("operation accounting".into());
    }
    Ok(m)
}
fn negatives(data: &Loaded) -> Result<Vec<Value>> {
    let b = &data.batches[0];
    let old = b.circuit.old_state.clone();
    let mut results = Vec::new();
    // Original valid record/path, but another voter's actual proof: reaches Groth16.
    let mut r = single_receiver(data);
    let mut m = Parts::default();
    let mut crypto_result = None;
    let rejection = r.apply(&b.circuit.records[0], 1, 0, &old, &b.paths[0], || {
        let result = individual_verify(data, b, &b.circuit.records[0], 1, &mut m);
        let valid = matches!(result, Ok(true));
        crypto_result = Some(result);
        valid
    });
    if rejection != Err(intake::Rejection::Proof)
        || m.validity_calls != 1
        || !matches!(crypto_result, Some(Ok(false)))
        || r.cursor() != 0
        || !r.spent().is_empty()
        || r.state() != old
    {
        return Err("individual crypto negative".into());
    }
    results.push(json!({"route":Route::Individual.name(),"case":"row0_with_original_row1_proof","rejection":"Proof","groth16_reached":true,"membership_passed":true,"checkpoint_unchanged":true}));
    let mut r = receiver(data);
    let mut m = Parts::default();
    let mut crypto_result = None;
    let rejection = r.apply(&b.circuit, || {
        let result = individual_verify(data, b, &b.circuit.records[0], 1, &mut m)?;
        crypto_result = Some(result);
        Ok(result)
    });
    if rejection != Err(receiver::Rejection::ProofVerification)
        || m.validity_calls != 1
        || crypto_result != Some(false)
        || r.next() != 1
        || r.spent_count() != 0
        || r.state() != old
    {
        return Err("transaction batch crypto negative".into());
    }
    results.push(json!({"route":Route::TransactionBatch.name(),"case":"row0_with_original_row1_proof","rejection":"ProofVerification","groth16_reached":true,"accepted_root_passed":true,"checkpoint_unchanged":true}));
    for route in [Route::Direct, Route::State] {
        let mut bad = b.circuit.clone();
        bad.new_state[0].c1 += generator();
        let mut r = receiver(data);
        let mut m = Parts::default();
        let rejection = r.apply(&bad, || batch_verify(data, b, &bad, route, 0, &mut m));
        if rejection != Err(receiver::Rejection::ProofVerification)
            || m.validity_calls != 1
            || m.validity_successes != 1
            || !m.extra_called
            || m.extra_ok
            || r.next() != 1
            || r.spent_count() != 0
            || r.state() != old
        {
            return Err("changed state negative".into());
        }
        results.push(json!({"route":route.name(),"case":"changed_first_new_state_before_acceptance","rejection":"ProofVerification","snarkpack_passed":true,"route_specific_rejected":true,"checkpoint_unchanged":true}));
    }
    Ok(results)
}
fn communication(data: &Loaded) -> Result<Value> {
    let sum = |key: &str| {
        data.communication
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r[key].as_u64().unwrap())
            .sum::<u64>()
    };
    let mut rows = Vec::new();
    for route in ROUTES {
        let validity = if matches!(route, Route::Individual | Route::TransactionBatch) {
            8 * 192
        } else {
            sum("aggregate_bytes")
        };
        let state = if route == Route::State {
            sum("state_proof_bytes")
        } else {
            0
        };
        let path = if route == Route::Individual {
            8 * 2 * 32
        } else {
            0
        };
        rows.push(json!({"route":route.name(),"validity_proof_bytes":validity,"ballot_matrix_bytes":8*25*32,
            "state_proof_bytes":state,"membership_path_bytes":path,"accepted_roots_bytes_shared_per_flow":64,
            "submitted_ct_old_new_bytes":if route==Route::Individual {8*3*2*32} else {2*2*3*2*32},
            "components_not_a_complete_wire_envelope":true,"full_network_bytes":null,"calldata":null,
            "individual_new_CT_is_derived_not_transmitted":route==Route::Individual,
            "matrix_represents_canonical_records_do_not_add_records_again":true}));
    }
    Ok(json!(rows))
}
// The four receiver algorithms above are copied unchanged from the frozen B1 source.
pub(super) fn run_fresh(data: &Loaded) -> Result<Value> {
    let mut rows = Vec::new();
    for route in ROUTES {
        let t = Instant::now();
        let parts = flow(data, route, 0)?;
        rows.push(json!({"route":route.name(),"two_batch_flow_ms":milliseconds(t),"packing_ms":parts.packing_ms,
            "validity_ms":parts.validity_ms,"fold_or_state_verify_ms":parts.extra_ms,"validity_calls":parts.validity_calls,
            "checkpoint_writes":parts.checkpoint_writes,"same_ct1_ct2_and_spent8":true}));
    }
    Ok(
        json!({"routes":rows,"necessary_negatives":negatives(data)?,"communication_components":communication(data)?,
        "sample_count_per_route":1,"independent_new_elections":0,"proof_pools":1,"same_reconstructed_archived_election":true,
        "statistical_performance_claim":false}),
    )
}
