// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

contract CathedralSPHINCSVerifier {
    uint256 internal constant N = 16;
    uint256 internal constant W = 8;
    uint256 internal constant L = 43;
    uint256 internal constant K = 8;
    uint256 internal constant A = 16;
    uint256 internal constant D = 2;
    uint256 internal constant H_TOTAL = 24;
    uint256 internal constant H_PER_LAYER = H_TOTAL / D;

    uint256 internal constant SIG_SIZE = 3952;
    uint256 internal constant PK_ROOT_SIZE = N;

    // Use the optimized target sum for tests (150) instead of 0 to allow the python keygen
    // to actually find a valid signature by grinding within a reasonable timeframe.
    uint256 internal constant WOTS_TARGET_SUM = 150;

    function verifySPHINCS(
        bytes32 message,
        bytes calldata signature,
        bytes32 publicKeyRoot
    ) external pure returns (bool) {
        if (signature.length != SIG_SIZE) {
            revert("Invalid signature length");
        }

        uint256 offset = 0;

        bytes32 randomizer;
        assembly {
            randomizer := calldataload(signature.offset)
        }
        randomizer = bytes32(uint256(randomizer) & (type(uint256).max << 128));
        offset += N;

        uint256 forsTotalSize = K * (N + A * N);
        uint256 wotsSize = L * N;
        uint256 merkleAuthSizeLayer0 = H_PER_LAYER * N;
        uint256 merkleAuthSizeLayer1 = H_PER_LAYER * N;

        bytes32 md = keccak256(abi.encodePacked(randomizer, message));
        uint256 idx_tree;
        uint256 idx_leaf;

        assembly {
            // Updated derivation logic. md[16:20]
            // md is 32 bytes.
            // md[16:20] means bytes at index 16, 17, 18, 19
            // This corresponds to shifting right by 96 bits (12 bytes from the end)
            // Actually: mdFirst64 was shr(192), which gives bytes 0-7.
            // Python uses:
            // md_int = int.from_bytes(md[16:20], 'big')
            // idx_tree = (md_int >> 20) & 0xFFF
            // idx_leaf = (md_int >> 8) & 0xFFF
            // So we need to shift right by 128 bits (to get the top 16 bytes out)
            // No, md[16:20] means the 4 bytes starting at byte 16.
            // which are bits 128 to 159 from the MSB, or bits 96 to 127 from the LSB.
            // Let's just shift right by 96. That puts md[16:20] at the bottom 32 bits.

            let mdSlice := shr(96, md)
            idx_tree := and(shr(20, mdSlice), 0xFFF)
            idx_leaf := and(shr(8, mdSlice), 0xFFF)
        }

        bytes32 forsPK = _reconstructFORSPublicKey(
            signature[offset:offset + forsTotalSize],
            md
        );
        offset += forsTotalSize;

        bytes32 layer0Node = _verifyWOTSC(
            signature[offset:offset + wotsSize],
            forsPK
        );
        offset += wotsSize;

        layer0Node = _verifyMerklePath(
            layer0Node,
            signature[offset:offset + merkleAuthSizeLayer0],
            idx_leaf
        );
        offset += merkleAuthSizeLayer0;

        bytes32 layer1Node = _verifyWOTSC(
            signature[offset:offset + wotsSize],
            layer0Node
        );
        offset += wotsSize;

        bytes32 computedRoot = _verifyMerklePath(
            layer1Node,
            signature[offset:offset + merkleAuthSizeLayer1],
            idx_tree
        );
        offset += merkleAuthSizeLayer1;

        bytes32 mask = bytes32(uint256(type(uint128).max) << 128);
        return (computedRoot & mask) == (publicKeyRoot & mask);
    }

    function _hashN(bytes32 data) internal pure returns (bytes32) {
        return bytes32(uint256(keccak256(abi.encodePacked(data))) & (type(uint256).max << 128));
    }

    function _baseWDigits(bytes32 msgHash, uint256 outLen) internal pure returns (uint256[] memory, uint256 sum) {
        uint256[] memory digits = new uint256[](outLen);
        uint256 bitLen = 3; // for W=8, log2(8) = 3

        uint256 inIdx = 0;
        uint256 bitsInBuffer = 0;
        uint256 buffer = 0;

        sum = 0;

        for (uint256 i = 0; i < outLen; i++) {
            if (bitsInBuffer < bitLen) {
                // extract 1 byte from msgHash at inIdx
                uint256 b = uint256(uint8(msgHash[inIdx]));
                buffer = (buffer << 8) | b;
                inIdx++;
                bitsInBuffer += 8;
            }

            uint256 shift = bitsInBuffer - bitLen;
            uint256 digit = (buffer >> shift) & ((1 << bitLen) - 1);
            bitsInBuffer -= bitLen;
            digits[i] = digit;
            sum += digit;
        }

        return (digits, sum);
    }

    function _reconstructFORSPublicKey(
        bytes calldata forsData,
        bytes32 md
    ) internal pure returns (bytes32) {

        bytes32[] memory forsRoots = new bytes32[](K);
        uint256 offset = 0;

        for (uint256 i = 0; i < K; i++) {
            bytes32 leafSecret;
            assembly {
                leafSecret := and(calldataload(add(forsData.offset, offset)), shl(128, not(0)))
            }
            offset += N;

            bytes32 root = _hashN(leafSecret);

            bytes32[] memory authPath = new bytes32[](A);
            for (uint256 j = 0; j < A; j++) {
                assembly {
                    let pos := add(forsData.offset, offset)
                    mstore(add(authPath, mul(add(j, 1), 32)), and(calldataload(pos), shl(128, not(0))))
                }
                offset += N;
            }

            // SPHINCS+ FORS uses strictly the top 128 bits:
            // Python uses: leaf_idx = (md_int >> (128 - shift - A)) & ((1 << A) - 1)
            // Where md_int is the top 128 bits.
            uint256 shift = i * A;
            // Shifting by 256-128 = 128 to get top 128 bits, then right shift by (128 - shift - A)
            // Total shift = 128 + 128 - shift - A = 256 - shift - A.
            // Which is exactly what we already had!
            uint256 leafIdx = (uint256(md) >> (256 - shift - A)) & ((1 << A) - 1);

            for (uint256 j = 0; j < A; j++) {
                if ((leafIdx >> j) & 1 == 0) {
                    root = bytes32(uint256(keccak256(abi.encodePacked(root, authPath[j]))) & (type(uint256).max << 128));
                } else {
                    root = bytes32(uint256(keccak256(abi.encodePacked(authPath[j], root))) & (type(uint256).max << 128));
                }
            }
            forsRoots[i] = bytes32(uint256(root) & (type(uint256).max << 128));
        }

        return keccak256(abi.encodePacked(forsRoots));
    }

    function _verifyWOTSC(
        bytes calldata wotsSig,
        bytes32 message
    ) internal pure returns (bytes32) {
        bytes32[] memory chains = new bytes32[](L);
        uint256 offset = 0;

        bytes32 msgHash = _hashN(message);
        (uint256[] memory msgDigits, uint256 digitSum) = _baseWDigits(msgHash, 43);

        // Critical: Verify the WOTS+C target sum matches!
        require(digitSum == WOTS_TARGET_SUM, "WOTS+ target sum mismatch");

        for (uint256 i = 0; i < L; i++) {
            bytes32 val;
            assembly {
                val := and(calldataload(add(wotsSig.offset, offset)), shl(128, not(0)))
            }

            uint256 digit = i < msgDigits.length ? msgDigits[i] : 0;

            for (uint256 j = 0; j < (W - 1 - digit); j++) {
                val = _hashN(val);
            }

            chains[i] = val;
            offset += N;
        }

        return bytes32(uint256(keccak256(abi.encodePacked(chains))) & (type(uint256).max << 128));
    }

    function _verifyMerklePath(
        bytes32 leaf,
        bytes calldata authPath,
        uint256 treeIndex
    ) internal pure returns (bytes32) {
        bytes32 node = bytes32(uint256(leaf) & (type(uint256).max << 128));
        uint256 idx = treeIndex;
        uint256 pathLength = authPath.length / N;

        for (uint256 i = 0; i < pathLength; i++) {
            bytes32 sibling;
            assembly {
                sibling := and(calldataload(add(authPath.offset, mul(i, 16))), shl(128, not(0)))
            }
            if ((idx >> i) & 1 == 0) {
                node = bytes32(uint256(keccak256(abi.encodePacked(node, sibling))) & (type(uint256).max << 128));
            } else {
                node = bytes32(uint256(keccak256(abi.encodePacked(sibling, node))) & (type(uint256).max << 128));
            }
        }
        return node;
    }
}
