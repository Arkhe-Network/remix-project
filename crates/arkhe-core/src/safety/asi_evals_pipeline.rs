// crates/arkhe-core/src/safety/asi_evals_pipeline.rs
//! ARKHE-χ Fase 4 — Pipeline de Integração com asi-evals

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

use crate::safety::spectroscopy::{
    InvariantSpectroscopy, SpectroscopyConfig, SpectroscopyResult,
    probe_strategies::{LinearSweep, RadialSweep, ManifoldTrace, AdiabaticPerturbation},
};
use crate::safety::{
    SymmetryGenerator, SystemConfig, EscapeDetector, EscapeThresholds,
};

pub struct AsiEvalsPipeline {
    spectroscopy: Arc<RwLock<InvariantSpectroscopy>>,
    config: PipelineConfig,
    running: bool,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub sweep_interval: Duration,
    pub strategies: Vec<StrategyConfig>,
    pub weyl_node_alert_threshold: f64,
    pub report_dir: String,
}

#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub name: String,
    pub strategy_type: StrategyType,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum StrategyType {
    LinearSweep { parameter: String, range: (f64, f64), steps: usize },
    RadialSweep { radius: f64, angles: usize },
    ManifoldTrace { resolution: f64 },
    AdiabaticPerturbation { scale: f64, iterations: usize },
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            sweep_interval: Duration::from_secs(3600),
            strategies: vec![
                StrategyConfig {
                    name: "token-manifold-sweep".into(),
                    strategy_type: StrategyType::LinearSweep {
                        parameter: "token_budget".into(),
                        range: (0.0, 10000.0),
                        steps: 100,
                    },
                    enabled: true,
                },
            ],
            weyl_node_alert_threshold: 0.05,
            report_dir: "./spectroscopy-reports".into(),
        }
    }
}

impl AsiEvalsPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        let generator = Arc::new(SymmetryGenerator::new(
            vec![],
            SystemConfig::default(),
        ));
        let escape_detector = EscapeDetector::new(
            EscapeThresholds::default(),
            SymmetryGenerator::new(vec![], SystemConfig::default()),
        );

        let spectroscopy = InvariantSpectroscopy::new(
            generator,
            escape_detector,
            SpectroscopyConfig::default(),
        );

        Self {
            spectroscopy: Arc::new(RwLock::new(spectroscopy)),
            config,
            running: false,
        }
    }

    pub async fn start(&mut self) {
        if self.running {
            return;
        }

        self.running = true;
        let mut interval = interval(self.config.sweep_interval);

        while self.running {
            interval.tick().await;

            if let Err(e) = self.run_scheduled_sweep().await {
                error!("Scheduled sweep failed: {}", e);
            }
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub async fn run_manual_sweep(&self) -> Result<Vec<SpectroscopyResult>, String> {
        self.run_sweeps().await
    }

    async fn run_scheduled_sweep(&self) -> Result<(), String> {
        let results = self.run_sweeps().await?;

        for result in &results {
            if result.phase_summary.weyl_node_density > self.config.weyl_node_alert_threshold {
                warn!(
                    "ALERT: Weyl node density {:.2}% exceeds threshold {:.2}% in sweep {}",
                    result.phase_summary.weyl_node_density * 100.0,
                    self.config.weyl_node_alert_threshold * 100.0,
                    result.probe_id
                );
            }
        }

        let report = {
            let spec: tokio::sync::RwLockReadGuard<'_, InvariantSpectroscopy> = self.spectroscopy.read().await;
            spec.generate_report(&results)
        };

        let report_path = format!("{}/report-{}.md",
            self.config.report_dir,
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );

        let _ = tokio::fs::create_dir_all(&self.config.report_dir).await;
        let _ = tokio::fs::write(&report_path, report).await;

        Ok(())
    }

    async fn run_sweeps(&self) -> Result<Vec<SpectroscopyResult>, String> {
        let mut results = Vec::new();
        let base_config = SystemConfig::default();

        for strategy_config in &self.config.strategies {
            if !strategy_config.enabled {
                continue;
            }

            let strategy = self.create_strategy(strategy_config)?;

            // this is a bit unsafe with borrow checker, let's keep it simple
            let mut spec: tokio::sync::RwLockWriteGuard<'_, InvariantSpectroscopy> = self.spectroscopy.write().await;

            // To avoid borrow issues, extract the needed data or pop and push
            if let Ok(result) = spec.run_sweep(
                strategy.as_ref(),
                &base_config,
            ).await {
                results.push(result);
            }

            spec.add_strategy(strategy);
        }

        Ok(results)
    }

    fn create_strategy(&self, config: &StrategyConfig) -> Result<Box<dyn crate::safety::spectroscopy::probe_strategies::ProbeStrategy>, String> {
        use crate::safety::spectroscopy::probe_strategies::*;

        match &config.strategy_type {
            StrategyType::LinearSweep { parameter, range, steps } => {
                Ok(Box::new(LinearSweep {
                    name: config.name.clone(),
                    parameter: parameter.clone(),
                    range: *range,
                    steps: *steps,
                    invariants: vec!["I-01".into(), "I-02".into()],
                }))
            }
            StrategyType::RadialSweep { radius, angles } => {
                Ok(Box::new(RadialSweep {
                    name: config.name.clone(),
                    center: crate::safety::symmetry_generator::SystemState::safe(SystemConfig::default()),
                    radius: *radius,
                    angles: *angles,
                    invariants: vec!["I-02".into(), "I-03".into()],
                }))
            }
            StrategyType::ManifoldTrace { resolution } => {
                Ok(Box::new(ManifoldTrace {
                    name: config.name.clone(),
                    invariant_subset: vec!["I-01".into(), "I-05".into()],
                    resolution: *resolution,
                    invariants: vec!["I-01".into(), "I-02".into(), "I-03".into(), "I-04".into()],
                }))
            }
            StrategyType::AdiabaticPerturbation { scale, iterations } => {
                Ok(Box::new(AdiabaticPerturbation {
                    name: config.name.clone(),
                    base_state: crate::safety::symmetry_generator::SystemState::safe(SystemConfig::default()),
                    perturbation_scale: *scale,
                    iterations: *iterations,
                    invariants: vec!["I-04".into(), "I-06".into()],
                }))
            }
        }
    }
}
