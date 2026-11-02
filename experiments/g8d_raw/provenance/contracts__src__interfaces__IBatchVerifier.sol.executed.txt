// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// Reverts unless EVERY ordered ballot statement has a valid proof.
/// No implementation returning a host-provided success flag is acceptable.
interface IBatchVerifier {
    function verifyBatch(bytes[] calldata proofs, uint256[][] calldata inputs) external view;
    function verificationKeyDigest() external view returns (bytes32);
}
