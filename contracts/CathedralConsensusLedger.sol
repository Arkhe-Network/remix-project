// SPDX-License-Identifier: MIT
// Cathedral ARKHE v28.3 — Consensus Ledger On-Chain (stub)
// Selo: CATHEDRAL-ARKHE-v28.3-CONSENSUS-LEDGER-2026-06-16
// Arquiteto ORCID: 0009-0005-2697-4668

pragma solidity ^0.8.19;

/// @title Cathedral Consensus Ledger
/// @author Cathedral ARKHE
/// @notice Registro imutável de decisões multi-agente ancorado em blockchain.
contract CathedralConsensusLedger {
    // --- Estruturas ---
    struct Decision {
        bytes32 decisionId;
        bytes32 workflowId;
        uint64 timestamp;
        bytes32 proposalHash;      // hash do conteúdo da proposta
        address[] participants;    // endereços dos agentes (futuro)
        bool outcome;              // aprovado/rejeitado
        uint32 votesFor;
        uint32 votesAgainst;
        bytes signature;           // assinatura SPHINCS+ (armazenada como bytes)
        uint256 blockNumber;
    }

    // --- Eventos ---
    event DecisionRecorded(
        bytes32 indexed decisionId,
        bytes32 indexed workflowId,
        uint64 timestamp,
        bool outcome,
        uint32 votesFor,
        uint32 votesAgainst
    );

    event DecisionOverridden(
        bytes32 indexed decisionId,
        address indexed overriddenBy,
        string reason
    );

    // --- Estado ---
    mapping(bytes32 => Decision) public decisions;
    mapping(bytes32 => bool) public decisionExists;
    uint256 public decisionCount;

    // Apenas oráculos autorizados podem registrar decisões
    mapping(address => bool) public authorizedOracles;
    address public guardian;

    // --- Modificadores ---
    modifier onlyAuthorized() {
        require(authorizedOracles[msg.sender] || msg.sender == guardian, "Not authorized");
        _;
    }

    modifier onlyGuardian() {
        require(msg.sender == guardian, "Only guardian");
        _;
    }

    // --- Construtor ---
    constructor(address _guardian) {
        guardian = _guardian;
    }

    // --- Administração ---
    function addOracle(address oracle) external onlyGuardian {
        authorizedOracles[oracle] = true;
    }

    function removeOracle(address oracle) external onlyGuardian {
        authorizedOracles[oracle] = false;
    }

    // --- Registro de decisão (função principal) ---
    function recordDecision(
        bytes32 workflowId,
        bytes32 proposalHash,
        address[] calldata participants,
        bool outcome,
        uint32 votesFor,
        uint32 votesAgainst,
        bytes calldata signature
    ) external onlyAuthorized returns (bytes32 decisionId) {
        decisionId = keccak256(abi.encodePacked(workflowId, block.timestamp, votesFor, votesAgainst));

        require(!decisionExists[decisionId], "Decision already recorded");

        decisions[decisionId] = Decision({
            decisionId: decisionId,
            workflowId: workflowId,
            timestamp: uint64(block.timestamp),
            proposalHash: proposalHash,
            participants: participants,
            outcome: outcome,
            votesFor: votesFor,
            votesAgainst: votesAgainst,
            signature: signature,
            blockNumber: block.number
        });

        decisionExists[decisionId] = true;
        decisionCount++;

        emit DecisionRecorded(decisionId, workflowId, uint64(block.timestamp), outcome, votesFor, votesAgainst);

        return decisionId;
    }

    // --- Sobrescrita de decisão (emergência) ---
    function overrideDecision(
        bytes32 decisionId,
        bool newOutcome,
        string calldata reason
    ) external onlyGuardian {
        require(decisionExists[decisionId], "Decision does not exist");
        Decision storage d = decisions[decisionId];
        d.outcome = newOutcome;
        emit DecisionOverridden(decisionId, msg.sender, reason);
    }

    // --- Consultas ---
    function getDecision(bytes32 decisionId) external view returns (Decision memory) {
        require(decisionExists[decisionId], "Decision not found");
        return decisions[decisionId];
    }

    function verifyProposalHash(bytes32 decisionId, bytes32 proposalHash) external view returns (bool) {
        require(decisionExists[decisionId], "Decision not found");
        return decisions[decisionId].proposalHash == proposalHash;
    }

    // --- Utilitários para interface com TemporalChain ---
    function computeDecisionId(
        bytes32 workflowId,
        uint64 timestamp,
        uint32 votesFor,
        uint32 votesAgainst
    ) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(workflowId, timestamp, votesFor, votesAgainst));
    }
}
