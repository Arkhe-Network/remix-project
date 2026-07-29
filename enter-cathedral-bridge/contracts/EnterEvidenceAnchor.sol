// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import "./QuantumTimestampOracle.sol";
import "../../rbb-cathedral-bridge-1042/contracts/CathedralSPHINCSVerifier.sol";

contract EnterEvidenceAnchor {
    struct BatchRecord {
        bytes32 rootHash;
        uint64 quantumTick;
        bytes signature;      // SPHINCS- signature (3952 bytes)
        uint256 anchoredAt;
        address submittedBy;
    }

    mapping(bytes32 => BatchRecord) public batches; // rootHash -> record
    mapping(uint64 => bool) public usedTicks;
    uint64 public lastTick;

    CathedralSPHINCSVerifier public verifier;
    QuantumTimestampOracle public oracle;
    address public authorizedSubmitter;

    event BatchAnchored(bytes32 indexed rootHash, uint64 indexed tick, address submitter);

    constructor(address _verifier, address _oracle, address _authorizedSubmitter) {
        verifier = CathedralSPHINCSVerifier(_verifier);
        oracle = QuantumTimestampOracle(_oracle);
        authorizedSubmitter = _authorizedSubmitter;
    }

    modifier onlyAuthorized() {
        require(msg.sender == authorizedSubmitter, "Not authorized");
        _;
    }

    function anchorBatch(
        bytes32 rootHash,
        uint64 tick,
        bytes32 blockHash,
        bytes calldata signature
    ) external onlyAuthorized {
        require(!usedTicks[tick], "Tick already used");
        require(tick > lastTick, "Non-monotonic tick");
        require(batches[rootHash].rootHash == bytes32(0), "Batch already exists");

        // 1. Verify quantum timestamp (optional consistency)
        (uint64 oracleTick, ) = oracle.getTimestamp();
        require(oracleTick == tick, "Tick mismatch");

        // 2. Verify SPHINCS- signature
        bytes memory msgToVerify = abi.encodePacked(rootHash, tick, blockHash);
        require(verifier.verifySPHINCS(keccak256(msgToVerify), signature, oracle.publicKeyRoot()), "Invalid signature");

        // 3. Store
        batches[rootHash] = BatchRecord(rootHash, tick, signature, block.timestamp, msg.sender);
        usedTicks[tick] = true;
        lastTick = tick;

        emit BatchAnchored(rootHash, tick, msg.sender);
    }

    function verifyEvidence(bytes32 evidenceHash, bytes32 rootHash, bytes32[] calldata proof) external view returns (bool) {
        BatchRecord memory rec = batches[rootHash];
        require(rec.rootHash != bytes32(0), "Batch not found");
        // Recompute root from proof and compare
        bytes32 computed = evidenceHash;
        for (uint i = 0; i < proof.length; i++) {
            if (computed < proof[i]) computed = keccak256(abi.encodePacked(computed, proof[i]));
            else computed = keccak256(abi.encodePacked(proof[i], computed));
        }
        return (computed == rootHash);
    }
}
