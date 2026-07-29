import pytest
import numpy as np
from arkhe_sdk.substrates.orchestrator_rsi_1076_3 import OrchestratorRSI

def test_orchestrator_rsi_run():
    orchestrator = OrchestratorRSI(theosis_threshold=0.8)
    X = np.random.rand(100, 2)
    dX = np.random.rand(100, 2)

    results = orchestrator.run(X, dX, ["dx_dt", "dy_dt"])

    assert len(results) == 2
    assert results[0].deployed == True
    assert results[1].deployed == True

    assert len(orchestrator.system_state['substrates']) == 2

def test_orchestrator_export_metrics():
    orchestrator = OrchestratorRSI()
    X = np.random.rand(100, 2)
    dX = np.random.rand(100, 2)
    orchestrator.run(X, dX, ["dx_dt"])

    metrics = orchestrator.export_metrics()
    assert metrics['substrate'] == '1076.3'
    assert metrics['total_cycles'] == 1
    assert metrics['successful_deploys'] == 1
