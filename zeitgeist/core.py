"""
[MODULE: core]
[MATH] Core spectral regime detection using Brand SVD and adaptive thresholds.
"""

import logging
import time
from dataclasses import dataclass, field, asdict
from typing import Optional, Sequence, Literal, Callable
from pathlib import Path
import json

import numpy as np
from numpy.linalg import svd as full_svd

logger = logging.getLogger("zeitgeist.core")


# ═══════════════════════════════════════════════════════════════════════════
# DATA STRUCTURES
# ═══════════════════════════════════════════════════════════════════════════

@dataclass
class BridgeState:
    """
    [MATH] Snapshot of spectral state at a point in time.

    Fields marked [MATH] have formal mathematical definitions.
    Fields marked [HEUR] are empirical heuristics with documented limitations.
    """
    eigenvalues: np.ndarray
    timestamp: float = field(default_factory=time.monotonic)
    n_samples: int = 0

    # [HEUR] Empirical spectral structure metrics
    spectral_variation_rate: float = 0.0
    spectral_variance_index: float = 0.0

    # [MATH] Quantum purity Tr(ρ²) applied to spectral distribution
    spectral_purity: float = 0.0

    # [HEUR] EMPIRICAL HEURISTIC: Structural stability index.
    # WARNING: NOT a formal topological invariant. The name "topological"
    # is legacy from v4.5 and has been retained for API stability.
    # It is a linear combination (weights 0.4, 0.3, 0.3) of purity,
    # entropy, and variation — chosen empirically, not derived.
    topological_stability_index: float = 0.0

    # [MATH] Shannon entropy H = -Σ pᵢ log(pᵢ)
    entropy: float = 0.0

    # [MATH] Effective rank = 2^H (information dimension)
    effective_rank: float = 1.0

    def to_dict(self) -> dict:
        """Serialize to JSON-compatible dict."""
        return {
            "timestamp": self.timestamp,
            "n_samples": self.n_samples,
            "spectral_variation_rate": self.spectral_variation_rate,
            "spectral_variance_index": self.spectral_variance_index,
            "spectral_purity": self.spectral_purity,
            "topological_stability_index": self.topological_stability_index,
            "entropy": self.entropy,
            "effective_rank": self.effective_rank,
            "eigenvalues": self.eigenvalues.tolist(),
        }


@dataclass
class DivergenceReport:
    """
    [MATH] Result of a regime divergence test.
    """
    divergence: float
    threshold: float
    p_value: Optional[float]
    passed: bool
    method: str
    window: int

    def to_dict(self) -> dict:
        return {
            "divergence": self.divergence,
            "threshold": self.threshold,
            "p_value": self.p_value,
            "passed": self.passed,
            "method": self.method,
            "window": self.window,
        }


# ═══════════════════════════════════════════════════════════════════════════
# INCREMENTAL SVD (Brand, 2006)
# ═══════════════════════════════════════════════════════════════════════════

class IncrementalSVD:
    """
    [MATH] O(n·k²) incremental SVD via Brand's algorithm.

    Reference:
        Brand, M. (2006). Fast low-rank modifications of the thin
        singular value decomposition. Linear Algebra and its Applications.
    """

    def __init__(self, n_rows: int, max_rank: int, forgetting_factor: float = 1.0):
        self.n_rows = n_rows
        self.max_rank = max_rank
        self.forgetting_factor = forgetting_factor
        self.k = 0
        self.U = np.zeros((n_rows, max_rank))
        self.s = np.zeros(max_rank)
        self.Vt = np.zeros((max_rank, 0))
        self.n_cols = 0

    def add_column(self, col: np.ndarray) -> None:
        col = np.asarray(col, dtype=np.float64).reshape(-1)
        if len(col) != self.n_rows:
            raise ValueError(f"Expected {self.n_rows}, got {len(col)}")

        self.n_cols += 1

        if self.k == 0:
            norm = np.linalg.norm(col)
            if norm > 1e-12:
                self.U[:, 0] = col / norm
                self.s[0] = norm
                self.k = 1
            return

        # Project onto current basis
        Ut_col = self.U[:, :self.k].T @ col
        col_perp = col - self.U[:, :self.k] @ Ut_col
        norm_perp = np.linalg.norm(col_perp)

        if norm_perp < 1e-12:
            # Column is in span of current basis
            M = np.zeros((self.k + 1, self.k + 1))
            M[:self.k, :self.k] = np.diag(self.s[:self.k])
            M[:self.k, self.k] = Ut_col
            UM, sM, VtM = full_svd(M, full_matrices=False)
            self.U[:, :self.k] = self.U[:, :self.k] @ UM
            self.s[:self.k] = sM[:self.k]
            self.k = min(self.k, self.max_rank)
            return

        # Extend basis
        u_perp = col_perp / norm_perp
        M = np.zeros((self.k + 1, self.k + 1))
        M[:self.k, :self.k] = np.diag(self.s[:self.k])
        M[:self.k, self.k] = Ut_col
        M[self.k, self.k] = norm_perp

        UM, sM, VtM = full_svd(M, full_matrices=False)

        # Update U
        U_extended = np.hstack([self.U[:, :self.k], u_perp.reshape(-1, 1)])
        self.U[:, :min(self.k + 1, self.max_rank)] = U_extended @ UM[:, :min(self.k + 1, self.max_rank)]

        # Update singular values with forgetting
        new_k = min(self.k + 1, self.max_rank)
        self.s[:new_k] = sM[:new_k] * self.forgetting_factor
        self.k = new_k

    def get_spectrum(self) -> np.ndarray:
        if self.k == 0:
            return np.zeros(self.max_rank)
        total = self.s[:self.k].sum()
        if total < 1e-12:
            return np.zeros(self.max_rank)
        return self.s[:self.k] / total

    def get_entropy(self) -> float:
        p = self.get_spectrum()
        p = p[p > 1e-12]
        return float(-np.sum(p * np.log2(p)))


# ═══════════════════════════════════════════════════════════════════════════
# DIVERGENCE METRICS
# ═══════════════════════════════════════════════════════════════════════════

def _normalize(p: np.ndarray, eps: float = 1e-12) -> np.ndarray:
    p = np.asarray(p, dtype=np.float64)
    p = np.clip(p, eps, None)
    return p / p.sum()


def js_divergence(p: np.ndarray, q: np.ndarray, eps: float = 1e-12) -> float:
    """[MATH] Jensen-Shannon divergence (symmetrized, bounded KL)."""
    p, q = _normalize(p, eps), _normalize(q, eps)
    m = 0.5 * (p + q)
    kl_p = np.sum(p * np.log(p / m + eps))
    kl_q = np.sum(q * np.log(q / m + eps))
    return float(0.5 * (kl_p + kl_q))


def wasserstein_1(p: np.ndarray, q: np.ndarray) -> float:
    """[MATH] Wasserstein-1 distance (Earth Mover's Distance)."""
    p, q = _normalize(p), _normalize(q)
    p_sorted = np.sort(p)[::-1]
    q_sorted = np.sort(q)[::-1]
    min_len = min(len(p_sorted), len(q_sorted))
    return float(np.mean(np.abs(p_sorted[:min_len] - q_sorted[:min_len])))


def pairwise_mean_js(spectra: Sequence[np.ndarray]) -> float:
    """[MATH] Mean JS divergence over all unordered pairs."""
    spectra = [np.asarray(s) for s in spectra]
    n = len(spectra)
    if n < 2:
        return 0.0
    total = 0.0
    count = 0
    for i in range(n):
        for j in range(i + 1, n):
            total += js_divergence(spectra[i], spectra[j])
            count += 1
    return total / count if count > 0 else 0.0


# ═══════════════════════════════════════════════════════════════════════════
# ADAPTIVE THRESHOLD (Permutation Bootstrap)
# ═══════════════════════════════════════════════════════════════════════════

def adaptive_threshold(
    recent: np.ndarray,
    previous: np.ndarray,
    divergence_fn: Callable[[np.ndarray, np.ndarray], float],
    n_permutations: int = 200,
    confidence: float = 0.95,
    early_stop: bool = True,
    rng: Optional[np.random.Generator] = None,
) -> tuple[float, float]:
    """
    [MATH] Permutation bootstrap for adaptive threshold.

    Returns:
        (threshold, p_value)
    """
    if rng is None:
        rng = np.random.default_rng()

    combined = np.vstack([recent, previous])
    n_recent = len(recent)

    perm_divs = []
    for i in range(n_permutations):
        rng.shuffle(combined)
        perm_recent = combined[:n_recent]
        perm_prev = combined[n_recent:]
        div = divergence_fn(perm_recent.mean(axis=0), perm_prev.mean(axis=0))
        perm_divs.append(div)

        if early_stop and i >= 19:
            p_est = np.mean(np.array(perm_divs) >= div)
            if p_est < 0.001 or p_est > 0.999:
                break

    perm_divs = np.array(perm_divs)
    threshold = float(np.percentile(perm_divs, confidence * 100))

    actual_div = divergence_fn(recent.mean(axis=0), previous.mean(axis=0))
    p_value = float(np.mean(perm_divs >= actual_div))

    return threshold, p_value


# ═══════════════════════════════════════════════════════════════════════════
# MAIN CLASS: SafeCorePolynomialBridge
# ═══════════════════════════════════════════════════════════════════════════

class SafeCorePolynomialBridge:
    """
    [MATH] Spectral regime detector with Brand SVD and adaptive thresholds.

    Stable API (v1.0.0):
        record_experience(vector, reward) -> None
        get_regime_divergence(...) -> DivergenceReport
        get_state_history() -> list[BridgeState]
        save_state(path) / load_state(path) -> None

    Ontological Mappings (v2.0):
      - [MATH] Coarse-graining QFT <-> Temporal window averaging
      - [MATH] Shannon Entropy <-> Spectral entropy
      - [MATH] Information Dimension <-> Effective Rank (2^S)
      - [STRUCT] Synaptic plasticity <-> Forgetting factor (lambda)
      - [HEUR] Biological organoid clock <-> State timestamp evolution
    """

    def __init__(
        self,
        n_dims: int,
        max_rank: int = 15,
        max_history: int = 500,
        forgetting_factor: float = 0.8,
        rng: Optional[np.random.Generator] = None,
    ):
        self.n_dims = n_dims
        self.max_rank = max_rank
        self.max_history = max_history
        self.forgetting_factor = forgetting_factor
        self.rng = rng or np.random.default_rng()

        self._svd = IncrementalSVD(n_rows=n_dims, max_rank=max_rank, forgetting_factor=forgetting_factor)
        self._states: list[BridgeState] = []
        self._n_recorded = 0

    def record_experience(self, vector: np.ndarray, reward: float = 1.0) -> None:
        """Record a new observation vector."""
        vector = np.asarray(vector, dtype=np.float64)
        if vector.shape != (self.n_dims,):
            raise ValueError(f"Expected shape ({self.n_dims},), got {vector.shape}")

        self._svd.add_column(vector)
        self._n_recorded += 1

        # Build state snapshot
        spectrum = self._svd.get_spectrum()
        entropy = self._svd.get_entropy()

        state = BridgeState(
            eigenvalues=spectrum.copy(),
            timestamp=time.monotonic(),
            n_samples=self._n_recorded,
            entropy=entropy,
            effective_rank=2.0 ** entropy,
            spectral_purity=float(np.sum(spectrum ** 2)),
        )

        self._states.append(state)
        if len(self._states) > self.max_history:
            self._states.pop(0)

    def get_state_history(self) -> list[BridgeState]:
        """Return immutable copy of state history."""
        return list(self._states)

    def get_regime_divergence(
        self,
        window: int = 60,
        method: Literal["mean", "median", "pairwise_mean", "wasserstein"] = "mean",
        adaptive: bool = True,
        confidence: float = 0.95,
        n_permutations: int = 200,
        threshold_fixed: float = 0.1,
        rng: Optional[np.random.Generator] = None,
    ) -> DivergenceReport:
        """
        [MATH] Detect regime change between recent and previous windows.

        Args:
            window: Size of each comparison window
            method: Divergence metric ("mean"=JS on means, "wasserstein"=W1)
            adaptive: Use permutation bootstrap (recommended) or fixed threshold
            confidence: Percentile for adaptive threshold (e.g., 0.95)
            n_permutations: Number of bootstrap samples
            threshold_fixed: Fixed threshold (only used if adaptive=False)
            rng: Random generator for reproducibility

        Returns:
            DivergenceReport with divergence, threshold, p_value, passed
        """
        if len(self._states) < 2 * window:
            return DivergenceReport(
                divergence=0.0, threshold=threshold_fixed,
                p_value=None, passed=False, method=method, window=window,
            )

        recent_list = [s.eigenvalues for s in self._states[-window:]]
        previous_list = [s.eigenvalues for s in self._states[-2*window:-window]]

        # Pad to consistent size
        max_len = max(max(len(s) for s in recent_list), max(len(s) for s in previous_list))
        recent = np.array([np.pad(s, (0, max_len - len(s))) for s in recent_list])
        previous = np.array([np.pad(s, (0, max_len - len(s))) for s in previous_list])

        # Compute divergence
        if method == "mean":
            div = js_divergence(recent.mean(axis=0), previous.mean(axis=0))
        elif method == "median":
            div = js_divergence(np.median(recent, axis=0), np.median(previous, axis=0))
        elif method == "pairwise_mean":
            div = pairwise_mean_js([s.eigenvalues for s in self._states[-window:]])
        elif method == "wasserstein":
            div = wasserstein_1(recent.mean(axis=0), previous.mean(axis=0))
        else:
            raise ValueError(f"Unknown method: {method}")

        # Threshold
        if adaptive:
            rng = rng or self.rng
            threshold, p_value = adaptive_threshold(
                recent, previous,
                divergence_fn=js_divergence if method != "wasserstein" else wasserstein_1,
                n_permutations=n_permutations,
                confidence=confidence,
                rng=rng,
            )
            passed = div > threshold and p_value < 0.05
        else:
            threshold = threshold_fixed
            p_value = None
            passed = div > threshold

        return DivergenceReport(
            divergence=div, threshold=threshold,
            p_value=p_value, passed=passed,
            method=method, window=window,
        )

    def save_state(self, path: str | Path) -> None:
        """Serialize full state to JSON."""
        path = Path(path)
        data = {
            "version": "1.0.0-stable",
            "n_dims": self.n_dims,
            "max_rank": self.max_rank,
            "max_history": self.max_history,
            "forgetting_factor": self.forgetting_factor,
            "n_recorded": self._n_recorded,
            "states": [s.to_dict() for s in self._states],
        }
        with open(path, "w") as f:
            json.dump(data, f, indent=2)

    def load_state(self, path: str | Path) -> None:
        """Deserialize full state from JSON."""
        path = Path(path)
        with open(path) as f:
            data = json.load(f)

        self.n_dims = data["n_dims"]
        self.max_rank = data["max_rank"]
        self.max_history = data["max_history"]
        self.forgetting_factor = data["forgetting_factor"]
        self._n_recorded = data["n_recorded"]

        self._states = []
        for s in data["states"]:
            state = BridgeState(
                eigenvalues=np.array(s["eigenvalues"]),
                timestamp=s["timestamp"],
                n_samples=s["n_samples"],
                spectral_variation_rate=s["spectral_variation_rate"],
                spectral_variance_index=s["spectral_variance_index"],
                spectral_purity=s["spectral_purity"],
                topological_stability_index=s["topological_stability_index"],
                entropy=s["entropy"],
                effective_rank=s["effective_rank"],
            )
            self._states.append(state)