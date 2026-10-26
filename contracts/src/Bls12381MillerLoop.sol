// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

// Miller point/line formulas and NAF schedule are adapted from
// @noble/curves 2.4.0, abstract/bls.js and bls12-381.js:
// noble-curves - MIT License (c) 2022 Paul Miller (paulmillr.com).
//
// The MIT License (MIT)
// Copyright (c) 2022 Paul Miller (https://paulmillr.com)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the “Software”), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// Base/tower arithmetic is copied from this project's Fp12 product component;
// that historical component is intentionally left unchanged.
/// @notice Experimental software Miller-loop arithmetic component only.
/// This is NOT a pairing/GT comparison or SnarkPack verifier: no final
/// exponentiation, curve/subgroup validation, proof binding or identity policy.
/// Input points are six canonical 64-byte BE Fp values in EIP-2537 order:
/// G1.x, G1.y, G2.x.c0, G2.x.c1, G2.y.c0, G2.y.c1.
/// Every projective point update and line coefficient is computed internally.
/// Fp12 output: c0/c1 -> Fp6.c0/c1/c2 -> Fp2.c0/c1, 64-byte BE Fp each.
contract Bls12381MillerLoop {
    uint256 private constant P_HI = 0x1a0111ea397fe69a4b1ba7b6434bacd7;
    uint256 private constant P_LO =
        0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab;

    struct Fp { uint256 hi; uint256 lo; }
    struct Fp2 { Fp c0; Fp c1; }
    struct Fp6 { Fp2 c0; Fp2 c1; Fp2 c2; }
    struct Fp12 { Fp6 c0; Fp6 c1; }
    struct Point2 { Fp2 x; Fp2 y; Fp2 z; }
    struct Line { Fp2 c0; Fp2 c1; Fp2 c2; }

    /// @dev Prefixes below 64 are raw, unconjugated loop accumulators.
    /// At 64 steps the negative-x conjugation matches Noble pairing(..., false).
    function verifyMillerPrefixOrRevert(bytes calldata points, uint256 steps,
        bytes calldata expected) external view returns (bytes memory computed)
    {
        computed = _verify(points, steps, expected);
    }

    function verifyMillerLoopOrRevert(bytes calldata points, bytes calldata expected)
        external view returns (bytes memory computed)
    {
        computed = _verify(points, 64, expected);
    }

    function _verify(bytes calldata points, uint256 steps, bytes calldata expected)
        private view returns (bytes memory computed)
    {
        require(points.length == 384, "POINTS_LENGTH");
        require(expected.length == 768, "FP12_LENGTH");
        require(steps > 0 && steps <= 64, "MILLER_STEPS");
        computed = _encode(_miller(points, steps));
        require(keccak256(computed) == keccak256(expected), "INVALID_MILLER_RESULT");
    }

    // Exact NAfDecomposition of |x|=0xd201000000010000 from pinned Noble.
    // The carry above its original leading bit yields 64 (not 63) loop steps.
    function _naf() private pure returns (int8[64] memory digits) {
        uint256 a = 0xd201000000010000;
        uint256 pos = 64;
        while (a > 1) {
            int8 digit;
            if ((a & 1) != 0) {
                if ((a & 3) == 3) { digit = -1; a += 1; }
                else digit = 1;
            }
            digits[--pos] = digit;
            a >>= 1;
        }
        assert(pos == 0);
    }

    function _miller(bytes calldata points, uint256 steps)
        private view returns (Fp12 memory f)
    {
        Fp memory px = _readFp(points, 0);
        Fp memory py = _readFp(points, 64);
        Point2 memory q = Point2(_read2(points, 128), _read2(points, 256), _one2());
        Point2 memory r = Point2(q.x, q.y, _one2());
        int8[64] memory naf = _naf();
        f = Fp12(Fp6(_one2(), _zero2(), _zero2()),
            Fp6(_zero2(), _zero2(), _zero2()));
        Line memory ell;
        for (uint256 i; i < steps; ++i) {
            // Match Noble's skip of square(ONE) on its first iteration.
            if (i != 0) f = _mul12(f, f);
            (r, ell) = _pointDouble(r);
            f = _lineProduct(f, ell, px, py);
            if (naf[i] != 0) {
                Fp2 memory qy = naf[i] == -1 ? _neg2(q.y) : q.y;
                (r, ell) = _pointAdd(r, q.x, qy);
                f = _lineProduct(f, ell, px, py);
            }
        }
        // BLS12-381 x is negative. Do not conjugate incomplete prefixes.
        if (steps == 64) f = Fp12(f.c0, _neg6(f.c1));
    }

    // Homogeneous projective doubling, including its exact line coefficients.
    function _pointDouble(Point2 memory p)
        private view returns (Point2 memory r, Line memory ell)
    {
        Fp2[5] memory t;
        t[0] = _mul2(p.y, p.y);
        t[1] = _mul2(p.z, p.z);
        t[2] = _timesB(_triple2(t[1]));
        t[3] = _triple2(t[2]);
        Fp2 memory yz = _add2(p.y, p.z);
        t[4] = _sub2(_sub2(_mul2(yz, yz), t[1]), t[0]);
        ell = Line(_sub2(t[2], t[0]), _triple2(_mul2(p.x, p.x)), _neg2(t[4]));
        r.x = _half2(_mul2(_mul2(_sub2(t[0], t[3]), p.x), p.y));
        Fp2 memory h = _half2(_add2(t[0], t[3]));
        r.y = _sub2(_mul2(h, h), _triple2(_mul2(t[2], t[2])));
        r.z = _mul2(t[0], t[4]);
    }

    // Mixed homogeneous addition; qy is negated for the signed -1 NAF digit.
    function _pointAdd(Point2 memory p, Fp2 memory qx, Fp2 memory qy)
        private view returns (Point2 memory r, Line memory ell)
    {
        Fp2[6] memory t;
        t[0] = _sub2(p.y, _mul2(qy, p.z));
        t[1] = _sub2(p.x, _mul2(qx, p.z));
        ell = Line(_sub2(_mul2(t[0], qx), _mul2(t[1], qy)), _neg2(t[0]), t[1]);
        t[2] = _mul2(t[1], t[1]);
        t[3] = _mul2(t[2], t[1]);
        t[4] = _mul2(t[2], p.x);
        t[5] = _add2(_sub2(t[3], _add2(t[4], t[4])),
            _mul2(_mul2(t[0], t[0]), p.z));
        r.x = _mul2(t[1], t[5]);
        r.y = _sub2(_mul2(_sub2(t[4], t[5]), t[0]), _mul2(t[3], p.y));
        r.z = _mul2(p.z, t[3]);
    }

    // Multiplicative twist uses sparse slots 0,1,4. The general tower product
    // intentionally prioritizes auditability over a separate sparse optimizer.
    function _lineProduct(Fp12 memory f, Line memory ell, Fp memory px, Fp memory py)
        private view returns (Fp12 memory)
    {
        Fp12 memory sparse = Fp12(
            Fp6(ell.c0, _scale2(ell.c1, px), _zero2()),
            Fp6(_zero2(), _scale2(ell.c2, py), _zero2()));
        return _mul12(f, sparse);
    }

    function _zero2() private pure returns (Fp2 memory) {
        return Fp2(Fp(0, 0), Fp(0, 0));
    }
    function _one2() private pure returns (Fp2 memory) {
        return Fp2(Fp(0, 1), Fp(0, 0));
    }
    function _neg2(Fp2 memory a) private pure returns (Fp2 memory) {
        return _sub2(_zero2(), a);
    }
    function _neg6(Fp6 memory a) private pure returns (Fp6 memory) {
        return Fp6(_neg2(a.c0), _neg2(a.c1), _neg2(a.c2));
    }
    function _half2(Fp2 memory a) private pure returns (Fp2 memory) {
        return Fp2(_half(a.c0), _half(a.c1));
    }
    function _triple2(Fp2 memory a) private pure returns (Fp2 memory) {
        return _add2(_add2(a, a), a);
    }
    function _scale2(Fp2 memory a, Fp memory b) private view returns (Fp2 memory) {
        return Fp2(_mul(a.c0, b), _mul(a.c1, b));
    }
    function _timesB(Fp2 memory a) private pure returns (Fp2 memory) {
        // Twist B=4(1+u), as pinned Noble Fp2mulByB.
        Fp2 memory four = _add2(a, a);
        four = _add2(four, four);
        return _xi(four);
    }

    function _ge(Fp memory a, Fp memory b) private pure returns (bool) {
        return a.hi > b.hi || (a.hi == b.hi && a.lo >= b.lo);
    }
    function _rawAdd(Fp memory a, Fp memory b) private pure returns (Fp memory r) {
        unchecked {
            uint256 lo = a.lo + b.lo;
            r = Fp(a.hi + b.hi + (lo < a.lo ? 1 : 0), lo);
        }
    }
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
        Fp memory even = a;
        if ((a.lo & 1) != 0) even = _rawAdd(a, Fp(P_HI, P_LO));
        r = Fp(even.hi >> 1, (even.lo >> 1) | (even.hi << 255));
    }
    function _square(Fp memory a) private view returns (Fp memory r) {
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
        return _half(_half(_sub(_square(_add(a, b)), _square(_sub(a, b)))));
    }
    function _add2(Fp2 memory a, Fp2 memory b) private pure returns (Fp2 memory) {
        return Fp2(_add(a.c0, b.c0), _add(a.c1, b.c1));
    }
    function _sub2(Fp2 memory a, Fp2 memory b) private pure returns (Fp2 memory) {
        return Fp2(_sub(a.c0, b.c0), _sub(a.c1, b.c1));
    }
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
