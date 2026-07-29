#!/usr/bin/env python3
"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (Orchestrator)                 ║
║ Loop Principal: Percepção -> Cognição -> Segurança -> Ação -> Imaginação ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-ORCHESTRATOR-2026-06-14                  ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝
"""

import asyncio
import logging
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any

import torch
import numpy as np

from .vision import VisionEncoder
from .ontology import SymbolicSafetyEngine
from .world_model import WorldModelRSSM, RSSMState
from .rl_agent import SACAgent
from .rust_bridge import RustBridgeStub, HNSWQuery

logger = logging.getLogger("cathedral.v16.orchestrator")


@dataclass
class PerceptionFrame:
    """Frame de percepção processado pelo pipeline."""
    raw_image: np.ndarray
    cls_embedding: torch.Tensor
    patch_embeddings: torch.Tensor
    detected_entities: List[Dict]
    timestamp: float = field(default_factory=time.time)


@dataclass
class ActionProposal:
    """Proposta de ação do agente RL."""
    action: torch.Tensor
    value_estimate: float
    entropy: float
    safety_approved: bool = False
    symbolic_violations: List[str] = field(default_factory=list)


@dataclass
class ImaginedTrajectory:
    """Trajetória imaginada pelo World Model."""
    states: List[torch.Tensor]
    actions: List[torch.Tensor]
    rewards: List[float]
    cumulative_return: float
    safety_score: float
    horizon: int


class CathedralOrchestrator:
    """
    Orchestrator do Substrato 3000: integra visão, ontologia, world model e RL.
    Loop: Percepção -> ViT -> Ontologia -> RL -> Segurança -> Ação -> Imaginação
    """
    def __init__(self,
                 action_dim: int = 4,
                 embed_dim: int = 256,
                 deter_dim: int = 256,
                 stoch_dim: int = 32,
                 device: str = "cpu",
                 imagine_horizon: int = 15,
                 safety_threshold: float = 0.8):

        self.device = torch.device(device)
        self.action_dim = action_dim
        self.embed_dim = embed_dim
        self.imagine_horizon = imagine_horizon
        self.safety_threshold = safety_threshold

        # === Módulos ===
        self.vision = VisionEncoder(embed_dim=embed_dim, device=device)
        self.ontology = SymbolicSafetyEngine()
        self.world_model = WorldModelRSSM(
            action_dim=action_dim,
            embed_dim=embed_dim,
            deter_dim=deter_dim,
            stoch_dim=stoch_dim,
        ).to(self.device)

        feature_dim = deter_dim + stoch_dim
        self.rl_agent = SACAgent(
            state_dim=feature_dim,
            action_dim=action_dim,
            device=device,
        )

        self.rust_bridge = RustBridgeStub()

        # === Estado ===
        self.current_rssm_state: Optional[RSSMState] = None
        self.current_perception: Optional[PerceptionFrame] = None
        self._episode_count = 0
        self._step_count = 0
        self._cycle_times: deque = deque(maxlen=100)
        self._safety_blocks = 0
        self._imagination_stats: deque = deque(maxlen=100)

        logger.info("CathedralOrchestrator v16.0.0 inicializado (device=%s)", device)

    async def perceive(self, raw_image: np.ndarray) -> PerceptionFrame:
        """
        Etapa 1: Percepção Visual.
        Processa imagem bruta via ViT e extrai entidades.
        """
        start = time.monotonic()

        # Normaliza imagem para ViT (ImageNet stats)
        img_tensor = self._preprocess_image(raw_image)

        # Extrai embeddings
        cls_emb, patch_emb = self.vision.extract_for_cognition(img_tensor)

        # Detecção simbólica simplificada (placeholder para detector real)
        entities = self._detect_entities_from_patches(patch_emb)

        # Atualiza ontologia
        self.ontology.update_state_from_perception(entities)

        frame = PerceptionFrame(
            raw_image=raw_image,
            cls_embedding=cls_emb,
            patch_embeddings=patch_emb,
            detected_entities=entities,
        )
        self.current_perception = frame

        # Atualiza RSSM com observação real
        if self.current_rssm_state is None:
            self.current_rssm_state = self.world_model.initial_state(
                batch_size=1, device=self.device
            )

        # Ação dummy para primeira observação (será sobrescrita)
        dummy_action = torch.zeros(1, self.action_dim, device=self.device)
        self.current_rssm_state = self.world_model.observe(
            cls_emb, dummy_action, self.current_rssm_state
        )

        latency = (time.monotonic() - start) * 1000
        logger.debug("Percepção: %.2fms", latency)
        return frame

    def _preprocess_image(self, img: np.ndarray) -> torch.Tensor:
        """Pré-processa imagem numpy para tensor ViT."""
        # Assume img é [H, W, C] uint8 -> [B, C, H, W] float32 normalizado
        if img.dtype == np.uint8:
            img = img.astype(np.float32) / 255.0

        # Resize para 224x224 se necessário
        if img.shape[:2] != (224, 224):
            # Simplified resize — em produção usar torchvision.transforms
            img = self._simple_resize(img, (224, 224))

        # ImageNet normalization
        mean = np.array([0.485, 0.456, 0.406])
        std = np.array([0.229, 0.224, 0.225])
        img = (img - mean) / std

        # HWC -> CHW
        img = np.transpose(img, (2, 0, 1))
        return torch.from_numpy(img).unsqueeze(0).float()

    def _simple_resize(self, img: np.ndarray, target: Tuple[int, int]) -> np.ndarray:
        """Resize simplificado via numpy."""
        h, w = target
        return np.array([
            [img[int(i * img.shape[0] / h), int(j * img.shape[1] / w)]
             for j in range(w)]
            for i in range(h)
        ])

    def _detect_entities_from_patches(self, patch_emb: torch.Tensor) -> List[Dict]:
        """Placeholder: extrai entidades dos patch embeddings."""
        # Em produção: detector de objetos (YOLO/RT-DETR) ou clustering nos patches
        B, N, D = patch_emb.shape
        # Simula 3 entidades detectadas
        return [
            {"id": f"obj_{i}", "type": "SpatialEntity",
             "velocity": float(torch.randn(1).item() * 2),
             "fragile": i % 2 == 0,
             "patch_indices": list(range(i * N // 3, (i + 1) * N // 3))}
            for i in range(3)
        ]

    async def propose_action(self, deterministic: bool = False) -> ActionProposal:
        """
        Etapa 2: Proposta de Ação via SAC.
        O agente propõe ação baseada no estado latente RSSM.
        """
        if self.current_rssm_state is None:
            raise RuntimeError("Percepção não processada. Chame perceive() primeiro.")

        start = time.monotonic()

        features = self.current_rssm_state.get_features()
        action = self.rl_agent.select_action(features.squeeze(0).cpu().detach().numpy(), deterministic)

        # Estimativa de valor (Q-value)
        with torch.no_grad():
            q1, q2 = self.rl_agent.critic(features, action.unsqueeze(0).to(self.device))
            value = torch.min(q1, q2).item()

        # Entropia da política
        with torch.no_grad():
            _, log_std = self.rl_agent.actor(features)
            entropy = (0.5 * torch.log(2 * torch.pi * torch.e * log_std.exp().pow(2))).sum().item()

        proposal = ActionProposal(
            action=action,
            value_estimate=value,
            entropy=entropy,
        )

        latency = (time.monotonic() - start) * 1000
        logger.debug("Proposta de ação: %.2fms", latency)
        return proposal

    async def validate_safety(self, proposal: ActionProposal,
                              agent_id: str = "cathedral_agent") -> ActionProposal:
        """
        Etapa 3: Validação Simbólica de Segurança.
        Submete a ação proposta ao motor OWL+SWRL+Z3.
        """
        start = time.monotonic()

        # Extrai entidades alvo (simplificado: primeiro objeto detectado)
        target_id = "obj_0"  # Default
        if self.current_perception and self.current_perception.detected_entities:
            target_id = self.current_perception.detected_entities[0]["id"]

        # Força da ação como proxy para magnitude
        force = float(torch.norm(proposal.action).item())

        # Validação Z3
        is_safe = self.ontology.validate_action_safety(
            agent_id=agent_id,
            action_name="proposed_action",
            target_id=target_id,
            force=force,
        )

        proposal.safety_approved = is_safe
        if not is_safe:
            self._safety_blocks += 1
            proposal.symbolic_violations.append(f"force={force:.2f} exceeds safe threshold for {target_id}")
            logger.warning("Ação BLOQUEADA por violação simbólica: %s", proposal.symbolic_violations)

        latency = (time.monotonic() - start) * 1000
        logger.debug("Validação de segurança: %.2fms", latency)
        return proposal

    async def imagine(self, proposal: ActionProposal) -> ImaginedTrajectory:
        """
        Etapa 4: Imaginação via World Model.
        "Sonha" k passos no futuro para avaliar consequências da ação.
        """
        start = time.monotonic()

        if self.current_rssm_state is None:
            raise RuntimeError("Percepção não processada.")

        # Gera sequência de ações imaginadas
        actions = []
        with torch.no_grad():
            state = self.current_rssm_state.clone()
            for _ in range(self.imagine_horizon):
                action, _ = self.rl_agent.actor.sample(state.get_features())
                actions.append(action)

        actions_tensor = torch.stack(actions)  # [H, 1, action_dim]

        # Rola imaginação
        features, rewards, continues = self.world_model.imagine_rollout(
            start_state=state,
            actions=actions_tensor,
        )

        # Calcula retorno acumulado com desconto
        gamma = self.rl_agent.gamma
        cumulative = 0.0
        returns = []
        for r in reversed(rewards.squeeze(-1).cpu().tolist()):
            # Handle possible batch dimension
            if isinstance(r, list):
                r_val = r[0]
            else:
                r_val = r
            cumulative = r_val + gamma * cumulative
            returns.insert(0, cumulative)

        # Pontuação de segurança da trajetória imaginada
        safety_score = float(torch.mean(continues).item())

        trajectory = ImaginedTrajectory(
            states=[f.squeeze(0) for f in features],
            actions=[a.squeeze(0) for a in actions],
            rewards=rewards.squeeze(-1).cpu().tolist(),
            cumulative_return=returns[0] if returns else 0.0,
            safety_score=safety_score,
            horizon=self.imagine_horizon,
        )

        self._imagination_stats.append({
            "return": trajectory.cumulative_return,
            "safety": safety_score,
            "latency_ms": (time.monotonic() - start) * 1000,
        })

        latency = (time.monotonic() - start) * 1000
        logger.debug("Imaginação (%d passos): %.2fms", self.imagine_horizon, latency)
        return trajectory

    async def execute_action(self, proposal: ActionProposal) -> Dict:
        """
        Etapa 5: Execução da Ação (ou fallback seguro).
        Se a ação foi bloqueada pela segurança simbólica, executa ação nula ou exploratória.
        """
        if not proposal.safety_approved:
            # Ação segura de fallback: pequena perturbação aleatória
            safe_action = torch.randn(self.action_dim) * 0.1
            logger.info("Executando ação de fallback segura (bloqueio simbólico)")
            return {
                "action": safe_action.tolist(),
                "safety_approved": False,
                "fallback": True,
                "value": proposal.value_estimate,
            }

        return {
            "action": proposal.action.tolist(),
            "safety_approved": True,
            "fallback": False,
            "value": proposal.value_estimate,
            "entropy": proposal.entropy,
        }

    async def learn_from_experience(self, reward: float, next_image: Optional[np.ndarray] = None):
        """
        Etapa 6: Aprendizado.
        Atualiza World Model e RL Agent com transição real.
        """
        if self.current_rssm_state is None or self.current_perception is None:
            return

        # Embedding para HNSW
        embedding = self.current_rssm_state.get_features().squeeze(0).cpu().tolist()

        # Próximo estado (se disponível)
        if next_image is not None:
            next_frame = await self.perceive(next_image)
            next_state = self.current_rssm_state.get_features().squeeze(0).cpu().detach().numpy()
        else:
            next_state = self.current_rssm_state.get_features().squeeze(0).cpu().detach().numpy()

        # Armazena transição
        current_state = self.current_rssm_state.get_features().squeeze(0).cpu().detach().numpy()
        # Recupera última ação proposta
        # (Em loop real, a ação executada seria armazenada)
        action = torch.zeros(self.action_dim).numpy()  # Placeholder

        self.rl_agent.store_transition(
            state=current_state,
            action=action,
            reward=reward,
            next_state=next_state,
            done=False,
            embedding=embedding,
            episode_id=self._episode_count,
        )

        # Atualiza redes
        if self._step_count % 4 == 0:  # Update every 4 steps
            stats = self.rl_agent.update(batch_size=64)
            logger.debug("RL update: %s", stats)

        self._step_count += 1

    async def run_cycle(self, raw_image: np.ndarray, reward: float = 0.0) -> Dict:
        """
        Executa um ciclo completo: Percepção -> Ação -> Segurança -> Imaginação -> Execução -> Aprendizado.
        """
        cycle_start = time.monotonic()

        # 1. Percepção
        await self.perceive(raw_image)

        # 2. Proposta de ação
        proposal = await self.propose_action()

        # 3. Validação de segurança
        proposal = await self.validate_safety(proposal)

        # 4. Imaginação
        trajectory = await self.imagine(proposal)

        # 5. Execução
        execution = await self.execute_action(proposal)

        # 6. Aprendizado
        await self.learn_from_experience(reward)

        cycle_time = (time.monotonic() - cycle_start) * 1000
        self._cycle_times.append(cycle_time)

        return {
            "cycle_time_ms": cycle_time,
            "action": execution,
            "safety_approved": proposal.safety_approved,
            "imagined_return": trajectory.cumulative_return,
            "imagined_safety": trajectory.safety_score,
            "step": self._step_count,
            "episode": self._episode_count,
        }

    def get_stats(self) -> Dict:
        """Estatísticas do orchestrator."""
        avg_cycle = sum(self._cycle_times) / len(self._cycle_times) if self._cycle_times else 0
        avg_imagination = sum(s["return"] for s in self._imagination_stats) / len(self._imagination_stats) if self._imagination_stats else 0

        return {
            "version": "v16.0.0",
            "substrato": 3000,
            "episodes": self._episode_count,
            "steps": self._step_count,
            "avg_cycle_time_ms": round(avg_cycle, 2),
            "safety_blocks": self._safety_blocks,
            "avg_imagined_return": round(avg_imagination, 4),
            "buffer_size": self.rl_agent.replay_buffer.get_stats()["size"],
            "ontology": self.ontology.get_ontology_stats(),
            "rl_stats": self.rl_agent.get_stats(),
        }
