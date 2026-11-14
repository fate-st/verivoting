// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./VeriVotingFrozenP3Verifier.sol";

/// G8-D: the authoritative receiver itself stores Winner, not an independent facade.
/// Existing batch/freeze bodies and the original G4-D statement are inherited unchanged.
contract VeriVotingWinnerOnlySettlement is VeriVotingAuthenticatedSettlement {
    address public immutable initializer;
    VeriVotingFrozenP3Verifier public immutable frozenP3Verifier;
    bool public p3Bound;
    bool public winnerDisclosed;
    uint64 public winner;
    uint8 public winnerSelector;

    event P3Bound(address indexed verifier);
    event WinnerDisclosed(bytes32 indexed electionId, bytes32 indexed freezeSeal, uint64 winner, uint8 selector);

    constructor(VeriVotingCertificateGateV2 gate, Binding memory b, bytes32[2] memory roots,
        bytes memory ballotVk, bytes memory stateVk, VeriVotingFrozenP3Verifier expectedP3)
        VeriVotingAuthenticatedSettlement(gate, b, roots, ballotVk, stateVk)
    {
        require(address(expectedP3) != address(0), "EMPTY_P3");
        initializer = msg.sender;
        // Predetermined CREATE address, deployed immediately afterwards in the local run.
        // This is an experiment deployment trust root, not caller-selected verification.
        frozenP3Verifier = expectedP3;
    }

    function bindFrozenP3() external {
        require(msg.sender == initializer, "INITIALIZER");
        require(!p3Bound, "P3_ALREADY_BOUND");
        require(nextBatchIndex == 1 && spentCount == 0 && !frozen, "ALREADY_STARTED");
        require(address(frozenP3Verifier.receiver()) == address(this)
            && address(frozenP3Verifier.certificateGate()) == address(certificateGate), "P3_LINKS");
        VeriVotingTrusteeCommitments store = frozenP3Verifier.commitmentStore();
        require(address(store.receiver()) == address(this)
            && address(store.certificateGate()) == address(certificateGate), "COMMITMENT_LINKS");
        require(frozenP3Verifier.ARK_VK_SHA256() == G8CExperimentPins.P3_ARK_VK_SHA256
            && frozenP3Verifier.EIP_VK_SHA256() == G8CExperimentPins.P3_EIP_VK_SHA256
            && frozenP3Verifier.EIP_VK_KECCAK256() == G8CExperimentPins.P3_EIP_VK_KECCAK256
            && frozenP3Verifier.verifier().verificationKeyDigest() == G8CExperimentPins.P3_EIP_VK_KECCAK256
            && frozenP3Verifier.verifier().publicInputCount() == 60, "PINNED_P3_VK");
        p3Bound = true;
        emit P3Bound(address(frozenP3Verifier));
    }

    function discloseWinner(bytes calldata proofNegAEip2537, uint8 selector, uint64 proposedWinner) external {
        require(!winnerDisclosed, "ALREADY_DISCLOSED");
        require(p3Bound, "P3_NOT_BOUND");
        // View/STATICCALL: verification cannot reenter a state-writing path.
        // Do NOT accept a cached off-chain success flag or caller-supplied inputs.
        frozenP3Verifier.verifyFrozenP3(proofNegAEip2537, selector, proposedWinner);
        winner = proposedWinner;
        winnerSelector = selector;
        winnerDisclosed = true;
        emit WinnerDisclosed(this.electionBinding().electionId, frozenTarget, proposedWinner, selector);
    }
}
