// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./CathedralKlerosBridge.sol";
import "./PNKTheosisOracle.sol";

/**
 * @title CathedralKlerosBridgeWithVoting
 * @dev Extension of the CathedralKlerosBridge adding "Theosis-weighted voting".
 *      Jurors with higher Theosis have extra weight in certain RBB/Cathedral-linked courts.
 */
contract CathedralKlerosBridgeWithVoting is CathedralKlerosBridge {
    PNKTheosisOracle public theosisOracle;

    // Mapping disputeID to voting options to total weighted votes
    // disputeID => (rulingOption => totalWeightedVotes)
    mapping(uint256 => mapping(uint256 => uint256)) public weightedVotes;

    // Track if a juror has already voted on a dispute to prevent double voting
    // disputeID => (juror => hasVoted)
    mapping(uint256 => mapping(address => bool)) public hasVoted;

    event VoteCast(uint256 indexed disputeID, address indexed juror, uint256 ruling, uint256 weight);
    event DisputeResolvedWithWeightedVoting(uint256 indexed disputeID, uint256 winningRuling);

    constructor(address _crossChainMessenger, address _theosisOracle)
        CathedralKlerosBridge(_crossChainMessenger)
    {
        theosisOracle = PNKTheosisOracle(_theosisOracle);
    }

    /**
     * @dev Cast a vote on a bridge dispute. The weight of the vote is determined by the juror's Theosis level.
     *      Base weight is 1. Theosis adds to this weight.
     * @param _disputeID The ID of the dispute on the RBB side
     * @param _ruling The ruling option chosen
     */
    function castWeightedVote(uint256 _disputeID, uint256 _ruling) external {
        require(!hasVoted[_disputeID][msg.sender], "Juror already voted");

        uint256 theosisLevel = theosisOracle.getTheosis(msg.sender);

        // Example logic: Theosis directly acts as a multiplier or addition.
        // If Theosis is modeled as a large integer scaled by 10**18 or similar.
        // For simplicity, we add the Theosis level to a base weight of 1.
        uint256 weight = 1 + theosisLevel;

        weightedVotes[_disputeID][_ruling] += weight;
        hasVoted[_disputeID][msg.sender] = true;

        emit VoteCast(_disputeID, msg.sender, _ruling, weight);
    }

    /**
     * @dev Resolves the dispute based on the accumulated weighted votes.
     *      This could be called by a keeper, oracle, or bridge messenger.
     * @param _disputeID The ID of the dispute to resolve
     * @param _rulingOptions The possible ruling options to check
     */
    function resolveWeightedDispute(uint256 _disputeID, uint256[] calldata _rulingOptions) external onlyMessenger {
        uint256 winningRuling = 0;
        uint256 maxVotes = 0;

        for(uint256 i = 0; i < _rulingOptions.length; i++) {
            uint256 r = _rulingOptions[i];
            if(weightedVotes[_disputeID][r] > maxVotes) {
                maxVotes = weightedVotes[_disputeID][r];
                winningRuling = r;
            }
        }

        // Emit our custom event
        emit DisputeResolvedWithWeightedVoting(_disputeID, winningRuling);

        // Can optionally also emit the parent event if we are sending back to L1
        emit DisputeResolvedOnRBB(_disputeID, winningRuling);
    }
}
