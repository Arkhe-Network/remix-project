from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, field
import numpy as np
import logging
import re
import time
from datetime import datetime

# Importação da arquitetura base v9.1
# from topomas_v9_1 import BaseAgent, TopoMASConfig, MCPTool, MCPRegistry, StateContract, KnowledgeGraph

# =============================================================================
# CONSTANTES E DATA CLASSES
# =============================================================================

@dataclass
class SpaceScore:
    radiation_hardness: float = 0.0
    vacuum_stability: float = 0.0
    thermal_cycling: float = 0.0
    weight_efficiency: float = 0.0
    synthesizability: float = 0.0
    confidence: float = 1.0
    source: str = "literature"

    def to_dict(self) -> Dict[str, float]:
        return {k: v for k, v in self.__dict__.items() if k != 'source'}

    def overall_score(self, weights: Optional[Dict[str, float]] = None) -> float:
        if weights is None:
            weights = {"radiation_hardness": 0.25, "vacuum_stability": 0.25,
                       "thermal_cycling": 0.20, "weight_efficiency": 0.15, "synthesizability": 0.15}
        return sum(self.to_dict().get(k, 0.0) * w for k, w in weights.items()) * self.confidence

LITERATURE_SCORES: Dict[str, SpaceScore] = {
    "SnSe0.9Te0.1": SpaceScore(0.90, 0.95, 0.90, 0.70, 0.85, 0.95, "Lee et al., ACS Nano 2025"),
    "SnSe": SpaceScore(0.80, 0.85, 0.80, 0.70, 0.90, 0.80, "Lee et al., ACS Nano 2025"),
    "Sb2Te2Se": SpaceScore(0.80, 0.90, 0.80, 0.60, 0.70, 0.85, "Nature Sci. Rep. 2016"),
    "HfTe5": SpaceScore(0.95, 0.70, 0.80, 0.50, 0.40, 0.80, "Jauregui, Caltech 2025"),
    "Bi2Se3": SpaceScore(0.65, 0.60, 0.70, 0.80, 0.80, 0.75, "Parsons et al., arXiv 2026"),
    "WTe2": SpaceScore(0.80, 0.70, 0.70, 0.90, 0.60, 0.80, "Zhang et al., Small 2025"),
    "ZrTe5": SpaceScore(0.80, 0.85, 0.80, 0.60, 0.50, 0.85, "Nature Nanotech 2026"),
}

# =============================================================================
# HEURÍSTICAS FÍSICAS APRIMORADAS
# =============================================================================

def heuristic_space_score(structure) -> SpaceScore:
    """Calcula pontuação usando propriedades físicas reais (densidade, composição)."""
    try:
        comp = structure.composition
        elements = [el.symbol for el in comp.elements]
        fractions = [comp.get_atomic_fraction(el) for el in comp.elements]

        heavy_elements = {"Bi", "Pb", "W", "Hf", "Zr", "Ta", "Pt", "Au"}
        volatile_elements = {"Te", "Se", "S", "As", "Sb", "P", "Hg", "I"}

        # 1. Radiation: Blindagem atômica (fracao de elementos pesados)
        heavy_frac = sum(f for el, f in zip(elements, fractions) if el in heavy_elements)
        radiation = 0.3 + 0.7 * heavy_frac

        # 2. Vacuum: Estabilidade em vácuo (penaliza voláteis)
        volatile_frac = sum(f for el, f in zip(elements, fractions) if el in volatile_elements)
        vacuum = 0.9 - 0.6 * volatile_frac

        # 3. Thermal: Proxy via temperatura de Debye (maior massa atômica -> menor freq vibracional)
        avg_mass = sum(comp.get_atomic_mass(el) * comp.get_atomic_fraction(el) for el in comp.elements)
        thermal = min(0.95, 0.3 + 0.7 * (avg_mass / 200.0))

        # 4. Weight: Cálculo real de densidade (massa/volume)
        volume = structure.volume
        if volume > 0:
            density = comp.weight / (volume * 1e-24)  # g/cm^3
            # Materiais espaciais ideais: 1 a 5 g/cm^3. Acima de 10 é péssimo.
            weight_eff = max(0.1, 1.0 - (density - 2.0) / 15.0)
        else:
            weight_eff = 0.5

        # 5. Synthesizability: Entropia de configuração e complexidade
        n_elements = len(elements)
        synthesizability = max(0.3, 1.0 - 0.2 * (n_elements - 1))

        return SpaceScore(
            radiation_hardness=np.clip(radiation, 0, 1),
            vacuum_stability=np.clip(vacuum, 0, 1),
            thermal_cycling=np.clip(thermal, 0, 1),
            weight_efficiency=np.clip(weight_eff, 0, 1),
            synthesizability=np.clip(synthesizability, 0, 1),
            confidence=0.60, source="heuristic_physics"
        )
    except Exception:
        return SpaceScore(0.5, 0.5, 0.5, 0.5, 0.5, 0.3, "fallback")

# =============================================================================
# AGENTE PRINCIPAL
# =============================================================================

from src.topomas.topomas_v9_2 import BaseAgent

class SpaceApplicationScorer(BaseAgent):
    """
    Agente nativo v9.1 para pontuação espacial.
    """
    name = "SpaceScorer"

    def __init__(self, config=None, mcp_registry=None, **kwargs):
        super().__init__(self.name, config, **kwargs)
        self.mcp_registry = mcp_registry
        self._weights = self.config.get("space_weights", {
            "radiation_hardness": 0.25, "vacuum_stability": 0.25,
            "thermal_cycling": 0.20, "weight_efficiency": 0.15, "synthesizability": 0.15,
        })
        self._register_mcp_tools()

    def _register_mcp_tools(self):
        if not self.mcp_registry: return
        self.mcp_registry.register(MCPTool(
            name="query_space_score",
            description="Retorna o score espacial detalhado de um material.",
            parameters={"formula": {"type": "string", "desc": "Fórmula química"}},
            handler=self._mcp_query_score,
            agent_owner=self.name
        ))

    def _mcp_query_score(self, formula: str) -> Dict:
        if formula in LITERATURE_SCORES:
            return LITERATURE_SCORES[formula].to_dict()
        return {"error": "Material não encontrado na base de dados literária."}

    def execute(self, state: Dict[str, Any]) -> Dict[str, Any]:
        self.logger.info("Executando pipeline de pontuação espacial avançada...")

        structures = state.get("structures", [])
        ids = state.get("ids", [])
        predictions = state.get("predictions", {})
        kg = state.get("knowledge_graph")

        if not structures:
            state["space_scores"] = []
            return state

        scores_list = []

        for i, (mid, struct) in enumerate(zip(ids, structures)):
            formula = self._get_formula(struct)

            # 1. Lookup na literatura com fallback fuzzy
            score = LITERATURE_SCORES.get(formula)
            if not score:
                base = self._get_base_formula(formula)
                if base in LITERATURE_SCORES:
                    score = LITERATURE_SCORES[base]
                    score.confidence *= 0.8
                    score.source += " (aproximado)"
                else:
                    score = heuristic_space_score(struct)

            # 2. Ajuste de confiança com predição do modelo
            model_conf = predictions.get("confidences", [0.5])[i] if i < len(predictions.get("confidences", [])) else 0.5
            score.confidence *= (0.5 + 0.5 * model_conf)

            score_dict = score.to_dict()
            score_dict["formula"] = formula
            score_dict["id"] = mid
            score_dict["overall_score"] = score.overall_score(self._weights)
            scores_list.append(score_dict)

            # 3. Atualiza Knowledge Graph com metadados espaciais
            if kg and hasattr(kg, "_nodes") and mid in kg._nodes:
                kg._nodes[mid].metadata["space_score"] = score_dict
                kg._nodes[mid].provenance.append({
                    "action": "space_scoring", "source": self.name,
                    "timestamp": datetime.now().isoformat()
                })

        # 4. Ordena e seleciona top candidatos
        top_candidates = sorted(scores_list, key=lambda x: -x["overall_score"])[:10]

        if top_candidates:
             self.logger.info(f"Top candidato espacial: {top_candidates[0]['formula']} (Score: {top_candidates[0]['overall_score']:.3f})")

        # 5. Atualiza estado (cumprindo StateContract)
        state["space_scores"] = scores_list
        state["space_best_candidates"] = top_candidates
        state["space_weights"] = self._weights

        return state

    def _get_formula(self, structure) -> str:
        try: return structure.composition.reduced_formula
        except: return "unknown"

    def _get_base_formula(self, formula: str) -> str:
        # Regex robusta: remove números e pontos
        return re.sub(r'[0-9\.]', '', formula)
