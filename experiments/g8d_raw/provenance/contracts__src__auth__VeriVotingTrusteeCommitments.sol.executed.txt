// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./VeriVotingCertificateGate.sol";
import "./JubjubVerificationShare.sol";
import "../VeriVotingSettlement.sol";

/// G8-B ONLY: authenticate PUBLIC commitment records for the actual frozen CT2.
/// NOT P3/CP/opening verification and NOT a Winner-writing contract.
/// The authorized deployed gate/receiver addresses remain bootstrap trust roots.
contract VeriVotingTrusteeCommitments {
    uint64 public constant N = 5;
    uint64 public constant Q_CERT = 3; // DKG certification, not decryption
    uint64 public constant TAU = 3; // decryption threshold, not certification
    VeriVotingCertificateGate public immutable certificateGate;
    VeriVotingSettlement public immutable receiver;
    JubjubSchnorrVerifier public immutable verifier;
    VeriVotingSettlement.Binding private binding;
    JubjubSchnorrVerifier.Point[5] private identityKeys;
    JubjubSchnorrVerifier.Point[3] private coefficients;
    bytes32[6] private ciphertexts;
    bytes32 public immutable keyCoreDigest;
    bool public frozenBound;
    bytes32 public freezeSeal;
    bytes32 public cpTarget;
    uint64 public commitmentCount;
    mapping(uint32 => bool) public submitted;
    mapping(uint32 => bytes32) public packageCommitment;
    mapping(uint32 => bytes32) public verifiedShare;
    struct Submission {
        uint32 trusteeId;
        bytes32 electionId;
        bytes32 finalContext;
        bytes32 target;
        JubjubSchnorrVerifier.Point verificationShare;
        JubjubSchnorrVerifier.Point commitment;
        JubjubSchnorrVerifier.Signature signature;
    }
    event FrozenBound(bytes32 seal, bytes32 target, bytes32 keyCoreDigest);
    event CommitmentAuthenticated(uint32 indexed trusteeId, bytes32 commitment, bytes32 verificationShare, bytes32 target);

    constructor(VeriVotingCertificateGate gate, VeriVotingSettlement source,
        VeriVotingSettlement.Binding memory b, bytes32 manifestDigest, bytes32 transcriptDigest,
        JubjubSchnorrVerifier.Point[5] memory keys, JubjubSchnorrVerifier.Point[3] memory a)
    {
        certificateGate = gate;
        receiver = source;
        verifier = gate.verifier();
        require(gate.N() == N && gate.T() == 2 && gate.Q_CERT() == Q_CERT && gate.TAU() == TAU,"PROFILE");
        require(source.bindingDigest() == keccak256(abi.encode(b)),"RECEIVER_BINDING");
        require(b.finalContext == gate.finalContext() && b.keyCoreDigest == gate.keyCoreDigest(),"CERTIFIED_CONTEXT");
        for (uint256 i; i < 5; ++i) require(verifier.validatePoint(keys[i],false),"IDENTITY_KEY");
        for (uint256 i; i < 3; ++i) require(verifier.validatePoint(a[i],i!=0),"COEFFICIENT");
        // Exact original KeyCore bytes. Thus neither identities nor coefficients
        // can be replaced while retaining the gate's certified digest.
        bytes memory core = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/KEYCORE/v2"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(b.electionId,b.policyPre,manifestDigest,b.prekeyContext,b.dkgSession),
            abi.encodePacked(N,uint64(2),TAU,Q_CERT,uint64(5)));
        for (uint256 i; i < 5; ++i) core = bytes.concat(core,abi.encodePacked(uint32(i+1),keys[i].encoded));
        core = bytes.concat(core,abi.encodePacked(uint64(5),uint32(1),uint32(2),uint32(3),uint32(4),uint32(5)));
        for (uint256 i; i < 3; ++i) core = bytes.concat(core,abi.encodePacked(a[i].encoded));
        require(sha256(bytes.concat(core,abi.encodePacked(transcriptDigest))) == gate.keyCoreDigest()
            && a[0].encoded == b.publicKey,"KEY_CORE");
        keyCoreDigest = b.keyCoreDigest;
        binding = b;
        identityKeys = keys;
        coefficients = a;
    }

    function bindFrozen() external {
        require(!frozenBound,"ALREADY_BOUND");
        certificateGate.requireAuthorized();
        require(certificateGate.certifiedRoot() == binding.registrationRoot
            && certificateGate.certifiedCloseDigest() == binding.registrationCloseDigest
            && certificateGate.certifiedDisclosureDigest() == binding.disclosureDigest,"CERTIFICATE_BINDING");
        (uint64 next,uint64 count,bool isFrozen,bytes32 seal,bytes32 target,bytes32[6] memory state) = receiver.checkpoint();
        require(isFrozen && next == 3 && count == 8,"NOT_FROZEN");
        bytes memory t = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/TALLY/TARGET/v3"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(binding.electionId,binding.policyPre,binding.finalContext,
                binding.registrationCloseDigest,binding.disclosureDigest,uint64(3)));
        for (uint256 i; i < 6; ++i) t = bytes.concat(t,state[i]);
        require(sha256(t) == target,"CP_TARGET_BINDING");
        ciphertexts = state;
        cpTarget = target;
        freezeSeal = seal;
        frozenBound = true;
        emit FrozenBound(seal,target,keyCoreDigest);
    }

    function frozenCiphertexts() external view returns (bytes32[6] memory) { return ciphertexts; }
    function checkpoint() external view returns (bool bound, bytes32 seal, bytes32 target, uint64 count, bool ready) {
        return (frozenBound,freezeSeal,cpTarget,commitmentCount,frozenBound && commitmentCount >= TAU);
    }
    function requireP3Ready() external view {
        require(frozenBound && commitmentCount >= TAU,"DECRYPTION_QUORUM");
    }
    function submitCommitment(Submission calldata s) external {
        require(frozenBound,"NOT_BOUND");
        require(s.electionId == binding.electionId,"ELECTION_ID");
        require(s.finalContext == binding.finalContext,"FINAL_CONTEXT");
        require(s.target == cpTarget,"CP_TARGET");
        require(s.trusteeId > 0 && s.trusteeId <= N,"TRUSTEE_ID");
        require(!submitted[s.trusteeId],"DUPLICATE_TRUSTEE");
        require(verifier.validatePoint(s.verificationShare,false)
            && JubjubVerificationShare.matches(coefficients,s.trusteeId,s.verificationShare),"VERIFICATION_SHARE");
        require(verifier.validatePoint(s.commitment,true),"COMMITMENT_POINT");
        // Original package.rs commitment_message(v5), NOT a new signature scheme.
        bytes32 message = sha256(bytes.concat(CanonicalVV.prefix("VV/JUBJUB/PRIVATE-PACKAGE/COMMIT/v5"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(binding.electionId,binding.policyPre,binding.finalContext,
                binding.registrationCloseDigest,binding.disclosureDigest,cpTarget,s.trusteeId,s.commitment.encoded)));
        require(verifier.verify(identityKeys[s.trusteeId-1],bytes("PRIVATE-PACKAGE-COMMITMENT-SIGNATURE"),message,s.signature),"BAD_PACKAGE_SIGNATURE");
        // No private CP response, usable share, opening or tally is accepted.
        submitted[s.trusteeId] = true;
        packageCommitment[s.trusteeId] = s.commitment.encoded;
        verifiedShare[s.trusteeId] = s.verificationShare.encoded;
        ++commitmentCount;
        emit CommitmentAuthenticated(s.trusteeId,s.commitment.encoded,s.verificationShare.encoded,cpTarget);
    }
}
