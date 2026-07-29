"""
Cathedral ARKHE v9.0 LOGOS — Hashtree Bridge
Integra Hashtree.cc (content-addressed storage P2P) com governança
descentralizada da Cathedral para persistência de substratos e
registro de evolução autônoma.
Arquiteto: ORCID 0009-0005-2697-4668
Selo: HASHTREE-BRIDGE-v9.0.0-2026-06-08
"""

import hashlib
import json
import time
from dataclasses import dataclass, field
from typing import Dict, List, Optional


@dataclass
class HashtreeConfig:
    """Configuração do bridge Hashtree ↔ Cathedral."""
    npub: str = "npub1cathedralarkhe..."
    nsec: str = ""
    visibility: str = "link_visible"
    relays: List[str] = field(default_factory=lambda: [
        "wss://relay.damus.io",
        "wss://relay.nostr.band",
        "wss://nos.lol",
    ])
    canonize_interval: int = 100
    persist_substrates: bool = True
    persist_telemetry: bool = True
    persist_governance: bool = True
    chunk_size: int = 65536
    deduplication: bool = True
    require_multi_sig: bool = True
    multi_sig_threshold: int = 3


class HashtreeCanonizer:
    """Canoniza substratos Cathedral na Hashtree (Merkle root + Nostr)."""

    def __init__(self, config: HashtreeConfig):
        self.config = config
        self._canonized: Dict[str, str] = {}
        self._history: List[Dict] = []

    def _compute_merkle_root(self, content: Dict) -> str:
        serialized = json.dumps(content, sort_keys=True, ensure_ascii=False)
        return hashlib.sha3_256(serialized.encode()).hexdigest()

    def canonize_substrate(self, substrate_id: str,
                           substrate_data: Dict,
                           telemetry: Optional[Dict] = None) -> Dict:
        content = {
            "substrate_id": substrate_id,
            "version": "9.0.0",
            "codename": "LOGOS",
            "seal": f"{substrate_id}-v9.0.0-2026-06-08",
            "data": substrate_data,
            "telemetry": telemetry or {},
            "timestamp": time.time(),
            "architect": "ORCID 0009-0005-2697-4668",
        }

        merkle_root = self._compute_merkle_root(content)
        visibility_hash = f"#{self.config.visibility}" if self.config.visibility != "public" else ""
        htree_url = f"htree://{self.config.npub}/{substrate_id}{visibility_hash}"

        self._canonized[substrate_id] = merkle_root
        record = {
            "substrate_id": substrate_id,
            "merkle_root": merkle_root,
            "htree_url": htree_url,
            "timestamp": content["timestamp"],
            "visibility": self.config.visibility,
        }
        self._history.append(record)

        return {
            "status": "canonized",
            "substrate_id": substrate_id,
            "merkle_root": merkle_root,
            "htree_url": htree_url,
            "seal": content["seal"],
            "record": record,
        }

    def verify_substrate(self, substrate_id: str,
                         expected_merkle_root: str) -> bool:
        actual = self._canonized.get(substrate_id)
        return actual == expected_merkle_root if actual else False

    def get_canonized_history(self) -> List[Dict]:
        return self._history.copy()

    def get_telemetry(self) -> Dict:
        return {
            "module": "HashtreeCanonizer",
            "version": "9.0.0",
            "substrate": "v9-decentralized",
            "seal": "HASHTREE-BRIDGE-v9.0.0-2026-06-08",
            "n_canonized": len(self._canonized),
            "n_history": len(self._history),
            "visibility": self.config.visibility,
            "relays": len(self.config.relays),
            "deduplication": self.config.deduplication,
        }


class HashtreeGovernanceBridge:
    """Bridge de governança descentralizada usando Hashtree."""

    def __init__(self, config: HashtreeConfig):
        self.config = config
        self.canonizer = HashtreeCanonizer(config)
        self._proposals: Dict[str, Dict] = {}
        self._decisions: List[Dict] = []

    def propose_governance_change(self, proposal_id: str,
                                   description: str,
                                   affected_substrates: List[str],
                                   proposer_npub: str) -> Dict:
        proposal = {
            "proposal_id": proposal_id,
            "description": description,
            "affected_substrates": affected_substrates,
            "proposer": proposer_npub,
            "timestamp": time.time(),
            "status": "proposed",
            "votes": {},
            "signatures": [],
        }

        canon = self.canonizer.canonize_substrate(f"proposal_{proposal_id}", proposal)

        self._proposals[proposal_id] = {
            **proposal,
            "merkle_root": canon["merkle_root"],
            "htree_url": canon["htree_url"],
        }

        return {
            "status": "proposed",
            "proposal_id": proposal_id,
            "merkle_root": canon["merkle_root"],
            "htree_url": canon["htree_url"],
            "required_signatures": self.config.multi_sig_threshold,
        }

    def sign_proposal(self, proposal_id: str,
                      signer_npub: str,
                      signature: str) -> Dict:
        if proposal_id not in self._proposals:
            return {"status": "error", "error": "Proposal not found"}

        proposal = self._proposals[proposal_id]
        proposal["signatures"].append({
            "signer": signer_npub,
            "signature": signature,
            "timestamp": time.time(),
        })

        if len(proposal["signatures"]) >= self.config.multi_sig_threshold:
            proposal["status"] = "approved"
            self._decisions.append({
                "proposal_id": proposal_id,
                "decision": "approved",
                "signatures": len(proposal["signatures"]),
                "timestamp": time.time(),
            })

        return {
            "status": proposal["status"],
            "proposal_id": proposal_id,
            "signatures": len(proposal["signatures"]),
            "required": self.config.multi_sig_threshold,
        }

    def get_governance_history(self) -> List[Dict]:
        return self._decisions.copy()

    def get_telemetry(self) -> Dict:
        return {
            "module": "HashtreeGovernanceBridge",
            "version": "9.0.0",
            "substrate": "v9-decentralized",
            "seal": "HASHTREE-GOVERNANCE-v9.0.0-2026-06-08",
            "n_proposals": len(self._proposals),
            "n_decisions": len(self._decisions),
            "multi_sig_threshold": self.config.multi_sig_threshold,
            "canonizer": self.canonizer.get_telemetry(),
        }
