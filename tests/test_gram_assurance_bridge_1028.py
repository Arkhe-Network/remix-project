import pytest
import numpy as np
from arkhe_sdk.substrates.gram_assurance_bridge_1028 import GramAssuranceBridge, LPRMValueHead, SafetyCaseGSN

def test_lprm_value_head():
    head = LPRMValueHead(dim=512)
    z = np.random.randn(512)
    score = head.evaluate(z)
    assert 0.0 <= score <= 1.0

def test_gram_assurance_bridge():
    bridge = GramAssuranceBridge(claim="Test claim", context="Test context", lprm_dim=512)
    bridge.build_standard_structure()

    np.random.seed(42)
    trajectory = [np.random.randn(512) for _ in range(8)]

    result = bridge.evaluate_trajectory(trajectory, zk_proof="1234567890abcdef")
    assert result["safety_case_status"] in ["SATISFIED", "FAILED"]
    assert len(result["confidence_trajectory"]) == 8
