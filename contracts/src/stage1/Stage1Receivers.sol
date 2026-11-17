// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "../IndividualGroth16BatchVerifier.sol";
import "../auth/VeriVotingCertificateGateV2.sol";
import "../G8CExperimentPins.sol";
import "../JubjubPointOps.sol";

/// Isolated fixed D5/W32 evaluation, NOT a SnarkPack verifier or production deployment.
abstract contract Stage1ReceiverBase {
    struct Binding {
        bytes32 electionId; bytes32 policyPre; bytes32 dkgSession; bytes32 keyCoreDigest;
        bytes32 publicKey; bytes32 finalContext; bytes32 registrationRoot;
        bytes32 registrationCloseDigest; bytes32 disclosureDigest; bytes32 p3VkSha256;
        bytes32 prekeyContext; bytes32 registrationAuthority; bytes32 registrationCertificateSha256;
        uint64 region; uint64 registeredWeight;
    }
    struct Batch {
        uint64 index; bytes32 root; bytes32[6] oldState; bytes32[6] newState;
        bytes[] ballotProofs; uint256[][] ballotInputs;
    }
    Binding internal binding;
    VeriVotingCertificateGateV2 public immutable certificateGate;
    bytes32 public immutable bindingDigest;
    bytes32 public immutable scheduleDigest;
    uint64 public immutable plannedBatches;
    bytes32[] public acceptedRoots;
    bytes32[6] internal encryptedState;
    mapping(bytes32 => bool) public spentNullifier;
    uint64 public nextBatchIndex = 1;
    uint64 public spentCount;
    uint8 public positionInBatch;
    bool public frozen;
    bytes32 public frozenTarget;
    event BallotAccepted(uint64 indexed batch, uint8 indexed position, bytes32 nullifier);
    event BatchAccepted(uint64 indexed batch, bytes32 root);
    event TargetFrozen(bytes32 target);

    constructor(VeriVotingCertificateGateV2 gate, Binding memory b, bytes32[] memory roots) {
        require(roots.length > 0 && roots.length <= 8,"SCHEDULE_SIZE");
        require(keccak256(abi.encode(b)) == G8CExperimentPins.BINDING_DIGEST,"PINNED_BINDING");
        bytes32[8] memory expected = [
            bytes32(0x91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134),
            bytes32(0x4e3a7649aa9d7455509a0b052d85c6745b38cb1f2416045c08ffd25094837fcf),
            bytes32(0xa0cf1466eb98fdc53dc4f284272d3df551602cf0a4bf93bb6cb8ec0f02fb61bb),
            bytes32(0x4ed7305567ea1cc493f89b07a648a9a6b08b091a0f0dc59ce4d94c456e917a35),
            bytes32(0x98d2fa804a41c2b7aba5ec0a1c18172a1a6814491fb0dc503feed2959cf7a4d2),
            bytes32(0x2a6381e3be0cbfad8890ef903957b26eee9653372dc0bcf0ea762ae82442edf6),
            bytes32(0xed1d94f55a8ad36d3cc3fcb7b3b77f18cf5641f14e072475d1585f84f689dd14),
            bytes32(0xe791042a384b5cac52c2b601ac28698961b17166b66b83b0d06dec6bdc0c60c8)
        ];
        for(uint256 i; i<roots.length; ++i) require(roots[i] == expected[i],"PINNED_PREFIX");
        require(gate.keyCoreDigest() == b.keyCoreDigest && gate.finalContext() == b.finalContext,"GATE_CONTEXT");
        require(b.region == 4 && b.registeredWeight == 32,"FIXED_D5_W32");
        binding=b; certificateGate=gate; bindingDigest=keccak256(abi.encode(b));
        scheduleDigest=keccak256(abi.encode(roots)); plannedBatches=uint64(roots.length); acceptedRoots=roots;
        for(uint256 j; j<6; ++j) encryptedState[j]=bytes32(bytes1(0x01));
    }
    function electionBinding() external view returns(Binding memory) { return binding; }
    function checkpoint() external view returns(uint64 next,uint64 count,uint8 position,bool isFrozen,bytes32 target,bytes32[6] memory state) {
        return(nextBatchIndex,spentCount,positionInBatch,frozen,frozenTarget,encryptedState);
    }
    function _certified() internal view {
        certificateGate.requireAuthorized();
        VeriVotingCertificateGateV2.Expected memory e=certificateGate.certifiedBinding();
        require(e.electionId==binding.electionId && e.policyPre==binding.policyPre
            && e.prekeyContext==binding.prekeyContext && e.dkgSession==binding.dkgSession
            && e.keyCoreDigest==binding.keyCoreDigest && e.publicKey==binding.publicKey
            && e.finalContext==binding.finalContext,"CERTIFIED_KEY_BINDING");
        require(certificateGate.certifiedRoot()==binding.registrationRoot
            && certificateGate.certifiedCloseDigest()==binding.registrationCloseDigest
            && certificateGate.certifiedDisclosureDigest()==binding.disclosureDigest
            && certificateGate.certifiedAuthority()==binding.registrationAuthority
            && certificateGate.certifiedCertificateSha256()==binding.registrationCertificateSha256,"CERTIFIED_REGISTRATION_BINDING");
        require(certificateGate.certifiedWRegistered()==32 && certificateGate.certifiedWMax()==32
            && certificateGate.certifiedNRegistered()==32,"CERTIFIED_WEIGHT");
    }
    function _header(uint64 index,bytes32[6] calldata old) internal view {
        _certified(); require(!frozen,"FROZEN");
        require(index==nextBatchIndex && index<=plannedBatches,"NEXT_INDEX");
        for(uint256 j; j<6; ++j) require(old[j]==encryptedState[j],"OLD_STATE");
    }
    function _record(uint256[] calldata row) internal view returns(bytes32 nf) {
        require(row.length==25,"BALLOT_LAYOUT");
        require(_matches(row,0,binding.electionId) && _matches(row,2,binding.finalContext) && row[4]==4,"CONTEXT");
        require(_matches(row,5,binding.registrationRoot),"REGISTRATION_ROOT");
        require(_matches(row,9,binding.keyCoreDigest) && _matches(row,11,binding.publicKey),"KEY_CONTEXT");
        nf=_unpack(row[7],row[8]); require(!spentNullifier[nf],"SPENT_NULLIFIER");
    }
    function _leaf(uint64 index,uint64 position,uint256[] calldata row) internal view returns(bytes32) {
        return sha256(bytes.concat(CanonicalVV.prefix("VV/BATCH/LEAF/v2"),
            abi.encodePacked(binding.electionId,binding.finalContext,binding.region,index,uint64(4),position,uint64(25)),abi.encodePacked(row)));
    }
    function _node(bytes32 left,bytes32 right) internal pure returns(bytes32) {
        return sha256(bytes.concat(CanonicalVV.prefix("VV/BATCH/NODE/v2"),abi.encodePacked(left,right)));
    }
    function _preflight(Batch calldata b) internal view returns(bytes32[4] memory nf) {
        _header(b.index,b.oldState); require(positionInBatch==0,"PARTIAL_BATCH");
        require(b.ballotInputs.length==4 && b.ballotProofs.length==4,"B4_REQUIRED");
        bytes32[4] memory leaves;
        for(uint256 i; i<4; ++i) {
            nf[i]=_record(b.ballotInputs[i]);
            for(uint256 j; j<i; ++j) require(nf[i]!=nf[j],"DUPLICATE_NULLIFIER");
            leaves[i]=_leaf(b.index,uint64(i),b.ballotInputs[i]);
        }
        require(b.root==acceptedRoots[b.index-1],"ACCEPTED_ROOT");
        require(_node(_node(leaves[0],leaves[1]),_node(leaves[2],leaves[3]))==b.root,"BATCH_ROOT");
    }
    function _commit(Batch calldata b,bytes32[4] memory nf) internal {
        for(uint256 i; i<4; ++i) spentNullifier[nf[i]]=true;
        encryptedState=b.newState; spentCount+=4; ++nextBatchIndex;
        emit BatchAccepted(b.index,b.root);
    }
    function _fold(uint256[][] calldata rows) internal view returns(bytes32[6] memory result) {
        JubjubPointOps.Extended[6] memory sums;
        for(uint256 j; j<6; ++j) sums[j]=JubjubPointOps.decodeVerifiedCiphertext(encryptedState[j]);
        for(uint256 i; i<rows.length; ++i) for(uint256 j; j<6; ++j)
            sums[j]=JubjubPointOps.add(sums[j],JubjubPointOps.decodeVerifiedCiphertext(_unpack(rows[i][13+2*j],rows[i][14+2*j])));
        for(uint256 j; j<6; ++j) result[j]=JubjubPointOps.encode(sums[j]);
    }
    function freeze() external {
        _certified(); require(!frozen,"FROZEN");
        require(positionInBatch==0 && nextBatchIndex==plannedBatches+1 && spentCount==4*plannedBatches,"INCOMPLETE");
        // Independent stage-one version, never the historical G6 CT2 seal.
        frozenTarget=sha256(abi.encode("VV/STAGE1/FROZEN/v1",bindingDigest,scheduleDigest,plannedBatches,spentCount,encryptedState));
        frozen=true; emit TargetFrozen(frozenTarget);
    }
    function _pack(bytes32 raw) internal pure returns(uint256 a,uint256 b) {
        for(uint256 i; i<31; ++i) a|=uint256(uint8(raw[i]))<<(8*i); b=uint8(raw[31]);
    }
    function _matches(uint256[] calldata row,uint256 offset,bytes32 raw) internal pure returns(bool) {
        (uint256 a,uint256 b)=_pack(raw); return row[offset]==a && row[offset+1]==b;
    }
    function _unpack(uint256 a,uint256 b) internal pure returns(bytes32) {
        require(a<(uint256(1)<<248) && b<256,"PACKING"); uint256 r=b;
        for(uint256 i; i<31; ++i) r|=((a>>(8*i))&255)<<(8*(31-i)); return bytes32(r);
    }
    function _reverse64(uint64 v) internal pure returns(uint256 r) {
        for(uint256 i; i<8; ++i) r=(r<<8)|((uint256(v)>>(8*i))&255);
    }
}

/// Four non-atomic transactions per B4. Previously accepted ballots survive a later failure.
contract Stage1LegacySettlement is Stage1ReceiverBase {
    Bls12381Groth16Verifier public immutable ballotVerifier;
    constructor(VeriVotingCertificateGateV2 g,Binding memory b,bytes32[] memory roots,bytes memory vk)
        Stage1ReceiverBase(g,b,roots) {
        require(sha256(vk)==G8CExperimentPins.BALLOT_VK_SHA256,"PINNED_BALLOT_VK");
        ballotVerifier=new Bls12381Groth16Verifier(vk,25);
    }
    function submitBallot(uint64 index,uint8 position,bytes32[6] calldata old,uint256[] calldata row,bytes calldata proof,bytes32[2] calldata path) external {
        _header(index,old); require(position==positionInBatch && position<4,"POSITION");
        bytes32 nf=_record(row);
        bytes32 leaf=_leaf(index,position,row);
        bytes32 middle=(position&1)==0?_node(leaf,path[0]):_node(path[0],leaf);
        bytes32 root=(position&2)==0?_node(middle,path[1]):_node(path[1],middle);
        require(root==acceptedRoots[index-1],"BATCH_MEMBERSHIP");
        ballotVerifier.verifyOrRevert(proof,row);
        for(uint256 j; j<6; ++j) encryptedState[j]=JubjubPointOps.encode(JubjubPointOps.add(
            JubjubPointOps.decodeVerifiedCiphertext(encryptedState[j]),
            JubjubPointOps.decodeVerifiedCiphertext(_unpack(row[13+2*j],row[14+2*j]))));
        spentNullifier[nf]=true; ++spentCount; emit BallotAccepted(index,position,nf);
        ++positionInBatch;
        if(positionInBatch==4) { positionInBatch=0; ++nextBatchIndex; emit BatchAccepted(index,root); }
    }
}

contract Stage1DirectFoldSettlement is Stage1ReceiverBase {
    IndividualGroth16BatchVerifier public immutable batchVerifier;
    constructor(VeriVotingCertificateGateV2 g,Binding memory b,bytes32[] memory roots,bytes memory vk)
        Stage1ReceiverBase(g,b,roots) {
        require(sha256(vk)==G8CExperimentPins.BALLOT_VK_SHA256,"PINNED_BALLOT_VK");
        batchVerifier=new IndividualGroth16BatchVerifier(vk);
    }
    function submitBatch(Batch calldata b) external {
        bytes32[4] memory nf=_preflight(b); batchVerifier.verifyBatch(b.ballotProofs,b.ballotInputs);
        bytes32[6] memory result=_fold(b.ballotInputs);
        for(uint256 j; j<6; ++j) require(result[j]==b.newState[j],"DIRECT_FOLD_MISMATCH");
        _commit(b,nf);
    }
}

contract Stage1StateProofSettlement is Stage1ReceiverBase {
    IndividualGroth16BatchVerifier public immutable batchVerifier;
    Bls12381Groth16Verifier public immutable stateVerifier;
    constructor(VeriVotingCertificateGateV2 g,Binding memory b,bytes32[] memory roots,bytes memory vk,bytes memory svk)
        Stage1ReceiverBase(g,b,roots) {
        require(sha256(vk)==G8CExperimentPins.BALLOT_VK_SHA256 && sha256(svk)==G8CExperimentPins.STATE_VK_SHA256,"PINNED_VKS");
        batchVerifier=new IndividualGroth16BatchVerifier(vk); stateVerifier=new Bls12381Groth16Verifier(svk,37);
    }
    function submitBatch(Batch calldata b,bytes calldata stateProof) external {
        bytes32[4] memory nf=_preflight(b); batchVerifier.verifyBatch(b.ballotProofs,b.ballotInputs);
        uint256[] memory x=new uint256[](37);
        (x[0],x[1])=_pack(binding.electionId); (x[2],x[3])=_pack(binding.finalContext);
        x[4]=_reverse64(binding.region); x[5]=_reverse64(b.index); x[6]=_reverse64(4);
        (x[7],x[8])=_pack(b.root); (x[9],x[10])=_pack(binding.keyCoreDigest); (x[11],x[12])=_pack(binding.publicKey);
        for(uint256 j; j<6; ++j) { (x[13+2*j],x[14+2*j])=_pack(encryptedState[j]); (x[25+2*j],x[26+2*j])=_pack(b.newState[j]); }
        stateVerifier.verifyOrRevert(stateProof,x); _commit(b,nf);
    }
}
