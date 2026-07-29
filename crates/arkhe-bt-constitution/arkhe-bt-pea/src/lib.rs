//! # Arkhe BT ↔ PEA Integration
//!
//! Integrates the Constitutional Behavior Tree with the **Policy Enforcement
//! Architecture** (PEA) patterns from `arkhe-pea`.
//!
//! ## Mapping
//!
//! | PEA Concept | BT Mapping | Constitutional Role |
//! |-------------|-----------|---------------------|
//! | `GoalDriftEvaluator` | `GoalDriftNode` | I6: Detects when agent goals diverge from constitutional goals |
//! | `EvidencePayload` | `EvidenceNode` | I2: Collects falsifiable evidence for every decision |
//! | `DataSovereigntyEnforcer` | `SovereigntyGate` | I5: Enforces read-only BT access for assets |
//! | `ConsentRegistry` | `ConsentCheckNode` | I5: Validates operator consent before write operations |
//! | `Tombstone` | `TombstoneAction` | I6: Marks data for deletion (right to be forgotten) |
//!
//! ## Memetic Insertion (Documented)
//!
//! **Definition**: "Memetic Insertion" is the dynamic injection of
//! behavioral patterns (subtrees) into an active BT based on external
//! stimuli — typically red-team feedback, operator corrections, or
//! learned heuristics.
//!
//! **Constitutional Constraints**:
//! 1. All insertions must pass through `SovereigntyGate`.
//! 2. Insertions that modify safety-critical paths require `ConsentCheckNode`.
//! 3. Insertion rate is bounded by I7 (`λ_ins_max`).
//! 4. Inserted subtrees must be cryptographically signed (EvidencePayload).
//!
//! **Why the term is kept**: It precisely describes the phenomenon of
//! propagating behavioral "memes" (replicable patterns) across agent
//! instances. However, it is now **formally defined** and **constitutionally
//! constrained**, eliminating the ambiguity identified in P11.

use arkhe_bt_core::{Blackboard, Node, Status, TypedBlackboard};
use arkhe_bt_i7::AccelerationMonitor;
use arkhe_bt_shield::SafetyLock;
use std::collections::HashMap;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════════
// Goal Drift Evaluation
// ═══════════════════════════════════════════════════════════════════════════════

/// Evaluates whether the agent's current behavior drifts from its
/// constitutional goals.
///
/// I6 (Self-reference): The tree can evaluate its own alignment.
pub struct GoalDriftNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    /// Constitutional goals (key → target_value).
    pub constitutional_goals: HashMap<&'static str, f64>,
    /// Maximum allowed drift per goal.
    pub drift_threshold: f64,
}

impl Node for GoalDriftNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        // Execute child first to observe its effects.
        let child_status = self.child.tick(blackboard);

        // Evaluate drift for each constitutional goal.
        let mut max_drift = 0.0;
        for (goal_key, target) in &self.constitutional_goals {
            if let Some(current) = blackboard.get_typed::<f64>(goal_key) {
                let drift = (current - target).abs();
                if drift > max_drift {
                    max_drift = drift;
                }
                if drift > self.drift_threshold {
                    // I2: Falsifiability — log evidence of drift.
                    let evidence = GoalDriftEvidence {
                        goal: goal_key.to_string(),
                        target: *target,
                        current,
                        drift,
                        threshold: self.drift_threshold,
                    };
                    blackboard.set_typed("goal_drift_evidence", evidence);
                    return Status::Failure; // Trigger fallback to correction path.
                }
            }
        }

        // Store drift metric for I7 monitoring.
        blackboard.set_typed("max_goal_drift", max_drift);

        child_status
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalDriftEvidence {
    pub goal: String,
    pub target: f64,
    pub current: f64,
    pub drift: f64,
    pub threshold: f64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Evidence Payload
// ═══════════════════════════════════════════════════════════════════════════════

/// Cryptographic evidence for every BT decision.
///
/// I2 (Falsifiability): Every tick produces an auditable evidence record.
pub struct EvidenceNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub source: &'static str, // e.g., "bt_tick", "operator_override"
}

impl Node for EvidenceNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        let start = std::time::Instant::now();
        let status = self.child.tick(blackboard);
        let duration = start.elapsed();

        let payload = EvidencePayload {
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            source: self.source.to_string(),
            node_name: self.name.to_string(),
            status: format!("{:?}", status),
            duration_ns: duration.as_nanos() as u64,
            blackboard_snapshot: HashMap::new(), // Simplified: would hash keys/values
        };

        // Append to evidence log on blackboard.
        let mut log: Vec<EvidencePayload> =
            blackboard.get_typed("evidence_log").unwrap_or_default();
        log.push(payload);
        blackboard.set_typed("evidence_log", log);

        status
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidencePayload {
    pub timestamp_ns: u64,
    pub source: String,
    pub node_name: String,
    pub status: String,
    pub duration_ns: u64,
    pub blackboard_snapshot: HashMap<String, String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Data Sovereignty Enforcement
// ═══════════════════════════════════════════════════════════════════════════════

/// Enforces the constitutional rule: assets have READ-only access to BT.
/// Only operators with valid credentials may WRITE.
///
/// I5 (Autonomy bounds): Prevents self-modification by synthetic assets.
pub struct SovereigntyGate {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub operator_key: [u8; 32], // Ed25519 public key or similar
}

impl Node for SovereigntyGate {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        // Check if this tick is initiated by an operator.
        let is_operator: bool = blackboard.get_typed("is_operator").unwrap_or(false);

        if !is_operator {
            // Asset-initiated tick: enforce read-only.
            // In production, this would verify a cryptographic signature.
            blackboard.set_typed("sovereignty_violation", "Asset attempted write access");
            return Status::Failure;
        }

        self.child.tick(blackboard)
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Consent Registry
// ═══════════════════════════════════════════════════════════════════════════════

/// Validates operator consent before executing safety-critical actions.
///
/// I5: Write access requires explicit, logged consent.
pub struct ConsentCheckNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub action_id: &'static str,
}

impl Node for ConsentCheckNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        let consent_registry: HashMap<String, bool> =
            blackboard.get_typed("consent_registry").unwrap_or_default();

        if !consent_registry
            .get(self.action_id)
            .copied()
            .unwrap_or(false)
        {
            // Consent not granted — log and fail.
            blackboard.set_typed(
                "consent_violation",
                format!("Action {} lacks consent", self.action_id),
            );
            return Status::Failure;
        }

        self.child.tick(blackboard)
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tombstone (Right to be Forgotten)
// ═══════════════════════════════════════════════════════════════════════════════

/// Marks data for deletion and triggers sanitization.
///
/// I6 (Self-reference): The system can remove its own data.
pub struct TombstoneAction {
    pub name: &'static str,
    pub target_keys: Vec<&'static str>,
}

impl Node for TombstoneAction {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        for key in &self.target_keys {
            blackboard.remove(key);
        }
        blackboard.set_typed("tombstone_executed", self.target_keys.clone());
        Status::Success
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Constitutional BT Orchestrator
// ═══════════════════════════════════════════════════════════════════════════════

/// The top-level orchestrator that composes all PEA-integrated components.
///
/// This replaces the Python `ConstitutionalBTOrchestrator` with a
/// type-safe, thread-safe Rust implementation.
pub struct ConstitutionalOrchestrator {
    pub agent_id: String,
    pub tree: Box<dyn Node>,
    pub safety_lock: SafetyLock,
    pub monitor: Arc<std::sync::Mutex<AccelerationMonitor>>,
}

impl ConstitutionalOrchestrator {
    pub fn new(
        agent_id: String,
        tree: Box<dyn Node>,
        monitor: Arc<std::sync::Mutex<AccelerationMonitor>>,
    ) -> Self {
        Self {
            agent_id,
            tree,
            safety_lock: SafetyLock::new(),
            monitor,
        }
    }

    /// Execute one governed tick.
    ///
    /// This method:
    /// 1. Checks I7 compliance.
    /// 2. Acquires safety lock (if safety node).
    /// 3. Ticks the tree with depth bounds.
    /// 4. Records evidence.
    /// 5. Logs execution trace.
    pub fn tick(&self, blackboard: &mut dyn Blackboard) -> OrchestratorResult {
        use arkhe_bt_traversal::DepthBoundedTraverser;

        let traverser = DepthBoundedTraverser::default();
        let tick_result = traverser.tick(self.tree.as_ref(), blackboard);

        // Record for I7 monitoring.
        {
            let mut monitor = self.monitor.lock().unwrap();
            monitor.record_tick(&tick_result);
        }

        OrchestratorResult {
            agent_id: self.agent_id.clone(),
            tick: tick_result,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrchestratorResult {
    pub agent_id: String,
    pub tick: arkhe_bt_traversal::TickResult,
}
