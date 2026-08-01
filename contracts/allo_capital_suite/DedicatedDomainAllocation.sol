// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./AlloPool.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

/// @title DedicatedDomainAllocation — DDA mechanism
/// @notice Delegates funding power to trusted stewards within specific domains
contract DedicatedDomainAllocation is AccessControl {
    bytes32 public constant STEWARD_ROLE = keccak256("STEWARD_ROLE");

    struct Domain {
        string name;
        string description;
        address steward;
        uint256 allocatedBudget;
        uint256 spentBudget;
        bool active;
        // Cannot put mapping inside a struct returned to external clients without an internal workaround.
        // We will store mapping separately or return other fields.
    }

    struct DomainInfo {
        string name;
        string description;
        address steward;
        uint256 allocatedBudget;
        uint256 spentBudget;
        bool active;
    }

    AlloPool public pool;
    mapping(bytes32 => Domain) internal _domains;
    mapping(bytes32 => mapping(address => bool)) public domainApprovedRecipients;
    mapping(bytes32 => address[]) public domainRecipientsList;
    bytes32[] public domainIds;

    event DomainCreated(bytes32 indexed domainId, string name, address indexed steward);
    event DomainFunded(bytes32 indexed domainId, uint256 amount);
    event DomainAllocation(bytes32 indexed domainId, address indexed recipient, uint256 amount);
    event RecipientApproved(bytes32 indexed domainId, address indexed recipient);

    constructor(address _pool) {
        pool = AlloPool(payable(_pool));
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    /// @notice Create a new domain with a steward
    function createDomain(
        bytes32 domainId,
        string calldata name,
        string calldata description,
        address steward
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(_domains[domainId].steward == address(0), "Domain exists");
        require(steward != address(0), "Invalid steward");

        Domain storage domain = _domains[domainId];
        domain.name = name;
        domain.description = description;
        domain.steward = steward;
        domain.active = true;
        domainIds.push(domainId);

        _grantRole(STEWARD_ROLE, steward);
        emit DomainCreated(domainId, name, steward);
    }

    /// @notice Fund a domain
    function fundDomain(bytes32 domainId, uint256 amount) external payable {
        require(_domains[domainId].active, "Domain inactive");
        require(msg.value == amount || amount == 0, "Invalid amount");

        Domain storage domain = _domains[domainId];
        domain.allocatedBudget += amount;

        emit DomainFunded(domainId, amount);
    }

    /// @notice Approve a recipient for a domain (only steward)
    function approveRecipient(bytes32 domainId, address recipient)
        external
        onlyRole(STEWARD_ROLE)
    {
        require(_domains[domainId].steward == msg.sender, "Not steward");
        require(!domainApprovedRecipients[domainId][recipient], "Already approved");

        domainApprovedRecipients[domainId][recipient] = true;
        domainRecipientsList[domainId].push(recipient);

        emit RecipientApproved(domainId, recipient);
    }

    /// @notice Allocate funds to a recipient within a domain (only steward)
    function allocateToRecipient(
        bytes32 domainId,
        address recipient,
        uint256 amount
    ) external onlyRole(STEWARD_ROLE) {
        require(_domains[domainId].steward == msg.sender, "Not steward");
        require(_domains[domainId].active, "Domain inactive");
        require(domainApprovedRecipients[domainId][recipient], "Recipient not approved");
        require(_domains[domainId].spentBudget + amount <= _domains[domainId].allocatedBudget, "Insufficient budget");

        Domain storage domain = _domains[domainId];
        domain.spentBudget += amount;

        pool.allocate(recipient, amount);
        emit DomainAllocation(domainId, recipient, amount);
    }

    /// @notice Get all domains
    function getDomains() external view returns (bytes32[] memory) {
        return domainIds;
    }

    /// @notice Get domain recipients
    function getDomainRecipients(bytes32 domainId) external view returns (address[] memory) {
        return domainRecipientsList[domainId];
    }

    function getDomain(bytes32 domainId) external view returns (DomainInfo memory) {
        Domain storage domain = _domains[domainId];
        return DomainInfo({
            name: domain.name,
            description: domain.description,
            steward: domain.steward,
            allocatedBudget: domain.allocatedBudget,
            spentBudget: domain.spentBudget,
            active: domain.active
        });
    }
}