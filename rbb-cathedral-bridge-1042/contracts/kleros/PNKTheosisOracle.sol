// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title PNKTheosisOracle
 * @dev Oracle storing Theosis levels for Kleros jurors.
 *      Allows authorized accounts (like WormGraph/Cathedral nodes) to update Theosis scores.
 */
contract PNKTheosisOracle {
    mapping(address => uint256) public jurorTheosis;
    mapping(address => bool) public authorizedUpdaters;

    address public owner;

    event TheosisUpdated(address indexed juror, uint256 newTheosisLevel);
    event UpdaterAdded(address indexed updater);
    event UpdaterRemoved(address indexed updater);

    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner can call");
        _;
    }

    modifier onlyUpdater() {
        require(authorizedUpdaters[msg.sender] || msg.sender == owner, "Not authorized to update");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    function addUpdater(address _updater) external onlyOwner {
        authorizedUpdaters[_updater] = true;
        emit UpdaterAdded(_updater);
    }

    function removeUpdater(address _updater) external onlyOwner {
        authorizedUpdaters[_updater] = false;
        emit UpdaterRemoved(_updater);
    }

    function updateTheosis(address _juror, uint256 _theosisLevel) external onlyUpdater {
        jurorTheosis[_juror] = _theosisLevel;
        emit TheosisUpdated(_juror, _theosisLevel);
    }

    function getTheosis(address _juror) external view returns (uint256) {
        return jurorTheosis[_juror];
    }
}
