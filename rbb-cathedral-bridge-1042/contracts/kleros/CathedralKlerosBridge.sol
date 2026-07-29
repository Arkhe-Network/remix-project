// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title CathedralKlerosBridge
 * @dev Base bridge contract connecting Kleros disputes between Arbitrum and RBB.
 */
contract CathedralKlerosBridge {
    address public owner;

    // Addresses for bridging (e.g., Vea relayers or cross-chain messaging)
    address public crossChainMessenger;

    event DisputeBridged(uint256 indexed disputeID, address indexed court, string rbbData);
    event DisputeResolvedOnRBB(uint256 indexed disputeID, uint256 ruling);

    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner");
        _;
    }

    modifier onlyMessenger() {
        require(msg.sender == crossChainMessenger || msg.sender == owner, "Only messenger");
        _;
    }

    constructor(address _crossChainMessenger) {
        owner = msg.sender;
        crossChainMessenger = _crossChainMessenger;
    }

    function updateMessenger(address _newMessenger) external onlyOwner {
        crossChainMessenger = _newMessenger;
    }

    function bridgeDisputeToRBB(uint256 _disputeID, address _court, string calldata _rbbData) external {
        // Logic to send message to RBB via the Vea Relay / Arbitrum bridge
        // (Simplified for this architecture)
        emit DisputeBridged(_disputeID, _court, _rbbData);
    }

    function resolveDisputeFromRBB(uint256 _disputeID, uint256 _ruling) external onlyMessenger {
        // Receive the resolved ruling from RBB
        emit DisputeResolvedOnRBB(_disputeID, _ruling);
    }
}
