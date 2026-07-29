#!/usr/bin/env python3
"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (Benchmarks)                   ║
║ Métricas de avaliação: sample efficiency, forgetting, causal, energy   ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-BENCHMARKS-2026-06-14                   ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝
"""

import time
import math
import logging
from collections import deque, defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any

import numpy as np
import torch

logger = logging.getLogger("cathedral.v16.benchmarks")


@dataclass
class BenchmarkResult:
    """Resultado de uma métrica de benchmark."""
    metric_name: str
    value: float
    unit: str
    timestamp: float = field(default_factory=time.time)
    metadata: Dict[str, Any] = field(default_factory=dict)


class SampleEfficiencyTracker:
    """
    Rastreia sample efficiency: recompensa acumulada por número de interações.
    Métrica crítica para agentes embodied (interações reais são caras).
    """
    def __init__(self, window_size: int = 100):
        self.window_size = window_size
        self.episode_returns: deque = deque(maxlen=window_size)
        self.episode_lengths: deque = deque(maxlen=window_size)
        self.total_interactions = 0
        self.total_episodes = 0
        self._start_time = time.time()

    def record_episode(self, episode_return: float, episode_length: int):
        self.episode_returns.append(episode_return)
        self.episode_lengths.append(episode_length)
        self.total_interactions += episode_length
        self.total_episodes += 1

    def get_metrics(self) -> Dict[str, BenchmarkResult]:
        if not self.episode_returns:
            return {}

        returns = list(self.episode_returns)
        lengths = list(self.episode_lengths)

        return {
            "sample_efficiency": BenchmarkResult(
                metric_name="sample_efficiency",
                value=sum(returns) / max(self.total_interactions, 1),
                unit="return_per_interaction",
                metadata={"total_interactions": self.total_interactions}
            ),
            "avg_episode_return": BenchmarkResult(
                metric_name="avg_episode_return",
                value=sum(returns) / len(returns),
                unit="reward",
            ),
            "avg_episode_length": BenchmarkResult(
                metric_name="avg_episode_length",
                value=sum(lengths) / len(lengths),
                unit="steps",
            ),
            "interactions_per_hour": BenchmarkResult(
                metric_name="interactions_per_hour",
                value=self.total_interactions / max((time.time() - self._start_time) / 3600, 0.001),
                unit="interactions/hour",
            ),
        }


class CatastrophicForgettingMonitor:
    """
    Monitora catastrophic forgetting em aprendizado contínuo.
    Compara performance em tarefas antigas vs. novas.
    """
    def __init__(self, num_tasks: int = 10):
        self.num_tasks = num_tasks
        self.task_performances: Dict[int, deque] = defaultdict(lambda: deque(maxlen=50))
        self.task_learned_at: Dict[int, int] = {}
        self._current_task = 0

    def record_performance(self, task_id: int, performance: float, step: int):
        self.task_performances[task_id].append(performance)
        if task_id not in self.task_learned_at:
            self.task_learned_at[task_id] = step
        self._current_task = task_id

    def get_forgetting_metrics(self) -> Dict[str, BenchmarkResult]:
        if len(self.task_learned_at) < 2:
            return {}

        # Forward Transfer: performance na tarefa N após aprender tarefa N-1
        # Backward Transfer: performance em tarefas antigas após aprender novas

        forward_transfers = []
        backward_transfers = []

        for task_id in sorted(self.task_learned_at.keys()):
            if task_id == 0:
                continue

            prev_task = task_id - 1
            if prev_task in self.task_performances and task_id in self.task_performances:
                # Forward: melhoria na tarefa nova devido à anterior
                prev_perf = list(self.task_performances[prev_task])[-1] if self.task_performances[prev_task] else 0
                curr_perf = list(self.task_performances[task_id])[0] if self.task_performances[task_id] else 0
                forward_transfers.append(curr_perf - prev_perf)

        # Backward: queda de performance em tarefas antigas
        for task_id in sorted(self.task_learned_at.keys())[:-1]:
            if task_id not in self.task_performances:
                continue
            perfs = list(self.task_performances[task_id])
            if len(perfs) >= 2:
                initial = perfs[0]
                final = perfs[-1]
                backward_transfers.append(final - initial)

        metrics = {}
        if forward_transfers:
            metrics["forward_transfer"] = BenchmarkResult(
                metric_name="forward_transfer",
                value=sum(forward_transfers) / len(forward_transfers),
                unit="delta_performance",
            )
        if backward_transfers:
            metrics["backward_transfer"] = BenchmarkResult(
                metric_name="backward_transfer",
                value=sum(backward_transfers) / len(backward_transfers),
                unit="delta_performance",
            )
            metrics["forgetting_rate"] = BenchmarkResult(
                metric_name="forgetting_rate",
                value=sum(1 for b in backward_transfers if b < 0) / len(backward_transfers),
                unit="ratio",
            )

        return metrics


class CausalDiscoveryEvaluator:
    """
    Avalia qualidade da descoberta causal do motor causal.
    Compara grafo inferido com ground truth (quando disponível).
    """
    def __init__(self):
        self.predicted_edges: List[Tuple[str, str]] = []
        self.ground_truth_edges: List[Tuple[str, str]] = []
        self._edge_confidences: Dict[Tuple[str, str], float] = {}

    def set_ground_truth(self, edges: List[Tuple[str, str]]):
        self.ground_truth_edges = edges

    def add_predicted_edge(self, cause: str, effect: str, confidence: float = 1.0):
        edge = (cause, effect)
        self.predicted_edges.append(edge)
        self._edge_confidences[edge] = confidence

    def evaluate(self) -> Dict[str, BenchmarkResult]:
        if not self.ground_truth_edges:
            return {}

        gt_set = set(self.ground_truth_edges)
        pred_set = set(self.predicted_edges)

        tp = len(gt_set & pred_set)
        fp = len(pred_set - gt_set)
        fn = len(gt_set - pred_set)

        precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
        recall = tp / (tp + fn) if (tp + fn) > 0 else 0.0
        f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0.0

        return {
            "causal_precision": BenchmarkResult(
                metric_name="causal_precision", value=precision, unit="ratio"
            ),
            "causal_recall": BenchmarkResult(
                metric_name="causal_recall", value=recall, unit="ratio"
            ),
            "causal_f1": BenchmarkResult(
                metric_name="causal_f1", value=f1, unit="ratio"
            ),
        }


class ThermalPowerMonitor:
    """
    Monitora eficiência térmica e energética do sistema.
    Target: ~20W (cérebro humano) para inferência contínua.
    """
    def __init__(self, target_power_w: float = 20.0):
        self.target_power_w = target_power_w
        self.power_readings: deque = deque(maxlen=1000)
        self.temp_readings: deque = deque(maxlen=1000)
        self._start_time = time.time()

        try:
            import psutil
            self._has_psutil = True
        except ImportError:
            self._has_psutil = False

    def record(self, power_w: Optional[float] = None, temp_c: Optional[float] = None):
        """Registra leitura de potência e temperatura."""
        if power_w is not None:
            self.power_readings.append(power_w)
        if temp_c is not None:
            self.temp_readings.append(temp_c)

        # Fallback: estimativa via CPU usage
        if power_w is None and self._has_psutil:
            try:
                import psutil
                cpu_percent = psutil.cpu_percent(interval=0.1)
                # Estimativa grosseira: 65W TDP * utilização
                estimated = 65.0 * (cpu_percent / 100.0)
                self.power_readings.append(estimated)
            except Exception:
                pass

    def get_metrics(self) -> Dict[str, BenchmarkResult]:
        metrics = {}

        if self.power_readings:
            powers = list(self.power_readings)
            avg_power = sum(powers) / len(powers)
            peak_power = max(powers)

            metrics["avg_power_w"] = BenchmarkResult(
                metric_name="avg_power_w", value=avg_power, unit="watts"
            )
            metrics["peak_power_w"] = BenchmarkResult(
                metric_name="peak_power_w", value=peak_power, unit="watts"
            )
            metrics["power_efficiency"] = BenchmarkResult(
                metric_name="power_efficiency",
                value=self.target_power_w / max(avg_power, 0.1),
                unit="target_ratio",
                metadata={"target_w": self.target_power_w}
            )
            metrics["power_target_met"] = BenchmarkResult(
                metric_name="power_target_met",
                value=1.0 if avg_power <= self.target_power_w * 1.2 else 0.0,
                unit="boolean",
            )

        if self.temp_readings:
            temps = list(self.temp_readings)
            metrics["avg_temp_c"] = BenchmarkResult(
                metric_name="avg_temp_c",
                value=sum(temps) / len(temps),
                unit="celsius",
            )
            metrics["thermal_throttle_risk"] = BenchmarkResult(
                metric_name="thermal_throttle_risk",
                value=1.0 if max(temps) > 85 else 0.0,
                unit="boolean",
            )

        return metrics


class CathedralBenchmarkSuite:
    """
    Suite completa de benchmarks para o Substrato 3000.
    Integra todas as métricas de avaliação.
    """
    def __init__(self, target_power_w: float = 20.0):
        self.sample_efficiency = SampleEfficiencyTracker()
        self.forgetting = CatastrophicForgettingMonitor()
        self.causal = CausalDiscoveryEvaluator()
        self.thermal = ThermalPowerMonitor(target_power_w=target_power_w)
        self._all_results: List[BenchmarkResult] = []

    def record_episode(self, episode_return: float, episode_length: int):
        self.sample_efficiency.record_episode(episode_return, episode_length)

    def record_task_performance(self, task_id: int, performance: float, step: int):
        self.forgetting.record_performance(task_id, performance, step)

    def record_causal_edge(self, cause: str, effect: str, confidence: float = 1.0):
        self.causal.add_predicted_edge(cause, effect, confidence)

    def set_causal_ground_truth(self, edges: List[Tuple[str, str]]):
        self.causal.set_ground_truth(edges)

    def record_thermal(self, power_w: Optional[float] = None, temp_c: Optional[float] = None):
        self.thermal.record(power_w, temp_c)

    def run_full_evaluation(self) -> Dict[str, BenchmarkResult]:
        """Executa avaliação completa e retorna todas as métricas."""
        all_metrics = {}

        all_metrics.update(self.sample_efficiency.get_metrics())
        all_metrics.update(self.forgetting.get_forgetting_metrics())
        all_metrics.update(self.causal.evaluate())
        all_metrics.update(self.thermal.get_metrics())

        # Composite score
        scores = []
        if "sample_efficiency" in all_metrics:
            scores.append(min(all_metrics["sample_efficiency"].value * 10, 1.0))
        if "causal_f1" in all_metrics:
            scores.append(all_metrics["causal_f1"].value)
        if "power_efficiency" in all_metrics:
            scores.append(min(all_metrics["power_efficiency"].value, 1.0))
        if "forgetting_rate" in all_metrics:
            scores.append(1.0 - all_metrics["forgetting_rate"].value)

        if scores:
            all_metrics["composite_score"] = BenchmarkResult(
                metric_name="composite_score",
                value=sum(scores) / len(scores),
                unit="normalized",
                metadata={"components": len(scores)}
            )

        self._all_results.extend(all_metrics.values())
        return all_metrics

    def get_report(self) -> str:
        """Gera relatório textual de benchmark."""
        metrics = self.run_full_evaluation()
        lines = ["=" * 60, "  CATHEDRAL ARKHE v16.0.0 — BENCHMARK REPORT", "=" * 60]

        for name, result in sorted(metrics.items()):
            lines.append(f"  {name:30s}: {result.value:10.4f} {result.unit}")

        lines.append("=" * 60)
        return "\n".join(lines)
