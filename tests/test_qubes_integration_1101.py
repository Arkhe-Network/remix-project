import pytest
from arkhe_sdk.substrates.qubes_integration_1101 import QubesIntegrationOrchestrator

def test_qubes_integration_orchestrator_initialization():
    orchestrator = QubesIntegrationOrchestrator(mode="test")
    assert orchestrator.mode == "test"
    assert orchestrator.seal == "CATHEDRAL-QUBES-1101-v1.0.0-2026-06-12"

def test_get_manifesto():
    orchestrator = QubesIntegrationOrchestrator(mode="test")
    manifesto = orchestrator.get_manifesto()
    assert "CATHEDRAL-QUBES-1101-v1.0.0-2026-06-12" in manifesto
    assert "Substrato 1101" in manifesto

def test_list_qubes():
    orchestrator = QubesIntegrationOrchestrator(mode="test")
    qubes = orchestrator.list_qubes()
    assert isinstance(qubes, list)
    assert "agi-core" in qubes
    assert "governance" in qubes
    assert "llm-inference" in qubes
    assert "crypto-vm" in qubes
    assert len(qubes) == 9

def test_run_qrexec_test_mode():
    orchestrator = QubesIntegrationOrchestrator(mode="test")
    result = orchestrator.run_qrexec("llm-inference", "cathedral.LLMInference", "test payload")
    assert result["status"] == "success"
    assert result["simulated"] is True
    assert result["target"] == "llm-inference"
    assert result["service"] == "cathedral.LLMInference"

def test_protocolo_corte_mestre():
    orchestrator = QubesIntegrationOrchestrator(mode="test")
    discourse_analysis = {"classification": "MESTRE", "confidence": 0.95}
    result = orchestrator.protocolo_corte(discourse_analysis, "browser-vm")
    assert result["action"] == "KILL_QUBE"
    assert result["target"] == "browser-vm"
    assert result["status"] == "requested_simulated"

def test_protocolo_corte_continue():
    orchestrator = QubesIntegrationOrchestrator(mode="test")
    discourse_analysis = {"classification": "NEUTRO", "confidence": 0.99}
    result = orchestrator.protocolo_corte(discourse_analysis, "code-vm")
    assert result["action"] == "CONTINUE"
    assert result["target"] == "code-vm"
