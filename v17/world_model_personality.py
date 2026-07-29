"""
Cathedral ARKHE v17.0 - RSSM with Personality (Estado Latente Moldável)
Adiciona um vetor de personalidade aprendível que modifica a tomada de decisão.
"""
import torch
import torch.nn as nn
import numpy as np

class RSSMWithPersonality(nn.Module):
    def __init__(self, deter_dim=256, stoch_dim=32, hidden_dim=256, action_dim=4):
        super().__init__()
        self.deter_dim = deter_dim
        self.stoch_dim = stoch_dim

        # Rede GRU simplificada
        self.gru = nn.GRUCell(deter_dim + action_dim, hidden_dim)
        self.fc_deter = nn.Linear(hidden_dim, deter_dim)
        self.fc_stoch = nn.Linear(hidden_dim, stoch_dim * 2) # Mean e LogVar

        # O NÚCLEO DA PERSONALIDADE: Um vetor que倾向于 certos estados estocásticos
        self.personality_bias = nn.Parameter(torch.zeros(stoch_dim))
        self.personality_scale = nn.Parameter(torch.ones(stoch_dim) * 0.1)

        self.init_state()

    def init_state(self, batch_size=1):
        self.deter_state = torch.zeros(batch_size, self.deter_dim)

    def forward(self, vision_features: torch.Tensor, action: torch.Tensor) -> torch.Tensor:
        # Concatena estado determinístico passad com ação
        x = torch.cat([self.deter_state, action], dim=-1)
        h = self.gru(x, self.deter_state)

        self.deter_state = torch.tanh(self.fc_deter(h))
        stoch_params = self.fc_stoch(h)
        mean, logvar = stoch_params.chunk(2, dim=-1)

        # Amostragem com reparameterization trick
        std = torch.exp(0.5 * logvar)
        eps = torch.randn_like(std)
        stoch_state = mean + eps * std

        # INJEÇÃO DE PERSONALIDADE: Desloca a distribuição estocástica
        # Isso faz com que o agente tenha uma tendência inata (ex: cauteloso, exploratório)
        stoch_state = stoch_state + (self.personality_bias * self.personality_scale)

        # Retorna estado concatenado (288 dims)
        return torch.cat([self.deter_state, stoch_state], dim=-1)

    def update_personality_from_reward(self, reward: float, lr: float = 0.001):
        """Ajusta a personalidade com base no feedback do ambiente (Reinforcement)."""
        with torch.no_grad():
            # Se a recompensa foi boa, reforça a personalidade atual
            # Se foi ruim, afasta levemente
            gradient = lr * reward
            self.personality_bias.add_(gradient * self.personality_bias.data.sign())
