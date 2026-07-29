#!/usr/bin/env python3
"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (RL Agent: SAC + Episodic)    ║
║ Soft Actor-Critic com treinamento dentro do World Model (Dreamer-style) ║
║ e Episodic Prioritized Replay Buffer via HNSW.                            ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-RLAGENT-2026-06-14                       ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
import torch.optim as optim
import math
import random
import time
from collections import deque, defaultdict
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple, Any


@dataclass
class Transition:
    """Uma transição (s, a, r, s', done) com embedding para HNSW."""
    state: torch.Tensor
    action: torch.Tensor
    reward: float
    next_state: torch.Tensor
    done: bool
    embedding: List[float]          # Para indexação HNSW
    timestamp: float = 0.0
    td_error: float = 1.0             # Para Prioritized Experience Replay
    episode_id: int = 0


class HNSWReplayBuffer:
    """
    Replay Buffer com indexação HNSW para busca eficiente de episódios similares.
    Prioridade baseada em TD-error + similaridade episódica.
    """
    def __init__(self, capacity: int = 100000, dim: int = 256, m: int = 16,
                 ef_construction: int = 200, alpha: float = 0.6):
        self.capacity = capacity
        self.dim = dim
        self.m = m
        self.ef_construction = ef_construction
        self.alpha = alpha  # Prioritization exponent

        self._transitions: Dict[int, Transition] = {}
        self._next_id = 0
        self._hnsw_nodes: Dict[int, Dict] = {}
        self._entry_point: Optional[int] = None
        self._max_level = 0
        self._episode_map: Dict[int, List[int]] = defaultdict(list)

    def _random_level(self) -> int:
        level = 0
        while random.random() < 0.5 and level < 16:
            level += 1
        return level

    def _distance(self, a: List[float], b: List[float]) -> float:
        return math.sqrt(sum((x - y) ** 2 for x, y in zip(a, b)))

    def _hnsw_add(self, tid: int, embedding: List[float]):
        level = self._random_level()
        node = {
            "id": tid,
            "vector": embedding,
            "level": level,
            "neighbors": {l: [] for l in range(level + 1)},
        }
        self._hnsw_nodes[tid] = node

        if self._entry_point is None:
            self._entry_point = tid
            self._max_level = level
            return

        curr = self._entry_point
        for l in range(self._max_level, -1, -1):
            if l > level:
                continue
            best = curr
            best_dist = self._distance(self._hnsw_nodes[best]["vector"], embedding)
            improved = True
            while improved:
                improved = False
                for nid in self._hnsw_nodes[best]["neighbors"].get(l, []):
                    d = self._distance(self._hnsw_nodes[nid]["vector"], embedding)
                    if d < best_dist:
                        best = nid
                        best_dist = d
                        improved = True
            curr = best

            candidates = []
            for nid in self._hnsw_nodes:
                if nid == tid:
                    continue
                if self._hnsw_nodes[nid]["level"] >= l:
                    d = self._distance(self._hnsw_nodes[nid]["vector"], embedding)
                    candidates.append((d, nid))
            candidates.sort()
            for _, nid in candidates[:self.m]:
                node["neighbors"][l].append(nid)
                self._hnsw_nodes[nid]["neighbors"][l].append(tid)

        if level > self._max_level:
            self._max_level = level
            self._entry_point = tid

    def _hnsw_search(self, embedding: List[float], k: int = 10, ef: int = 50) -> List[Tuple[int, float]]:
        if self._entry_point is None:
            return []

        curr = self._entry_point
        for l in range(self._max_level, -1, -1):
            best = curr
            best_dist = self._distance(self._hnsw_nodes[best]["vector"], embedding)
            improved = True
            while improved:
                improved = False
                for nid in self._hnsw_nodes[best]["neighbors"].get(l, []):
                    d = self._distance(self._hnsw_nodes[nid]["vector"], embedding)
                    if d < best_dist:
                        best = nid
                        best_dist = d
                        improved = True
            curr = best

        candidates = []
        visited = {curr}
        queue = [curr]
        while queue and len(candidates) < ef:
            nid = queue.pop(0)
            d = self._distance(self._hnsw_nodes[nid]["vector"], embedding)
            candidates.append((d, nid))
            for neighbor in self._hnsw_nodes[nid]["neighbors"].get(0, []):
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append(neighbor)

        candidates.sort()
        return [(nid, d) for d, nid in candidates[:k]]

    def add(self, transition: Transition):
        """Adiciona uma transição ao buffer com indexação HNSW."""
        if len(self._transitions) >= self.capacity:
            # Remove oldest
            oldest = min(self._transitions.keys())
            del self._transitions[oldest]
            if oldest in self._hnsw_nodes:
                del self._hnsw_nodes[oldest]

        tid = self._next_id
        self._next_id += 1
        transition.timestamp = time.time()
        self._transitions[tid] = transition
        self._episode_map[transition.episode_id].append(tid)
        self._hnsw_add(tid, transition.embedding)

    def sample(self, batch_size: int, beta: float = 0.4) -> Tuple[List[Transition], List[int], List[float]]:
        """
        Amostragem priorizada combinando TD-error + similaridade HNSW.
        Retorna: transições, índices, pesos de importância.
        """
        if not self._transitions:
            return [], [], []

        # Prioridade = (|TD-error| + epsilon)^alpha * (1 + episodic_bonus)
        priorities = []
        for tid, tr in self._transitions.items():
            td_priority = (abs(tr.td_error) + 1e-6) ** self.alpha
            # Episodic bonus: more recent = higher priority
            recency_bonus = 1.0 + 0.1 * (time.time() - tr.timestamp) / 3600
            priorities.append((tid, td_priority * recency_bonus))

        total = sum(p for _, p in priorities)
        probs = [p / total for _, p in priorities]

        # Sample
        sampled_indices = random.choices(range(len(priorities)), weights=probs, k=batch_size)
        tids = [priorities[i][0] for i in sampled_indices]

        transitions = [self._transitions[tid] for tid in tids]

        # Importance sampling weights
        weights = []
        N = len(self._transitions)
        for i in sampled_indices:
            prob = probs[i]
            weight = (N * prob) ** (-beta)
            weights.append(weight)

        # Normalize weights
        max_weight = max(weights) if weights else 1.0
        weights = [w / max_weight for w in weights]

        return transitions, tids, weights

    def update_td_errors(self, indices: List[int], td_errors: List[float]):
        """Atualiza TD-errors após treinamento."""
        for idx, td in zip(indices, td_errors):
            if idx in self._transitions:
                self._transitions[idx].td_error = td

    def retrieve_episodic(self, query_embedding: List[float], k: int = 5) -> List[Transition]:
        """Recupera transições similares usando HNSW."""
        results = self._hnsw_search(query_embedding, k=k)
        return [self._transitions[tid] for tid, _ in results if tid in self._transitions]

    def get_stats(self) -> Dict:
        return {
            "size": len(self._transitions),
            "capacity": self.capacity,
            "episodes": len(self._episode_map),
            "hnsw_nodes": len(self._hnsw_nodes),
        }


class Actor(nn.Module):
    """Actor SAC: política estocástica com reparametrization."""
    def __init__(self, state_dim: int, action_dim: int, hidden_dim: int = 256,
                 log_std_min: float = -20, log_std_max: float = 2):
        super().__init__()
        self.log_std_min = log_std_min
        self.log_std_max = log_std_max

        self.net = nn.Sequential(
            nn.Linear(state_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        )
        self.mean_layer = nn.Linear(hidden_dim, action_dim)
        self.log_std_layer = nn.Linear(hidden_dim, action_dim)

    def forward(self, state: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
        x = self.net(state)
        mean = self.mean_layer(x)
        log_std = self.log_std_layer(x)
        log_std = torch.clamp(log_std, self.log_std_min, self.log_std_max)
        return mean, log_std

    def sample(self, state: torch.Tensor, deterministic: bool = False) -> Tuple[torch.Tensor, torch.Tensor]:
        mean, log_std = self.forward(state)
        if deterministic:
            return torch.tanh(mean), torch.zeros_like(mean)

        std = log_std.exp()
        normal = torch.distributions.Normal(mean, std)
        x_t = normal.rsample()
        action = torch.tanh(x_t)

        # Log prob correction for tanh
        log_prob = normal.log_prob(x_t)
        log_prob -= torch.log(1 - action.pow(2) + 1e-6)
        log_prob = log_prob.sum(1, keepdim=True)

        return action, log_prob


class Critic(nn.Module):
    """Critic SAC: Q-network dupla para reduzir overestimation."""
    def __init__(self, state_dim: int, action_dim: int, hidden_dim: int = 256):
        super().__init__()
        self.q1 = nn.Sequential(
            nn.Linear(state_dim + action_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, 1)
        )
        self.q2 = nn.Sequential(
            nn.Linear(state_dim + action_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, 1)
        )

    def forward(self, state: torch.Tensor, action: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
        x = torch.cat([state, action], dim=-1)
        return self.q1(x), self.q2(x)


class SACAgent:
    """
    Soft Actor-Critic para treinamento dentro do World Model.
    Treina no espaço latente do RSSM, não na observação bruta.
    """
    def __init__(self, state_dim: int, action_dim: int,
                 lr: float = 3e-4, gamma: float = 0.99,
                 tau: float = 0.005, alpha: float = 0.2,
                 hidden_dim: int = 256, device: str = "cpu"):
        self.state_dim = state_dim
        self.action_dim = action_dim
        self.gamma = gamma
        self.tau = tau
        self.device = torch.device(device)

        # Networks
        self.actor = Actor(state_dim, action_dim, hidden_dim).to(self.device)
        self.critic = Critic(state_dim, action_dim, hidden_dim).to(self.device)
        self.critic_target = Critic(state_dim, action_dim, hidden_dim).to(self.device)
        self.critic_target.load_state_dict(self.critic.state_dict())

        # Entropy temperature (auto-tuned)
        self.log_alpha = torch.tensor([math.log(alpha)], requires_grad=True, device=self.device)
        self.target_entropy = -action_dim

        # Optimizers
        self.actor_optimizer = optim.Adam(self.actor.parameters(), lr=lr)
        self.critic_optimizer = optim.Adam(self.critic.parameters(), lr=lr)
        self.alpha_optimizer = optim.Adam([self.log_alpha], lr=lr)

        # Replay buffer
        self.replay_buffer = HNSWReplayBuffer(capacity=100000, dim=state_dim)

        self._update_count = 0

    def select_action(self, state: torch.Tensor, deterministic: bool = False) -> torch.Tensor:
        """Seleciona ação dado estado latente do RSSM."""
        state = torch.FloatTensor(state).to(self.device)
        if state.dim() == 1:
            state = state.unsqueeze(0)
        action, _ = self.actor.sample(state, deterministic)
        return action.squeeze(0).cpu().detach()

    def store_transition(self, state, action, reward, next_state, done,
                         embedding: List[float], episode_id: int = 0):
        """Armazena transição no replay buffer HNSW."""
        transition = Transition(
            state=torch.FloatTensor(state),
            action=torch.FloatTensor([action]) if isinstance(action, (int, float)) else torch.FloatTensor(action),
            reward=reward,
            next_state=torch.FloatTensor(next_state),
            done=done,
            embedding=embedding,
            episode_id=episode_id,
        )
        self.replay_buffer.add(transition)

    def update(self, batch_size: int = 256, imagined: bool = False) -> Dict:
        """
        Atualiza redes SAC. Se imagined=True, usa dados do World Model.
        """
        if len(self.replay_buffer._transitions) < batch_size:
            return {"status": "insufficient_data"}

        transitions, indices, weights = self.replay_buffer.sample(batch_size)

        states = torch.stack([t.state for t in transitions]).to(self.device)
        actions = torch.stack([t.action for t in transitions]).to(self.device)
        rewards = torch.FloatTensor([t.reward for t in transitions]).to(self.device).unsqueeze(1)
        next_states = torch.stack([t.next_state for t in transitions]).to(self.device)
        dones = torch.FloatTensor([t.done for t in transitions]).to(self.device).unsqueeze(1)
        weights_t = torch.FloatTensor(weights).to(self.device).unsqueeze(1)

        # --- Critic Update ---
        with torch.no_grad():
            next_actions, next_log_probs = self.actor.sample(next_states)
            q1_target, q2_target = self.critic_target(next_states, next_actions)
            q_target = torch.min(q1_target, q2_target) - self.log_alpha.exp() * next_log_probs
            q_target = rewards + (1 - dones) * self.gamma * q_target

        q1, q2 = self.critic(states, actions)
        critic_loss = (weights_t * (F.mse_loss(q1, q_target, reduction='none') +
                                     F.mse_loss(q2, q_target, reduction='none'))).mean()

        self.critic_optimizer.zero_grad()
        critic_loss.backward()
        self.critic_optimizer.step()

        # --- Actor Update ---
        new_actions, log_probs = self.actor.sample(states)
        q1_new, q2_new = self.critic(states, new_actions)
        q_new = torch.min(q1_new, q2_new)
        actor_loss = (self.log_alpha.exp().detach() * log_probs - q_new).mean()

        self.actor_optimizer.zero_grad()
        actor_loss.backward()
        self.actor_optimizer.step()

        # --- Alpha Update ---
        alpha_loss = -(self.log_alpha * (log_probs + self.target_entropy).detach()).mean()
        self.alpha_optimizer.zero_grad()
        alpha_loss.backward()
        self.alpha_optimizer.step()

        # --- Soft Update Target ---
        for param, target_param in zip(self.critic.parameters(), self.critic_target.parameters()):
            target_param.data.copy_(self.tau * param.data + (1 - self.tau) * target_param.data)

        # Update TD errors in buffer
        with torch.no_grad():
            td_errors = (q1 - q_target).abs().squeeze().cpu().tolist()
        self.replay_buffer.update_td_errors(indices, td_errors)

        self._update_count += 1

        return {
            "critic_loss": critic_loss.item(),
            "actor_loss": actor_loss.item(),
            "alpha": self.log_alpha.exp().item(),
            "q_mean": q1.mean().item(),
            "update_count": self._update_count,
            "mode": "imagined" if imagined else "real",
        }

    def get_stats(self) -> Dict:
        return {
            "actor_params": sum(p.numel() for p in self.actor.parameters()),
            "critic_params": sum(p.numel() for p in self.critic.parameters()),
            "alpha": self.log_alpha.exp().item(),
            "buffer": self.replay_buffer.get_stats(),
            "updates": self._update_count,
        }
