// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./IndividualGroth16BatchVerifier.sol";

/// G8 PHASE 1 ONLY: real ballot/state verification -> continuous CT2 -> freeze.
/// NOT complete Winner-only settlement: no signature/certificate verification
/// or winner-writing endpoint exists yet. Initial anchors are deployer-provisioned.
contract VeriVotingSettlement {
    struct Binding {
        bytes32 electionId;
        bytes32 policyPre;
        bytes32 dkgSession;
        bytes32 keyCoreDigest;
        bytes32 publicKey;
        bytes32 finalContext;
        bytes32 registrationRoot;
        bytes32 registrationCloseDigest;
        bytes32 disclosureDigest;
        bytes32 p3VkSha256;
        bytes32 prekeyContext;
        bytes32 registrationAuthority;
        bytes32 registrationCertificateSha256;
        uint64 region;
        uint64 registeredWeight;
    }
    struct Batch {
        uint64 index;
        bytes32 root;
        bytes32[6] oldState;
        bytes32[6] newState;
        bytes[] ballotProofs;
        uint256[][] ballotInputs;
        bytes stateProof;
    }
    Binding private binding;
    bytes32 public immutable bindingDigest;
    IBatchVerifier public immutable batchVerifier;
    Bls12381Groth16Verifier public immutable stateVerifier;
    bytes32[2] public acceptedRoots;
    bytes32[6] private encryptedState;
    mapping(bytes32 => bool) public spentNullifier;
    uint64 public nextBatchIndex = 1;
    uint64 public spentCount;
    bool public frozen;
    bytes32 public frozenTarget;
    bytes32 public cpTarget;
    string private constant RELEASE = "WINNER_ONLY_V1";
    string private constant RULE = "highest-weight-smallest-id-on-tie";
    string private constant SUITE = "VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2";
    event BatchAccepted(uint64 indexed index, bytes32 indexed root);
    event TargetFrozen(bytes32 indexed target, bytes32 cpTarget);

    constructor(Binding memory b, bytes32[2] memory roots, bytes memory ballotVk, bytes memory stateVk) {
        require(b.region == 4 && b.registeredWeight == 32, "FIXED_D5_W32_REGION4");
        require(roots[0] != bytes32(0) && roots[1] != bytes32(0), "EMPTY_SCHEDULE");
        binding = b;
        bindingDigest = keccak256(abi.encode(b));
        acceptedRoots = roots;
        // Concrete, real implementations, not caller-injected mock addresses.
        batchVerifier = new IndividualGroth16BatchVerifier(ballotVk);
        stateVerifier = new Bls12381Groth16Verifier(stateVk, 37);
        for (uint256 i; i < 6; ++i) encryptedState[i] = bytes32(bytes1(0x01));
    }

    function checkpoint() external view returns (
        uint64 next, uint64 count, bool isFrozen, bytes32 target, bytes32 cp, bytes32[6] memory state
    ) {
        return (nextBatchIndex, spentCount, frozen, frozenTarget, cpTarget, encryptedState);
    }

    function submitBatch(Batch calldata b) external {
        require(!frozen, "FROZEN");
        require(b.index == nextBatchIndex && b.index <= 2, "NEXT_INDEX");
        require(b.ballotInputs.length == 4 && b.ballotProofs.length == 4, "B4_REQUIRED");
        for (uint256 j; j < 6; ++j) require(b.oldState[j] == encryptedState[j], "OLD_STATE");
        bytes32[4] memory nullifiers;
        for (uint256 i; i < 4; ++i) {
            _checkRecord(b.ballotInputs[i]);
            nullifiers[i] = _unpack32(b.ballotInputs[i][7], b.ballotInputs[i][8]);
            require(!spentNullifier[nullifiers[i]], "SPENT_NULLIFIER");
            for (uint256 j; j < i; ++j) require(nullifiers[i] != nullifiers[j], "DUPLICATE_NULLIFIER");
        }
        require(b.root == acceptedRoots[b.index - 1], "ACCEPTED_ROOT");
        require(_batchRoot(b) == b.root, "BATCH_ROOT");
        // EXACT same matrix enters root computation and actual ballot verifiers.
        batchVerifier.verifyBatch(b.ballotProofs, b.ballotInputs);
        // Build all 37 state inputs here; no off-chain "verification passed" flag.
        stateVerifier.verifyOrRevert(b.stateProof, _stateInputs(b));
        // Check-then-write; a failed verifier reverts the entire transaction.
        for (uint256 i; i < 4; ++i) spentNullifier[nullifiers[i]] = true;
        encryptedState = b.newState;
        spentCount += 4;
        ++nextBatchIndex;
        emit BatchAccepted(b.index, b.root);
    }

    function freeze() external {
        require(!frozen, "FROZEN");
        require(nextBatchIndex == 3 && spentCount == 8, "INCOMPLETE");
        // Match the ORIGINAL canonical G6 CP-target and freeze-seal encodings.
        bytes memory t = bytes.concat(_prefix("VV/JUBJUB/TALLY/TARGET/v3"), _text(SUITE));
        t = bytes.concat(t, abi.encodePacked(binding.electionId, binding.policyPre, binding.finalContext,
            binding.registrationCloseDigest, binding.disclosureDigest, uint64(3)));
        for (uint256 i; i < 6; ++i) t = bytes.concat(t, encryptedState[i]);
        bytes32 cp = sha256(t);
        bytes memory e = _prefix("VV/G6/NATIVE-WINNER-ONLY/FROZEN/v1");
        e = bytes.concat(e, abi.encodePacked(binding.electionId, binding.policyPre, binding.dkgSession,
            binding.keyCoreDigest, binding.publicKey));
        e = bytes.concat(e, abi.encodePacked(binding.finalContext, binding.registrationRoot,
            binding.registrationCloseDigest, binding.disclosureDigest, cp));
        e = bytes.concat(e, _text(RELEASE), _text(RULE), _text(_lowerHex(binding.p3VkSha256)));
        e = bytes.concat(e, abi.encodePacked(binding.registeredWeight, nextBatchIndex, spentCount,
            acceptedRoots[0], acceptedRoots[1]));
        for (uint256 i; i < 3; ++i) e = bytes.concat(e, abi.encodePacked(uint32(i + 1), encryptedState[2*i], encryptedState[2*i+1]));
        cpTarget = cp;
        frozenTarget = sha256(e);
        frozen = true;
        emit TargetFrozen(frozenTarget, cp);
    }

    function _checkRecord(uint256[] calldata row) private view {
        require(row.length == 25, "BALLOT_LAYOUT");
        require(_pairMatches(row, 0, binding.electionId) && _pairMatches(row, 2, binding.finalContext)
            && row[4] == binding.region, "CONTEXT");
        require(_pairMatches(row, 5, binding.registrationRoot), "REGISTRATION_ROOT");
        require(_pairMatches(row, 9, binding.keyCoreDigest) && _pairMatches(row, 11, binding.publicKey), "KEY_CONTEXT");
    }

    function _batchRoot(Batch calldata b) private view returns (bytes32) {
        bytes32[4] memory leaves;
        bytes memory prefix = _prefix("VV/BATCH/LEAF/v2");
        for (uint256 i; i < 4; ++i) {
            leaves[i] = sha256(bytes.concat(prefix, abi.encodePacked(binding.electionId, binding.finalContext,
                binding.region, b.index, uint64(4), uint64(i), uint64(25)), abi.encodePacked(b.ballotInputs[i])));
        }
        bytes memory node = _prefix("VV/BATCH/NODE/v2");
        bytes32 left = sha256(abi.encodePacked(node, leaves[0], leaves[1]));
        bytes32 right = sha256(abi.encodePacked(node, leaves[2], leaves[3]));
        return sha256(abi.encodePacked(node, left, right));
    }

    function _stateInputs(Batch calldata b) private view returns (uint256[] memory x) {
        x = new uint256[](37);
        _put32(x, 0, binding.electionId);
        _put32(x, 2, binding.finalContext);
        x[4] = _reverse64(binding.region);
        x[5] = _reverse64(b.index);
        x[6] = _reverse64(4);
        _put32(x, 7, b.root);
        _put32(x, 9, binding.keyCoreDigest);
        _put32(x, 11, binding.publicKey);
        for (uint256 j; j < 6; ++j) {
            _put32(x, 13 + 2*j, encryptedState[j]);
            _put32(x, 25 + 2*j, b.newState[j]);
        }
    }

    function _put32(uint256[] memory x, uint256 offset, bytes32 raw) private pure {
        (x[offset], x[offset+1]) = _pack32(raw);
    }
    function _pairMatches(uint256[] calldata x, uint256 offset, bytes32 raw) private pure returns (bool) {
        (uint256 a, uint256 b) = _pack32(raw);
        return x[offset] == a && x[offset+1] == b;
    }
    function _pack32(bytes32 raw) private pure returns (uint256 a, uint256 b) {
        for (uint256 i; i < 31; ++i) a |= uint256(uint8(raw[i])) << (8*i);
        b = uint8(raw[31]);
    }
    function _unpack32(uint256 a, uint256 b) private pure returns (bytes32) {
        require(a < (uint256(1) << 248) && b < 256, "PACKING");
        uint256 result = b;
        for (uint256 i; i < 31; ++i) result |= ((a >> (8*i)) & 255) << (8*(31-i));
        return bytes32(result);
    }
    function _reverse64(uint64 v) private pure returns (uint256 out) {
        for (uint256 i; i < 8; ++i) out = (out << 8) | ((uint256(v) >> (8*i)) & 255);
    }
    function _prefix(string memory domain) private pure returns (bytes memory) {
        return bytes.concat(_text("VV-CANONICAL-V1"), _text(domain));
    }
    function _text(string memory v) private pure returns (bytes memory) {
        return abi.encodePacked(uint64(bytes(v).length), bytes(v));
    }
    function _lowerHex(bytes32 v) private pure returns (string memory) {
        bytes memory out = new bytes(64); bytes memory digits = "0123456789abcdef";
        for (uint256 i; i < 32; ++i) { out[2*i] = digits[uint8(v[i]) >> 4]; out[2*i+1] = digits[uint8(v[i]) & 15]; }
        return string(out);
    }
}
