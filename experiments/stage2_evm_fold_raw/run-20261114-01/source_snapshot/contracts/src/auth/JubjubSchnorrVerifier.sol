// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./Sha512.sol";
import "./CanonicalVV.sol";

/// Exact certificate/package-message signature scheme used by vv-friendly-dkg.
/// NOT secp256k1, NOT CP's wide H2F. A 32-byte message digest is re-encoded here.
contract JubjubSchnorrVerifier {
    uint256 public constant P = 52435875175126190479447740508185965837690552500527637822603658699938581184513;
    uint256 public constant Q = 6554484396890773809930967563523245729705921265872317281365359162392183254199;
    uint256 private constant D = 19257038036680949359750312669786877991949435402254120286184196891950884077233;
    uint256 private constant GX = 8076246640662884909881801758704306714034609987455869804520522091855516602923;
    uint256 private constant GY = 13262374693698910701929044844600465831413122818447359594527400194675274060458;
    struct Point { uint256 x; uint256 y; bytes32 encoded; }
    struct Signature { Point nonce; uint256 response; uint16 counter; }
    struct Extended { uint256 x; uint256 y; uint256 z; uint256 t; }

    function hashSha512(bytes memory data) external pure returns (bytes memory) {
        (bytes32 hi,bytes32 lo) = Sha512.digest(data); return abi.encodePacked(hi,lo);
    }
    function validatePoint(Point memory p, bool allowIdentity) public pure returns (bool) {
        if (p.x >= P || p.y >= P) return false;
        if (!allowIdentity && p.x == 0 && p.y == 1) return false;
        uint256 xx = mulmod(p.x,p.x,P); uint256 yy = mulmod(p.y,p.y,P);
        if (addmod(yy,P-xx,P) != addmod(1,mulmod(D,mulmod(xx,yy,P),P),P)) return false;
        bytes32 encoded = CanonicalVV.littleEndian32(p.y);
        if (p.x > P-p.x) encoded |= bytes32(uint256(0x80)); // Ark TEFlags: high bit of LAST LE byte.
        if (encoded != p.encoded) return false;
        return _identity(_mul(_extended(p),Q));
    }
    function verify(Point memory publicKey, bytes memory domain, bytes32 message, Signature memory sig) public pure returns (bool) {
        if (sig.response >= Q || sig.counter >= 128 || !validatePoint(publicKey,false) || !validatePoint(sig.nonce,false)) return false;
        (uint256 challenge,uint16 counter,bool found) = signatureChallenge(publicKey.encoded,sig.nonce.encoded,domain,message);
        if (!found || counter != sig.counter) return false;
        Extended memory left = _mul(Extended(GX,GY,1,mulmod(GX,GY,P)),sig.response);
        Extended memory right = _add(_extended(sig.nonce),_mul(_extended(publicKey),challenge));
        return left.z != 0 && right.z != 0 && mulmod(left.x,right.z,P) == mulmod(right.x,left.z,P)
            && mulmod(left.y,right.z,P) == mulmod(right.y,left.z,P);
    }
    function signatureChallenge(bytes32 publicKey, bytes32 nonce, bytes memory domain, bytes32 message)
        public pure returns (uint256 candidate,uint16 counter,bool found)
    {
        bytes memory transcript = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/SCHNORR/TRANSCRIPT/v2"),CanonicalVV.text(CanonicalVV.SUITE),
            CanonicalVV.blob(domain),abi.encodePacked(publicKey,nonce),CanonicalVV.blob(abi.encodePacked(message)));
        bytes memory base = bytes.concat(CanonicalVV.prefix("VV/JUBJUB/H2S/v2"),CanonicalVV.text(CanonicalVV.SUITE),
            CanonicalVV.text("SCHNORR-SIGNATURE"),CanonicalVV.blob(transcript));
        for (uint16 i; i < 128; ++i) {
            (bytes32 hi,) = Sha512.digest(bytes.concat(base,abi.encodePacked(i)));
            uint256 value;
            for (uint256 j; j < 32; ++j) value |= uint256(uint8(hi[j]) & (j == 31 ? 15 : 255)) << (8*j);
            if (value < Q) return (value,i,true); // FIRST canonical candidate, never reduction.
        }
        return (0,0,false);
    }
    function _extended(Point memory p) private pure returns (Extended memory) { return Extended(p.x,p.y,1,mulmod(p.x,p.y,P)); }
    function _identity(Extended memory p) private pure returns (bool) { return p.z != 0 && p.x == 0 && p.y == p.z; }
    function _mul(Extended memory p, uint256 scalar) private pure returns (Extended memory r) {
        r = Extended(0,1,1,0);
        while (scalar != 0) { if ((scalar & 1) != 0) r = _add(r,p); p = _add(p,p); scalar >>= 1; }
    }
    // EFD add-2008-hwcd-3: complete extended Edwards addition, a=-1, k=2d.
    function _add(Extended memory p, Extended memory q) private pure returns (Extended memory r) {
        uint256 a = mulmod(addmod(p.y,P-p.x,P),addmod(q.y,P-q.x,P),P);
        uint256 b = mulmod(addmod(p.y,p.x,P),addmod(q.y,q.x,P),P);
        uint256 c = mulmod(mulmod(p.t,q.t,P),2*D,P);
        uint256 d = mulmod(2,mulmod(p.z,q.z,P),P);
        uint256 e = addmod(b,P-a,P); uint256 f = addmod(d,P-c,P);
        uint256 g = addmod(d,c,P); uint256 h = addmod(b,a,P);
        r = Extended(mulmod(e,f,P),mulmod(g,h,P),mulmod(f,g,P),mulmod(e,h,P));
    }
}
