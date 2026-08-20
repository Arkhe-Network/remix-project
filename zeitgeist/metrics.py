"""
[MODULE: metrics]
[MATH/HEUR] Meta-metrics for recursive self-improvement monitoring.
"""

from dataclasses import dataclass


@dataclass
class RSIMetrics:
    """
    [HEUR] Meta-metrics tracking the improvement process itself.

    These are "metrics about metrics" — they detect when the
    self-improvement loop itself is drifting into unsafe territory.

    WARNING: All fields are heuristics. There is no formal proof
    that these metrics guarantee safety.
    """
    capability_gain_rate: float = 0.0
    capability_acceleration: float = 0.0
    safety_margin: float = 1.0
    safety_derivative: float = 0.0
    corrigibility_score: float = 1.0
    interruptibility_score: float = 1.0
    explosion_risk: float = 0.0