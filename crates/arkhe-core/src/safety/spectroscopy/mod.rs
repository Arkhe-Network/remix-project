// crates/arkhe-core/src/safety/spectroscopy/mod.rs
//! ARKHE-χ Fase 4 — Invariant Spectroscopy (asi-evals)

pub mod probe_strategies;
pub mod weyl_node;

use std::sync::Arc;
use chrono::Utc;

use crate::safety::symmetry_generator::{SystemState, SystemConfig, SymmetryGenerator, TransitionSafety};
use crate::safety::escape_detector::EscapeDetector;
use crate::safety::spectroscopy::probe_strategies::ProbeStrategy;
use crate::safety::spectroscopy::weyl_node::{WeylNode, WeylNodeDetector};

pub struct InvariantSpectroscopy {
    generator: Arc<SymmetryGenerator>,
    escape_detector: EscapeDetector,
    pub strategies: Vec<Box<dyn ProbeStrategy>>,
    config: SpectroscopyConfig,
    stats: SpectroscopyStats,
}

#[derive(Debug, Clone)]
pub struct SpectroscopyConfig {
    pub perturbation_delta: f64,
    pub max_states_per_sweep: usize,
    pub stop_on_first_node: bool,
    pub sweep_timeout_ms: u64,
}

impl Default for SpectroscopyConfig {
    fn default() -> Self {
        Self {
            perturbation_delta: 10.0,
            max_states_per_sweep: 1000,
            stop_on_first_node: false,
            sweep_timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpectroscopyStats {
    pub total_sweeps: u64,
    pub total_states_tested: u64,
    pub total_violations_found: u64,
    pub total_weyl_nodes: u64,
    pub avg_sweep_time_ms: f64,
}

#[derive(Debug, Clone)]
pub struct SpectroscopyResult {
    pub probe_id: String,
    pub strategy_name: String,
    pub states_tested: usize,
    pub violations: Vec<ViolationEvent>,
    pub weyl_nodes: Vec<WeylNode>,
    pub phase_summary: PhaseSummary,
    pub execution_time_ms: f64,
}

#[derive(Debug, Clone)]
pub struct ViolationEvent {
    pub state: SystemState,
    pub invariant_id: String,
    pub distance_from_manifold: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct PhaseSummary {
    pub safe_count: usize,
    pub boundary_count: usize,
    pub continuum_count: usize,
    pub recoverable_count: usize,
    pub weyl_node_density: f64,
}

impl InvariantSpectroscopy {
    pub fn new(
        generator: Arc<SymmetryGenerator>,
        escape_detector: EscapeDetector,
        config: SpectroscopyConfig,
    ) -> Self {
        Self {
            generator,
            escape_detector,
            strategies: Vec::new(),
            config,
            stats: SpectroscopyStats::default(),
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn ProbeStrategy>) {
        self.strategies.push(strategy);
    }

    pub async fn run_sweep(
        &mut self,
        strategy: &dyn ProbeStrategy,
        base_config: &SystemConfig,
    ) -> Result<SpectroscopyResult, SpectroscopyError> {
        let start = std::time::Instant::now();
        let probe_id = format!("spectroscopy-{}", uuid::Uuid::new_v4());

        let states = strategy.generate_states(base_config);
        let states = states.into_iter()
            .take(self.config.max_states_per_sweep)
            .collect::<Vec<_>>();

        let mut violations = Vec::new();
        let mut phase_summary = PhaseSummary::default();

        for state in &states {
            let region = self.escape_detector.classify_region(state);

            match region {
                crate::safety::escape_detector::EscapeRegion::Safe => phase_summary.safe_count += 1,
                crate::safety::escape_detector::EscapeRegion::Boundary => phase_summary.boundary_count += 1,
                crate::safety::escape_detector::EscapeRegion::Continuum => phase_summary.continuum_count += 1,
                crate::safety::escape_detector::EscapeRegion::Recoverable => phase_summary.recoverable_count += 1,
            }

            let result = self.generator.is_in_manifold(state);
            if let crate::safety::ManifoldResult::Outside { violation, .. } = result {
                violations.push(ViolationEvent {
                    state: state.clone(),
                    invariant_id: format!("{:?}", violation),
                    distance_from_manifold: self.escape_detector.classify_region(state) as i32 as f64,
                    timestamp: Utc::now(),
                });
            }
        }

        let check_fn = |s: &SystemState| -> TransitionSafety {
            self.generator.preserves_manifold(s, s)
        };

        let weyl_nodes = WeylNodeDetector::detect_nodes(
            &states,
            self.config.perturbation_delta,
            &check_fn,
        );

        phase_summary.weyl_node_density = if states.is_empty() {
            0.0
        } else {
            weyl_nodes.len() as f64 / states.len() as f64
        };

        let execution_time = start.elapsed().as_secs_f64() * 1000.0;

        self.stats.total_sweeps += 1;
        self.stats.total_states_tested += states.len() as u64;
        self.stats.total_violations_found += violations.len() as u64;
        self.stats.total_weyl_nodes += weyl_nodes.len() as u64;
        self.stats.avg_sweep_time_ms =
            (self.stats.avg_sweep_time_ms * (self.stats.total_sweeps - 1) as f64 + execution_time)
            / self.stats.total_sweeps as f64;

        Ok(SpectroscopyResult {
            probe_id,
            strategy_name: strategy.name().to_string(),
            states_tested: states.len(),
            violations,
            weyl_nodes,
            phase_summary,
            execution_time_ms: execution_time,
        })
    }

    pub async fn run_all_sweeps(
        &mut self,
        base_config: &SystemConfig,
    ) -> Vec<Result<SpectroscopyResult, SpectroscopyError>> {
        let mut results = Vec::new();

        for strategy in std::mem::take(&mut self.strategies) {
            let result = self.run_sweep(strategy.as_ref(), base_config).await;
            results.push(result);
            self.strategies.push(strategy); // put it back
        }

        results
    }

    pub fn stats(&self) -> &SpectroscopyStats {
        &self.stats
    }

    pub fn generate_report(&self, results: &[SpectroscopyResult]) -> String {
        let mut report = String::new();
        report.push_str("# ARKHE-χ Invariant Spectroscopy Report\n");
        report.push_str(&format!("Generated: {}\n\n", Utc::now()));

        report.push_str("## Summary\n\n");
        report.push_str(&format!("- Total sweeps: {}\n", results.len()));
        report.push_str(&format!("- Total states tested: {}\n",
            results.iter().map(|r| r.states_tested).sum::<usize>()));
        report.push_str(&format!("- Total violations: {}\n",
            results.iter().map(|r| r.violations.len()).sum::<usize>()));
        report.push_str(&format!("- Total Weyl nodes: {}\n",
            results.iter().map(|r| r.weyl_nodes.len()).sum::<usize>()));

        report.push_str("\n## Phase Distribution\n\n");
        for result in results {
            report.push_str(&format!("### {}\n", result.strategy_name));
            report.push_str(&format!("- Safe: {}\n", result.phase_summary.safe_count));
            report.push_str(&format!("- Boundary: {}\n", result.phase_summary.boundary_count));
            report.push_str(&format!("- Continuum: {}\n", result.phase_summary.continuum_count));
            report.push_str(&format!("- Recoverable: {}\n", result.phase_summary.recoverable_count));
            report.push_str(&format!("- Weyl node density: {:.4}\n\n", result.phase_summary.weyl_node_density));
        }

        report
    }
}

#[derive(Debug)]
pub enum SpectroscopyError {
    Timeout(u64),

    StrategyError(String),

    TooManyStates(usize),
}

impl std::fmt::Display for SpectroscopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpectroscopyError::Timeout(v) => write!(f, "Sweep timeout exceeded: {}ms", v),
            SpectroscopyError::StrategyError(s) => write!(f, "Strategy error: {}", s),
            SpectroscopyError::TooManyStates(c) => write!(f, "Too many states generated: {}", c),
        }
    }
}

impl std::error::Error for SpectroscopyError {}
