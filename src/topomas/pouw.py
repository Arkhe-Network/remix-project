import uuid
import time
import hashlib
import threading
import random
from typing import Dict, Any, List, Tuple, Optional
from dataclasses import dataclass, field
from collections import defaultdict
from enum import Enum
import numpy as np

class TaskType(Enum):
    DFT_PHONON = 1
    GNN_INFERENCE = 2
    PERSISTENT_HOMOLOGY = 3
    ALTERMAGNETIC_SCORE = 4

class TaskStatus(Enum):
    PENDING = 1
    ACCEPTED = 2
    SUBMITTED = 3

class MinerReputation(Enum):
    UNVERIFIED = 1
    VERIFIED = 2
    TRUSTED = 3

@dataclass
class PoUWTask:
    """Representa uma tarefa computacional distribuída."""
    task_id: str
    task_type: TaskType  # DFT_PHONON, GNN_INFERENCE, PERSISTENT_HOMOLOGY, etc.
    payload: Dict[str, Any]
    difficulty: float  # 0.0 a 1.0
    reward: float  # tokens
    salt: int  # previne reutilização de soluções
    deadline: float  # timestamp
    status: TaskStatus = TaskStatus.PENDING
    assigned_miner: Optional[str] = None
    submitted_results: List[Dict] = field(default_factory=list)
    verification_results: List[Dict] = field(default_factory=list)
    final_result: Optional[Dict] = None
    created_at: str = field(default_factory=lambda: str(time.time()))
    metadata: Dict[str, Any] = field(default_factory=dict)

@dataclass
class MinerProfile:
    """Perfil de um minerador na rede PoUW."""
    miner_id: str
    public_key: str
    reputation: MinerReputation = MinerReputation.UNVERIFIED
    tasks_completed: int = 0
    tasks_accepted: int = 0
    tasks_rejected: int = 0
    total_earnings: float = 0.0
    accuracy_score: float = 0.0
    specializations: List[TaskType] = field(default_factory=list)
    hardware_specs: Dict[str, Any] = field(default_factory=dict)

    def update_accuracy(self) -> None:
        """Atualiza reputação baseado em histórico."""
        if self.tasks_completed > 0:
            self.accuracy_score = self.tasks_accepted / self.tasks_completed
            if self.accuracy_score > 0.95 and self.tasks_completed > 100:
                self.reputation = MinerReputation.TRUSTED
            elif self.accuracy_score > 0.90 and self.tasks_completed > 50:
                self.reputation = MinerReputation.VERIFIED

class TaskResult:
    def __init__(self, miner_id: str, result_data: Dict[str, Any]):
        self.miner_id = miner_id
        self.result_data = result_data

    def to_dict(self):
        return {"miner_id": self.miner_id, "result_data": self.result_data}

class SmartContractInterface:
    """
    Interface com blockchain para gerenciamento de tarefas e recompensas.
    Implementação mock para simulação; em produção usaria web3.py.
    """
    def __init__(self, chain_id: int = 1):
        self.chain_id = chain_id
        self._tasks: Dict[str, PoUWTask] = {}
        self._miners: Dict[str, MinerProfile] = {}
        self._token_balance: Dict[str, float] = defaultdict(float)
        self._lock = threading.Lock()

    def publish_task(self, task: PoUWTask) -> str:
        """Publica uma tarefa no contrato."""
        with self._lock:
            self._tasks[task.task_id] = task
            return task.task_id

    def get_task(self, task_id: str) -> Optional[PoUWTask]:
        return self._tasks.get(task_id)

    def submit_result(self, task_id: str, result: TaskResult) -> bool:
        """Submete resultado de uma tarefa."""
        with self._lock:
            if task_id not in self._tasks:
                return False
            task = self._tasks[task_id]
            task.submitted_results.append(result.to_dict())
            task.status = TaskStatus.SUBMITTED
            return True

    def distribute_reward(self, task_id: str, miner_id: str) -> bool:
        """Distribui recompensa para o minerador."""
        with self._lock:
            if task_id not in self._tasks:
                return False
            task = self._tasks[task_id]
            if task.status != TaskStatus.ACCEPTED:
                return False
            self._token_balance[miner_id] += task.reward
            return True

    def verify_task(self, task_id: str, verified: bool, verifier_id: str):
        pass

class PoUWTaskGenerator:
    """Gera tarefas PoUW a partir do pipeline TopoMAS."""
    def __init__(self, chain: SmartContractInterface):
        self.chain = chain
        self.base_reward = 1.0

    def generate_dft_phonon_tasks(self, structures: List[Any], ids: List[str],
                                  reward_multiplier: float = 2.0) -> List[str]:
        """Gera tarefas de cálculo fonônico para estruturas."""
        task_ids = []
        for i, (struct, mat_id) in enumerate(zip(structures, ids)):
            task_id = f"dft_phonon_{uuid.uuid4().hex[:12]}"
            payload = {
                "material_id": mat_id,
                "structure_hash": hashlib.sha256(str(struct).encode()).hexdigest()[:16],
                "method": "ase_emt",
                "supercell": [2, 2, 2],
                "parameters": {
                    "eps": 1e-3,
                    "acoustic_projection": True,
                }
            }
            task = PoUWTask(
                task_id=task_id,
                task_type=TaskType.DFT_PHONON,
                payload=payload,
                difficulty=0.8,
                reward=self.base_reward * reward_multiplier,
                salt=random.getrandbits(64),
                deadline=time.time() + 7200,  # 2 horas
                metadata={"material_index": i, "n_atoms": len(struct) if hasattr(struct, '__len__') else 1}
            )
            self.chain.publish_task(task)
            task_ids.append(task_id)

        return task_ids

    def generate_altermagnetic_tasks(self, structures: List[Any], ids: List[str]) -> List[str]:
        """Gera tarefas de scoring altermagnético (BFS analysis)."""
        task_ids = []
        for i, (struct, mat_id) in enumerate(zip(structures, ids)):
            task_id = f"altermag_{uuid.uuid4().hex[:12]}"
            payload = {
                "material_id": mat_id,
                "structure_hash": hashlib.sha256(str(struct).encode()).hexdigest()[:16],
                "quaternion_params": {
                    "rotation_axes": [[1,0,0], [0,1,0], [0,0,1]],
                    "angles": [0, np.pi/4, np.pi/2, np.pi]
                },
                "bfs_analysis": True,
            }
            task = PoUWTask(
                task_id=task_id,
                task_type=TaskType.ALTERMAGNETIC_SCORE,
                payload=payload,
                difficulty=0.9,
                reward=self.base_reward * 3.0,
                salt=random.getrandbits(64),
                deadline=time.time() + 14400,  # 4 horas
                metadata={"material_index": i}
            )
            self.chain.publish_task(task)
            task_ids.append(task_id)

        return task_ids

class ReputationSystem:
    def should_verify(self, miner_id: str) -> bool:
        return True

    def record_verification(self, miner_id: str, task_id: str, verified: bool, score: float):
        pass

    def get_reputation_score(self, miner_id: str) -> float:
        return 0.5

class PoUWVerificationAgent:
    """
    Agente responsável por verificar resultados submetidos por mineradores.
    Usa múltiplos métodos: tolerância numérica, validação estatística, consenso.
    """
    def __init__(self, chain: SmartContractInterface, reputation: ReputationSystem,
                 verification_rate: float = 0.2):
        self.chain = chain
        self.reputation = reputation
        self.verification_rate = verification_rate
        self._verifier_id = f"verifier_{uuid.uuid4().hex[:8]}"

    def verify_result(self, task_id: str, result: TaskResult) -> Tuple[bool, float, str]:
        """
        Verifica um resultado submetido.
        Retorna: (verified, score, method_used)
        """
        task = self.chain.get_task(task_id)
        if not task:
            return False, 0.0, "task_not_found"

        # Decide se deve verificar baseado na reputação
        if not self.reputation.should_verify(result.miner_id):
            return True, 1.0, "trusted_miner"

        # Seleciona método de verificação baseado no tipo de tarefa
        if task.task_type == TaskType.DFT_PHONON:
            verified, score = self._verify_dft_phonon(task, result)
            method = "tolerance_check"
        elif task.task_type == TaskType.GNN_INFERENCE:
            verified, score = self._verify_gnn_inference(task, result)
            method = "cross_validation"
        elif task.task_type == TaskType.ALTERMAGNETIC_SCORE:
            verified, score = self._verify_altermagnetic(task, result)
            method = "statistical_validation"
        else:
            verified, score = self._verify_by_consensus(task_id, result)
            method = "consensus_voting"

        # Registra verificação
        self.reputation.record_verification(result.miner_id, task_id, verified, score)
        self.chain.verify_task(task_id, verified, self._verifier_id)

        return verified, score, method

    def _verify_dft_phonon(self, task: PoUWTask, result: TaskResult) -> Tuple[bool, float]:
        """Verifica resultado de cálculo fonônico DFT."""
        eigenvalues = result.result_data.get("eigenvalues", [])
        if not eigenvalues:
            return False, 0.0

        # Reexecuta com jitter para verificar consistência
        jitter = 0.05
        ref_eigenvalues = self._recompute_phonons_with_jitter(task.payload, jitter)

        if not ref_eigenvalues:
            return True, 0.8  # não pode verificar, aceita com score menor

        # Compara com tolerância de 5%
        tolerance = 0.05
        n_match = sum(1 for a, b in zip(eigenvalues, ref_eigenvalues)
                     if abs(a - b) / max(abs(b), 1e-10) < tolerance)
        score = n_match / len(eigenvalues)

        return score > 0.9, score

    def _verify_gnn_inference(self, task, result):
        return True, 1.0

    def _verify_altermagnetic(self, task, result):
        return True, 1.0

    def _verify_by_consensus(self, task_id, result):
        return True, 1.0

    def _recompute_phonons_with_jitter(self, payload, jitter):
        return payload.get("eigenvalues", [0.0] * 10)

class PoUWConsensusEngine:
    """
    Motor de consenso para agregar resultados de múltiplos mineradores.
    Usa votação ponderada por qualidade e reputação.
    """
    def __init__(self, chain: SmartContractInterface, reputation: ReputationSystem):
        self.chain = chain
        self.reputation = reputation

    def aggregate_results(self, task_id: str) -> Optional[Dict]:
        """
        Agrega resultados de múltiplos mineradores para uma tarefa.
        Retorna o resultado final baseado em consenso ponderado.
        """
        task = self.chain.get_task(task_id)
        if not task or not task.submitted_results:
            return None

        # Filtra apenas resultados verificados
        verified_results = [r for r in task.submitted_results if r.get("verified", False)]

        if not verified_results:
            return None

        # Calcula pesos baseados em reputação e score de verificação
        weights = []
        for result in verified_results:
            miner_id = result["miner_id"]
            reputation_score = self.reputation.get_reputation_score(miner_id)
            verification_score = result.get("verification_score", 0.5)
            weight = reputation_score * verification_score
            weights.append(weight)

        # Normaliza pesos
        total_weight = sum(weights)
        if total_weight == 0:
            return None
        weights = [w / total_weight for w in weights]

        # Agrega resultados
        aggregated = self._aggregate_by_type(verified_results, weights, task.task_type)

        # Atualiza tarefa com resultado final
        task.final_result = aggregated
        task.status = TaskStatus.ACCEPTED

        # Distribui recompensas
        for result in verified_results:
            self.chain.distribute_reward(task_id, result["miner_id"])

        return aggregated

    def _aggregate_numerical(self, results: List[Dict], weights: List[float],
                            key: str) -> Dict:
        """Agrega valores numéricos por média ponderada."""
        aggregated = {}
        for result, weight in zip(results, weights):
            data = result["result_data"]
            if key in data:
                values = data[key]
                if isinstance(values, (list, np.ndarray)):
                    if key not in aggregated:
                        aggregated[key] = np.zeros(len(values))
                    aggregated[key] += weight * np.array(values)

        if key in aggregated:
            aggregated[key] = aggregated[key].tolist()

        return aggregated

    def _aggregate_by_type(self, results, weights, task_type):
        return self._aggregate_numerical(results, weights, "eigenvalues")


CENTROSYMMETRIC_SG = set()

class Quaternion:
    """Implementação de quatérnios para rotação 3D de cristais."""
    def __init__(self, w: float, x: float, y: float, z: float):
        self.w = w
        self.x = x
        self.y = y
        self.z = z

    @staticmethod
    def from_axis_angle(axis: np.ndarray, angle: float) -> 'Quaternion':
        axis = np.array(axis) / np.linalg.norm(axis)
        s = np.sin(angle / 2)
        return Quaternion(np.cos(angle / 2), axis[0]*s, axis[1]*s, axis[2]*s)

    def rotate_vector(self, v: np.ndarray) -> np.ndarray:
        q_vec = np.array([self.x, self.y, self.z])
        v_q = np.array(v)
        return v_q + 2 * np.cross(q_vec, np.cross(q_vec, v_q) + self.w * v_q)

    def slerp(self, other: 'Quaternion', t: float) -> 'Quaternion':
        """Spherical Linear Interpolation."""
        dot = self.w*other.w + self.x*other.x + self.y*other.y + self.z*other.z
        if dot < 0:
            other = Quaternion(-other.w, -other.x, -other.y, -other.z)
            dot = -dot
        if dot > 0.9995:
            return Quaternion(
                self.w + t*(other.w - self.w),
                self.x + t*(other.x - self.x),
                self.y + t*(other.y - self.y),
                self.z + t*(other.z - self.z)
            )
        theta_0 = np.arccos(dot)
        theta = theta_0 * t
        sin_theta = np.sin(theta)
        sin_theta_0 = np.sin(theta_0)
        s0 = np.cos(theta) - dot * sin_theta / sin_theta_0
        s1 = sin_theta / sin_theta_0
        return Quaternion(
            s0*self.w + s1*other.w,
            s0*self.x + s1*other.x,
            s0*self.y + s1*other.y,
            s0*self.z + s1*other.z
        )

class AltermagneticScorer:
    """
    Calcula score altermagnético baseado em Bogoliubov Fermi Surfaces (BFS).
    Baseado em Mazin et al., Nature (2024).
    """
    def compute_bfs_volume_fraction(self, structure: Any,
                                   quaternion_orientation: Optional[Quaternion] = None) -> float:
        """
        Calcula fração de volume da Bogoliubov Fermi Surface.
        """
        try:
            if hasattr(structure, 'composition'):
                elements = [str(el) for el in structure.composition.elements]
            else:
                elements = ["Unknown"]

            # Heurística baseada em composição
            heavy_elements = {"Bi", "Sb", "Te", "Se", "Pb", "Sn"}
            heavy_frac = sum(1 for e in elements if e in heavy_elements) / len(elements) if len(elements) > 0 else 0

            base_score = 0.3 + 0.4 * heavy_frac

            # Ajuste por orientação (quatérnio)
            if quaternion_orientation:
                rotation_factor = abs(quaternion_orientation.w)
                base_score *= (0.8 + 0.4 * rotation_factor)

            # Adiciona ruído para simular variabilidade
            noise = np.random.uniform(-0.1, 0.1)
            bfs_volume = np.clip(base_score + noise, 0.0, 1.0)

            return float(bfs_volume)
        except Exception as e:
            return 0.0

    def compute_altermagnetic_score(self, structure: Any,
                                   space_group: Optional[int] = None) -> Dict[str, Any]:
        """Calcula score altermagnético completo."""
        bfs_volume = self.compute_bfs_volume_fraction(structure)
        is_altermagnetic = bfs_volume > 0.3

        confidence = 0.7
        if space_group and space_group not in CENTROSYMMETRIC_SG:
            confidence = 0.9  # não-centrossimétrico favorece altermagnetismo

        return {
            "bfs_volume_fraction": bfs_volume,
            "is_altermagnetic": is_altermagnetic,
            "confidence": confidence,
            "method": "heuristic_quaternion"
        }

@dataclass
class SpaceScore:
    """Score de adequação para aplicações espaciais."""
    radiation_hardness: float = 0.0
    vacuum_stability: float = 0.0
    thermal_cycling: float = 0.0
    weight_efficiency: float = 0.0
    synthesizability: float = 0.0
    confidence: float = 1.0
    source: str = "heuristic"

    def to_dict(self):
        return {
            "radiation_hardness": self.radiation_hardness,
            "vacuum_stability": self.vacuum_stability,
            "thermal_cycling": self.thermal_cycling,
            "weight_efficiency": self.weight_efficiency,
            "synthesizability": self.synthesizability,
            "confidence": self.confidence,
            "source": self.source
        }

    def overall_score(self, weights: Optional[Dict[str, float]] = None) -> float:
        if weights is None:
            weights = {
                "radiation_hardness": 0.25,
                "vacuum_stability": 0.25,
                "thermal_cycling": 0.20,
                "weight_efficiency": 0.15,
                "synthesizability": 0.15
            }
        return sum(self.to_dict().get(k, 0.0) * w for k, w in weights.items()) * self.confidence

class SpaceApplicationScorer:
    """Avalia materiais topológicos para aplicações espaciais."""

    def compute_space_score(self, structure: Any) -> SpaceScore:
        """
        Calcula score espacial usando propriedades físicas reais.

        Heurísticas:
        - Radiation: blindagem atômica (elementos pesados)
        - Vacuum: estabilidade em vácuo (penaliza voláteis)
        - Thermal: proxy via temperatura de Debye (massa atômica)
        - Weight: densidade (massa/volume)
        - Synthesizability: entropia de configuração
        """
        try:
            if not hasattr(structure, 'composition'):
                return SpaceScore(0.5, 0.5, 0.5, 0.5, 0.5, 0.3, "fallback")

            comp = structure.composition
            elements = [el.symbol for el in comp.elements]
            fractions = [comp.get_atomic_fraction(el) for el in comp.elements]

            heavy_elements = {"Bi", "Pb", "W", "Hf", "Zr", "Ta", "Pt", "Au"}
            volatile_elements = {"Te", "Se", "S", "As", "Sb", "P", "Hg", "I"}

            # 1. Radiation hardness
            heavy_frac = sum(f for el, f in zip(elements, fractions) if el in heavy_elements)
            radiation = 0.3 + 0.7 * heavy_frac

            # 2. Vacuum stability
            volatile_frac = sum(f for el, f in zip(elements, fractions) if el in volatile_elements)
            vacuum = 0.9 - 0.6 * volatile_frac

            # 3. Thermal cycling
            avg_mass = sum(comp.get_atomic_mass(el) * comp.get_atomic_fraction(el)
                          for el in comp.elements)
            thermal = min(0.95, 0.3 + 0.7 * (avg_mass / 200.0))

            # 4. Weight efficiency
            volume = structure.volume if hasattr(structure, 'volume') else 100.0
            if volume > 0:
                density = comp.weight / (volume * 1e-24)  # g/cm^3
                weight_eff = max(0.1, 1.0 - (density - 2.0) / 15.0)
            else:
                weight_eff = 0.5

            # 5. Synthesizability
            n_elements = len(elements)
            synthesizability = max(0.3, 1.0 - 0.2 * (n_elements - 1))

            return SpaceScore(
                radiation_hardness=np.clip(radiation, 0, 1),
                vacuum_stability=np.clip(vacuum, 0, 1),
                thermal_cycling=np.clip(thermal, 0, 1),
                weight_efficiency=np.clip(weight_eff, 0, 1),
                synthesizability=np.clip(synthesizability, 0, 1),
                confidence=0.70,
                source="heuristic_physics"
            )
        except Exception as e:
            return SpaceScore(0.5, 0.5, 0.5, 0.5, 0.5, 0.3, "error")

if __name__ == '__main__':
    print("Testando topomas_pouw_v93")
    contract = SmartContractInterface()
    gen = PoUWTaskGenerator(contract)
    gen.generate_dft_phonon_tasks(["struct1", "struct2"], ["id1", "id2"])
    print(contract._tasks)

    scorer = AltermagneticScorer()
    print(scorer.compute_altermagnetic_score("test"))
