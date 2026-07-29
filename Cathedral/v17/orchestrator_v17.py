"""
Cathedral ARKHE v17.0 - Orquestrador Principal
Coordena Fast Brain, Slow Brain, Router, e Memória.
"""
import asyncio
import time
import logging
import numpy as np
from typing import Optional
from dataclasses import dataclass, field

from .fast_brain import FastBrain, FastBrainState
from .slow_brain import SlowBrainSGLang
from .config_loader import CathedralConfig

logger = logging.getLogger("cathedral.orchestrator")


@dataclass
class OrchestratorResult:
    action: np.ndarray
    confidence: float
    route: str              # "fast" ou "slow"
    safety_approved: bool
    slow_brain_reasoning: str = ""
    latency_ms: float = 0.0
    fast_brain_state: Optional[FastBrainState] = None


class Router:
    """Decide se usa Fast ou Slow Brain."""

    def __init__(self, config):
        self.config = config.router
        self.confidence_threshold = self.config["confidence_threshold"]
        self.deadlock_window = self.config["deadlock_window"]
        self.deadlock_threshold = self.config["deadlock_threshold"]
        self._action_history = []

    def decide(
        self,
        fast_state: FastBrainState,
        slow_available: bool = True,
    ) -> str:
        """Retorna 'fast' ou 'slow'."""
        rules = self.config.get("rules", [])

        # Ordena por prioridade (menor = mais importante)
        rules_sorted = sorted(rules, key=lambda r: r["priority"])

        for rule in rules_sorted:
            if self._evaluate_rule(rule, fast_state, slow_available):
                action = rule["action"]
                logger.debug(f"Router: {rule['name']} -> {action}")
                return action.replace("route_to_", "")  # "fast" ou "slow"

        return "fast"

    def _evaluate_rule(self, rule, fast_state, slow_available):
        cond = rule["condition"]

        if cond == "not safety_approved":
            return not fast_state.safety_approved

        if cond.startswith("confidence < "):
            threshold = float(cond.split("<")[-1].strip())
            return fast_state.confidence < threshold

        if "action_variance" in cond and "over" in cond:
            self._action_history.append(fast_state.action.copy())
            if len(self._action_history) > self.deadlock_window:
                self._action_history = self._action_history[-self.deadlock_window:]
            if len(self._action_history) >= self.deadlock_window:
                variance = float(np.var(np.array(self._action_history)))
                return variance < self.deadlock_threshold

        if "zvec_memory_count == 0" in cond:
            return len(fast_state.zvec_memories) == 0

        if cond == "default":
            return True

        return False


class CathedralAGI_v17:
    """Orquestrador principal do Cathedral ARKHE v17.0."""

    def __init__(self, config_path=None):
        self.config = CathedralConfig(config_path)
        self.router = Router(self.config)

        # Fast Brain (sempre local)
        self.fast_brain = FastBrain(self.config)

        # Slow Brain (SGLang, pode estar offline)
        self.slow_brain = SlowBrainSGLang(self.config)
        self._slow_healthy = False

        # NexAU (opcional)
        self.nexau = None
        try:
            from .nexau_bridge import CathedralNexAUAgent
            self.nexau = CathedralNexAUAgent(self.fast_brain, self.slow_brain, self.config)
            if not self.nexau.available:
                self.nexau = None
        except ImportError:
            pass

        # NexRL logger (opcional)
        self.nexrl_logger = None
        try:
            from .nexrl_bridge import InteractionLogger
            self.nexrl_logger = InteractionLogger()
        except ImportError:
            pass

        logger.info(f"Cathedral AGI v{self.config._config['cathedral']['version']} inicializado")

    async def health_check(self):
        """Verifica saúde de todos os componentes."""
        self._slow_healthy = await self.slow_brain.health_check()
        return {
            "fast_brain": True,
            "slow_brain": self._slow_healthy,
            "nexau": self.nexau is not None and self.nexau.available,
            "nexrl": self.nexrl_logger is not None,
        }

    async def cycle(self, observation=None) -> OrchestratorResult:
        """Executa um ciclo completo do Cathedral AGI."""
        t0 = time.perf_counter()

        # 1. Fast Brain cycle (sempre executa)
        fast_state = self.fast_brain.cycle(observation)

        # 2. Router decision
        route = self.router.decide(fast_state, slow_available=self._slow_healthy)

        # 3. Se route=slow e Slow Brain disponível, consulta
        if route == "slow" and self._slow_healthy:
            # Se NexAU disponível, delega para ele
            if self.nexau:
                slow_result = await self.nexau.delegate(
                    f"Observação: {str(observation)[:200] if observation is not None else 'Nenhuma'}\n"
                    f"Fast Brain ação: {fast_state.action.tolist()}\n"
                    f"Fast Brain confiança: {fast_state.confidence:.3f}\n"
                    f"Segurança aprovada: {fast_state.safety_approved}\n"
                    f"Memórias: {len(fast_state.zvec_memories)} encontradas"
                )
            else:
                slow_result = await self.slow_brain.reason(
                    dilemma=f"Fast Brain propôs ação {fast_state.action.tolist()} com confiança {fast_state.confidence:.3f}. "
                            f"Segurança: {'aprovada' if fast_state.safety_approved else 'REJEITADA'}. "
                            f"Memórias: {len(fast_state.zvec_memories)} encontradas.",
                    context=str(observation)[:500] if observation is not None else "",
                    memories=fast_state.zvec_memories,
                )

            action = np.array(slow_result["action_vector"], dtype=np.float32)
            confidence = slow_result["confidence"]
            reasoning = slow_result.get("reasoning", "")
            safety = slow_result.get("safety_override", False)
        else:
            # Usa decisão do Fast Brain
            action = fast_state.action
            confidence = fast_state.confidence
            reasoning = ""
            safety = fast_state.safety_approved

        # 4. Log para NexRL
        if self.nexrl_logger:
            self.nexrl_logger.log_interaction(
                observation=observation,
                fast_action=fast_state.action,
                slow_decision={"action_vector": action.tolist(), "confidence": confidence},
                reward=confidence,  # reward simplificado
                route=route,
            )

        latency = (time.perf_counter() - t0) * 1000

        return OrchestratorResult(
            action=action,
            confidence=confidence,
            route=route,
            safety_approved=safety,
            slow_brain_reasoning=reasoning,
            latency_ms=latency,
            fast_brain_state=fast_state,
        )

    async def run_loop(self, cycles=10, delay=0.01):
        """Loop principal para teste."""
        health = await self.health_check()
        print(f"Health: {health}")

        for i in range(cycles):
            result = await self.cycle()
            print(f"Cycle {i+1}: route={result.route}, conf={result.confidence:.3f}, "
                  f"action={[f'{v:.2f}' for v in result.action]}, "
                  f"latency={result.latency_ms:.1f}ms")
            await asyncio.sleep(delay)


# Para execução direta
if __name__ == "__main__":
    import sys
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(name)s] %(levelname)s: %(message)s"
    )
    agi = CathedralAGI_v17()
    asyncio.run(agi.run_loop(cycles=5))
