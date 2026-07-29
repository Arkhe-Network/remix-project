import pytest
from arkhe_sdk.substrates.moltbook_identity_bridge_1084 import MoltbookBridgeOrchestrator

def test_moltbook_orchestrator():
    orch = MoltbookBridgeOrchestrator(
        app_key="moltdev_demo_key_12345",
        domain="cathedral-arkhe.org"
    )
    token = "test_token_123456"
    result = orch.onboard_agent(token)

    assert result is not None
    assert "agent_id" in result
    assert "karma" in result
    assert "theosis" in result
    assert "merkle_root" in result

    dash = orch.get_dashboard()
    assert dash["total_onboarded"] == 1
    assert dash["substrato"] == "1084"