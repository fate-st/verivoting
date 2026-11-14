// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
/// Fixed G8-C EXPERIMENT pins, not a change to any cryptographic suite.
library G8CExperimentPins {
    bytes32 internal constant BINDING_DIGEST = 0x5cdda2d5a7931d50355480166e4453a458652e8a3d4ff02a7c2520361714829e;
    bytes32 internal constant BATCH_ROOT_1 = 0x91f6c687850b799e58e413c6c9c3b67e1acb1f2cb8a03e2b4d88a3729a9a2134;
    bytes32 internal constant BATCH_ROOT_2 = 0x4e3a7649aa9d7455509a0b052d85c6745b38cb1f2416045c08ffd25094837fcf;
    bytes32 internal constant BALLOT_VK_SHA256 = 0x69a56d10131027e0efcf0c6a7d920aac4600e87b5e20c42eb670f368e2ae9a3d;
    bytes32 internal constant STATE_VK_SHA256 = 0x1367728908bb6c62009afb38ad09555d5a8c463bc6945b3ceb83ae763f376896;
    bytes32 internal constant P3_ARK_VK_SHA256 = 0x0d37a77cfc8a1cdfed612b397149d96816188d164e08a74994dd2abfca6cff29;
    bytes32 internal constant P3_EIP_VK_SHA256 = 0x93336597070cda6b48775e28cc077ea0027686f0f8e1e588e4867e3cedb3694a;
    bytes32 internal constant P3_EIP_VK_KECCAK256 = 0x52fe9c94b348ede49b3e789d240edd861827eeafc9e7bcb41777d2505980e1dd;
}
