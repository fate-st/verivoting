// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./Bls12381Groth16Verifier.sol";
import "./interfaces/IBatchVerifier.sol";

/// Explicit NON-AGGREGATED fallback/baseline. Not a SnarkPack verifier.
contract IndividualGroth16BatchVerifier is IBatchVerifier {
    Bls12381Groth16Verifier public immutable verifier;
    bytes32 public immutable verificationKeyDigest;

    constructor(bytes memory vk) {
        verifier = new Bls12381Groth16Verifier(vk, 25);
        verificationKeyDigest = verifier.verificationKeyDigest();
    }

    function verifyBatch(bytes[] calldata proofs, uint256[][] calldata inputs) external view {
        require(proofs.length == 4 && inputs.length == 4, "B4_REQUIRED");
        for (uint256 i; i < 4; ++i) verifier.verifyOrRevert(proofs[i], inputs[i]);
    }
}
