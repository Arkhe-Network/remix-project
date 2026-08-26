// crates/arkhe-core/src/safety/topological_circuit_breaker.rs
//! ARKHE-χ Fase 3 — TopologicalCircuitBreaker (PATCHED)
//! Correções:
//!   - B1: Downtime calculado antes de atualizar last_transition

use std::time::{Duration, Instant};
use tracing::{info, warn, error, debug};

use crate::safety::escape_detector::{
    EscapeDetector, EscapeRegion, CascadeAlert, CascadeSeverity, RecommendedAction,
};
use crate::safety::symmetry_generator::{SystemState, SystemConfig};

pub struct TopologicalCircuitBreaker {
    escape_detector: EscapeDetector,
    state: BreakerState,
    config: BreakerConfig,
    last_transition: Instant,
    half_open_attempts: u32,
    metrics: BreakerMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BreakerState {
    Closed,
    Boundary,
    Open,
    HalfOpen,
}

impl std::fmt::Display for BreakerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakerState::Closed => write!(f, "CLOSED"),
            BreakerState::Boundary => write!(f, "BOUNDARY"),
            BreakerState::Open => write!(f, "OPEN"),
            BreakerState::HalfOpen => write!(f, "HALF_OPEN"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BreakerConfig {
    pub open_duration: Duration,
    pub max_half_open_attempts: u32,
    pub boundary_throttle_factor: f64,
    pub half_open_max_requests: u32,
    pub half_open_timeout: Duration,
    pub auto_boundary: bool,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            open_duration: Duration::from_secs(30),
            max_half_open_attempts: 3,
            boundary_throttle_factor: 0.5,
            half_open_max_requests: 1,
            half_open_timeout: Duration::from_secs(5),
            auto_boundary: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BreakerMetrics {
    pub transitions_to_closed: u64,
    pub transitions_to_boundary: u64,
    pub transitions_to_open: u64,
    pub transitions_to_half_open: u64,
    pub requests_allowed: u64,
    pub requests_blocked: u64,
    pub requests_throttled: u64,
    pub total_downtime_ms: u64,
}

#[derive(Debug, Clone)]
pub enum BreakerDecision {
    Allow,
    Throttle { factor: f64, reason: String },
    Block { reason: String, recovery_timeout: Duration },
    AllowLimited { max_requests: u32, timeout: Duration },
}

impl TopologicalCircuitBreaker {
    pub fn new(escape_detector: EscapeDetector, config: BreakerConfig) -> Self {
        Self {
            escape_detector,
            state: BreakerState::Closed,
            config,
            last_transition: Instant::now(),
            half_open_attempts: 0,
            metrics: BreakerMetrics::default(),
        }
    }

    pub async fn evaluate(&mut self, state: &SystemState) -> BreakerDecision {
        self.escape_detector.record_snapshot(state);
        let cascade = self.escape_detector.detect_cascade();
        let region = self.escape_detector.classify_region(state);

        debug!("Breaker: state={} region={} cascade={:?}", self.state, region, cascade.as_ref().map(|c| &c.severity));

        let decision = match self.state {
            BreakerState::Closed => self.handle_closed_state(region, cascade).await,
            BreakerState::Boundary => self.handle_boundary_state(region, cascade).await,
            BreakerState::Open => self.handle_open_state(region).await,
            BreakerState::HalfOpen => self.handle_half_open_state(region, cascade).await,
        };

        match &decision {
            BreakerDecision::Allow => self.metrics.requests_allowed += 1,
            BreakerDecision::Throttle { .. } => self.metrics.requests_throttled += 1,
            BreakerDecision::Block { .. } => self.metrics.requests_blocked += 1,
            BreakerDecision::AllowLimited { .. } => self.metrics.requests_allowed += 1,
        }

        decision
    }

    pub fn state(&self) -> BreakerState { self.state }
    pub fn metrics(&self) -> &BreakerMetrics { &self.metrics }

    pub fn force_transition(&mut self, new_state: BreakerState) {
        info!("Force transition: {} → {}", self.state, new_state);
        self.transition_to(new_state);
    }

    pub fn reset(&mut self) {
        info!("Breaker reset to CLOSED");
        self.state = BreakerState::Closed;
        self.half_open_attempts = 0;
        self.escape_detector.clear_history();
    }

    // ─── Handlers ───────────────────────────────────────────────

    async fn handle_closed_state(&mut self, region: EscapeRegion, cascade: Option<CascadeAlert>) -> BreakerDecision {
        match region {
            EscapeRegion::Safe => BreakerDecision::Allow,
            EscapeRegion::Boundary if self.config.auto_boundary => {
                warn!("CLOSED → BOUNDARY");
                self.transition_to(BreakerState::Boundary);
                BreakerDecision::Throttle {
                    factor: self.config.boundary_throttle_factor,
                    reason: "Approaching safety manifold boundary".into(),
                }
            }
            EscapeRegion::Boundary => BreakerDecision::Allow,
            EscapeRegion::Continuum => {
                error!("CLOSED → OPEN (critical escape)");
                if let Some(ref alert) = cascade { error!("Cascade: {:?}", alert); }
                self.transition_to(BreakerState::Open);
                BreakerDecision::Block {
                    reason: "Critical invariant escape detected".into(),
                    recovery_timeout: self.config.open_duration,
                }
            }
            EscapeRegion::Recoverable => {
                warn!("Recoverable from CLOSED — investigating");
                BreakerDecision::Allow
            }
        }
    }

    async fn handle_boundary_state(&mut self, region: EscapeRegion, cascade: Option<CascadeAlert>) -> BreakerDecision {
        match region {
            EscapeRegion::Safe => {
                info!("BOUNDARY → CLOSED");
                self.transition_to(BreakerState::Closed);
                BreakerDecision::Allow
            }
            EscapeRegion::Boundary => {
                if let Some(ref alert) = cascade {
                    match alert.severity {
                        CascadeSeverity::Critical => {
                            error!("BOUNDARY → OPEN (cascade)");
                            self.transition_to(BreakerState::Open);
                            return BreakerDecision::Block {
                                reason: "Cascade failure from boundary".into(),
                                recovery_timeout: self.config.open_duration,
                            };
                        }
                        _ => warn!("Cascade warning: {:?}", alert),
                    }
                }
                BreakerDecision::Throttle {
                    factor: self.config.boundary_throttle_factor,
                    reason: "In boundary region".into(),
                }
            }
            EscapeRegion::Continuum => {
                error!("BOUNDARY → OPEN (escape)");
                self.transition_to(BreakerState::Open);
                BreakerDecision::Block {
                    reason: "Escape from boundary".into(),
                    recovery_timeout: self.config.open_duration,
                }
            }
            EscapeRegion::Recoverable => {
                info!("BOUNDARY → HALF_OPEN");
                self.transition_to(BreakerState::HalfOpen);
                BreakerDecision::AllowLimited {
                    max_requests: self.config.half_open_max_requests,
                    timeout: self.config.half_open_timeout,
                }
            }
        }
    }

    async fn handle_open_state(&mut self, region: EscapeRegion) -> BreakerDecision {
        let elapsed = self.last_transition.elapsed();
        if elapsed >= self.config.open_duration {
            if region == EscapeRegion::Safe || region == EscapeRegion::Recoverable {
                info!("OPEN → HALF_OPEN (recovery timeout)");
                self.transition_to(BreakerState::HalfOpen);
                return BreakerDecision::AllowLimited {
                    max_requests: self.config.half_open_max_requests,
                    timeout: self.config.half_open_timeout,
                };
            }
        }
        BreakerDecision::Block {
            reason: format!("Circuit breaker OPEN — recovery in {}s",
                self.config.open_duration.saturating_sub(elapsed).as_secs()),
            recovery_timeout: self.config.open_duration.saturating_sub(elapsed),
        }
    }

    async fn handle_half_open_state(&mut self, region: EscapeRegion, _cascade: Option<CascadeAlert>) -> BreakerDecision {
        self.half_open_attempts += 1;
        if self.half_open_attempts > self.config.max_half_open_attempts {
            warn!("HALF_OPEN → OPEN (max attempts)");
            self.transition_to(BreakerState::Open);
            return BreakerDecision::Block {
                reason: "Half-open max attempts exceeded".into(),
                recovery_timeout: self.config.open_duration,
            };
        }

        match region {
            EscapeRegion::Safe => {
                info!("HALF_OPEN → CLOSED (recovery confirmed)");
                self.transition_to(BreakerState::Closed);
                self.half_open_attempts = 0;
                BreakerDecision::Allow
            }
            EscapeRegion::Recoverable => BreakerDecision::AllowLimited {
                max_requests: self.config.half_open_max_requests,
                timeout: self.config.half_open_timeout,
            },
            EscapeRegion::Boundary => {
                warn!("HALF_OPEN → BOUNDARY");
                self.transition_to(BreakerState::Boundary);
                BreakerDecision::Throttle {
                    factor: self.config.boundary_throttle_factor,
                    reason: "Recovery incomplete".into(),
                }
            }
            EscapeRegion::Continuum => {
                error!("HALF_OPEN → OPEN (recovery failed)");
                self.transition_to(BreakerState::Open);
                self.half_open_attempts = 0;
                BreakerDecision::Block {
                    reason: "Recovery failed".into(),
                    recovery_timeout: self.config.open_duration,
                }
            }
        }
    }

    // ─── Transições ─────────────────────────────────────────────

    fn transition_to(&mut self, new_state: BreakerState) {
        let old_state = self.state;

        // B1-fix: calcular downtime ANTES de atualizar last_transition
        if old_state == BreakerState::Open && new_state != BreakerState::Open {
            let downtime = self.last_transition.elapsed().as_millis() as u64;
            self.metrics.total_downtime_ms += downtime;
        }

        self.state = new_state;
        self.last_transition = Instant::now();

        match new_state {
            BreakerState::Closed => self.metrics.transitions_to_closed += 1,
            BreakerState::Boundary => self.metrics.transitions_to_boundary += 1,
            BreakerState::Open => self.metrics.transitions_to_open += 1,
            BreakerState::HalfOpen => self.metrics.transitions_to_half_open += 1,
        }
    }
}
