// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// @notice Experimental verifier for a single constructor-pinned BLS12-381 VK.
/// Proof bytes are EIP-2537 encode(-A) || encode(B) || encode(C), not arkworks
/// compressed serialization. Public inputs exclude Groth16's leading one.
contract Bls12381Groth16Verifier {
    uint256 private constant FR_MODULUS =
        0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001;
    bytes private verificationKey;
    uint256 public immutable publicInputCount;
    bytes32 public immutable verificationKeyDigest;

    constructor(bytes memory vk, uint256 inputCount) {
        require(vk.length == 896 + 128 * (inputCount + 1), "VK_LENGTH");
        verificationKey = vk;
        publicInputCount = inputCount;
        verificationKeyDigest = keccak256(vk);
    }

    function verifyOrRevert(bytes calldata proofNegAEip2537, uint256[] calldata inputs)
        external view
    {
        require(_verify(proofNegAEip2537, inputs), "INVALID_GROTH16");
    }

    function _verify(bytes calldata proofNegAEip2537, uint256[] calldata inputs)
        internal view returns (bool)
    {
        if (proofNegAEip2537.length != 512 || inputs.length != publicInputCount) return false;
        bytes memory vk = verificationKey;
        bytes memory msm = new bytes(160 * (inputs.length + 1));
        for (uint256 i = 0; i <= inputs.length; ++i) {
            uint256 scalar = i == 0 ? 1 : inputs[i - 1];
            if (scalar >= FR_MODULUS) return false;
            _copy(vk, 896 + 128 * i, msm, 160 * i, 128);
            assembly ("memory-safe") {
                mstore(add(add(msm, 160), mul(160, i)), scalar)
            }
        }
        // EIP-2537 MSM and pairing perform curve and prime-subgroup checks.
        (bool msmOk, bytes memory vkX) = address(0x0c).staticcall(msm);
        if (!msmOk || vkX.length != 128) return false;

        bytes memory proof = proofNegAEip2537;
        bytes memory pairs = new bytes(1536);
        // e(-A,B) * e(alpha,beta) * e(vk_x,gamma) * e(C,delta) == 1.
        _copy(proof, 0, pairs, 0, 384);
        _copy(vk, 0, pairs, 384, 384);
        _copy(vkX, 0, pairs, 768, 128);
        _copy(vk, 384, pairs, 896, 256);
        _copy(proof, 384, pairs, 1152, 128);
        _copy(vk, 640, pairs, 1280, 256);
        (bool pairingOk, bytes memory result) = address(0x0f).staticcall(pairs);
        return pairingOk && result.length == 32 && abi.decode(result, (uint256)) == 1;
    }

    function _copy(bytes memory source, uint256 from, bytes memory target,
        uint256 to, uint256 length) private pure
    {
        // All internal slices have fixed, word-aligned lengths.
        for (uint256 offset = 0; offset < length; offset += 32) {
            assembly ("memory-safe") {
                mstore(add(add(target, 32), add(to, offset)),
                    mload(add(add(source, 32), add(from, offset))))
            }
        }
    }
}
