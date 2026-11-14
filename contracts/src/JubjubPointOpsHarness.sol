// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./JubjubPointOps.sol";

/// Test interface only. Never used as a trusted accumulator in the receiver.
contract JubjubPointOpsHarness {
    function roundTrip(bytes32 p) external view returns (bytes32) { return JubjubPointOps.encode(JubjubPointOps.decodePrime(p)); }
    function add(bytes32 p,bytes32 q) external view returns (bytes32) {
        return JubjubPointOps.encode(JubjubPointOps.add(JubjubPointOps.decodePrime(p),JubjubPointOps.decodePrime(q)));
    }
}
