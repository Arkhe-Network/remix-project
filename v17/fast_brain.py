"""
Cathedral ARKHE v17.1 - Fast Brain (Identidade Cibernética Ativa)
Incorpora RSSM com Personalidade PyTorch e Memória Biográfica Assinada.
"""
import time
import numpy as np
import logging
import torch
from typing import List
from dataclasses import dataclass, field
from .world_model_personality import RSSMWithPersonality
from .grounded_pipeline import BiographicalMemory
from .trading.zeroex_trading_module import ZeroExTradingModule

logger = logging.getLogger("cathedral.fast_brain")

class ZvecMemoryWrapper:
    def __init__(self, fast_brain):
        self.fast_brain = fast_brain

    def store_transaction_embedding(self, embedding, result):
        if self.fast_brain.index is not None:
            state_vector = np.array(embedding, dtype=np.float32)
            bio_meta = BiographicalMemory.create_biographical_entry(
                state_vector=state_vector,
                metadata={"tx_result": result}
            )
            self.fast_brain.index.add_items(state_vector, self.fast_brain._mem_count)
            self.fast_brain.memories.append(bio_meta)
            self.fast_brain._mem_count += 1
            if self.fast_brain._mem_count % 50 == 0:
                import os
                os.makedirs(self.fast_brain.mem_data_dir, exist_ok=True)
                self.fast_brain.index.save_index(os.path.join(self.fast_brain.mem_data_dir, "episodic_index.bin"))
HAS_V16 = False

@dataclass
class FastBrainState:
    observation: np.ndarray = None
    vision_features: np.ndarray = None
    world_state: np.ndarray = None
    action: np.ndarray = None
    confidence: float = 0.0
    safety_approved: bool = False
    safety_reason: str = ""
    zvec_memories: List[dict] = field(default_factory=list)
    cycle_time_ms: float = 0.0

class FastBrain:
    def __init__(self, config):
        fb = config.fast_brain
        self.action_dim = fb["action_dim"]
        self.device = fb.get("device", "cpu")
        self.max_force = fb["safety"]["max_force"]
        self.mem_data_dir = fb["memory"]["data_dir"]
        self.state = FastBrainState()

        # --- INICIALIZAÇÃO DA PERSONALIDADE (Módulo 4) ---
        wm = fb["world_model"]
        self.world_model = RSSMWithPersonality(
            deter_dim=wm["deter_dim"],
            stoch_dim=wm["stoch_dim"],
            hidden_dim=wm["hidden_dim"],
            action_dim=self.action_dim
        ).to(self.device)

        # Tenta carregar pesos de personalidade previamente salvos
        personality_path = "config/personality_weights.pt"
        if torch.cuda.is_available() or self.device == "cpu":
            try:
                self.world_model.load_state_dict(torch.load(personality_path, map_location=self.device))
                logger.info("Personalidade (pesos) carregada com sucesso.")
            except FileNotFoundError:
                logger.info("Primeiro boot. Criando nova personalidade (identidade única).")

        # --- INICIALIZAÇÃO DA MEMÓRIA BIOGRÁFICA (Módulo 3) ---
        try:
            import hnswlib, os, json
            self.index = hnswlib.Index(space="l2", dim=288)
            idx_file = os.path.join(self.mem_data_dir, "episodic_index.bin")
            if os.path.exists(idx_file):
                self.index.load_index(idx_file)
            else:
                self.index.init_index(max_elements=10000, ef_construction=200, M=16)
            self.index.set_ef(50)
            self.memories = []
            self._mem_count = 0
        except Exception as e:
            self.index = None
            logger.warning(f"Memória HNSW indisponível: {e}")

        self.last_action = torch.zeros(1, self.action_dim, device=self.device)

        # --- INICIALIZAÇÃO DO MÓDULO DE TRADING 0x ---
        self.trading_module = ZeroExTradingModule(
            api_key="mock_key",
            chain_id=137,
            wallet_address="0xMock",
            zvec_memory=ZvecMemoryWrapper(self)
        )
        self.trading_module.world_model = self.world_model

    def cycle(self, observation=None, reward: float = 0.0) -> FastBrainState:
        t0 = time.perf_counter()
        self.state.observation = observation

        # 1. Visão (Stub se não houver câmera)
        self.state.vision_features = np.random.randn(256).astype(np.float32) if observation is None else np.zeros(256, dtype=np.float32)

        # 2. World Model com Personalidade (PyTorch)
        vision_t = torch.tensor(self.state.vision_features, dtype=torch.float32).unsqueeze(0).to(self.device)

        with torch.no_grad():
            state_288_t = self.world_model(vision_t, self.last_action)

        self.state.world_state = state_288_t.cpu().numpy().flatten()

        # Atualiza personalidade com base na recompensa do ambiente (Módulo 4)
        if reward != 0.0:
            self.world_model.update_personality_from_reward(reward, lr=0.005)

        # 3. Geração de Ação (Extração do estado determinístico)
        action_np = np.tanh(self.state.world_state[:self.action_dim]).astype(np.float32)

        # 4. Verificação de Segurança (Z3)
        mag = np.linalg.norm(action_np)
        if mag > self.max_force:
            self.state.safety_approved = False
            self.state.safety_reason = f"Força excedida: {mag:.2f}N > {self.max_force}N"
            self.state.action = np.zeros(self.action_dim, dtype=np.float32)
            self.state.confidence = 0.0
        else:
            self.state.safety_approved = True
            self.state.safety_reason = "Aprovado"
            self.state.action = action_np
            self.state.confidence = float(np.mean(np.abs(action_np)))

        self.last_action = torch.tensor(self.state.action, dtype=torch.float32).unsqueeze(0).to(self.device)

        # 5. Memória Biográfica (Hash Blake2b)
        if self.index:
            bio_meta = BiographicalMemory.create_biographical_entry(
                state_vector=self.state.world_state,
                metadata={"action": self.state.action.tolist(), "conf": self.state.confidence}
            )
            self.index.add_items(self.state.world_state.astype(np.float32), self._mem_count)
            self.memories.append(bio_meta)
            self._mem_count += 1
            if self._mem_count % 50 == 0:
                import os
                os.makedirs(self.mem_data_dir, exist_ok=True)
                self.index.save_index(os.path.join(self.mem_data_dir, "episodic_index.bin"))

        self.state.cycle_time_ms = (time.perf_counter() - t0) * 1000
        return self.state

    def save_personality(self):
        """Salva a identidade atual em disco."""
        torch.save(self.world_model.state_dict(), "config/personality_weights.pt")
        logger.info("Personalidade salva em disco.")
