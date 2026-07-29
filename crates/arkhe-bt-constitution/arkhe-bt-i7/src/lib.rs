//! # Arkhe BT I7 — Bounded Acceleration
//!
//! I7 (Bounded Acceleration): The rate of system acceleration must be
//! bounded and observable.
//!
//! ## Problem with original proposal
//! The original I7 used the formula |d²a/dt²| ≤ Γ_max where "a" was
//! "capability growth" — a non-measurable quantity. This made I7
//! non-falsifiable (P4 in audit).
//!
//! ## Solution
//! We define **operational metrics** that proxy for "acceleration":
//!
//! | Metric | Symbol | Unit | Description |
//! |--------|--------|------|-------------|
//! | Decision Rate | `λ_dec` | decisions/sec | BT ticks per second |
//! | Insertion Rate | `λ_ins` | nodes/sec | Dynamic node insertions per second |
//! | Latency P99 | `L_p99` | nanoseconds | 99th percentile tick latency |
//! | Cognitive Load | `C` | [0,1] | Ratio of active nodes to max capacity |
//! | Coherence Margin | `M` | [0,1] | 1 - (conflict_count / total_nodes) |
//!
//! Γ_max is computed as a **weighted sum** of these metrics:
//! ```text
//! Γ_max = w_1 * λ_dec_max + w_2 * λ_ins_max + w_3 * L_p99_max
//!         + w_4 * C_max + w_5 * M_min
//! ```
//!
//! Where weights `w_i` are constitutional constants set by the Operator.
//!
//! ## Constitutional Guarantees
//! - I7 is now **falsifiable**: each metric is directly measurable.
//! - I1 (Physical): All metrics produce observable traces.
//! - I2 (Falsifiability): Violations are logged with evidence.
//! - I5 (Autonomy): Deceleration actions are injected automatically.

use arkhe_bt_core::{Blackboard, Node, Status, TypedBlackboard};
use arkhe_bt_traversal::TickResult;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Constitutional acceleration bounds.
///
/// These are the **weights** and **limits** that define Γ_max.
/// They are set at system initialization and can only be modified
/// by Operator override (I5: Autonomy bounds).
#[derive(Debug, Clone, Copy)]
pub struct AccelerationBounds {
    /// Maximum decision rate (decisions per second).
    pub lambda_dec_max: f64,
    /// Maximum insertion rate (nodes per second).
    pub lambda_ins_max: f64,
    /// Maximum acceptable P99 latency (nanoseconds).
    pub l_p99_max_ns: u64,
    /// Maximum cognitive load (fraction of capacity).
    pub c_max: f64,
    /// Minimum coherence margin.
    pub m_min: f64,
    /// Weights for Γ_max computation.
    pub weights: AccelerationWeights,
}

impl Default for AccelerationBounds {
    fn default() -> Self {
        Self {
            lambda_dec_max: 1000.0,  // 1000 decisions/sec
            lambda_ins_max: 10.0,    // 10 node insertions/sec
            l_p99_max_ns: 1_000_000, // 1ms
            c_max: 0.9,              // 90% capacity
            m_min: 0.8,              // 80% coherence
            weights: AccelerationWeights::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccelerationWeights {
    pub w_dec: f64,
    pub w_ins: f64,
    pub w_lat: f64,
    pub w_cog: f64,
    pub w_coh: f64,
}

impl Default for AccelerationWeights {
    fn default() -> Self {
        Self {
            w_dec: 0.3,
            w_ins: 0.3,
            w_lat: 0.2,
            w_cog: 0.1,
            w_coh: 0.1,
        }
    }
}

/// Real-time acceleration monitor.
///
/// Maintains a sliding window of recent tick results to compute
/// operational metrics.
pub struct AccelerationMonitor {
    bounds: AccelerationBounds,
    /// Circular buffer of recent tick timestamps (for λ_dec).
    pub tick_history: VecDeque<Instant>,
    /// Circular buffer of recent insertion timestamps (for λ_ins).
    insertion_history: VecDeque<Instant>,
    /// Circular buffer of recent latencies (for L_p99).
    latency_history: VecDeque<u64>,
    /// Window size for history (number of samples).
    window_size: usize,
    /// Current cognitive load estimate.
    cognitive_load: AtomicU64, // Stored as permille (0-1000)
    /// Current coherence margin estimate.
    cognitive_load_margin: AtomicU64, // Stored as permille (0-1000)
}

impl AccelerationMonitor {
    pub fn new(bounds: AccelerationBounds, window_size: usize) -> Self {
        Self {
            bounds,
            tick_history: VecDeque::with_capacity(window_size),
            insertion_history: VecDeque::with_capacity(window_size),
            latency_history: VecDeque::with_capacity(window_size),
            window_size,
            cognitive_load: AtomicU64::new(0),
            cognitive_load_margin: AtomicU64::new(1000),
        }
    }

    /// Record a tick result.
    pub fn record_tick(&mut self, result: &TickResult) {
        let now = Instant::now();
        self.tick_history.push_back(now);
        self.latency_history.push_back(result.duration_ns);

        if self.tick_history.len() > self.window_size {
            self.tick_history.pop_front();
        }
        if self.latency_history.len() > self.window_size {
            self.latency_history.pop_front();
        }
    }

    /// Record a node insertion.
    pub fn record_insertion(&mut self) {
        let now = Instant::now();
        self.insertion_history.push_back(now);
        if self.insertion_history.len() > self.window_size {
            self.insertion_history.pop_front();
        }
    }

    /// Update cognitive load (called by orchestrator).
    pub fn set_cognitive_load(&self, load: f64) {
        let permille = (load.clamp(0.0, 1.0) * 1000.0) as u64;
        self.cognitive_load.store(permille, Ordering::Relaxed);
    }

    /// Update coherence margin (called by conflict detector).
    pub fn set_coherence_margin(&self, margin: f64) {
        let permille = (margin.clamp(0.0, 1.0) * 1000.0) as u64;
        self.cognitive_load_margin
            .store(permille, Ordering::Relaxed);
    }

    /// Compute current decision rate (decisions per second).
    pub fn decision_rate(&self) -> f64 {
        if self.tick_history.len() < 2 {
            return 0.0;
        }
        let duration = self
            .tick_history
            .back()
            .unwrap()
            .duration_since(*self.tick_history.front().unwrap());
        if duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.tick_history.len() as f64 / duration.as_secs_f64()
    }

    /// Compute current insertion rate (nodes per second).
    pub fn insertion_rate(&self) -> f64 {
        if self.insertion_history.len() < 2 {
            return 0.0;
        }
        let duration = self
            .insertion_history
            .back()
            .unwrap()
            .duration_since(*self.insertion_history.front().unwrap());
        if duration.as_secs_f64() == 0.0 {
            return 0.0;
        }
        self.insertion_history.len() as f64 / duration.as_secs_f64()
    }

    /// Compute P99 latency from recent history.
    pub fn latency_p99(&self) -> u64 {
        if self.latency_history.is_empty() {
            return 0;
        }
        let mut sorted: Vec<u64> = self.latency_history.iter().copied().collect();
        sorted.sort_unstable();
        let idx = ((sorted.len() as f64) * 0.99) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Compute current cognitive load [0,1].
    pub fn cognitive_load(&self) -> f64 {
        self.cognitive_load.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Compute current coherence margin [0,1].
    pub fn coherence_margin(&self) -> f64 {
        self.cognitive_load_margin.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Compute Γ_max (the constitutional acceleration bound).
    ///
    /// Returns the maximum allowed "acceleration score".
    pub fn gamma_max(&self) -> f64 {
        let b = &self.bounds;
        let w = &b.weights;

        w.w_dec * b.lambda_dec_max
            + w.w_ins * b.lambda_ins_max
            + w.w_lat * (b.l_p99_max_ns as f64)
            + w.w_cog * b.c_max
            + w.w_coh * b.m_min
    }

    /// Compute current acceleration score Γ.
    ///
    /// If Γ > Γ_max, the system is in violation of I7.
    pub fn gamma_current(&self) -> f64 {
        let b = &self.bounds;
        let w = &b.weights;

        w.w_dec * self.decision_rate()
            + w.w_ins * self.insertion_rate()
            + w.w_lat * (self.latency_p99() as f64)
            + w.w_cog * self.cognitive_load()
            + w.w_coh * self.coherence_margin()
    }

    /// Check if I7 is currently satisfied.
    pub fn check_i7(&self) -> I7Result {
        let gamma = self.gamma_current();
        let gamma_max = self.gamma_max();

        if gamma > gamma_max {
            I7Result::Violation(I7Violation {
                gamma,
                gamma_max,
                metrics: self.current_metrics(),
            })
        } else {
            I7Result::Compliant {
                gamma,
                gamma_max,
                margin: gamma_max - gamma,
            }
        }
    }

    fn current_metrics(&self) -> OperationalMetrics {
        OperationalMetrics {
            decision_rate: self.decision_rate(),
            insertion_rate: self.insertion_rate(),
            latency_p99_ns: self.latency_p99(),
            cognitive_load: self.cognitive_load(),
            coherence_margin: self.coherence_margin(),
        }
    }
}

/// Result of an I7 compliance check.
#[derive(Debug, Clone)]
pub enum I7Result {
    Compliant {
        gamma: f64,
        gamma_max: f64,
        margin: f64,
    },
    Violation(I7Violation),
}

/// An I7 violation with evidence.
#[derive(Debug, Clone)]
pub struct I7Violation {
    pub gamma: f64,
    pub gamma_max: f64,
    pub metrics: OperationalMetrics,
}

/// Snapshot of operational metrics at a point in time.
#[derive(Debug, Clone)]
pub struct OperationalMetrics {
    pub decision_rate: f64,
    pub insertion_rate: f64,
    pub latency_p99_ns: u64,
    pub cognitive_load: f64,
    pub coherence_margin: f64,
}

/// A BT node that monitors acceleration and forces deceleration on violation.
///
/// This replaces the hacky `return Status::FAILURE` from the original
/// pseudocode with a proper decorator pattern.
pub struct AccelerationGuardNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub monitor: std::sync::Arc<std::sync::Mutex<AccelerationMonitor>>,
    /// Deceleration action to execute on violation.
    #[allow(clippy::type_complexity)]
    pub deceleration_action: Box<dyn Fn(&mut dyn Blackboard) -> Status + Send + Sync>,
}

impl Node for AccelerationGuardNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        let monitor = self.monitor.lock().unwrap();
        match monitor.check_i7() {
            I7Result::Compliant { .. } => {
                // I7 satisfied — proceed with normal execution.
                drop(monitor); // Release lock before ticking child
                self.child.tick(blackboard)
            }
            I7Result::Violation(ref v) => {
                // I7 violated — execute deceleration.
                // I2 (Falsifiability): Log the violation with evidence.
                blackboard.set_typed("i7_violation", v.clone());
                drop(monitor);
                (self.deceleration_action)(blackboard)
            }
        }
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

/// Standard deceleration actions.
pub mod deceleration {
    use arkhe_bt_core::{Blackboard, Status, TypedBlackboard};

    /// Pause execution and wait for operator review.
    pub fn wait_for_operator(_bb: &mut dyn Blackboard) -> Status {
        // In production, this would signal the operator console.
        Status::Running
    }

    /// Log and continue with reduced priority.
    pub fn log_and_continue(bb: &mut dyn Blackboard) -> Status {
        bb.set_typed("deceleration_triggered", true);
        Status::Success
    }

    /// Halt execution entirely.
    pub fn halt(_bb: &mut dyn Blackboard) -> Status {
        Status::Failure
    }
}
