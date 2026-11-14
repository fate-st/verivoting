// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

library CanonicalVV {
    string internal constant SUITE = "VERIVOTING-JUBJUB-PROTOTYPE-G0.8-v2";
    function blob(bytes memory b) internal pure returns (bytes memory) { return abi.encodePacked(uint64(b.length), b); }
    function text(string memory s) internal pure returns (bytes memory) { return blob(bytes(s)); }
    function prefix(string memory domain) internal pure returns (bytes memory) { return bytes.concat(text("VV-CANONICAL-V1"), text(domain)); }
    function littleEndian32(uint256 v) internal pure returns (bytes32 out) {
        uint256 r; for (uint256 i; i < 32; ++i) r |= ((v >> (8*i)) & 255) << (8*(31-i));
        return bytes32(r);
    }
}
