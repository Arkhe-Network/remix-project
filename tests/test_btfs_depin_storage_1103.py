import json
import base64
from unittest.mock import patch, MagicMock
from arkhe_sdk.substrates.btfs_depin_storage_1103 import BTFSBridge, BTFSIntegrationOrchestrator

def test_btfs_integration_orchestrator_initialization():
    orchestrator = BTFSIntegrationOrchestrator(mode="test")
    assert orchestrator.mode == "test"
    assert orchestrator.seal == "BTFS-CATHEDRAL-1103-v1.0.0-2026-06-12"

def test_get_manifesto():
    orchestrator = BTFSIntegrationOrchestrator(mode="test")
    manifesto = orchestrator.get_manifesto()
    assert "BTFS-CATHEDRAL-1103-v1.0.0-2026-06-12" in manifesto
    assert "Substrato 1103" in manifesto

@patch("subprocess.run")
def test_store_success(mock_run):
    mock_result = MagicMock()
    mock_result.returncode = 0
    mock_result.stdout = '{"cid": "QmTestCID123", "encrypted": false}'
    mock_run.return_value = mock_result

    bridge = BTFSBridge()
    cid = bridge.store(b"hello world", encrypt=False)

    assert cid == "QmTestCID123"
    mock_run.assert_called_once()
    args, kwargs = mock_run.call_args
    assert args[0] == ["qrexec-client-vm", "btfs-gateway", "cathedral.BTFSStore"]
    payload = json.loads(kwargs["input"].decode())
    assert payload["content_base64"] == base64.b64encode(b"hello world").decode()
    assert payload["encrypt"] is False

@patch("subprocess.run")
def test_retrieve_success(mock_run):
    mock_result = MagicMock()
    mock_result.returncode = 0
    # The command returns a base64 encoded string wrapped in json by the test mock,
    # Actually _call wraps non-json output as {"raw": proc.stdout} if json fails.
    # In BTFSBridge.retrieve, it decodes base64 string from result.get("raw")
    # Let's mock a JSON response for simplicity (although script in README returns base64 string without json)
    # Wait, if script returns pure base64: `base64 -w0 /tmp/btfs_download.bin`,
    # then json.loads() will fail and `_call` returns {"raw": proc.stdout}.
    b64_content = base64.b64encode(b"hello world").decode()
    mock_result.stdout = b64_content
    mock_run.return_value = mock_result

    bridge = BTFSBridge()
    content = bridge.retrieve("QmTestCID123")

    assert content == b"hello world"
    mock_run.assert_called_once()
    args, kwargs = mock_run.call_args
    assert args[0] == ["qrexec-client-vm", "btfs-gateway", "cathedral.BTFSRetrieve"]
    payload = json.loads(kwargs["input"].decode())
    assert payload["cid"] == "QmTestCID123"

@patch("subprocess.run")
def test_list_providers_success(mock_run):
    mock_result = MagicMock()
    mock_result.returncode = 0
    mock_result.stdout = '{"providers": ["peer1", "peer2"]}'
    mock_run.return_value = mock_result

    bridge = BTFSBridge()
    providers = bridge.list_providers("QmTestCID123")

    assert providers == ["peer1", "peer2"]
    mock_run.assert_called_once()
    args, kwargs = mock_run.call_args
    assert args[0] == ["qrexec-client-vm", "btfs-gateway", "cathedral.BTFSProviderList"]
    payload = json.loads(kwargs["input"].decode())
    assert payload["cid"] == "QmTestCID123"
