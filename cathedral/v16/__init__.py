#!/usr/bin/env python3
"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (Embodied Cognitive Core)      ║
║ Pacote Python: cathedral.v16                                               ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-PACKAGE-2026-06-14                       ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝

Módulos:
  vision.py       — Vision Transformer (ViT) com timm
  ontology.py     — OWL dinâmico + SWRL + Z3 SMT
  world_model.py  — RSSM (DreamerV3-style) para imaginação
  rl_agent.py     — SAC + Episodic Prioritized Replay (HNSW)
  rust_bridge.py  — Interface Python/Rust (stub gRPC/ZeroMQ)
  orchestrator.py — Loop Principal Percepção-Ação-Imaginação
  benchmarks.py   — Métricas: sample efficiency, forgetting, causal, energy

Arquitetura Híbrida:
  Python (Control Plane): I/O bound, orchestration, training loop
  Rust   (Data Plane):    HNSW, inference GGUF, DVFS, network
"""

__version__ = "16.0.0"
__substrato__ = 3000
__selo__ = "CATHEDRAL-ARKHE-v16.0.0-2026-06-14"
__arquiteto__ = "ORCID 0009-0005-2697-4668"

from .vision import VisionEncoder
from .ontology import SymbolicSafetyEngine
from .world_model import WorldModelRSSM, RSSMState
from .rl_agent import SACAgent
from .rust_bridge import RustBridgeStub
from .orchestrator import CathedralOrchestrator
from .benchmarks import CathedralBenchmarkSuite

__all__ = [
    "VisionEncoder",
    "SymbolicSafetyEngine",
    "WorldModelRSSM",
    "RSSMState",
    "SACAgent",
    "RustBridgeStub",
    "CathedralOrchestrator",
    "CathedralBenchmarkSuite",
]
