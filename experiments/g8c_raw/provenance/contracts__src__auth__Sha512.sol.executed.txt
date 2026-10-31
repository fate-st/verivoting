// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// SHA-512, FIPS 180-4. Constants are the standard 80 fractional cube roots.
/// All hash work executes in EVM; there is no trusted host-supplied digest.
library Sha512 {
    function digest(bytes memory data) internal pure returns (bytes32 hi, bytes32 lo) {
        bytes memory k = hex"428a2f98d728ae227137449123ef65cdb5c0fbcfec4d3b2fe9b5dba58189dbbc3956c25bf348b53859f111f1b605d019923f82a4af194f9bab1c5ed5da6d8118d807aa98a303024212835b0145706fbe243185be4ee4b28c550c7dc3d5ffb4e272be5d74f27b896f80deb1fe3b1696b19bdc06a725c71235c19bf174cf692694e49b69c19ef14ad2efbe4786384f25e30fc19dc68b8cd5b5240ca1cc77ac9c652de92c6f592b02754a7484aa6ea6e4835cb0a9dcbd41fbd476f988da831153b5983e5152ee66dfaba831c66d2db43210b00327c898fb213fbf597fc7beef0ee4c6e00bf33da88fc2d5a79147930aa72506ca6351e003826f142929670a0e6e7027b70a8546d22ffc2e1b21385c26c9264d2c6dfc5ac42aed53380d139d95b3df650a73548baf63de766a0abb3c77b2a881c2c92e47edaee692722c851482353ba2bfe8a14cf10364a81a664bbc423001c24b8b70d0f89791c76c51a30654be30d192e819d6ef5218d69906245565a910f40e35855771202a106aa07032bbd1b819a4c116b8d2d0c81e376c085141ab532748774cdf8eeb9934b0bcb5e19b48a8391c0cb3c5c95a634ed8aa4ae3418acb5b9cca4f7763e373682e6ff3d6b2b8a3748f82ee5defb2fc78a5636f43172f6084c87814a1f0ab728cc702081a6439ec90befffa23631e28a4506cebde82bde9bef9a3f7b2c67915c67178f2e372532bca273eceea26619cd186b8c721c0c207eada7dd6cde0eb1ef57d4f7fee6ed17806f067aa72176fba0a637dc5a2c898a6113f9804bef90dae1b710b35131c471b28db77f523047d8432caab7b40c724933c9ebe0a15c9bebc431d67c49c100d4c4cc5d4becb3e42b6597f299cfc657e2a5fcb6fab3ad6faec6c44198c4a475817"; // SHA512_K64_STANDARD_CONSTANTS
        uint256[8] memory state = [uint256(0x6a09e667f3bcc908),0xbb67ae8584caa73b,0x3c6ef372fe94f82b,0xa54ff53a5f1d36f1,
            0x510e527fade682d1,0x9b05688c2b3e6c1f,0x1f83d9abfb41bd6b,0x5be0cd19137e2179];
        uint256 size = ((data.length + 17 + 127) / 128) * 128;
        bytes memory padded = new bytes(size);
        for (uint256 i; i < data.length; ++i) padded[i] = data[i];
        padded[data.length] = 0x80;
        uint128 bits = uint128(data.length) * 8;
        for (uint256 i; i < 16; ++i) padded[size - 1 - i] = bytes1(uint8(bits >> (8*i)));
        uint256[80] memory w;
        assembly ("memory-safe") {
            function rr(x, n) -> v { v := and(or(shr(n,x),shl(sub(64,n),x)),0xffffffffffffffff) }
            for { let blockOffset := 0 } lt(blockOffset,size) { blockOffset := add(blockOffset,128) } {
                for { let j := 0 } lt(j,16) { j := add(j,1) } {
                    mstore(add(w,mul(j,32)),shr(192,mload(add(add(padded,32),add(blockOffset,mul(j,8))))))
                }
                for { let j := 16 } lt(j,80) { j := add(j,1) } {
                    let x := mload(add(w,mul(sub(j,15),32)))
                    let y := mload(add(w,mul(sub(j,2),32)))
                    let s0 := xor(xor(rr(x,1),rr(x,8)),shr(7,x))
                    let s1 := xor(xor(rr(y,19),rr(y,61)),shr(6,y))
                    mstore(add(w,mul(j,32)),and(add(add(mload(add(w,mul(sub(j,16),32))),s0),add(mload(add(w,mul(sub(j,7),32))),s1)),0xffffffffffffffff))
                }
                let a := mload(state)
                let b := mload(add(state,32))
                let c := mload(add(state,64))
                let d := mload(add(state,96))
                let e := mload(add(state,128))
                let f := mload(add(state,160))
                let g := mload(add(state,192))
                let h := mload(add(state,224))
                for { let j := 0 } lt(j,80) { j := add(j,1) } {
                    let s1 := xor(xor(rr(e,14),rr(e,18)),rr(e,41))
                    let ch := xor(and(e,f),and(not(e),g))
                    let constant := shr(192,mload(add(add(k,32),mul(j,8))))
                    let t1 := and(add(add(add(h,s1),ch),add(constant,mload(add(w,mul(j,32))))),0xffffffffffffffff)
                    let s0 := xor(xor(rr(a,28),rr(a,34)),rr(a,39))
                    let maj := xor(xor(and(a,b),and(a,c)),and(b,c))
                    let t2 := and(add(s0,maj),0xffffffffffffffff)
                    h := g
                    g := f
                    f := e
                    e := and(add(d,t1),0xffffffffffffffff)
                    d := c
                    c := b
                    b := a
                    a := and(add(t1,t2),0xffffffffffffffff)
                }
                mstore(state,and(add(mload(state),a),0xffffffffffffffff))
                mstore(add(state,32),and(add(mload(add(state,32)),b),0xffffffffffffffff))
                mstore(add(state,64),and(add(mload(add(state,64)),c),0xffffffffffffffff))
                mstore(add(state,96),and(add(mload(add(state,96)),d),0xffffffffffffffff))
                mstore(add(state,128),and(add(mload(add(state,128)),e),0xffffffffffffffff))
                mstore(add(state,160),and(add(mload(add(state,160)),f),0xffffffffffffffff))
                mstore(add(state,192),and(add(mload(add(state,192)),g),0xffffffffffffffff))
                mstore(add(state,224),and(add(mload(add(state,224)),h),0xffffffffffffffff))
            }
            hi := or(or(shl(192,mload(state)),shl(128,mload(add(state,32)))),or(shl(64,mload(add(state,64))),mload(add(state,96))))
            lo := or(or(shl(192,mload(add(state,128))),shl(128,mload(add(state,160)))),or(shl(64,mload(add(state,192))),mload(add(state,224))))
        }
    }
}
