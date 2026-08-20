"""
[MODULE: validators]
[MATH] Validation framework for regime detection algorithms.
"""

import numpy as np
from typing import Sequence, Callable, Optional
from dataclasses import dataclass


@dataclass
class ValidationResult:
    """[MATH] Standardized validation metrics."""
    true_positives: int
    false_positives: int
    true_negatives: int
    false_negatives: int
    detection_delays: list[float]

    @property
    def sensitivity(self) -> float:
        """[MATH] Recall = TP / (TP + FN)"""
        denom = self.true_positives + self.false_negatives
        return self.true_positives / denom if denom > 0 else 0.0

    @property
    def specificity(self) -> float:
        """[MATH] TN / (TN + FP)"""
        denom = self.true_negatives + self.false_positives
        return self.true_negatives / denom if denom > 0 else 0.0

    @property
    def precision(self) -> float:
        """[MATH] TP / (TP + FP)"""
        denom = self.true_positives + self.false_positives
        return self.true_positives / denom if denom > 0 else 0.0

    @property
    def f1_score(self) -> float:
        """[MATH] Harmonic mean of precision and recall."""
        p, r = self.precision, self.sensitivity
        return 2 * p * r / (p + r) if (p + r) > 0 else 0.0

    @property
    def mean_detection_delay(self) -> float:
        """[MATH] Average delay between true change and detection."""
        return np.mean(self.detection_delays) if self.detection_delays else 0.0

    def __str__(self) -> str:
        return (
            f"ValidationResult("
            f"sensitivity={self.sensitivity:.3f}, "
            f"specificity={self.specificity:.3f}, "
            f"precision={self.precision:.3f}, "
            f"f1={self.f1_score:.3f}, "
            f"mean_delay={self.mean_detection_delay:.1f})"
        )


class SyntheticDataGenerator:
    """
    [MATH] Generate synthetic data with known regime changes for validation.
    """

    @staticmethod
    def gaussian_regime(
        n_samples: int,
        n_dims: int,
        active_dims: Sequence[int],
        mean_shift: float = 3.0,
        seed: Optional[int] = None,
    ) -> np.ndarray:
        """Generate samples with variance concentrated in active_dims."""
        rng = np.random.default_rng(seed)
        data = rng.standard_normal((n_samples, n_dims))
        data[:, list(active_dims)] += mean_shift
        return data

    @staticmethod
    def generate_trajectory(
        n_total: int,
        n_dims: int,
        change_points: Sequence[int],
        seed: Optional[int] = None,
    ) -> tuple[np.ndarray, list[int]]:
        """
        Generate trajectory with known regime changes.

        Returns:
            (data, change_points) where data.shape = (n_total, n_dims)
        """
        rng = np.random.default_rng(seed)
        data = np.zeros((n_total, n_dims))

        regimes = []
        start = 0
        for i, cp in enumerate(list(change_points) + [n_total]):
            active = list(range((i * 10) % n_dims, ((i + 1) * 10) % n_dims))
            if not active:
                active = [0]
            data[start:cp] = SyntheticDataGenerator.gaussian_regime(
                cp - start, n_dims, active, seed=seed + i if seed else None
            )
            regimes.append((start, cp, active))
            start = cp

        return data, list(change_points)


class RegimeValidator:
    """
    [MATH] Validate a regime detector against ground-truth labels.
    """

    def __init__(self, tolerance: int = 10):
        """
        Args:
            tolerance: Maximum acceptable delay (in samples) for a true positive.
        """
        self.tolerance = tolerance

    def validate(
        self,
        detected_changes: Sequence[int],
        true_changes: Sequence[int],
        total_samples: int,
    ) -> ValidationResult:
        """
        [MATH] Compute validation metrics.

        Args:
            detected_changes: Indices where detector fired
            true_changes: Ground-truth change points
            total_samples: Total length of the series

        Returns:
            ValidationResult with sensitivity, specificity, precision, F1, delay
        """
        tp = fp = fn = 0
        delays = []
        matched_true = set()
        matched_detected = set()

        for tc in true_changes:
            # Find nearest detection within tolerance
            best_d = None
            best_delay = float('inf')
            for dc in detected_changes:
                if dc in matched_detected:
                    continue
                delay = abs(dc - tc)
                if delay <= self.tolerance and delay < best_delay:
                    best_delay = delay
                    best_d = dc

            if best_d is not None:
                tp += 1
                delays.append(best_delay)
                matched_true.add(tc)
                matched_detected.add(best_d)
            else:
                fn += 1

        fp = len(detected_changes) - len(matched_detected)

        # TN: no change and no detection (approximate)
        # Conservative: count windows without changes or detections
        tn = max(0, total_samples - len(true_changes) * self.tolerance - fp)

        return ValidationResult(
            true_positives=tp,
            false_positives=fp,
            true_negatives=tn,
            false_negatives=fn,
            detection_delays=delays,
        )