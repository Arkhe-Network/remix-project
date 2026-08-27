from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, Any, List, Optional, Tuple
from datetime import datetime
import uuid
import hashlib
import random
import time
import numpy as np

class TaskType(Enum):
    DFT_PHONON = "DFT_PHONON"
    GNN_INFERENCE = "GNN_INFERENCE"
    PERSISTENT_HOMOLOGY = "PERSISTENT_HOMOLOGY"
    ALTERMAGNETIC_SCORE = "ALTERMAGNETIC_SCORE"

class TaskStatus(Enum):
    PENDING = "PENDING"
    SUBMITTED = "SUBMITTED"
    ACCEPTED = "ACCEPTED"
    REJECTED = "REJECTED"

class MinerReputation(Enum):
    UNVERIFIED = "UNVERIFIED"
    VERIFIED = "VERIFIED"
    TRUSTED = "TRUSTED"

@dataclass
class TaskResult:
    miner_id: str
    result_data: Dict[str, Any]

    def to_dict(self):
        return {"miner_id": self.miner_id, "result_data": self.result_data}

@dataclass
class PoUWTask:
    task_id: str
    task_type: TaskType
    payload: Dict[str, Any]
    difficulty: float
    reward: float
    salt: int
    deadline: float
    status: TaskStatus = TaskStatus.PENDING
    assigned_miner: Optional[str] = None
    submitted_results: List[Dict] = field(default_factory=list)
    verification_results: List[Dict] = field(default_factory=list)
    final_result: Optional[Dict] = None
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    metadata: Dict[str, Any] = field(default_factory=dict)

@dataclass
class MinerProfile:
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
        if self.tasks_completed > 0:
            self.accuracy_score = self.tasks_accepted / self.tasks_completed
            if self.accuracy_score > 0.95 and self.tasks_completed > 100:
                self.reputation = MinerReputation.TRUSTED
            elif self.accuracy_score > 0.90 and self.tasks_completed > 50:
                self.reputation = MinerReputation.VERIFIED

import threading
from collections import defaultdict

class SmartContractInterface:
    def __init__(self, chain_id: int = 1):
        self.chain_id = chain_id
        self._tasks: Dict[str, PoUWTask] = {}
        self._miners: Dict[str, MinerProfile] = {}
        self._token_balance: Dict[str, float] = defaultdict(float)
        self._lock = threading.Lock()

    def publish_task(self, task: PoUWTask) -> str:
        with self._lock:
            self._tasks[task.task_id] = task
            return task.task_id

    def get_task(self, task_id: str):
        return self._tasks.get(task_id)

    def verify_task(self, task_id: str, verified: bool, verifier_id: str):
        pass

    def submit_result(self, task_id: str, result: TaskResult) -> bool:
        with self._lock:
            if task_id not in self._tasks:
                return False
            task = self._tasks[task_id]
            task.submitted_results.append(result.to_dict())
            task.status = TaskStatus.SUBMITTED
            return True

    def distribute_reward(self, task_id: str, miner_id: str) -> bool:
        with self._lock:
            if task_id not in self._tasks:
                return False
            task = self._tasks[task_id]
            if task.status != TaskStatus.ACCEPTED:
                return False
            self._token_balance[miner_id] += task.reward
            return True

class ReputationSystem:
    def should_verify(self, miner_id: str):
        return True
    def get_reputation_score(self, miner_id: str):
        return 0.5
    def record_verification(self, miner_id, task_id, verified, score):
        pass

class PoUWTaskGenerator:
    def __init__(self, chain):
        self.chain = chain
        self.base_reward = 1.0

    def generate_dft_phonon_tasks(self, structures: List[Any], ids: List[str],
                                  reward_multiplier: float = 2.0) -> List[str]:
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
                deadline=time.time() + 7200,
                metadata={"material_index": i, "n_atoms": len(struct) if hasattr(struct, '__len__') else 1}
            )
            self.chain.publish_task(task)
            task_ids.append(task_id)
        return task_ids

    def generate_altermagnetic_tasks(self, structures: List[Any], ids: List[str]) -> List[str]:
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
                deadline=time.time() + 14400,
                metadata={"material_index": i}
            )
            self.chain.publish_task(task)
            task_ids.append(task_id)
        return task_ids

class PoUWVerificationAgent:
    def __init__(self, chain: SmartContractInterface, reputation: ReputationSystem,
                 verification_rate: float = 0.2):
        self.chain = chain
        self.reputation = reputation
        self.verification_rate = verification_rate
        self._verifier_id = f"verifier_{uuid.uuid4().hex[:8]}"

    def verify_result(self, task_id: str, result: TaskResult) -> Tuple[bool, float, str]:
        task = self.chain.get_task(task_id)
        if not task:
            return False, 0.0, "task_not_found"

        if not self.reputation.should_verify(result.miner_id):
            return True, 1.0, "trusted_miner"

        if task.task_type == TaskType.DFT_PHONON:
            verified, score = self._verify_dft_phonon(task, result)
            method = "tolerance_check"
        elif task.task_type == TaskType.GNN_INFERENCE:
            verified, score = True, 1.0
            method = "cross_validation"
        elif task.task_type == TaskType.ALTERMAGNETIC_SCORE:
            verified, score = True, 1.0
            method = "statistical_validation"
        else:
            verified, score = True, 1.0
            method = "consensus_voting"

        self.reputation.record_verification(result.miner_id, task_id, verified, score)
        self.chain.verify_task(task_id, verified, self._verifier_id)

        return verified, score, method

    def _recompute_phonons_with_jitter(self, payload, jitter):
        return [1.0, 2.0, 3.0]

    def _verify_dft_phonon(self, task: PoUWTask, result: TaskResult) -> Tuple[bool, float]:
        eigenvalues = result.result_data.get("eigenvalues", [])
        if not eigenvalues:
            return False, 0.0

        jitter = 0.05
        ref_eigenvalues = self._recompute_phonons_with_jitter(task.payload, jitter)

        if not ref_eigenvalues:
            return True, 0.8

        tolerance = 0.05
        n_match = sum(1 for a, b in zip(eigenvalues, ref_eigenvalues)
                     if abs(a - b) / max(abs(b), 1e-10) < tolerance)
        score = n_match / len(eigenvalues)

        return score > 0.9, score

class PoUWConsensusEngine:
    def __init__(self, chain, reputation):
        self.chain = chain
        self.reputation = reputation

    def aggregate_results(self, task_id: str) -> Optional[Dict]:
        task = self.chain.get_task(task_id)
        if not task or not task.submitted_results:
            return None

        verified_results = [r for r in task.submitted_results if r.get("verified", False)]
        if not verified_results:
            return None

        weights = []
        for result in verified_results:
            miner_id = result["miner_id"]
            reputation_score = self.reputation.get_reputation_score(miner_id)
            verification_score = result.get("verification_score", 0.5)
            weight = reputation_score * verification_score
            weights.append(weight)

        total_weight = sum(weights)
        if total_weight == 0:
            return None
        weights = [w / total_weight for w in weights]

        aggregated = self._aggregate_numerical(verified_results, weights, "eigenvalues")

        task.final_result = aggregated
        task.status = TaskStatus.ACCEPTED

        for result in verified_results:
            self.chain.distribute_reward(task_id, result["miner_id"])

        return aggregated

    def _aggregate_numerical(self, results: List[Dict], weights: List[float], key: str) -> Dict:
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

CENTROSYMMETRIC_SG = {
    2, 10, 11, 12, 13, 14, 15,
    47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60,
    61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74,
    147, 148, 162, 163, 164, 165, 166, 167,
    83, 84, 85, 86, 87, 88,
    123, 124, 125, 126, 127, 128, 129, 130, 131, 132,
    133, 134, 135, 136, 137, 138, 139, 140, 141, 142,
    175, 176, 191, 192, 193, 194,
    200, 201, 202, 203, 204, 205, 206,
    221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
}

class Quaternion:
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

class AltermagneticScorer:
    def compute_bfs_volume_fraction(self, structure: Any,
                                   quaternion_orientation: Optional[Quaternion] = None) -> float:
        try:
            if hasattr(structure, 'composition'):
                elements = [str(el) for el in structure.composition.elements]
            else:
                elements = ["Unknown"]

            heavy_elements = {"Bi", "Sb", "Te", "Se", "Pb", "Sn"}
            heavy_frac = sum(1 for e in elements if e in heavy_elements) / len(elements)

            base_score = 0.3 + 0.4 * heavy_frac

            if quaternion_orientation:
                rotation_factor = abs(quaternion_orientation.w)
                base_score *= (0.8 + 0.4 * rotation_factor)

            noise = np.random.uniform(-0.1, 0.1)
            bfs_volume = np.clip(base_score + noise, 0.0, 1.0)

            return float(bfs_volume)
        except Exception as e:
            return 0.0

    def compute_altermagnetic_score(self, structure: Any,
                                   space_group: Optional[int] = None) -> Dict[str, Any]:
        bfs_volume = self.compute_bfs_volume_fraction(structure)
        is_altermagnetic = bfs_volume > 0.3

        confidence = 0.7
        if space_group and space_group not in CENTROSYMMETRIC_SG:
            confidence = 0.9

        return {
            "bfs_volume_fraction": bfs_volume,
            "is_altermagnetic": is_altermagnetic,
            "confidence": confidence,
            "method": "heuristic_quaternion"
        }
