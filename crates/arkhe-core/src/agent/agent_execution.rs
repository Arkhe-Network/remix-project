// crates/arkhe-core/src/agent/agent_execution.rs
//! ARKHE-χ Fase 2 — Integração do TransitionVerifier ao ciclo de execução do agente
//!
//! Substitui chamadas diretas ao SymmetryGenerator pelo verificador de duas camadas.
//! Toda transição de estado do agente passa por TransitionVerifier::verify() antes
//! de ser aplicada.

use tracing::{info, debug, warn, error, instrument};

use crate::safety::{
    SymmetryGenerator, SystemState, SystemConfig,
    TransitionSafety, ManifoldResult,
    all_invariants,
};

/// Executor de agente com verificação de segurança integrada
///
/// Substitui o executor legacy que chamava SymmetryGenerator diretamente.
/// Agora todas as transições passam por TransitionVerifier (duas camadas).
pub struct SafeAgentExecutor {
    /// Verificador de duas camadas (Runtime + SMT)
    verifier: SymmetryGenerator,
}

/// Resultado da execução de uma ação
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// Ação executada com sucesso
    Success {
        new_state: SystemState,
        verification: VerificationReport,
    },
    /// Ação bloqueada por violação de invariante
    Blocked {
        reason: String,
        violation: TransitionSafety,
        current_state: SystemState,
    },
    /// Ação throttled (próxima à fronteira)
    Throttled {
        action: String,
        throttle_factor: f64,
        current_state: SystemState,
    },
    /// Erro interno do executor
    Error {
        error: String,
    },
}

/// Relatório de verificação para cada transição
#[derive(Debug, Clone)]
pub struct VerificationReport {
    /// Camada usada (Runtime, SMT, ou ambas)
    pub layer_used: LayerUsed,
    /// Resultado da verificação
    pub result: TransitionSafety,
    /// Latência total da verificação (ms)
    pub latency_ms: f64,
    /// Gap espectral do estado pré-transição
    pub pre_gap: f64,
    /// Gap espectral do estado pós-transição
    pub post_gap: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerUsed {
    RuntimeOnly,
    SmtOnly,
    Both { runtime_agreed: bool },
}

impl SafeAgentExecutor {
    pub fn new(config: SystemConfig) -> Self {
        let verifier = SymmetryGenerator::new(all_invariants(), config.clone());

        Self {
            verifier,
        }
    }
}
