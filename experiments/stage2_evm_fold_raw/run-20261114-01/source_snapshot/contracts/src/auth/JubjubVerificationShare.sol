// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import "./JubjubSchnorrVerifier.sol";

/// Public polynomial evaluation only. All points must first pass the original
/// verifier's canonical-encoding/subgroup checks. No inverse or host result.
library JubjubVerificationShare {
    uint256 private constant P = 52435875175126190479447740508185965837690552500527637822603658699938581184513;
    uint256 private constant D = 19257038036680949359750312669786877991949435402254120286184196891950884077233;
    struct E { uint256 x; uint256 y; uint256 z; uint256 t; }
    function matches(JubjubSchnorrVerifier.Point[3] memory a, uint32 id,
        JubjubSchnorrVerifier.Point memory claimed) internal pure returns (bool)
    {
        E memory sum = add(ext(a[0]), add(mul(ext(a[1]),id),mul(ext(a[2]),uint256(id)*id)));
        return sum.z != 0 && mulmod(claimed.x,sum.z,P) == sum.x && mulmod(claimed.y,sum.z,P) == sum.y;
    }
    function ext(JubjubSchnorrVerifier.Point memory p) private pure returns (E memory) {
        return E(p.x,p.y,1,mulmod(p.x,p.y,P));
    }
    function mul(E memory p, uint256 s) private pure returns (E memory r) {
        r = E(0,1,1,0);
        while (s != 0) { if ((s & 1) != 0) r = add(r,p); p = add(p,p); s >>= 1; }
    }
    // Same complete a=-1 extended Edwards formula as the original G8-A backend.
    function add(E memory p, E memory q) private pure returns (E memory r) {
        uint256 a = mulmod(addmod(p.y,P-p.x,P),addmod(q.y,P-q.x,P),P);
        uint256 b = mulmod(addmod(p.y,p.x,P),addmod(q.y,q.x,P),P);
        uint256 c = mulmod(mulmod(p.t,q.t,P),2*D,P);
        uint256 d = mulmod(2,mulmod(p.z,q.z,P),P);
        uint256 e = addmod(b,P-a,P); uint256 f = addmod(d,P-c,P);
        uint256 g = addmod(d,c,P); uint256 h = addmod(b,a,P);
        return E(mulmod(e,f,P),mulmod(g,h,P),mulmod(f,g,P),mulmod(e,h,P));
    }
}
