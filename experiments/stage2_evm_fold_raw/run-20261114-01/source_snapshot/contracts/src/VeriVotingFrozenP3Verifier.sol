// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./VeriVotingAuthenticatedSettlement.sol";
import "./auth/VeriVotingTrusteeCommitments.sol";

/// G8-C only. Build the ORIGINAL G4-D statement from authenticated frozen storage.
/// No caller-supplied 60-field array, private openings, success flag or Winner write.
contract VeriVotingFrozenP3Verifier {
    VeriVotingCertificateGateV2 public immutable certificateGate;
    VeriVotingAuthenticatedSettlement public immutable receiver;
    VeriVotingTrusteeCommitments public immutable commitmentStore;
    Bls12381Groth16Verifier public immutable verifier;
    bytes32 public constant ARK_VK_SHA256 = G8CExperimentPins.P3_ARK_VK_SHA256;
    bytes32 public constant EIP_VK_SHA256 = G8CExperimentPins.P3_EIP_VK_SHA256;
    bytes32 public constant EIP_VK_KECCAK256 = G8CExperimentPins.P3_EIP_VK_KECCAK256;

    constructor(VeriVotingCertificateGateV2 gate, VeriVotingAuthenticatedSettlement source,
        VeriVotingTrusteeCommitments commitments, bytes memory eipVk)
    {
        require(address(source.certificateGate()) == address(gate)
            && address(commitments.certificateGate()) == address(gate)
            && address(commitments.receiver()) == address(source),"CONTRACT_LINKS");
        require(source.bindingDigest() == G8CExperimentPins.BINDING_DIGEST
            && source.electionBinding().p3VkSha256 == ARK_VK_SHA256,"ARK_VK_BINDING");
        require(sha256(eipVk) == EIP_VK_SHA256 && keccak256(eipVk) == EIP_VK_KECCAK256,"PINNED_P3_VK");
        certificateGate = gate;
        receiver = source;
        commitmentStore = commitments;
        verifier = new Bls12381Groth16Verifier(eipVk,60);
        require(verifier.verificationKeyDigest() == EIP_VK_KECCAK256 && verifier.publicInputCount() == 60,"P3_VERIFIER");
    }
    function selectedTrustees(uint8 selector) public pure returns (uint32[3] memory) {
        require(selector < 10,"SELECTOR_RANGE");
        uint32[3][10] memory table = [
            [uint32(1),2,3],[uint32(1),2,4],[uint32(1),2,5],[uint32(1),3,4],[uint32(1),3,5],
            [uint32(1),4,5],[uint32(2),3,4],[uint32(2),3,5],[uint32(2),4,5],[uint32(3),4,5]
        ];
        return table[selector];
    }
    function buildP3Inputs(uint8 selector, uint64 proposedWinner) public view returns (uint256[] memory x) {
        receiver.requireCertifiedBinding();
        (,,bool isFrozen,bytes32 seal,bytes32 target,bytes32[6] memory ct) = receiver.checkpoint();
        require(isFrozen,"NOT_FROZEN");
        require(commitmentStore.frozenBound() && commitmentStore.freezeSeal() == seal
            && commitmentStore.cpTarget() == target,"COMMITMENT_TARGET");
        require(proposedWinner > 0 && proposedWinner <= 3,"WINNER_RANGE");
        uint32[3] memory ids = selectedTrustees(selector);
        VeriVotingCertificateGateV2.Expected memory b = certificateGate.certifiedBinding();
        x = new uint256[](60);
        uint256 offset;
        for (uint256 slot; slot < 3; ++slot) {
            uint32 id = ids[slot];
            require(commitmentStore.submitted(id),"MISSING_COMMITMENT");
            bytes memory prefix = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/GENERALIZED-CP/TRANSCRIPT/v3"),CanonicalVV.text(CanonicalVV.SUITE),
                abi.encodePacked(b.electionId,b.policyPre,b.finalContext,certificateGate.certifiedCloseDigest(),
                    certificateGate.certifiedDisclosureDigest(),target,id,commitmentStore.verifiedShare(id),uint64(3),ct[0],ct[2],ct[4],uint64(3)));
            offset = _appendPacked(x,offset,prefix);
            offset = _appendPacked(x,offset,abi.encodePacked(commitmentStore.packageCommitment(id)));
            require(offset == 17*(slot+1),"PREFIX_LAYOUT");
        }
        x[offset++] = selector; // G4-D index 51, not the historical Boolean selector.
        for (uint256 j; j < 3; ++j) offset = _appendPacked(x,offset,abi.encodePacked(ct[2*j+1]));
        x[offset++] = certificateGate.certifiedWRegistered();
        x[offset++] = proposedWinner;
        require(offset == 60,"P3_LAYOUT");
    }
    function verifyFrozenP3(bytes calldata proofNegAEip2537, uint8 selector, uint64 proposedWinner) external view {
        verifier.verifyOrRevert(proofNegAEip2537,buildP3Inputs(selector,proposedWinner));
    }
    // Exact Arkworks byte-slice ToConstraintField packing: <=31 LE bytes/field.
    // No modulo reduction, whole-digest scalar replacement or leading-one input.
    function _appendPacked(uint256[] memory x, uint256 offset, bytes memory wire) private pure returns (uint256) {
        for (uint256 start; start < wire.length; start += 31) {
            uint256 value;
            for (uint256 j; j < 31 && start+j < wire.length; ++j) value |= uint256(uint8(wire[start+j])) << (8*j);
            x[offset++] = value;
        }
        return offset;
    }
}
