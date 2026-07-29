import pytest
from cathedral_moe_centum import CathedralMoECentum, QPURoutingRequest

def test_qpu_routing_simulation():
    centum = CathedralMoECentum()
    req = QPURoutingRequest(
        token_id="tok_test",
        context_vector=[0.5, 0.5],
        available_experts=["e1", "e2", "e3"]
    )

    res = centum.simulate_qpu_routing(req, top_k=2)

    assert len(res.selected_experts) == 2
    assert res.confidence >= 0.85 and res.confidence <= 0.99
    assert res.quantum_latency_ms >= 5.0 and res.quantum_latency_ms <= 15.0

    # Check if experts are in cache
    for e in res.selected_experts:
        assert e in centum.active_experts_cache

def test_qpu_routing_empty():
    centum = CathedralMoECentum()
    req = QPURoutingRequest(
        token_id="tok_test",
        context_vector=[],
        available_experts=[]
    )
    res = centum.simulate_qpu_routing(req)
    assert len(res.selected_experts) == 0
    assert res.confidence == 0.0

def test_estimate_energy():
    centum = CathedralMoECentum(total_parameters=100_000_000_000_000)

    # Test for 1000 tokens/s
    res = centum.estimate_energy(tokens_per_second=1000, active_parameters_ratio=0.01)

    # Expected:
    # active_params = 1T
    # e_gb300 = 2.0 J
    # e_network = 0.5 J
    # e_qpu = 0.1 J
    # power_gpu_w = 2.0 * 1000 = 2000 W = 2.0 kW
    # power_network_kw = 0.5 kW
    # power_qpu_kw = 0.1 kW
    # total_power = 2.6 kW

    assert res["active_parameters_trillion"] == 1.0
    assert abs(res["power_gpu_kw"] - 2.0) < 1e-5
    assert abs(res["power_network_kw"] - 0.5) < 1e-5
    assert abs(res["power_qpu_kw"] - 0.1) < 1e-5
    assert abs(res["total_power_kw"] - 2.6) < 1e-5

def test_prototype_expert_offloading():
    centum = CathedralMoECentum()

    # First, add to cache
    centum.active_experts_cache.add("expert_test_99")

    # Offload to Arweave
    res = centum.prototype_expert_offloading("expert_test_99", to_arweave=True)

    assert "expert_test_99" not in centum.active_experts_cache
    assert res["expert_id"] == "expert_test_99"
    assert res["status"] == "offloaded"
    assert res["storage_medium"] == "arweave"
    assert res["latency_to_wake_ms"] == "4500.0"
    assert res["tx_hash"].startswith("arweave_tx_")

def test_prototype_expert_offloading_ceph():
    centum = CathedralMoECentum()
    res = centum.prototype_expert_offloading("expert_fast_1", to_arweave=False)

    assert res["storage_medium"] == "ceph"
    assert res["latency_to_wake_ms"] == "150.0"
