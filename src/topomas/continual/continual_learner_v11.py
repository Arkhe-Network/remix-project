import torch
import torch.nn as nn
from collections import defaultdict
from typing import Dict, Any, Callable

class ExperienceReplay:
    def __init__(self, capacity: int = 1000):
        self.capacity = capacity
        self.buffer = []
        self.position = 0

    def push(self, state, action):
        if len(self.buffer) < self.capacity:
            self.buffer.append(None)
        self.buffer[self.position] = (state, action)
        self.position = (self.position + 1) % self.capacity

    def sample(self, batch_size: int):
        import random
        if not self.buffer:
            return None
        batch = random.sample(self.buffer, min(batch_size, len(self.buffer)))
        states, actions = zip(*batch)
        return torch.stack(states), torch.stack(actions)

    def __len__(self):
        return len(self.buffer)

class ElasticWeightConsolidation:
    def __init__(self, model: nn.Module, importance: float = 1e4):
        self.model = model
        self.importance = importance
        self.fisher = defaultdict(float)
        self.optimal_params = {n: p.clone() for n, p in model.named_parameters() if p.requires_grad}
        self._has_fisher = False

    def compute_fisher(self, reference_loader, criterion: Callable[[torch.Tensor, torch.Tensor], torch.Tensor], device: str = "cpu") -> Dict[str, torch.Tensor]:
        self.model.eval()
        fisher_new = defaultdict(float)

        for inputs, targets in reference_loader:
            inputs, targets = inputs.to(device), targets.to(device)
            outputs = self.model(inputs)
            loss = criterion(outputs, targets)

            self.model.zero_grad()
            loss.backward(retain_graph=False)

            batch_size_actual = inputs.size(0)
            for name, param in self.model.named_parameters():
                if param.grad is None or not param.requires_grad:
                    continue
                # E[g_i²] ≈ B * E[(grad_mean)²]  (correção por amostras)
                grad_sq = param.grad.detach().clone() ** 2 * batch_size_actual
                if name not in fisher_new:
                    fisher_new[name] = grad_sq
                else:
                    fisher_new[name] += grad_sq

        n_samples = len(reference_loader.dataset)
        for name in fisher_new:
            self.fisher[name] = fisher_new[name] / max(1, n_samples)

        self.optimal_params = {n: p.clone() for n, p in self.model.named_parameters() if p.requires_grad}
        self._has_fisher = True
        return self.fisher

    def ewc_loss(self, device: str = "cpu") -> torch.Tensor:
        loss = torch.tensor(0.0, device=device)
        if not self._has_fisher:
            return loss

        for name, param in self.model.named_parameters():
            if name in self.fisher and param.requires_grad:
                loss += (self.fisher[name].to(device) * (param - self.optimal_params[name].to(device)) ** 2).sum()
        return self.importance * loss

class ContinualLearningAgent:
    def __init__(self, model, importance=1e4, replay_capacity=5000):
        self.ewc = ElasticWeightConsolidation(model, importance)
        self.replay = ExperienceReplay(replay_capacity)
