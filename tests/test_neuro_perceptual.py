import pytest
import numpy as np
from arkhe_sdk.perceptual_geometry_engine import PerceptualGeometryEngine
from arkhe_sdk.cortexmae_bridge import CortexMAEBridge
from arkhe_sdk.arkhe_os import ArkheOmniAgent, ArkheConfig

def test_perceptual_geometry_engine():
    engine = PerceptualGeometryEngine()

    # Test layer analysis
    color_peak = engine.analyze_layer("color", 12)
    assert color_peak["organization_score"] == 1.0
    assert color_peak["geometry"] == "Circular manifold (color wheel)"

    color_early = engine.analyze_layer("color", 2)
    assert color_early["organization_score"] < 0.1

    # Test manifold coords
    coords = engine.get_manifold_coords("color", num_points=5)
    assert len(coords) == 5
    assert "x" in coords[0] and "y" in coords[0]

    # Test RSA
    rdm1 = np.eye(3)
    rdm2 = np.eye(3)
    alignment = engine.rsa_alignment(rdm1, rdm2)
    assert alignment == 1.0

def test_cortexmae_bridge():
    bridge = CortexMAEBridge()

    # Test fMRI processing
    mock_data = b"fmri_raw_signal_data"
    result = bridge.process_fmri_surface(mock_data)
    assert result["projection"] == "flat-map-2d"
    assert len(result["embedding"]) == 32

    # Test decoding
    state = bridge.decode_state(result["embedding"])
    assert "task_category" in state
    assert state["confidence"] > 0.8

    # Test diagnostic
    report = bridge.get_diagnostic_report()
    assert report["fidelity_score"] == 0.9992

def test_omni_agent_integration():
    cfg = ArkheConfig(neuro_perceptual_enabled=True)
    agent = ArkheOmniAgent(cfg)
    status = agent.get_status()

    # Should have 21 base + 2 neuro-perceptual + 4 CATHEDRAL substrates = 27
    assert status["substrates_active"] == 27
    assert agent.perceptual_engine is not None
    assert agent.cortex_bridge is not None

    report = agent.omni_report()
    assert "934   Perceptual Geometry" in report
    assert "563.1  CortexMAE Bridge" in report
