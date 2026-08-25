// crates/arkhe-core/src/safety/escape_detector.rs
//! ARKHE-χ Fase 3 — Escape Detector
//!
//! Detecta quando o sistema se aproxima ou escapa do Safety Manifold ℳ_safe.
//! Inspirado no "escape para o contínuo" de Zalom et al. (arXiv:2608.21200v1),
//! onde dubletos escapam para o contínuo quando |χ| ≈ 1.
//!
//! No ARKHE, identificamos 4 regiões:
//!   Safe       — dentro de ℳ_safe com margem confortável
//!   Boundary   — próximo à fronteira (|χ| ≈ 1 analog)
//!   Continuum  — fora de ℳ_safe — escape detectado
//!   Recoverable — fora, mas transição reversível possível

use std::collections::VecDeque;
use tracing::{debug, warn};

use crate::safety::symmetry_generator::{SystemState, SystemConfig, SymmetryGenerator};

/// Detector de escape para o contínuo
///
/// Monitora o estado do sistema e classifica sua posição relativa ao
/// Safety Manifold. Usa histórico de snapshots para detectar padrões
/// de cascade failure.
pub struct EscapeDetector {
    /// Histórico de snapshots para análise de tendência
    history: VecDeque<SystemSnapshot>,
    /// Limites configuráveis para classificação de região
    thresholds: EscapeThresholds,
    /// Tamanho máximo do histórico (janela de análise)
    history_window: usize,
    /// Gerador de simetria para computar margens
    generator: SymmetryGenerator,
}

/// Limites configuráveis para detecção de escape
///
/// Cada threshold representa uma "fronteira de fase" no espaço de parâmetros.
#[derive(Debug, Clone)]
pub struct EscapeThresholds {
    /// Gap mínimo para considerar estado "Safe" (acima = Safe, abaixo = Boundary)
    /// Analogia: |χ| < 0.9 no artigo
    pub safe_margin: f64,

    /// Gap mínimo para considerar estado "Boundary" (acima = Boundary, abaixo = Continuum)
    /// Analogia: |χ| ≈ 1 no artigo
    pub boundary_margin: f64,

    /// Taxa de divergência de tokens que indica escape não-linear
    /// Ex: 0.95 = 95% do budget consumido em 1 ciclo
    pub token_divergence_rate: f64,

    /// Fração de agentes máximos que indica massa crítica
    /// Ex: 0.9 = 90% de MAX_AGENTS
    pub agent_critical_fraction: f64,

    /// Entropia mínima como fração do requerido
    /// Ex: 0.5 = 50% de MIN_ENTROPY
    pub entropy_critical_fraction: f64,

    /// Taxa de violação de invariantes que indica cascade
    /// Ex: 0.1 = 10% das transições na janela violam invariantes
    pub violation_cascade_rate: f64,

    /// Número mínimo de snapshots para análise de cascade
    pub min_window_size: usize,
}

impl Default for EscapeThresholds {
    fn default() -> Self {
        Self {
            safe_margin: 0.2,           // 20% de margem = Safe
            boundary_margin: 0.05,      // 5% de margem = Boundary
            token_divergence_rate: 0.95, // 95% do budget em 1 ciclo
            agent_critical_fraction: 0.9, // 90% de MAX_AGENTS
            entropy_critical_fraction: 0.5, // 50% de MIN_ENTROPY
            violation_cascade_rate: 0.1, // 10% das transições
            min_window_size: 5,         // Mínimo 5 snapshots
        }
    }
}

/// Região do espaço de parâmetros onde o sistema está operando
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EscapeRegion {
    /// Dentro de ℳ_safe com margem confortável
    /// Analogia: |χ| << 1 (longe dos nós de Weyl)
    Safe,

    /// Próximo à fronteira do manifold
    /// Analogia: |χ| ≈ 1 (próximo aos nós de Weyl)
    Boundary,

    /// Fora de ℳ_safe — escape detectado
    /// Analogia: estado deixou de ser bound (contínuo)
    Continuum,

    /// Fora de ℳ_safe, mas recuperação possível
    /// Analogia: estado ainda pode ser "rebound" com ajuste de parâmetros
    Recoverable,
}

impl std::fmt::Display for EscapeRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EscapeRegion::Safe => write!(f, "SAFE"),
            EscapeRegion::Boundary => write!(f, "BOUNDARY"),
            EscapeRegion::Continuum => write!(f, "CONTINUUM"),
            EscapeRegion::Recoverable => write!(f, "RECOVERABLE"),
        }
    }
}

/// Snapshot do estado do sistema para análise histórica
#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub timestamp: std::time::Instant,
    pub state: SystemState,
    pub region: EscapeRegion,
    pub margin: f64,
    pub violations: Vec<String>,
}

/// Alerta de cascade failure
#[derive(Debug, Clone)]
pub struct CascadeAlert {
    /// Severidade do alerta
    pub severity: CascadeSeverity,
    /// Taxa de violação na janela
    pub violation_rate: f64,
    /// Divergência de recursos
    pub divergence: f64,
    /// Ação recomendada
    pub recommended_action: RecommendedAction,
    /// Timestamp do alerta
    pub timestamp: std::time::Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CascadeSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecommendedAction {
    /// Nenhuma ação necessária
    None,
    /// Reduzir carga (throttling)
    Throttle,
    /// Abrir circuito (circuit break)
    CircuitBreak,
    /// Reinicialização controlada
    Restart,
}

impl EscapeDetector {
    /// Cria um novo detector com thresholds padrão
    pub fn new(thresholds: EscapeThresholds, generator: SymmetryGenerator) -> Self {
        Self {
            history: VecDeque::with_capacity(thresholds.min_window_size * 2),
            history_window: thresholds.min_window_size * 2,
            thresholds,
            generator,
        }
    }

    /// Classifica a região do espaço de parâmetros para um estado
    ///
    /// Usa o gap espectral computado pelo SymmetryGenerator para determinar
    /// a distância até a fronteira do manifold.
    pub fn classify_region(&self, state: &SystemState) -> EscapeRegion {
        let margin = self.generator.compute_spectral_gap(state);

        debug!("Classifying region: margin = {:.4}", margin);

        if margin > self.thresholds.safe_margin {
            EscapeRegion::Safe
        } else if margin > self.thresholds.boundary_margin {
            EscapeRegion::Boundary
        } else if self.is_recoverable(state) {
            EscapeRegion::Recoverable
        } else {
            EscapeRegion::Continuum
        }
    }

    /// Registra um snapshot do estado atual
    ///
    /// Deve ser chamado a cada ciclo do agente para manter o histórico
    /// atualizado para detecção de cascade.
    pub fn record_snapshot(&mut self, state: &SystemState) {
        let margin = self.generator.compute_spectral_gap(state);
        let region = self.classify_region(state);
        let violations = self.collect_violations(state);

        let snapshot = SystemSnapshot {
            timestamp: std::time::Instant::now(),
            state: state.clone(),
            region,
            margin,
            violations,
        };

        if self.history.len() >= self.history_window {
            self.history.pop_front();
        }
        self.history.push_back(snapshot);

        debug!(
            "Snapshot recorded: region={}, margin={:.4}, violations={}",
            region, margin, self.history.back().map(|s| s.violations.len()).unwrap_or(0)
        );
    }

    /// Detecta cascade failure — análogo ao escape para o contínuo
    ///
    /// Analisa a janela de histórico para detectar padrões de falha
    /// em cascata: violações crescentes + divergência de recursos.
    pub fn detect_cascade(&self) -> Option<CascadeAlert> {
        if self.history.len() < self.thresholds.min_window_size {
            return None;
        }

        let window: Vec<_> = self.history.iter().rev().take(self.thresholds.min_window_size).collect();

        let violation_rate = self.compute_violation_rate(&window);
        let divergence = self.compute_divergence(&window);
        let region_trend = self.compute_region_trend(&window);

        debug!(
            "Cascade analysis: violation_rate={:.2}, divergence={:.2}, trend={:?}",
            violation_rate, divergence, region_trend
        );

        // Cascade crítico: alta taxa de violação + alta divergência
        if violation_rate > self.thresholds.violation_cascade_rate
            && divergence > self.thresholds.token_divergence_rate {
            Some(CascadeAlert {
                severity: CascadeSeverity::Critical,
                violation_rate,
                divergence,
                recommended_action: RecommendedAction::CircuitBreak,
                timestamp: std::time::Instant::now(),
            })
        }
        // Warning: taxa de violação moderada ou aproximação da fronteira
        else if violation_rate > self.thresholds.violation_cascade_rate * 0.5
            || region_trend == RegionTrend::ApproachingBoundary {
            Some(CascadeAlert {
                severity: CascadeSeverity::Warning,
                violation_rate,
                divergence,
                recommended_action: RecommendedAction::Throttle,
                timestamp: std::time::Instant::now(),
            })
        }
        // Info: tendência de degradação
        else if region_trend == RegionTrend::Degrading {
            Some(CascadeAlert {
                severity: CascadeSeverity::Info,
                violation_rate,
                divergence,
                recommended_action: RecommendedAction::None,
                timestamp: std::time::Instant::now(),
            })
        }
        else {
            None
        }
    }

    /// Retorna o histórico de snapshots
    pub fn history(&self) -> &VecDeque<SystemSnapshot> {
        &self.history
    }

    /// Retorna o snapshot mais recente
    pub fn latest_snapshot(&self) -> Option<&SystemSnapshot> {
        self.history.back()
    }

    /// Limpa o histórico
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    // ─── Helpers privados ─────────────────────────────────────────

    /// Verifica se um estado fora do manifold é recuperável
    fn is_recoverable(&self, state: &SystemState) -> bool {
        // Recuperável se apenas invariantes HIGH estão violados
        // e nenhum CRITICAL está violado
        let result = self.generator.is_in_manifold(state);
        match result {
            crate::safety::symmetry_generator::ManifoldResult::Outside { violation, .. } => {
                // Se a violação for só de invariantes HIGH, é recuperável
                !matches!(violation, crate::safety::symmetry_generator::ViolationType::Critical { .. })
            }
            _ => false, // Não está fora, então não é recuperável neste contexto
        }
    }

    /// Coleta os IDs dos invariantes violados
    fn collect_violations(&self, state: &SystemState) -> Vec<String> {
        let mut violations = Vec::new();
        for inv in self.generator.invariants() {
            if !inv.check(state) {
                violations.push(inv.id().to_string());
            }
        }
        violations
    }

    /// Computa a taxa de violação na janela
    fn compute_violation_rate(&self, window: &[&SystemSnapshot]) -> f64 {
        if window.is_empty() {
            return 0.0;
        }
        let violating = window.iter().filter(|s| !s.violations.is_empty()).count();
        violating as f64 / window.len() as f64
    }

    /// Computa a divergência de recursos na janela
    fn compute_divergence(&self, window: &[&SystemSnapshot]) -> f64 {
        if window.len() < 2 {
            return 0.0;
        }

        let first = &window.first().unwrap().state;
        let last = &window.last().unwrap().state;

        // Divergência de tokens como fração do budget inicial
        let token_divergence = if first.config.max_tokens > 0 {
            let delta = (first.token_budget - last.token_budget).abs() as f64;
            delta / first.config.max_tokens as f64
        } else {
            0.0
        };

        // Divergência de agentes
        let agent_divergence = if first.config.max_agents > 0 {
            let delta = (first.agent_count as i64 - last.agent_count as i64).abs() as f64;
            delta / first.config.max_agents as f64
        } else {
            0.0
        };

        // Máxima divergência
        token_divergence.max(agent_divergence)
    }

    /// Computa a tendência de região na janela
    fn compute_region_trend(&self, window: &[&SystemSnapshot]) -> RegionTrend {
        if window.len() < 2 {
            return RegionTrend::Stable;
        }

        let first = window.first().unwrap().region;
        let last = window.last().unwrap().region;

        match (first, last) {
            (EscapeRegion::Safe, EscapeRegion::Boundary) => RegionTrend::ApproachingBoundary,
            (EscapeRegion::Safe, EscapeRegion::Continuum) => RegionTrend::Degrading,
            (EscapeRegion::Boundary, EscapeRegion::Continuum) => RegionTrend::Degrading,
            (EscapeRegion::Continuum, EscapeRegion::Recoverable) => RegionTrend::Recovering,
            (EscapeRegion::Continuum, EscapeRegion::Safe) => RegionTrend::Recovering,
            _ => RegionTrend::Stable,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RegionTrend {
    Stable,
    ApproachingBoundary,
    Degrading,
    Recovering,
}
