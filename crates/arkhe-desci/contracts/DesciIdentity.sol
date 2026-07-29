// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title DesciIdentity
 * @notice Registry de identidades DID para pesquisadores DeSci
 * @dev Mapeia DID → ORCID → metadados, com suporte a attestations
 */

struct Identity {
    string did;
    string orcidId;
    string controller;
    string name;
    string institution;
    uint256 registeredAt;
    bool active;
}

struct Attestation {
    string attesterDid;
    string claimType;
    string claimData;      // JSON-encoded
    uint256 issuedAt;
    uint256 expiresAt;
    bytes32 proofHash;
    bool revoked;
}

event IdentityRegistered(string indexed did, string orcidId);
event AttestationAdded(string indexed subjectDid, string claimType, string attesterDid);
event AttestationRevoked(string indexed subjectDid, string attesterDid);

contract DesciIdentity {
    mapping(string => Identity) public identities;
    // did => attesterDid => attestation index => Attestation
    mapping(string => mapping(string => Attestation[])) public attestations;

    uint256 public totalIdentities;
    address public admin;
    address public authority; // DID authority que pode emitir attestations

    modifier onlyAdmin() {
        require(msg.sender == admin, "Not admin");
        _;
    }

    modifier onlyAuthority() {
        require(msg.sender == authority, "Not authority");
        _;
    }

    constructor() {
        admin = msg.sender;
        authority = msg.sender;
    }

    function setAuthority(address _authority) external onlyAdmin {
        authority = _authority;
    }

    /**
     * @notice Registra uma identidade DID com ORCID opcional
     */
    function registerIdentity(
        string calldata _did,
        string calldata _orcidId,
        string calldata _controller,
        string calldata _name,
        string calldata _institution
    ) external {
        require(bytes(identities[_did].did).length == 0, "Already registered");
        require(bytes(_did).length > 0, "DID empty");

        identities[_did] = Identity({
            did: _did,
            orcidId: _orcidId,
            controller: _controller,
            name: _name,
            institution: _institution,
            registeredAt: block.timestamp,
            active: true
        });

        totalIdentities++;
        emit IdentityRegistered(_did, _orcidId);
    }

    /**
     * @notice Adiciona attestation a uma identidade
     */
    function addAttestation(
        string calldata _subjectDid,
        string calldata _attesterDid,
        string calldata _claimType,
        string calldata _claimData,
        uint256 _validitySeconds,
        bytes32 _proofHash
    ) external onlyAuthority {
        require(bytes(identities[_subjectDid].did).length > 0, "Identity not found");

        attestations[_subjectDid][_attesterDid].push(Attestation({
            attesterDid: _attesterDid,
            claimType: _claimType,
            claimData: _claimData,
            issuedAt: block.timestamp,
            expiresAt: block.timestamp + _validitySeconds,
            proofHash: _proofHash,
            revoked: false
        }));

        emit AttestationAdded(_subjectDid, _claimType, _attesterDid);
    }

    /**
     * @notice Revoca attestation
     */
    function revokeAttestation(
        string calldata _subjectDid,
        string calldata _attesterDid,
        uint256 _index
    ) external onlyAuthority {
        require(_index < attestations[_subjectDid][_attesterDid].length, "Invalid index");
        attestations[_subjectDid][_attesterDid][_index].revoked = true;
        emit AttestationRevoked(_subjectDid, _attesterDid);
    }

    /**
     * @notice Retorna uma identidade
     */
    function getIdentity(string calldata _did) external view returns (Identity memory) {
        require(bytes(identities[_did].did).length > 0, "Not found");
        return identities[_did];
    }

    /**
     * @notice Retorna número de attestations válidas
     */
    function getValidAttestationCount(
        string calldata _did,
        string calldata _attesterDid
    ) external view returns (uint256) {
        uint256 count = 0;
        Attestation[] storage atts = attestations[_did][_attesterDid];
        for (uint256 i = 0; i < atts.length; i++) {
            if (!atts[i].revoked && atts[i].expiresAt > block.timestamp) {
                count++;
            }
        }
        return count;
    }
}
