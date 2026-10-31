// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./JubjubSchnorrVerifier.sol";

/// G8-A standalone certificate gate. Bootstrap identity keys/authority remain trust
/// anchors. Receiver/P3/winner integration is explicitly NOT implemented here.
contract VeriVotingCertificateGate {
    uint64 public constant N = 5;
    uint64 public constant T = 2;
    uint64 public constant Q_CERT = N-T; // key certification quorum
    uint64 public constant TAU = T+1; // private DECRYPTION threshold, NOT a certificate count
    string private constant RULE = "highest-weight-smallest-id-on-tie";
    struct Expected {
        bytes32 electionId;
        bytes32 manifestDigest;
        bytes32 policyPre;
        bytes32 prekeyContext;
        bytes32 dkgSession;
        bytes32 keyCoreDigest;
        bytes32 finalContext;
        bytes32 publicKey;
        bytes32 registrationRoot;
        bytes32 closeDigest;
        bytes32 disclosureDigest;
    }
    struct Close {
        bytes32 electionId;
        bytes32 finalContext;
        bytes32 policyPre;
        bytes32 root;
        uint64 nRegistered;
        uint64 wRegistered;
        uint64 wMax;
        uint32 version;
        bytes32 canonicalRegistryDigest;
    }
    Expected private expected;
    JubjubSchnorrVerifier.Point[5] private authenticationKeys;
    JubjubSchnorrVerifier.Point private registrationAuthority;
    JubjubSchnorrVerifier public immutable verifier;
    bytes32 public immutable keyCoreDigest;
    bytes32 public immutable finalContext;
    bytes32 public immutable bootstrapDigest;
    mapping(uint32 => bool) public certifiedSigner;
    uint64 public certificateCount;
    bool public dkgCertified;
    bool public registrationCertified;
    bytes32 public certifiedRoot;
    bytes32 public certifiedCloseDigest;
    bytes32 public certifiedDisclosureDigest;
    event KeyCertified(bytes32 keyCoreDigest, uint64 quorum);
    event RegistrationCertified(bytes32 root, bytes32 closeDigest, bytes32 disclosureDigest);

    constructor(Expected memory b, JubjubSchnorrVerifier.Point[5] memory keys,
        JubjubSchnorrVerifier.Point memory authority, JubjubSchnorrVerifier.Point[3] memory coefficients,
        bytes32 transcriptDigest)
    {
        verifier = new JubjubSchnorrVerifier();
        for (uint256 i; i < 5; ++i) require(verifier.validatePoint(keys[i],false),"BOOTSTRAP_KEY");
        require(verifier.validatePoint(authority,false),"AUTHORITY_KEY");
        for (uint256 i; i < 3; ++i) require(verifier.validatePoint(coefficients[i],i!=0),"COEFFICIENT_POINT");
        authenticationKeys = keys;
        registrationAuthority = authority;
        bootstrapDigest = keccak256(abi.encode(b.electionId,b.manifestDigest,keys,authority,N,T,Q_CERT,TAU));
        bytes memory policy = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/REGISTRATION/POLICY/v1"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(b.electionId,authority.encoded,uint64(32)),CanonicalVV.text(RULE),abi.encodePacked(uint32(1)));
        bytes memory pre = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/POLICY-PRE/v3"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(b.electionId,b.manifestDigest,N,T,TAU,Q_CERT,uint64(5)));
        for (uint256 i; i < 5; ++i) pre = bytes.concat(pre,abi.encodePacked(uint32(i+1),keys[i].encoded));
        bytes32 prePolicy = sha256(bytes.concat(pre,abi.encodePacked(sha256(policy))));
        require(prePolicy == b.policyPre,"POLICY_PRE");
        bytes32 prekey = sha256(bytes.concat(CanonicalVV.prefix("VV/PREKEY/v3"),abi.encodePacked(b.electionId,prePolicy,b.manifestDigest)));
        require(prekey == b.prekeyContext,"PREKEY_CONTEXT");
        bytes32 session = sha256(bytes.concat(CanonicalVV.prefix("VV/DKG/SESSION/v1"),abi.encodePacked(prekey)));
        require(session == b.dkgSession,"DKG_SESSION");
        bytes memory core = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/KEYCORE/v2"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(b.electionId,prePolicy,b.manifestDigest,prekey,session),abi.encodePacked(N,T,TAU,Q_CERT,uint64(5)));
        for (uint256 i; i < 5; ++i) core = bytes.concat(core,abi.encodePacked(uint32(i+1),keys[i].encoded));
        core = bytes.concat(core,abi.encodePacked(uint64(5),uint32(1),uint32(2),uint32(3),uint32(4),uint32(5)));
        for (uint256 i; i < 3; ++i) core = bytes.concat(core,abi.encodePacked(coefficients[i].encoded));
        bytes32 digest = sha256(bytes.concat(core,abi.encodePacked(transcriptDigest)));
        require(digest == b.keyCoreDigest && coefficients[0].encoded == b.publicKey,"KEY_CORE");
        keyCoreDigest = digest;
        bytes32 context = sha256(bytes.concat(CanonicalVV.prefix("VV/FINALCONTEXT/v3"),abi.encodePacked(prekey,digest)));
        require(context == b.finalContext,"FINAL_CONTEXT");
        finalContext = context;
        expected = b;
    }

    function checkpoint() external view returns (uint64 count, bool dkg, bool registration,
        bytes32 root, bytes32 closeDigest, bytes32 disclosureDigest, bool authorized)
    {
        return (certificateCount,dkgCertified,registrationCertified,certifiedRoot,certifiedCloseDigest,
            certifiedDisclosureDigest,dkgCertified && registrationCertified);
    }
    function requireAuthorized() external view {
        require(dkgCertified && registrationCertified,"UNCERTIFIED");
    }
    function submitDkgSignature(uint32 trusteeId, bytes32 electionId, bytes32 context,
        JubjubSchnorrVerifier.Signature calldata sig) external
    {
        require(!dkgCertified,"DKG_ALREADY_CERTIFIED");
        require(electionId == expected.electionId,"ELECTION_ID");
        require(context == finalContext,"DKG_CONTEXT");
        require(trusteeId > 0 && trusteeId <= N,"TRUSTEE_ID");
        require(!certifiedSigner[trusteeId],"DUPLICATE_CERTIFIER");
        require(verifier.verify(authenticationKeys[trusteeId-1],bytes("DKG-CERT-SIGNATURE"),keyCoreDigest,sig),"BAD_DKG_SIGNATURE");
        certifiedSigner[trusteeId] = true;
        ++certificateCount;
        if (certificateCount >= Q_CERT) {
            dkgCertified = true;
            emit KeyCertified(keyCoreDigest,certificateCount);
        }
    }

    function authenticateRegistration(Close calldata c, JubjubSchnorrVerifier.Signature calldata sig) external {
        require(dkgCertified,"DKG_QUORUM");
        require(!registrationCertified,"REGISTRATION_ALREADY_CERTIFIED");
        require(c.electionId == expected.electionId && c.finalContext == finalContext && c.policyPre == expected.policyPre,"REGISTRATION_CONTEXT");
        require(c.root == expected.registrationRoot && c.wMax == 32 && c.wRegistered <= c.wMax
            && c.nRegistered > 0 && c.nRegistered <= 32 && c.version == 1,"REGISTRATION_FIELDS");
        bytes memory close = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/REGISTER/CLOSE/RECORD/v1"),CanonicalVV.text(CanonicalVV.SUITE),
            abi.encodePacked(c.electionId,c.finalContext,c.policyPre,c.root,c.nRegistered,c.wRegistered,c.wMax,c.version,c.canonicalRegistryDigest));
        bytes32 closeDigest = sha256(bytes.concat(CanonicalVV.prefix("VV/JUBJUB/REGISTER/CLOSE-DIGEST/v1"),CanonicalVV.blob(close)));
        require(closeDigest == expected.closeDigest,"REGISTRATION_CLOSE_DIGEST");
        bytes32 message = sha256(bytes.concat(CanonicalVV.prefix("VV/JUBJUB/REGISTER/CLOSE/v1"),CanonicalVV.blob(close)));
        require(verifier.verify(registrationAuthority,bytes("REGISTRATION-CLOSE-SIGNATURE"),message,sig),"BAD_REGISTRATION_SIGNATURE");
        bytes memory signature = abi.encodePacked(sig.nonce.encoded,CanonicalVV.littleEndian32(sig.response),sig.counter);
        bytes memory certificate = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/REGISTER/CERTIFICATE/v1"),CanonicalVV.blob(close),CanonicalVV.blob(signature));
        bytes32 disclosureDigest = sha256(bytes.concat(CanonicalVV.prefix("VV/JUBJUB/DISCLOSURE/CLOSE/v1"),CanonicalVV.blob(certificate),CanonicalVV.text(RULE)));
        require(disclosureDigest == expected.disclosureDigest,"DISCLOSURE_DIGEST");
        certifiedRoot = c.root;
        certifiedCloseDigest = closeDigest;
        certifiedDisclosureDigest = disclosureDigest;
        registrationCertified = true;
        emit RegistrationCertified(c.root,closeDigest,disclosureDigest);
    }
}
