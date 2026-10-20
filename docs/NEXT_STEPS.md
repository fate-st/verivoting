# Next gates

The current native core and real P1 Groth16 path pass. The remaining gates must
be implemented in this order without substituting mocks for measurements:

1. G1: pin and reproduce a real SnarkPack implementation over compatible
   ballot Groth16 proofs; add individual-verification and linear-fold baselines.
2. G2: implement the regional state transition circuit and bind its public
   inputs to the canonical context and registration close.
3. G3: move the existing Pedersen VSS/GJKR phases to five independently
   persisted OS processes with authenticated transport, restart and timeout
   tests. The current one-process protocol simulation is not G3.
4. G4: implement full P3 with non-native Ristretto arithmetic, exact in-circuit
   CP challenge recomputation, package openings/signatures, bounds and winner.
5. G5: add target-chain verifier contracts and separately report native time,
   verifier gas, calldata and DA bytes.

Do not promote a `BLOCKED_*` capability in the manifest until its positive and
negative tests, raw outputs and reproduction command are checked in.
