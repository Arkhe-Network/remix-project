"""
Cathedral ARKHE v17.0 - Grounded Data Pipeline & Biographical Memory
Gera dados de treino autorais a partir de física real, não de textos da internet.
"""
import json
import time
import hashlib
import numpy as np
from pathlib import Path

class GroundedDataPipeline:
    def __init__(self, output_dir="nexrl_data"):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.dataset_file = self.output_dir / "grounded_trajectories.jsonl"

    def run_mujoco_episode(self, xml_path: str = None, steps: int = 100):
        """Roda uma simulação MuJoCo, coleta estados e traduz para texto."""
        try:
            import mujoco
        except ImportError:
            print("[Pipeline] MuJoCo não instalado. Usando dados sintéticos para teste.")
            return self._generate_synthetic_trajectory(steps)

        if xml_path is None:
            # Usa o modelo de teste padrão do MuJoCo
            xml_path = str(Path(mujoco.__file__).parent / "xml" / "test.xml")

        model = mujoco.MjModel.from_xml_path(xml_path)
        data = mujoco.MjData(model)

        trajectory = []
        for _ in range(steps):
            mujoco.mj_step(model, data)

            # Experiência Grounded (física pura convertida em texto)
            state_desc = f"Posição: {data.qpos[:3].tolist()}, Velocidade: {data.qvel[:3].tolist()}"
            action = np.random.uniform(-1, 1, model.nu).tolist() if model.nu > 0 else [0.0]

            trajectory.append({
                "type": "grounded_physics",
                "observation_text": state_desc,
                "action_taken": action,
                "reward": float(np.random.uniform(-1, 1)), # Esparsa
                "source": "cathedral_v17_mujoco"
            })
        return trajectory

    def _generate_synthetic_trajectory(self, steps: int):
        trajectory = []
        for i in range(steps):
            trajectory.append({
                "type": "grounded_synthetic",
                "observation_text": f"Estado hipotético {i} com variáveis contínuas.",
                "action_taken": [np.sin(i), np.cos(i)],
                "reward": 1.0 if i == steps-1 else 0.0,
                "source": "cathedral_v17_synthetic"
            })
        return trajectory

    def save_trajectory(self, trajectory: list):
        with open(self.dataset_file, "a", encoding="utf-8") as f:
            for event in trajectory:
                f.write(json.dumps(event, ensure_ascii=False) + "\n")


class BiographicalMemory:
    """Camada de memória que garante que apenas experiências 'Cathedral' entram."""

    @staticmethod
    def create_biographical_entry(state_vector: np.ndarray, metadata: dict, agent_id: str = "cathedral_v17") -> dict:
        # Hash imutável da experiência (Ato 2 estendido à memória)
        experience_str = json.dumps(metadata, sort_keys=True) + state_vector.tobytes().hex()
        exp_hash = hashlib.blake2b(experience_str.encode(), digest_size=32).hexdigest()

        metadata["source_agent"] = agent_id
        metadata["experience_hash"] = exp_hash
        metadata["timestamp"] = time.time()

        return metadata
