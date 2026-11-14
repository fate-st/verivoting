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
// Sparse-arithmetic variant: persistent objects are deeply allocated before an arena
// watermark. Each arithmetic operation writes its live result into those
// objects before scratch rewind. Base field/point formulas stay unchanged; square/014 are specialized.
contract Bls12381MillerLoopAddLowLevel {
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
        // This must be a DEEP clone: the immutable Q must not share Fp limbs
        // with R, whose preallocated limbs will be updated in place.
        Point2 memory r = Point2(_clone2(q.x), _clone2(q.y), _one2());
        int8[64] memory naf = _naf();
        f = Fp12(Fp6(_one2(), _zero2(), _zero2()),
            Fp6(_zero2(), _zero2(), _zero2()));
        Line memory ell = Line(_zero2(), _zero2(), _zero2());
        uint256 scratch;
        assembly { scratch := mload(0x40) }
        for (uint256 i; i < steps; ++i) {
            // Match Noble's skip of square(ONE) on its first iteration.
            if (i != 0) {
                _copy12(f, _square12(f));
                _rewind(scratch);
            }
            _doubleInto(r, ell);
            _rewind(scratch);
            _copy12(f, _lineProduct(f, ell, px, py));
            _rewind(scratch);
            if (naf[i] != 0) {
                _addInto(r, ell, q, naf[i]);
                _rewind(scratch);
                _copy12(f, _lineProduct(f, ell, px, py));
                _rewind(scratch);
            }
        }
        // BLS12-381 x is negative. Do not conjugate incomplete prefixes.
        if (steps == 64) {
            _copy6(f.c1, _neg6(f.c1));
            _rewind(scratch);
        }
    }

    // Deliberately NOT marked memory-safe: Solidity's usual allocator assumes
    // a monotonically increasing free-memory pointer. This variant is pinned
    // to solc 0.8.30, optimizer 200, without viaIR. No pointer above 'scratch'
    // may survive one of the operation boundaries in _miller.
    function _rewind(uint256 scratch) private pure {
        assembly { mstore(0x40, scratch) }
    }

    function _clone2(Fp2 memory a) private pure returns (Fp2 memory) {
        return Fp2(Fp(a.c0.hi, a.c0.lo), Fp(a.c1.hi, a.c1.lo));
    }
    function _copyFp(Fp memory destination, Fp memory source) private pure {
        destination.hi = source.hi;
        destination.lo = source.lo;
    }
    function _copy2(Fp2 memory destination, Fp2 memory source) private pure {
        _copyFp(destination.c0, source.c0);
        _copyFp(destination.c1, source.c1);
    }
    function _copy6(Fp6 memory destination, Fp6 memory source) private pure {
        _copy2(destination.c0, source.c0);
        _copy2(destination.c1, source.c1);
        _copy2(destination.c2, source.c2);
    }
    function _copy12(Fp12 memory destination, Fp12 memory source) private pure {
        _copy6(destination.c0, source.c0);
        _copy6(destination.c1, source.c1);
    }
    function _copyPoint(Point2 memory destination, Point2 memory source) private pure {
        _copy2(destination.x, source.x);
        _copy2(destination.y, source.y);
        _copy2(destination.z, source.z);
    }
    function _copyLine(Line memory destination, Line memory source) private pure {
        _copy2(destination.c0, source.c0);
        _copy2(destination.c1, source.c1);
        _copy2(destination.c2, source.c2);
    }
    function _doubleInto(Point2 memory r, Line memory ell) private view {
        (Point2 memory next, Line memory line) = _pointDouble(r);
        _copyPoint(r, next);
        _copyLine(ell, line);
    }
    function _addInto(Point2 memory r, Line memory ell, Point2 memory q, int8 digit)
        private view
    {
        Fp2 memory qy = digit == -1 ? _neg2(q.y) : q.y;
        (Point2 memory next, Line memory line) = _pointAdd(r, q.x, qy);
        _copyPoint(r, next);
        _copyLine(ell, line);
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

    // Dedicated sparse-014 multiplication: 13 Fp2 products plus four
    // base-field products for scaling the line by Px/Py.
    function _lineProduct(Fp12 memory f, Line memory ell, Fp memory px, Fp memory py)
        private view returns (Fp12 memory)
    {
        Fp2 memory o1 = _scale2(ell.c1, px);
        Fp2 memory o4 = _scale2(ell.c2, py);
        Fp6 memory t0 = _mul6By01(f.c0, ell.c0, o1);
        Fp6 memory t1 = _mul6By1(f.c1, o4);
        Fp6 memory t2 = _mul6By01(_add6(f.c0, f.c1), ell.c0, _add2(o1, o4));
        return Fp12(_add6(t0, _timesV(t1)), _sub6(_sub6(t2, t0), t1));
    }

    // Five Fp2 products, rather than the six in a general Fp6 multiplication.
    function _mul6By01(Fp6 memory a, Fp2 memory b0, Fp2 memory b1)
        private view returns (Fp6 memory)
    {
        Fp2 memory t0 = _mul2(a.c0, b0);
        Fp2 memory t1 = _mul2(a.c1, b1);
        Fp2 memory c0 = _add2(t0, _xi(_sub2(_mul2(_add2(a.c1, a.c2), b1), t1)));
        Fp2 memory c1 = _sub2(_sub2(_mul2(_add2(a.c0, a.c1), _add2(b0, b1)), t0), t1);
        Fp2 memory c2 = _add2(_sub2(_mul2(_add2(a.c0, a.c2), b0), t0), t1);
        return Fp6(c0, c1, c2);
    }

    // Multiplication by (0,b1,0) in Fp6: three Fp2 products.
    function _mul6By1(Fp6 memory a, Fp2 memory b1) private view returns (Fp6 memory) {
        return Fp6(_xi(_mul2(a.c2, b1)), _mul2(a.c0, b1), _mul2(a.c1, b1));
    }

    // General Fp12 square, NOT cyclotomic squaring: Miller accumulators are
    // not yet in GT. Two Fp6 multiplications, 72 MODEXP calls rather than 108.
    function _square12(Fp12 memory a) private view returns (Fp12 memory) {
        Fp6 memory ab = _mul6(a.c0, a.c1);
        Fp6 memory c0 = _sub6(_sub6(
            _mul6(_add6(_timesV(a.c1), a.c0), _add6(a.c0, a.c1)), ab), _timesV(ab));
        return Fp12(c0, _add6(ab, ab));
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
    function _mul(Fp memory a, Fp memory b) private view returns (Fp memory r) {
        // Same identity, avoiding intermediate structs and per-square packing.
        assembly ("memory-safe") {
            function addMod(ah, al, bh, bl) -> rh, rl {
                rl := add(al, bl)
                rh := add(add(ah, bh), lt(rl, al))
                let ph := 0x1a0111ea397fe69a4b1ba7b6434bacd7
                let pl := 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab
                // Canonical inputs imply a sum below 2p.
                if or(gt(rh, ph), and(eq(rh, ph), iszero(lt(rl, pl)))) {
                    rh := sub(sub(rh, ph), lt(rl, pl))
                    rl := sub(rl, pl)
                }
            }
            function subMod(ah, al, bh, bl) -> rh, rl {
                rl := sub(al, bl)
                rh := sub(sub(ah, bh), lt(al, bl))
                if or(lt(ah, bh), and(eq(ah, bh), lt(al, bl))) {
                    let oldLo := rl
                    rl := add(rl, 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab)
                    rh := add(add(rh, 0x1a0111ea397fe69a4b1ba7b6434bacd7), lt(rl, oldLo))
                }
            }
            function halfMod(hi, lo) -> rh, rl {
                // Raw add p before shifting: do NOT reduce modulo p here.
                if and(lo, 1) {
                    let oldLo := lo
                    lo := add(lo, 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab)
                    hi := add(add(hi, 0x1a0111ea397fe69a4b1ba7b6434bacd7), lt(lo, oldLo))
                }
                rh := shr(1, hi)
                rl := or(shr(1, lo), shl(255, hi))
            }
            function square(input, output, hi, lo) -> rh, rl {
                mstore(add(input, 96), hi)
                mstore(add(input, 128), lo)
                let ok := staticcall(gas(), 5, input, 256, output, 64)
                if iszero(and(ok, eq(returndatasize(), 64))) {
                    // ABI Error("MODEXP_FAILED"), in allocated scratch.
                    mstore(input, shl(224, 0x08c379a0))
                    mstore(add(input, 4), 32)
                    mstore(add(input, 36), 13)
                    mstore(add(input, 68), 0x4d4f444558505f4641494c454400000000000000000000000000000000000000)
                    revert(input, 100)
                }
                rh := mload(output)
                rl := mload(add(output, 32))
            }

            // Stable 64-byte result and a reusable 256-byte MODEXP input.
            // Both calls reuse the result slot before its final value is set.
            r := mload(0x40)
            let input := add(r, 64)
            mstore(0x40, add(input, 256))
            mstore(input, 64)
            mstore(add(input, 32), 32)
            mstore(add(input, 64), 64)
            mstore(add(input, 160), 2)
            mstore(add(input, 192), 0x1a0111ea397fe69a4b1ba7b6434bacd7)
            mstore(add(input, 224), 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab)

            let leftHi, leftLo
            {
                let hi, lo := addMod(mload(a), mload(add(a, 32)), mload(b), mload(add(b, 32)))
                leftHi, leftLo := square(input, r, hi, lo)
            }
            let rightHi, rightLo
            {
                let hi, lo := subMod(mload(a), mload(add(a, 32)), mload(b), mload(add(b, 32)))
                rightHi, rightLo := square(input, r, hi, lo)
            }
            let hi, lo := subMod(leftHi, leftLo, rightHi, rightLo)
            hi, lo := halfMod(hi, lo)
            hi, lo := halfMod(hi, lo)
            mstore(r, hi)
            mstore(add(r, 32), lo)
        }
    }
    function _add2(Fp2 memory a, Fp2 memory b) private pure returns (Fp2 memory r) {
        assembly ("memory-safe") {
            function addInto(output, left, right) {
                let al := mload(add(left, 32))
                let bl := mload(add(right, 32))
                let lo := add(al, bl)
                let hi := add(add(mload(left), mload(right)), lt(lo, al))
                let ph := 0x1a0111ea397fe69a4b1ba7b6434bacd7
                let pl := 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab
                // Each input is canonical, so the sum is below 2p.
                if or(gt(hi, ph), and(eq(hi, ph), iszero(lt(lo, pl)))) {
                    hi := sub(sub(hi, ph), lt(lo, pl))
                    lo := sub(lo, pl)
                }
                mstore(output, hi)
                mstore(add(output, 32), lo)
            }
            // Fresh shell and two disjoint Fp values; never modify a or b.
            r := mload(0x40)
            let c0 := add(r, 64)
            let c1 := add(r, 128)
            mstore(0x40, add(r, 192))
            mstore(r, c0)
            mstore(add(r, 32), c1)
            addInto(c0, mload(a), mload(b))
            addInto(c1, mload(add(a, 32)), mload(add(b, 32)))
        }
    }
    function _sub2(Fp2 memory a, Fp2 memory b) private pure returns (Fp2 memory) {
        return Fp2(_sub(a.c0, b.c0), _sub(a.c1, b.c1));
    }
    function _mul2(Fp2 memory a, Fp2 memory b) private view returns (Fp2 memory r) {
        Fp memory sumA;
        Fp memory sumB;
        assembly ("memory-safe") {
            function addInto(output, left, right) {
                let al := mload(add(left, 32))
                let bl := mload(add(right, 32))
                let lo := add(al, bl)
                let hi := add(add(mload(left), mload(right)), lt(lo, al))
                let ph := 0x1a0111ea397fe69a4b1ba7b6434bacd7
                let pl := 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab
                // Each input is canonical, so one subtraction suffices.
                if or(gt(hi, ph), and(eq(hi, ph), iszero(lt(lo, pl)))) {
                    hi := sub(sub(hi, ph), lt(lo, pl))
                    lo := sub(lo, pl)
                }
                mstore(output, hi)
                mstore(add(output, 32), lo)
            }
            // Separate sum buffers; the decoded inputs are never overwritten.
            sumA := mload(0x40)
            sumB := add(sumA, 64)
            mstore(0x40, add(sumB, 64))
            addInto(sumA, mload(a), mload(add(a, 32)))
            addInto(sumB, mload(b), mload(add(b, 32)))
        }
        // The same three Karatsuba products: six real MODEXP calls.
        Fp memory t0 = _mul(a.c0, b.c0);
        Fp memory t1 = _mul(a.c1, b.c1);
        Fp memory t2 = _mul(sumA, sumB);
        assembly ("memory-safe") {
            function subtractInto(output, right) {
                let ah := mload(output)
                let al := mload(add(output, 32))
                let bh := mload(right)
                let bl := mload(add(right, 32))
                let lo := sub(al, bl)
                let hi := sub(sub(ah, bh), lt(al, bl))
                if or(lt(ah, bh), and(eq(ah, bh), lt(al, bl))) {
                    let oldLo := lo
                    lo := add(lo, 0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab)
                    hi := add(add(hi, 0x1a0111ea397fe69a4b1ba7b6434bacd7), lt(lo, oldLo))
                }
                mstore(output, hi)
                mstore(add(output, 32), lo)
            }
            // Each _mul owns its result. Finish c1 while t0 and t1 still
            // hold the original products; only then reuse t0 for c0.
            subtractInto(t2, t0)
            subtractInto(t2, t1)
            subtractInto(t0, t1)
            r := mload(0x40)
            mstore(0x40, add(r, 64))
            mstore(r, t0)
            mstore(add(r, 32), t2)
        }
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

