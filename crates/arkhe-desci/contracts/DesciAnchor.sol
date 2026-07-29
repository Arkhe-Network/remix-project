// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title DesciAnchor
 * @notice Ancora datasets DeSci on-chain com vínculo a DID e ORCID
 * @dev Deployed em SEI GigaChain (ou EVM sidechain)
 *
 * Fluxo: researcher (DID) → anchorDataset(CID, checksum, ORCID, trace) → evento emitido
 * Query: getAnchor(CID) → AnchorInfo
 */

struct Anchor {
    string cid;
    string checksumSha256;
    string authorDid;
    string orcidId;        // opcional
    string traceId;        // IC16 causal chain ID
    string metadataUri;    // IPFS URI para metadados completos
    string license;
    address owner;
    uint256 anchoredAt;
    uint256 blockHeight;
}

event DatasetAnchored(
    string indexed cid,
    string authorDid,
    string orcidId,
    uint256 blockHeight
);

event DatasetAnchoredWithTrace(
    string indexed cid,
    string traceId,
    string authorDid,
    uint256 blockHeight
);

contract DesciAnchor {
    // CID => Anchor
    mapping(string => Anchor) public anchors;
    // DID => count de anchors
    mapping(string => uint256) public anchorCounts;
    // ORCID => DID (para verificação)
    mapping(string => string) public orcidToDid;

    uint256 public totalAnchors;
    address public admin;

    modifier onlyAdmin() {
        require(msg.sender == admin, "Not admin");
        _;
    }

    constructor() {
        admin = msg.sender;
    }

    /**
     * @notice Ancora um dataset na blockchain
     * @param _cid IPFS CID do dataset
     * @param _checksum SHA-256 do arquivo original
     * @param _authorDid DID ARKHE do autor
     * @param _orcidId ORCID iD do autor (vazio se não aplicável)
     * @param _traceId IC16 trace ID (vazio se não aplicável)
     * @param _metadataUri IPFS URI para metadados JSON
     * @param _license Licença do dataset
     */
    function anchorDataset(
        string calldata _cid,
        string calldata _checksum,
        string calldata _authorDid,
        string calldata _orcidId,
        string calldata _traceId,
        string calldata _metadataUri,
        string calldata _license
    ) external {
        require(bytes(anchors[_cid].cid).length == 0, "Already anchored");
        require(bytes(_cid).length > 0, "CID empty");
        require(bytes(_authorDid).length > 0, "DID empty");

        anchors[_cid] = Anchor({
            cid: _cid,
            checksumSha256: _checksum,
            authorDid: _authorDid,
            orcidId: _orcidId,
            traceId: _traceId,
            metadataUri: _metadataUri,
            license: _license,
            owner: msg.sender,
            anchoredAt: block.timestamp,
            blockHeight: block.number
        });

        anchorCounts[_authorDid]++;
        totalAnchors++;

        if (bytes(_orcidId).length > 0) {
            orcidToDid[_orcidId] = _authorDid;
        }

        emit DatasetAnchored(_cid, _authorDid, _orcidId, block.number);

        if (bytes(_traceId).length > 0) {
            emit DatasetAnchoredWithTrace(_cid, _traceId, _authorDid, block.number);
        }
    }

    /**
     * @notice Retorna informações de um anchor
     */
    function getAnchor(string calldata _cid) external view returns (Anchor memory) {
        require(bytes(anchors[_cid].cid).length > 0, "Not found");
        return anchors[_cid];
    }

    /**
     * @notice Verifica se um CID está ancorado
     */
    function isAnchored(string calldata _cid) external view returns (bool) {
        return bytes(anchors[_cid].cid).length > 0;
    }

    /**
     * @notice Resolve ORCID para DID
     */
    function resolveOrcid(string calldata _orcidId) external view returns (string memory) {
        return orcidToDid[_orcidId];
    }

    /**
     * @notice Retorna número de anchors de um DID
     */
    function getAnchorCount(string calldata _did) external view returns (uint256) {
        return anchorCounts[_did];
    }
}
