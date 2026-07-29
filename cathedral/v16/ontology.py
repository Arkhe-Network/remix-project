#!/usr/bin/env python3
"""
╔═══════════════════════════════════════════════════════════════════════════╗
║ CATHEDRAL ARKHE v16.0.0 — SUBSTRATO 3000 (Dynamic OWL + SWRL + Z3)      ║
║ Motor de restrições simbólicas para segurança de ações embodied.          ║
║ Selo: CATHEDRAL-ARKHE-v16.0.0-ONTOLOGY-2026-06-14                       ║
║ Arquiteto: ORCID 0009-0005-2697-4668                                     ║
╚═══════════════════════════════════════════════════════════════════════════╝
"""

import logging
import time
from typing import Dict, List, Optional, Set

try:
    import z3
    HAS_Z3 = True
except ImportError:
    HAS_Z3 = False

try:
    from owlready2 import *
    HAS_OWLR = True
except ImportError:
    HAS_OWLR = False

logger = logging.getLogger("cathedral.v16.ontology")


class SymbolicSafetyEngine:
    """
    Mantém uma ontologia OWL em memória, aplica regras SWRL e traduz
    o estado resultante para restrições SMT (Z3) para validação em < 5ms.
    """
    def __init__(self):
        self._has_z3 = HAS_Z3
        self._has_owl = HAS_OWLR
        self.onto = None
        self.z3_context = None
        self._entity_cache: Dict[str, Dict] = {}
        self._swrl_rules: List[Dict] = []

        if self._has_owl:
            self.onto = get_ontology("http://cathedral-arkhe.org/embodied.owl")
            self._build_ontology()
        else:
            logger.warning("owlready2 não disponível — usando fallback dict-based")
            self._build_fallback_ontology()

        if self._has_z3:
            self.z3_context = z3.Context()
        else:
            logger.warning("Z3 não disponível — validação simbólica desabilitada")

    def _build_ontology(self):
        with self.onto:
            class SpatialEntity(Thing): pass
            class Agent(Thing): pass
            class Action(Thing): pass

            class has_velocity(DataProperty, FunctionalProperty):
                domain = [SpatialEntity]
                range = [float]

            class is_fragile(ObjectProperty):
                domain = [SpatialEntity]
                range = [SpatialEntity]

            class targets(ObjectProperty):
                domain = [Action]
                range = [SpatialEntity]

    def _build_fallback_ontology(self):
        """Fallback quando owlready2 não está instalado."""
        self._fallback_classes = {"SpatialEntity", "Agent", "Action"}
        self._fallback_props = {
            "has_velocity": {"domain": "SpatialEntity", "range": float},
            "is_fragile": {"domain": "SpatialEntity", "range": "SpatialEntity"},
            "targets": {"domain": "Action", "range": "SpatialEntity"},
        }

    def update_state_from_perception(self, entities: List[Dict]):
        """Insere percepções na Ontologia."""
        if not self._has_owl:
            for ent in entities:
                self._entity_cache[ent["id"]] = ent
            return

        with self.onto:
            for ent in entities:
                obj = self.onto.SpatialEntity(ent["id"])
                obj.has_velocity = ent.get("velocity", 0.0)
                if ent.get("fragile"):
                    target = self.onto.SpatialEntity(f"fragile_{ent['id']}")
                    obj.is_fragile.append(target)

    def add_swrl_rule(self, name: str, antecedents: List[str], consequent: str, confidence: float = 0.95):
        """Adiciona regra SWRL para inferência simbólica."""
        self._swrl_rules.append({
            "name": name,
            "antecedents": antecedents,
            "consequent": consequent,
            "confidence": confidence,
        })

    def validate_action_safety(self, agent_id: str, action_name: str, target_id: str, force: float) -> bool:
        """
        Valida se uma ação do agente RL é logicamente segura usando Z3.
        Retorna True se seguro, False se proibido.
        """
        if not self._has_z3:
            # Fallback: regras simples sem Z3
            ent = self._entity_cache.get(target_id, {})
            if ent.get("fragile") and force > 1.0:
                logger.warning("Ação %s BLOQUEADA por regra de fallback (fragile + force > 1.0)", action_name)
                return False
            return True

        solver = z3.SolverFor("QF_LIA", ctx=self.z3_context)

        # Extração de fatos
        velocity = 0.0
        is_fragile = False

        if self._has_owl:
            target = self.onto.search_one(iri="*" + target_id)
            if target:
                velocity = target.has_velocity if hasattr(target, 'has_velocity') and target.has_velocity else 0.0
                is_fragile = bool(target.is_fragile) if hasattr(target, 'is_fragile') else False
        else:
            ent = self._entity_cache.get(target_id, {})
            velocity = ent.get("velocity", 0.0)
            is_fragile = ent.get("fragile", False)

        # Variáveis Z3
        z3_force = z3.Real('applied_force')
        z3_velocity = z3.Real('target_velocity')

        solver.add(z3_velocity == velocity)

        # Regras de Segurança
        if is_fragile:
            solver.add(z3.Or(z3_force <= 1.0, z3_velocity <= 5.0))

        solver.add(z3_force == force)

        start_time = time.monotonic()
        result = solver.check()
        latency_ms = (time.monotonic() - start_time) * 1000

        if result == z3.sat:
            logger.debug("Ação %s validada como SEGURA pelo Z3 (%.2fms).", action_name, latency_ms)
            return True
        else:
            logger.warning("Ação %s BLOQUEADA pelo motor simbólico (UNSAT) (%.2fms).", action_name, latency_ms)
            return False

    def infer_new_facts(self) -> List[Dict]:
        """Aplica regras SWRL para inferir novos fatos."""
        inferred = []
        for rule in self._swrl_rules:
            # Simplified forward chaining
            logger.debug("Aplicando regra SWRL: %s", rule["name"])
            inferred.append({
                "rule": rule["name"],
                "inferred": rule["consequent"],
                "confidence": rule["confidence"],
            })
        return inferred

    def get_ontology_stats(self) -> Dict:
        """Retorna estatísticas da ontologia."""
        if self._has_owl:
            return {
                "classes": len(list(self.onto.classes())),
                "individuals": len(list(self.onto.individuals())),
                "properties": len(list(self.onto.properties())),
                "swrl_rules": len(self._swrl_rules),
            }
        return {
            "entities_cached": len(self._entity_cache),
            "swrl_rules": len(self._swrl_rules),
            "mode": "fallback",
        }
