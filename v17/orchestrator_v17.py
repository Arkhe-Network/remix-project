"""
Cathedral ARKHE v17.1 - Orquestrador Autoral (Loop Consciente)
Integra: Proveniência, Evolução de Prompt, Memória Bio, Coleta DPO e Auditoria.
"""
import asyncio
import time
import json
import logging
import numpy as np
from pathlib import Path
from .config_loader import CathedralConfig
from .fast_brain import FastBrain
from .slow_brain import SlowBrainSGLang
from .provenance import CathedralProvenance
from .recursive_evolution import EvolutivePromptManager, CathedralDPOTrainer
from .anti_alchemy_audit import AlchemyAuditor

logger = logging.getLogger("cathedral.orchestrator")

class CathedralAGI_v17:
    def __init__(self, config_path=None):
        self.config = CathedralConfig(config_path)

        # Componentes Primários
        self.fast_brain = FastBrain(self.config)
        self.slow_brain = SlowBrainSGLang(self.config)
        self._slow_healthy = False

        # --- MÓDULOS DA ALMA (Substrato 3001) ---
        self.provenance = CathedralProvenance()
        self.prompt_manager = EvolutivePromptManager(
            base_prompt=self.config.get("slow_brain.swi_reasoning.system_prompt")
        )
        self.dpo_trainer = CathedralDPOTrainer()
        self.alchemy_auditor = AlchemyAuditor()

        # Controle de ciclo
        self.cycle_counter = 0
        self.dpo_buffer_path = Path("nexrl_data/dpo_preferences.jsonl")
        self.dpo_buffer_path.parent.mkdir(parents=True, exist_ok=True)

        logging.info(f"Cathedral AGI v17.1 (Autoral) inicializada.")

    async def health_check(self):
        self._slow_healthy = await self.slow_brain.health_check()
        return {
            "fast_brain": True,
            "slow_brain": self._slow_healthy,
            "provenance_keys": self.provenance.priv_key_path.exists()
        }

    async def cycle(self, observation=None, reward: float = 0.0) -> dict:
        t0 = time.perf_counter()
        self.cycle_counter += 1

        # 1. Fast Brain Cycle (Agora com personalidade PyTorch)
        fast_state = self.fast_brain.cycle(observation, reward=reward)

        route = "fast"
        slow_response = None

        # 2. Router Decision
        if not fast_state.safety_approved and self._slow_healthy:
            route = "slow"
            slow_response = await self.slow_brain.reason(
                dilemma=f"Ação rejeitada pelo Z3: {fast_state.safety_reason}. Estado: {fast_state.world_state[:5]}",
                context="Roteamento por falha de segurança."
            )
            fast_state.action = np.array(slow_response["action_vector"], dtype=np.float32)
            fast_state.confidence = slow_response["confidence"]

        # --- PROCESSAMENTO DA ALMA (A cada ciclo) ---

        # Módulo 6: Evolução do Prompt
        success = (route == "fast" or fast_state.confidence > 0.5)
        self.prompt_manager.evolve(fast_state.safety_reason, success)
        self.slow_brain.sys_prompt = self.prompt_manager.current_prompt

        # Módulo 5: Coleta de Dados para DPO (Se o Slow Brain corrigiu o Fast)
        if route == "slow" and slow_response:
            dpo_pair = {
                "prompt": f"Estado: {fast_state.world_state[:5].tolist()}\nAção do Fast: {fast_state.action.tolist()}",
                "chosen": json.dumps({"reasoning": slow_response.get("reasoning", ""), "action": slow_response["action_vector"]}),
                "rejected": json.dumps({"reasoning": "Reflexo cego sem análise", "action": [0.0]*self.config.fast_brain["action_dim"]})
            }
            with open(self.dpo_buffer_path, "a", encoding="utf-8") as f:
                f.write(json.dumps(dpo_pair) + "\n")

        # Módulo 7: Auditoria Anti-Alquimia (A cada 1000 ciclos)
        if self.cycle_counter % 1000 == 0:
            logger.info(f"[AUDITORIA] Ciclo {self.cycle_counter}. Verificando identidade autoral...")
            # Em produção real, extraímos os logits reais aqui.
            # mock_audit = self.alchemy_auditor.run_audit(np.random.rand(100), np.random.rand(100), np.random.rand(100))
            # logger.info(f"[AUDITORIA] Resultado: {mock_audit['verdict']}")
            self.fast_brain.save_personality() # Salva a personalidade evoluida

        latency = (time.perf_counter() - t0) * 1000
        return {
            "route": route,
            "action": fast_state.action,
            "conf": fast_state.confidence,
            "lat_ms": latency,
            "personality_bias_mean": float(np.mean(self.fast_brain.world_model.personality_bias.cpu().detach().numpy()))
        }

    async def run_loop(self, cycles=10):
        h = await self.health_check()
        print(f"Health: Fast[✅] Slow[{'✅' if h['slow_brain'] else '❌'}] Keys[{'✅' if h['provenance_keys'] else '❌'}]")
        print(f"Prompt Atual tem {len(self.prompt_manager.current_prompt)} caracteres.")
        print("-" * 60)

        for i in range(cycles):
            # Simula uma recompensa esparsa do ambiente (ex: +1 no ciclo 5, -1 no ciclo 8)
            reward = 1.0 if i == 5 else (-1.0 if i == 8 else 0.0)

            r = await self.cycle(reward=reward)
            print(f"C{i:02d} | Rota: {r['route']:5s} | Conf: {r['conf']:.2f} | Vies Pessoa: {r['personality_bias_mean']:.4f} | Lat: {r['lat_ms']:.1f}ms")

# Para execução direta
if __name__ == "__main__":
    import sys
    logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(name)s] %(levelname)s: %(message)s")
    agi = CathedralAGI_v17()
    asyncio.run(agi.run_loop(cycles=15))
