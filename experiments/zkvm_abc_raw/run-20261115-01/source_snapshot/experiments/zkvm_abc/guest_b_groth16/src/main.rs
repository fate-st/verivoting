#![no_main]
use g17_common::{verify_batch, BatchInput};
use risc0_zkvm::guest::env;
risc0_zkvm::guest::entry!(main);

fn main() {
    let input: BatchInput = env::read();
    // The complete Groth16 equation is executed here, NOT a host success flag.
    let verified = verify_batch(&input).unwrap_or_else(|reason| panic!("{reason}"));
    // No success journal can be committed after a failed proof.
    env::commit(&verified);
}
