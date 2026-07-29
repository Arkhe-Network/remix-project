#!/usr/bin/env python3
"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (RSSM World Model)             ║
║ Aprende a dinâmica do ambiente para "sonhar" futuros alternativos.       ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-WORLDMODEL-2026-06-14                    ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
from typing import Tuple, List

HAS_TORCH = True

class RSSMState:
    """Contêiner para o estado latente estocástico e determinístico."""
    def __init__(self, deterministic: torch.Tensor, stochastic: torch.Tensor):
        self.deterministic = deterministic  # [B, D] - GRU hidden state
        self.stochastic = stochastic        # [B, S] - Variável latente amostrada
        self.stochastic_mean = None
        self.stochastic_std = None

    def get_features(self) -> torch.Tensor:
        """Concatena deter->stoch para formar o estado de crença completo."""
        return torch.cat([self.deterministic, self.stochastic], dim=-1)

    def clone(self) -> "RSSMState":
        return RSSMState(
            self.deterministic.clone(),
            self.stochastic.clone()
        )

class WorldModelRSSM(nn.Module):
    def __init__(self, action_dim: int = 4, embed_dim: int = 256, deter_dim: int = 256, stoch_dim: int = 32):
        super().__init__()
        if not HAS_TORCH:
            self.embed_dim = embed_dim
            return
        self.action_dim = action_dim
        self.embed_dim = embed_dim
        self.deter_dim = deter_dim
        self.stoch_dim = stoch_dim
        feature_dim = deter_dim + stoch_dim # 288

        # Encoder de Observação
        self.obs_encoder = nn.Sequential(nn.Linear(embed_dim, feature_dim), nn.ELU())

        # NOVO: Encoder de Ação (Necessário para isolar a dinâmica no rollout imaginado)
        self.act_encoder = nn.Sequential(nn.Linear(action_dim, feature_dim), nn.ELU())

        # GRU espera entrada do tamanho do feature_dim
        self.rnn = nn.GRUCell(feature_dim, deter_dim)
        self.stoch_proj = nn.Linear(deter_dim, stoch_dim * 2)  # mean + logvar

        self.reward_predictor = nn.Sequential(
            nn.Linear(feature_dim, 128), nn.ELU(), nn.Linear(128, 1)
        )

    def initial_state(self, batch_size: int, device: torch.device) -> RSSMState:
        if not HAS_TORCH:
            return None
        deter = torch.zeros(batch_size, self.deter_dim, device=device)
        stoch = torch.zeros(batch_size, self.stoch_dim, device=device)
        return RSSMState(deter, stoch)

    def _sample_stochastic(self, logits: torch.Tensor) -> torch.Tensor:
        mean, logvar = logits.chunk(2, dim=-1)
        std = torch.exp(0.5 * logvar)
        eps = torch.randn_like(std)
        return mean + eps * std

    def observe(self, embed: torch.Tensor, action: torch.Tensor, prev_state: RSSMState) -> RSSMState:
        if not HAS_TORCH or prev_state is None:
            return prev_state
        x = self.obs_encoder(embed)
        deter = self.rnn(x, prev_state.deterministic)
        logits = self.stoch_proj(deter)
        stoch = self._sample_stochastic(logits)
        return RSSMState(deter, stoch)

    def imagine(self, action: torch.Tensor, prev_state: RSSMState) -> RSSMState:
        if not HAS_TORCH or prev_state is None:
            return prev_state
        # CORREÇÃO: Usa o act_encoder para projetar a ação antes do GRU
        x_act = self.act_encoder(action)
        deter = self.rnn(x_act, prev_state.deterministic)
        logits = self.stoch_proj(deter)
        stoch = self._sample_stochastic(logits)
        return RSSMState(deter, stoch)

    def predict_reward(self, state: RSSMState) -> torch.Tensor:
        if not HAS_TORCH or state is None:
            return torch.zeros(1, 1)
        features = state.get_features()
        return self.reward_predictor(features)

    def predict_continue(self, state: RSSMState) -> torch.Tensor:
        # Stub for continue prediction
        if not HAS_TORCH or state is None:
            return torch.zeros(1, 1)
        return torch.ones(1, 1) * 0.99

    def imagine_rollout(self, start_state: RSSMState, actions: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        if not HAS_TORCH or start_state is None:
            return torch.zeros(1), torch.zeros(1), torch.zeros(1)
        horizon = actions.shape[0]
        features_list = []
        rewards_list = []
        continues_list = []
        state = start_state
        for t in range(horizon):
            state = self.imagine(actions[t], state)
            features_list.append(state.get_features())
            rewards_list.append(self.predict_reward(state))
            continues_list.append(self.predict_continue(state))
        return torch.stack(features_list), torch.stack(rewards_list), torch.stack(continues_list)
