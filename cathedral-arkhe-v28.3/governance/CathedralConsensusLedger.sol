// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract CathedralConsensusLedger {
    struct Decision {
        string policy_hash;
        uint256 timestamp;
        address proposer;
        uint256 votes_for;
        uint256 votes_against;
        bool approved;
    }

    mapping(uint256 => Decision) public decisions;
    uint256 public decisionCount;

    event DecisionProposed(uint256 id, string policy_hash, address proposer);
    event DecisionVoted(uint256 id, bool support, address voter);
    event DecisionFinalized(uint256 id, bool approved);

    function proposeDecision(string memory _policy_hash) public returns (uint256) {
        decisionCount++;
        decisions[decisionCount] = Decision({
            policy_hash: _policy_hash,
            timestamp: block.timestamp,
            proposer: msg.sender,
            votes_for: 0,
            votes_against: 0,
            approved: false
        });
        emit DecisionProposed(decisionCount, _policy_hash, msg.sender);
        return decisionCount;
    }

    function voteDecision(uint256 _id, bool _support) public {
        require(_id > 0 && _id <= decisionCount, "Invalid decision ID");
        Decision storage decision = decisions[_id];
        require(!decision.approved, "Decision already finalized");

        if (_support) {
            decision.votes_for++;
        } else {
            decision.votes_against++;
        }

        emit DecisionVoted(_id, _support, msg.sender);

        // Simple majority for example purposes
        if (decision.votes_for > decision.votes_against + 2) {
            decision.approved = true;
            emit DecisionFinalized(_id, true);
        }
    }
}
