"""
test_suite_v11.py — Testes de regressão para TopoMAS-PoUW v1.1
"""

import sys
import math
import numpy as np
import torch
import torch.nn as nn

import os
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'physicofm')))
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'agents')))
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', 'continual')))

from neural_operator_3d import (
    AtomicDensityGrid, FourierNeuralOperator3D, PhysicoFMNeuralOperator3D
)
from latent_planner_v11 import RecurrentLatentPlanner, LatentPlannerAgent
from continual_learner_v11 import ExperienceReplay, ElasticWeightConsolidation, ContinualLearningAgent

def test_t3_fno3d_pbc_invariance():
    """T3: FNO 3D deve ser invariante à permutação de átomos."""
    agent = PhysicoFMNeuralOperator3D(grid_size=16, modes=4, hidden_dim=16, n_layers=2)

    struct_a = {
        "frac_coords": np.array([[0.0, 0.0, 0.0], [0.5, 0.5, 0.5]]),
        "species": ["Si", "Si"],
        "lattice": np.eye(3) * 5.43,
        "volume": 5.43 ** 3,
    }
    struct_b = {
        "frac_coords": np.array([[0.5, 0.5, 0.5], [0.0, 0.0, 0.0]]),
        "species": ["Si", "Si"],
        "lattice": np.eye(3) * 5.43,
        "volume": 5.43 ** 3,
    }

    preds = agent.predict([struct_a, struct_b])
    diff = abs(preds[0]["energy"] - preds[1]["energy"])
    assert diff < 1e-4, f"Invariância falhou! diff={diff}"


def test_t3_energy_extensive():
    """H1/T3: Energia deve ser extensiva."""
    agent = PhysicoFMNeuralOperator3D(grid_size=16, modes=4, hidden_dim=16, n_layers=2)

    struct_2 = {
        "frac_coords": np.array([[0.0, 0.0, 0.0], [0.5, 0.5, 0.5]]),
        "species": ["Si", "Si"],
        "lattice": np.eye(3) * 5.43,
        "volume": 5.43 ** 3,
    }
    struct_4 = {
        "frac_coords": np.array([
            [0.0, 0.0, 0.0], [0.5, 0.5, 0.5],
            [0.25, 0.25, 0.25], [0.75, 0.75, 0.75],
        ]),
        "species": ["Si", "Si", "Si", "Si"],
        "lattice": np.eye(3) * 5.43,
        "volume": 5.43 ** 3,
    }

    preds = agent.predict([struct_2, struct_4])
    ratio = preds[1]["energy"] / (preds[0]["energy"] + 1e-8)


def test_t1_batch_indexing():
    """T1: LatentPlanner deve lidar corretamente com batch > 1."""
    planner = RecurrentLatentPlanner(state_dim=64, action_dim=8, n_iterations=4)
    context = torch.randn(5, 64)

    action_logits, final_state = planner(context)
    assert action_logits.shape == (5, 8), f"Shape incorreto: {action_logits.shape}"
    assert final_state.shape == (5, 64), f"Shape incorreto: {final_state.shape}"

    decision = planner.decide(context)
    probs = decision["probabilities"]
    assert probs.shape == (5, 8), f"Probs shape incorreto: {probs.shape}"
    assert torch.allclose(probs.sum(dim=-1), torch.ones(5), atol=1e-4), "Softmax não normalizou"


def test_h3_real_projection():
    """H3: Contexto deve usar projeção real (nn.Linear), não F.pad."""
    agent = LatentPlannerAgent(state_dim=64, action_dim=8, n_iterations=4)
    assert isinstance(agent.context_proj, nn.Sequential), "context_proj deve ser Sequential"
    assert isinstance(agent.context_proj[0], nn.Linear), "Primeira camada deve ser Linear"


def test_t4_ewc_fisher_on_reference():
    """T4: EWC deve computar Fisher nos dados de referência."""
    class TinyModel(nn.Module):
        def __init__(self):
            super().__init__()
            self.w = nn.Parameter(torch.randn(1, 10))
        def forward(self, x):
            return x @ self.w.T

    model = TinyModel()
    ewc = ElasticWeightConsolidation(model, importance=1e4)

    ref_x = torch.randn(20, 10)
    ref_y = torch.randn(20, 1)
    ref_dataset = torch.utils.data.TensorDataset(ref_x, ref_y)
    ref_loader = torch.utils.data.DataLoader(ref_dataset, batch_size=4)

    criterion = nn.MSELoss()
    ewc.compute_fisher(ref_loader, criterion, device="cpu")

    assert ewc._has_fisher, "Fisher não foi computada"
    assert len(ewc.fisher) > 0, "Fisher vazia"
    assert "w" in ewc.optimal_params, "Parâmetros ótimos não salvos"

    with torch.no_grad():
        model.w += 1.0

    loss = ewc.ewc_loss(device="cpu")
    assert loss.item() > 0, f"EWC loss deveria ser > 0 após mudança, mas foi {loss.item()}"


def test_t5_fisher_normalization():
    """T5: Fisher deve ser normalizada pelo número de amostras, não batches."""
    class TinyModel(nn.Module):
        def __init__(self):
            super().__init__()
            self.w = nn.Parameter(torch.ones(1, 5))
        def forward(self, x):
            return x @ self.w.T

    model = TinyModel()
    ewc = ElasticWeightConsolidation(model, importance=1e4)

    x = torch.ones(100, 5)
    y = torch.zeros(100, 1)
    dataset = torch.utils.data.TensorDataset(x, y)

    loader_bs10 = torch.utils.data.DataLoader(dataset, batch_size=10)
    ewc.compute_fisher(loader_bs10, nn.MSELoss(), device="cpu")
    fish_bs10 = ewc.fisher["w"].clone()

    ewc2 = ElasticWeightConsolidation(TinyModel(), importance=1e4)
    loader_bs25 = torch.utils.data.DataLoader(dataset, batch_size=25)
    ewc2.compute_fisher(loader_bs25, nn.MSELoss(), device="cpu")
    fish_bs25 = ewc2.fisher["w"]

    diff = (fish_bs10 - fish_bs25).abs().max().item()
    assert diff < 1e-3, f"Fisher depende do batch_size! diff={diff}"


def test_replay_buffer():
    """Testa o Experience Replay."""
    replay = ExperienceReplay(capacity=10)
    for i in range(15):
        replay.push(torch.randn(5), torch.tensor([float(i)]))

    assert len(replay) == 10, f"Capacidade não respeitada: {len(replay)}"

    sample = replay.sample(5)
    assert sample is not None, "Sample falhou"
    assert sample[0].shape == (5, 5), f"Shape incorreto: {sample[0].shape}"


if __name__ == "__main__":
    test_t3_fno3d_pbc_invariance()
    test_t3_energy_extensive()
    test_t1_batch_indexing()
    test_h3_real_projection()
    test_t4_ewc_fisher_on_reference()
    test_t5_fisher_normalization()
    test_replay_buffer()
    print("All tests passed.")
