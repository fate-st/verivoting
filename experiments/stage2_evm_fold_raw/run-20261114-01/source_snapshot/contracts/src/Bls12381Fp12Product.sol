// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// @notice Experimental Fp12 multiplication component, NOT a SnarkPack verifier.
/// Canonical codec: 12 Fp coefficients, each 64-byte big endian, in the tower
/// Fp12.c0/c1 -> Fp6.c0/c1/c2 -> Fp2.c0/c1. No GT subgroup or proof binding check.
contract Bls12381Fp12Product {
    uint256 private constant P_HI = 0x1a0111ea397fe69a4b1ba7b6434bacd7;
    uint256 private constant P_LO =
        0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab;

    struct Fp { uint256 hi; uint256 lo; }
    struct Fp2 { Fp c0; Fp c1; }
    struct Fp6 { Fp2 c0; Fp2 c1; Fp2 c2; }
    struct Fp12 { Fp6 c0; Fp6 c1; }

    function verifyProductOrRevert(bytes calldata left, bytes calldata right,
        bytes calldata expected) external view returns (bytes memory computed)
    {
        require(expected.length == 768, "FP12_LENGTH");
        computed = _encode(_mul12(_decode(left), _decode(right)));
        require(keccak256(computed) == keccak256(expected), "INVALID_FP12_PRODUCT");
    }

    function _ge(Fp memory a, Fp memory b) private pure returns (bool) {
        return a.hi > b.hi || (a.hi == b.hi && a.lo >= b.lo);
    }

    function _rawAdd(Fp memory a, Fp memory b) private pure returns (Fp memory r) {
        unchecked {
            uint256 lo = a.lo + b.lo;
            // All callers add values below p, or one p to a value below p.
            // The high limb is therefore below 2^126, with no high overflow.
            r = Fp(a.hi + b.hi + (lo < a.lo ? 1 : 0), lo);
        }
    }

    // Called only with a >= b; unchecked low-word subtraction implements borrow.
    function _rawSub(Fp memory a, Fp memory b) private pure returns (Fp memory r) {
        unchecked { r = Fp(a.hi - b.hi - (a.lo < b.lo ? 1 : 0), a.lo - b.lo); }
    }

    function _add(Fp memory a, Fp memory b) private pure returns (Fp memory r) {
        r = _rawAdd(a, b);
        Fp memory p = Fp(P_HI, P_LO);
        if (_ge(r, p)) r = _rawSub(r, p);
    }

    function _sub(Fp memory a, Fp memory b) private pure returns (Fp memory) {
        if (_ge(a, b)) return _rawSub(a, b);
        return _rawSub(Fp(P_HI, P_LO), _rawSub(b, a));
    }

    function _half(Fp memory a) private pure returns (Fp memory r) {
        // Do not reduce a+p before shifting: (a+p)/2 represents a/2 mod p.
        Fp memory even = a;
        if ((a.lo & 1) != 0) even = _rawAdd(a, Fp(P_HI, P_LO));
        r = Fp(even.hi >> 1, (even.lo >> 1) | (even.hi << 255));
    }

    function _square(Fp memory a) private view returns (Fp memory r) {
        // EIP-198 MODEXP: [baseLen, expLen, modulusLen, base(2 words),
        // exponent, modulus(2 words)]. This handles the FULL 381-bit modulus.
        uint256[8] memory input = [uint256(64), 32, 64, a.hi, a.lo, 2, P_HI, P_LO];
        r = Fp(0, 0);
        bool ok;
        uint256 outputSize;
        assembly ("memory-safe") {
            ok := staticcall(gas(), 5, input, 256, r, 64)
            outputSize := returndatasize()
        }
        require(ok && outputSize == 64, "MODEXP_FAILED");
    }

    function _mul(Fp memory a, Fp memory b) private view returns (Fp memory) {
        // xy = ((x+y)^2 - (x-y)^2)/4 mod p: two true MODEXP calls,
        // no 256-bit mulmod truncation and no host-provided product witness.
        return _half(_half(_sub(_square(_add(a, b)), _square(_sub(a, b)))));
    }

    function _add2(Fp2 memory a, Fp2 memory b) private pure returns (Fp2 memory) {
        return Fp2(_add(a.c0, b.c0), _add(a.c1, b.c1));
    }

    function _sub2(Fp2 memory a, Fp2 memory b) private pure returns (Fp2 memory) {
        return Fp2(_sub(a.c0, b.c0), _sub(a.c1, b.c1));
    }

    // u^2 = -1; three base-field multiplications.
    function _mul2(Fp2 memory a, Fp2 memory b) private view returns (Fp2 memory) {
        Fp memory t0 = _mul(a.c0, b.c0);
        Fp memory t1 = _mul(a.c1, b.c1);
        return Fp2(_sub(t0, t1),
            _sub(_sub(_mul(_add(a.c0, a.c1), _add(b.c0, b.c1)), t0), t1));
    }

    function _xi(Fp2 memory a) private pure returns (Fp2 memory) {
        return Fp2(_sub(a.c0, a.c1), _add(a.c0, a.c1));
    }

    function _add6(Fp6 memory a, Fp6 memory b) private pure returns (Fp6 memory) {
        return Fp6(_add2(a.c0, b.c0), _add2(a.c1, b.c1), _add2(a.c2, b.c2));
    }

    function _sub6(Fp6 memory a, Fp6 memory b) private pure returns (Fp6 memory) {
        return Fp6(_sub2(a.c0, b.c0), _sub2(a.c1, b.c1), _sub2(a.c2, b.c2));
    }

    // v^3 = xi = 1+u; six Fp2 multiplications (Karatsuba).
    function _mul6(Fp6 memory a, Fp6 memory b) private view returns (Fp6 memory) {
        Fp2 memory t0 = _mul2(a.c0, b.c0);
        Fp2 memory t1 = _mul2(a.c1, b.c1);
        Fp2 memory t2 = _mul2(a.c2, b.c2);
        Fp2 memory c0 = _add2(t0,
            _xi(_sub2(_sub2(_mul2(_add2(a.c1, a.c2), _add2(b.c1, b.c2)), t1), t2)));
        Fp2 memory c1 = _add2(
            _sub2(_sub2(_mul2(_add2(a.c0, a.c1), _add2(b.c0, b.c1)), t0), t1), _xi(t2));
        Fp2 memory c2 = _add2(
            _sub2(_sub2(_mul2(_add2(a.c0, a.c2), _add2(b.c0, b.c2)), t0), t2), t1);
        return Fp6(c0, c1, c2);
    }

    function _timesV(Fp6 memory a) private pure returns (Fp6 memory) {
        return Fp6(_xi(a.c2), a.c0, a.c1);
    }

    // w^2=v. Three Fp6 multiplications = 54 base multiplies = 108 MODEXP calls.
    function _mul12(Fp12 memory a, Fp12 memory b) private view returns (Fp12 memory) {
        Fp6 memory t0 = _mul6(a.c0, b.c0);
        Fp6 memory t1 = _mul6(a.c1, b.c1);
        return Fp12(_add6(t0, _timesV(t1)),
            _sub6(_sub6(_mul6(_add6(a.c0, a.c1), _add6(b.c0, b.c1)), t0), t1));
    }

    function _readFp(bytes calldata data, uint256 offset) private pure returns (Fp memory r) {
        uint256 hi;
        uint256 lo;
        assembly ("memory-safe") {
            hi := calldataload(add(data.offset, offset))
            lo := calldataload(add(add(data.offset, offset), 32))
        }
        r = Fp(hi, lo);
        require(!_ge(r, Fp(P_HI, P_LO)), "NONCANONICAL_FP");
    }

    function _read2(bytes calldata data, uint256 offset) private pure returns (Fp2 memory) {
        return Fp2(_readFp(data, offset), _readFp(data, offset + 64));
    }

    function _read6(bytes calldata data, uint256 offset) private pure returns (Fp6 memory) {
        return Fp6(_read2(data, offset), _read2(data, offset + 128), _read2(data, offset + 256));
    }

    function _decode(bytes calldata data) private pure returns (Fp12 memory) {
        require(data.length == 768, "FP12_LENGTH");
        return Fp12(_read6(data, 0), _read6(data, 384));
    }

    function _writeFp(bytes memory data, uint256 offset, Fp memory a) private pure {
        uint256 hi = a.hi;
        uint256 lo = a.lo;
        assembly ("memory-safe") {
            mstore(add(add(data, 32), offset), hi)
            mstore(add(add(data, 64), offset), lo)
        }
    }

    function _write2(bytes memory data, uint256 offset, Fp2 memory a) private pure {
        _writeFp(data, offset, a.c0);
        _writeFp(data, offset + 64, a.c1);
    }

    function _write6(bytes memory data, uint256 offset, Fp6 memory a) private pure {
        _write2(data, offset, a.c0);
        _write2(data, offset + 128, a.c1);
        _write2(data, offset + 256, a.c2);
    }

    function _encode(Fp12 memory a) private pure returns (bytes memory data) {
        data = new bytes(768);
        _write6(data, 0, a.c0);
        _write6(data, 384, a.c1);
    }
}
