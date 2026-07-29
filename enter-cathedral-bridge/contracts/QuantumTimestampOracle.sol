// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

interface QuantumTimestampOracle {
    function getTimestamp() external view returns (uint64 tick, bytes memory signature);
    function publicKeyRoot() external view returns (bytes32);
}
