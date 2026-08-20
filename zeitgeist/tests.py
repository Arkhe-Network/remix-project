"""
[MODULE: tests]
[MATH] Comprehensive test suite for zeitgeist v1.0.0-stable.
"""

import numpy as np
import tempfile
import os

from zeitgeist.core import (
    SafeCorePolynomialBridge,
    BridgeState,
    js_divergence,
    wasserstein_1,
    IncrementalSVD,
)
from zeitgeist.validators import (
    SyntheticDataGenerator,
    RegimeValidator,
    ValidationResult,
)


def test_js_properties():
    """[MATH] T1: JS divergence properties."""
    p = np.array([0.5, 0.3, 0.2])
    q = np.array([0.2, 0.5, 0.3])

    div = js_divergence(p, q)
    assert div >= 0, "JS must be non-negative"
    assert div <= 1.0, "JS must be bounded by 1"
    assert abs(js_divergence(p, p)) < 1e-10, "JS(p,p) = 0"
    assert abs(js_divergence(p, q) - js_divergence(q, p)) < 1e-10, "JS is symmetric"
    print("  T1: JS Properties — PASSOU")


def test_wasserstein_properties():
    """[MATH] T2: Wasserstein-1 properties."""
    p = np.array([0.5, 0.3, 0.2])
    q = np.array([0.2, 0.5, 0.3])

    w1 = wasserstein_1(p, q)
    assert w1 >= 0, "W1 must be non-negative"
    assert abs(wasserstein_1(p, p)) < 1e-10, "W1(p,p) = 0"
    print("  T2: Wasserstein Properties — PASSOU")


def test_incremental_svd():
    """[MATH] T3: Brand SVD correctness."""
    svd = IncrementalSVD(n_rows=10, max_rank=5)
    rng = np.random.default_rng(42)

    for _ in range(20):
        svd.add_column(rng.standard_normal(10))

    assert svd.k > 0, "SVD must have positive rank"
    spectrum = svd.get_spectrum()
    assert abs(spectrum.sum() - 1.0) < 1e-6, "Spectrum must sum to 1"
    assert svd.get_entropy() >= 0, "Entropy must be non-negative"

    # Check orthogonality
    U = svd.U[:, :svd.k]
    UtU = U.T @ U
    assert np.max(np.abs(UtU - np.eye(svd.k))) < 1e-10, "U must be orthogonal"
    print("  T3: Incremental SVD — PASSOU")


def test_regime_detection():
    """[MATH] T4-T7: Regime detection on synthetic data."""
    rng = np.random.default_rng(123)
    bridge = SafeCorePolynomialBridge(n_dims=20, max_rank=15, rng=rng)

    # Phase A
    for _ in range(80):
        vec = rng.standard_normal(20)
        vec[:10] += 3.5
        bridge.record_experience(vec, 0.8)

    # Phase B
    for _ in range(80):
        vec = rng.standard_normal(20)
        vec[10:] += 3.5
        bridge.record_experience(vec, 0.8)

    # Test all methods
    methods = ["mean", "median", "pairwise_mean", "wasserstein"]
    for method in methods:
        report = bridge.get_regime_divergence(
            window=60, method=method, adaptive=True, confidence=0.95,
            n_permutations=200, rng=np.random.default_rng(123),
        )
        assert report.passed, f"Method {method} should detect regime change"
        assert report.p_value is not None and report.p_value < 0.05
        print(f"  T4-T7: Regime ({method}) — PASSOU")


def test_save_load():
    """[MATH] T8: State persistence."""
    bridge = SafeCorePolynomialBridge(n_dims=10, max_rank=5)
    for _ in range(20):
        bridge.record_experience(np.random.randn(10))

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
        path = f.name

    try:
        bridge.save_state(path)
        bridge2 = SafeCorePolynomialBridge(n_dims=10, max_rank=5)
        bridge2.load_state(path)

        assert len(bridge2.get_state_history()) == len(bridge.get_state_history())
        print("  T8: Save/Load — PASSOU")
    finally:
        os.unlink(path)


def test_validation_framework():
    """[MATH] T9: Validation metrics on controlled synthetic data."""
    # Ground truth: changes at 100 and 300
    true_changes = [100, 300]
    total = 500

    # Perfect detector
    validator = RegimeValidator(tolerance=10)
    result = validator.validate([100, 300], true_changes, total)
    assert result.sensitivity == 1.0, "Perfect detector should have sensitivity=1"
    assert result.precision == 1.0, "Perfect detector should have precision=1"
    assert result.f1_score == 1.0, "Perfect detector should have F1=1"

    # Detector with one false positive and one missed
    result2 = validator.validate([105, 200], true_changes, total)
    assert result2.sensitivity == 0.5, "Should detect 1 of 2"
    assert result2.precision == 0.5, "1 TP, 1 FP"

    # Detector with delay
    result3 = validator.validate([108], true_changes, total)
    assert result3.sensitivity == 0.5, "Should detect 1 of 2 with delay"
    assert result3.mean_detection_delay == 8.0, "Delay should be 8"

    print("  T9: Validation Framework — PASSOU")
    print(f"       Perfect: {result}")
    print(f"       Imperfect: {result2}")


def run_all_tests():
    """Run complete test suite."""
    print("=" * 70)
    print("ZEITGEIST v1.0.0-stable — Test Suite")
    print("=" * 70)

    test_js_properties()
    test_wasserstein_properties()
    test_incremental_svd()
    test_regime_detection()
    test_save_load()
    test_validation_framework()

    print("=" * 70)
    print("ALL TESTS PASSED")
    print("=" * 70)


if __name__ == "__main__":
    run_all_tests()