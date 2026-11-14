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
/// @notice Isolated cost instrumentation, NOT a Miller loop or proof verifier.
contract Bls12381FieldLowLevelProfile {
    uint256 private constant P_HI = 0x1a0111ea397fe69a4b1ba7b6434bacd7;
    uint256 private constant P_LO =
        0x64774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab;

    struct Fp { uint256 hi; uint256 lo; }
    struct Fp2 { Fp c0; Fp c1; }

    // The gasleft window excludes input decoding and output encoding, but
    // includes compiler/call bookkeeping and its own final gasleft operation.
    function profileFpMul(bytes calldata left, bytes calldata right)
        external view returns (bytes memory computed, uint256 windowGas,
            uint256 freeBefore, uint256 freeAfter)
    {
        require(left.length == 64 && right.length == 64, "FP_LENGTH");
        Fp memory a = _readFp(left, 0);
        Fp memory b = _readFp(right, 0);
        assembly ("memory-safe") { freeBefore := mload(0x40) }
        uint256 start = gasleft();
        Fp memory product = _mul(a, b);
        windowGas = start - gasleft();
        assembly ("memory-safe") { freeAfter := mload(0x40) }
        computed = new bytes(64);
        _writeFp(computed, 0, product);
    }

    function profileFp2Mul(bytes calldata left, bytes calldata right)
        external view returns (bytes memory computed, uint256 windowGas,
            uint256 freeBefore, uint256 freeAfter)
    {
        require(left.length == 128 && right.length == 128, "FP2_LENGTH");
        Fp2 memory a = _read2(left, 0);
        Fp2 memory b = _read2(right, 0);
        assembly ("memory-safe") { freeBefore := mload(0x40) }
        uint256 start = gasleft();
        Fp2 memory product = _mul2(a, b);
        windowGas = start - gasleft();
        assembly ("memory-safe") { freeAfter := mload(0x40) }
        computed = new bytes(128);
        _write2(computed, 0, product);
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
}


