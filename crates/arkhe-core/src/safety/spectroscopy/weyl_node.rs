// crates/arkhe-core/src/safety/spectroscopy/weyl_node.rs
//! ARKHE-χ Fase 4 — Detecção de "Nós de Weyl" no Espaço de Parâmetros
//!
//! No artigo, nós de Weyl são pontos onde bandas de ABS cruzam,
//! com cargas topológicas ±1 (singleto) ou ±2 (dubletos).
//!
//! No ARKHE, um "WeylNode" é um ponto no espaço de parâmetros onde
//! um invariante falha ao sair do manifold — análogo a crossing.

use crate::safety::symmetry_generator::{SystemState, TransitionSafety};

/// "Nó de Weyl" no espaço de parâmetros de software
///
/// Representa um ponto onde um invariante falha ao perturbar
/// ligeiramente fora do Safety Manifold.
#[derive(Debug, Clone)]
pub struct WeylNode {
    /// Localização no espaço de parâmetros
    pub location: SystemState,

    /// Carga topológica: +1 (violação emerge ao sair) ou -1 (violação resolve ao entrar)
    pub charge: i32,

    /// ID do invariante que falha neste nó
    pub invariant_id: String,

    /// "Tamanho do gap" — quão rápido a falha se propaga ao perturbar
    /// Valor pequeno = gap abre rapidamente (falha aguda)
    /// Valor grande = gap abre lentamente (falha gradual)
    pub gap_size: f64,

    /// Direção da violação: Emerging (sai do manifold) ou Resolving (entra)
    pub direction: ViolationDirection,

    /// Timestamp da detecção
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationDirection {
    /// A violação emerge ao sair do manifold (carga +1)
    Emerging,
    /// A violação desaparece ao entrar no manifold (carga -1)
    Resolving,
}

/// Detector de nós de Weyl
///
/// Varre o espaço de parâmetros procurando pontos onde invariantes
/// falham de forma isolada — característica de singularidades topológicas.
pub struct WeylNodeDetector;

impl WeylNodeDetector {
    /// Detecta nós de Weyl em uma coleção de estados testados
    ///
    /// Algoritmo:
    /// 1. Para cada estado, verificar se está no manifold
    /// 2. Se estiver, perturbar ligeiramente em cada direção
    /// 3. Se a perturbação causar violação isolada → Weyl node
    /// 4. Computar carga e gap
    pub fn detect_nodes(
        states: &[SystemState],
        perturbation_delta: f64,
        check_fn: &dyn Fn(&SystemState) -> TransitionSafety,
    ) -> Vec<WeylNode> {
        let mut nodes = Vec::new();

        for state in states {
            // Verificar se o estado base está no manifold
            let base_result = check_fn(state);

            // Só procuramos nós próximos ao manifold
            let is_near_manifold = matches!(
                base_result,
                TransitionSafety::Safe | TransitionSafety::Degraded { .. }
            );

            if !is_near_manifold {
                continue;
            }

            // Perturbar em múltiplas direções
            let directions = 8; // 8 direções no espaço de parâmetros
            let mut violations_found = Vec::new();

            for dir in 0..directions {
                let angle = 2.0 * std::f64::consts::PI * (dir as f64 / directions as f64);
                let mut perturbed = state.clone();

                // Perturbação paramétrica
                perturbed.token_budget += (perturbation_delta * angle.cos()) as i64;
                perturbed.agent_count = ((perturbed.agent_count as f64 + perturbation_delta * angle.sin()) as u32).max(0);

                let perturbed_result = check_fn(&perturbed);

                if let TransitionSafety::CriticalEscape { violation, .. } = perturbed_result {
                    violations_found.push((dir, violation, perturbed.clone()));
                }
            }

            // Se encontramos violações isoladas em direções específicas,
            // caracterizamos como nós de Weyl
            for (dir, violation, perturbed) in &violations_found {
                // Carga: +1 se violação emerge, -1 se resolve
                let charge = Self::compute_charge(&base_result, check_fn, state, perturbed);

                // Gap: distância até a fronteira / taxa de recuperação
                let gap_size = Self::compute_gap(state, perturbed, perturbation_delta);

                // Direção
                let direction = if charge > 0 {
                    ViolationDirection::Emerging
                } else {
                    ViolationDirection::Resolving
                };

                nodes.push(WeylNode {
                    location: perturbed.clone(),
                    charge,
                    invariant_id: format!("VIOLATION-{}", dir),
                    gap_size,
                    direction,
                    detected_at: chrono::Utc::now(),
                });
            }
        }

        nodes
    }

    /// Computa a carga topológica de uma violação
    fn compute_charge(
        base_result: &TransitionSafety,
        check_fn: &dyn Fn(&SystemState) -> TransitionSafety,
        base: &SystemState,
        perturbed: &SystemState,
    ) -> i32 {
        // Perturbar na direção oposta
        let mut opposite = base.clone();
        opposite.token_budget = 2 * base.token_budget - perturbed.token_budget;
        opposite.agent_count = (2 * base.agent_count as i64 - perturbed.agent_count as i64) as u32;

        let opposite_result = check_fn(&opposite);

        match (base_result, opposite_result) {
            (TransitionSafety::Safe, TransitionSafety::CriticalEscape { .. }) => 1,
            (TransitionSafety::CriticalEscape { .. }, TransitionSafety::Safe) => -1,
            _ => {
                // Determinístico via hash do estado
                let hash = base.token_budget.wrapping_add(base.agent_count as i64) as i32;
                if hash % 2 == 0 { 1 } else { -1 }
            }
        }
    }

    /// Computa o "gap" — quão rápido o sistema se recupera
    fn compute_gap(base: &SystemState, perturbed: &SystemState, delta: f64) -> f64 {
        let dist = ((base.token_budget - perturbed.token_budget).pow(2) as f64
            + (base.agent_count as i64 - perturbed.agent_count as i64).pow(2) as f64)
            .sqrt();

        if dist < 1e-10 {
            1.0 // Gap máximo (recuperação instantânea)
        } else {
            (delta / dist).min(1.0)
        }
    }
}
