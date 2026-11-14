// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// Original Jubjub/Arkworks compressed encoding. No BLS G1 substitution.
library JubjubPointOps {
    uint256 internal constant P = 52435875175126190479447740508185965837690552500527637822603658699938581184513;
    uint256 internal constant Q = 6554484396890773809930967563523245729705921265872317281365359162392183254199;
    uint256 internal constant D = 19257038036680949359750312669786877991949435402254120286184196891950884077233;
    uint256 internal constant ODD = (P - 1) >> 32;
    struct Extended { uint256 x; uint256 y; uint256 z; uint256 t; }

    function pow(uint256 base, uint256 exponent) private view returns (uint256 result) {
        bytes memory input = abi.encode(uint256(32),uint256(32),uint256(32),base,exponent,P);
        bool ok;
        assembly ("memory-safe") {
            ok := staticcall(gas(),5,add(input,32),mload(input),0,32)
            if eq(returndatasize(),32) { result := mload(0) }
            ok := and(ok,eq(returndatasize(),32))
        }
        require(ok,"JUBJUB_MODEXP");
    }
    function sqrt(uint256 n) private view returns (uint256 x) {
        if (n == 0) return 0;
        // BLS12-381 scalar field has 2-adicity 32; 7 is a quadratic nonresidue.
        uint256 c = pow(7,ODD);
        x = pow(n,(ODD+1)/2);
        uint256 t = pow(n,ODD);
        uint256 m = 32;
        for (uint256 step; step < 32; ++step) {
            if (t == 1) { require(mulmod(x,x,P) == n,"JUBJUB_SQRT"); return x; }
            uint256 i = 1; uint256 v = mulmod(t,t,P);
            while (i < m && v != 1) { v = mulmod(v,v,P); ++i; }
            require(i < m,"JUBJUB_NOT_ON_CURVE");
            uint256 b = c;
            for (uint256 j; j < m-i-1; ++j) b = mulmod(b,b,P);
            x = mulmod(x,b,P); c = mulmod(b,b,P); t = mulmod(t,c,P); m = i;
        }
        revert("JUBJUB_SQRT_BOUND");
    }
    function fromLE(bytes32 raw) private pure returns (uint256 v) {
        for (uint256 i; i < 32; ++i) v |= uint256(uint8(raw[i])) << (8*i);
    }
    function toLE(uint256 v) private pure returns (bytes32) {
        uint256 out;
        for (uint256 i; i < 32; ++i) out |= ((v >> (8*i)) & 255) << (8*(31-i));
        return bytes32(out);
    }
    function encodedAffine(uint256 x,uint256 y) private pure returns (bytes32 out) {
        out = toLE(y);
        if (x > P-x) out |= bytes32(uint256(0x80));
    }
    function decodeCanonical(bytes32 raw) private view returns (Extended memory r) {
        uint256 bits = fromLE(raw);
        bool negative = (bits >> 255) != 0;
        uint256 y = bits & ((uint256(1) << 255)-1);
        require(y < P,"JUBJUB_ENCODING");
        uint256 yy = mulmod(y,y,P);
        uint256 den = addmod(1,mulmod(D,yy,P),P);
        require(den != 0,"JUBJUB_NOT_ON_CURVE");
        uint256 xx = mulmod(addmod(yy,P-1,P),pow(den,P-2),P);
        uint256 x = sqrt(xx);
        if ((x > P-x) != negative) x = (P-x) % P;
        require(encodedAffine(x,y) == raw,"JUBJUB_ENCODING");
        require(addmod(yy,P-mulmod(x,x,P),P) == addmod(1,mulmod(D,mulmod(mulmod(x,x,P),yy,P),P),P),"JUBJUB_NOT_ON_CURVE");
        r = Extended(x,y,1,mulmod(x,y,P));
    }
    function identity(Extended memory p) private pure returns (bool) { return p.z != 0 && p.x == 0 && p.y == p.z; }
    function add(Extended memory p, Extended memory q) internal pure returns (Extended memory r) {
        // Same complete extended Edwards a=-1 formula as the original Schnorr verifier.
        uint256 a = mulmod(addmod(p.y,P-p.x,P),addmod(q.y,P-q.x,P),P);
        uint256 b = mulmod(addmod(p.y,p.x,P),addmod(q.y,q.x,P),P);
        uint256 c = mulmod(mulmod(p.t,q.t,P),2*D,P);
        uint256 d = mulmod(2,mulmod(p.z,q.z,P),P);
        uint256 e = addmod(b,P-a,P); uint256 f = addmod(d,P-c,P);
        uint256 g = addmod(d,c,P); uint256 h = addmod(b,a,P);
        r = Extended(mulmod(e,f,P),mulmod(g,h,P),mulmod(f,g,P),mulmod(e,h,P));
    }
    function decodePrime(bytes32 raw) internal view returns (Extended memory p) {
        p = decodeCanonical(raw);
        Extended memory q = p; Extended memory product = Extended(0,1,1,0);
        uint256 scalar = Q;
        while (scalar != 0) { if ((scalar & 1) != 0) product = add(product,q); q = add(q,q); scalar >>= 1; }
        require(identity(product),"JUBJUB_SUBGROUP");
    }
    /// INTERNAL ONLY: receiver calls this only AFTER the exact ciphertext bytes
    /// pass the original ballot SNARK. That relation fixes C0=rG,C1=rPK+vwG,
    /// with certified prime-order PK; old state is identity or a prior such sum.
    /// Canonical encoding/curve recovery are still computed here on EVM.
    function decodeVerifiedCiphertext(bytes32 raw) internal view returns (Extended memory) {
        return decodeCanonical(raw);
    }
    function encode(Extended memory p) internal view returns (bytes32) {
        require(p.z != 0,"JUBJUB_PROJECTIVE");
        uint256 inv = pow(p.z,P-2);
        return encodedAffine(mulmod(p.x,inv,P),mulmod(p.y,inv,P));
    }
}
