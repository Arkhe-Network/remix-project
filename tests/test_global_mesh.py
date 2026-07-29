import pytest
import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../arkhe_sdk')))

def test_imports():
    from bridge_nostr_tor_ipfs import NostrTorIpfsBridge
    from nostr.nostr_relay import CathedralNostrRelay
    from tor.tor_service import TorMeshNode
    from ipfs.ipfs_backbone import IPFSBackbone
    from enterprise_mind import EnterpriseMind
    from self_reflexive_cathedral import SelfReflexiveCathedral
    assert True
