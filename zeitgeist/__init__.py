"""
═══════════════════════════════════════════════════════════════════════════
zeitgeist — Spectral Regime Detection Library
═══════════════════════════════════════════════════════════════════════════

A stable, validated library for detecting regime changes in high-dimensional
data streams using incremental SVD and information-theoretic divergences.

Version: 1.0.0-stable
License: MIT (Safe-Core Compatible)
Python: >=3.10

Quick Start:
    >>> from zeitgeist import SafeCorePolynomialBridge
    >>> bridge = SafeCorePolynomialBridge(n_dims=100, max_rank=20)
    >>> for vector in data_stream:
    ...     bridge.record_experience(vector)
    >>> report = bridge.get_regime_divergence(window=60, adaptive=True)
    >>> print(f"Regime change detected: {report.passed}")

Ontology (v2.0):
    [MATH]   — Isomorphism or verifiable homomorphism
    [STRUCT] — Architectural analogy
    [HEUR]   — Design inspiration, generates testable hypotheses
"""

__version__ = "1.0.0-stable"
__author__ = "Arkhe(n) Research Group"

from .core import SafeCorePolynomialBridge, BridgeState, DivergenceReport
from .metrics import RSIMetrics
from .validators import RegimeValidator, SyntheticDataGenerator

__all__ = [
    "SafeCorePolynomialBridge",
    "BridgeState",
    "DivergenceReport",
    "RSIMetrics",
    "RegimeValidator",
    "SyntheticDataGenerator",
]
