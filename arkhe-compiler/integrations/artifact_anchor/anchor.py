#!/usr/bin/env python3
# anchor.py – Ancora artefatos na TemporalChain

import hashlib
import json
import time
import argparse
import sys
from pathlib import Path
from typing import Union

# Make sure this works as a script called from the root dir
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))
from integrations.artifact_anchor.temporal_chain_client import TemporalChainClient

class AnchorArtifact:
    def __init__(self, temporal_client):
        self.temporal = temporal_client

    def anchor(self, source_file: Path, artifact_file: Path, passes: list) -> str:
        """Ancora um artefato gerado (C, G‑code, VHDL, Lean proof)."""
        # Conteúdo do artefato
        if not artifact_file.exists():
            artifact_file.parent.mkdir(parents=True, exist_ok=True)
            artifact_file.write_text("")
        artifact_content = artifact_file.read_bytes()
        artifact_hash = hashlib.sha3_256(artifact_content).hexdigest()

        # Metadados
        payload = {
            "source": str(source_file),
            "artifact": str(artifact_file),
            "hash": artifact_hash,
            "passes": passes,
            "timestamp": time.time(),
            "size_bytes": len(artifact_content)
        }

        # Se for Lean, extrai teoremas
        if artifact_file.suffix == ".lean":
            payload["theorems"] = self._extract_theorems(artifact_content)

        # Ancora na TemporalChain
        seal = self.temporal.anchor_event("compilation_artifact", payload)
        print(f"🔗 Ancorado {artifact_file.name} → {seal[:16]}...")
        return seal

    def _extract_theorems(self, content: bytes) -> list:
        import re
        text = content.decode("utf-8")
        return re.findall(r'theorem\s+(\w+)', text)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--artifact", required=True)
    parser.add_argument("--passes", required=True)
    args = parser.parse_args()
    anchor = AnchorArtifact(TemporalChainClient())
    anchor.anchor(Path(args.source), Path(args.artifact), args.passes.split(','))
