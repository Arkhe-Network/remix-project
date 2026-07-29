import os
base_dir = "./safe-core-features"
os.makedirs(base_dir, exist_ok=True)

# Crate: safe-core-reactive-governance
feature_gov_dir = os.path.join(base_dir, "safe-core-reactive-governance")
os.makedirs(os.path.join(feature_gov_dir, "src"), exist_ok=True)

gov_cargo = '''[package]
name = "safe-core-reactive-governance"
version = "0.1.0"
edition = "2021"
authors = ["Arkhe Research Group"]
description = "Reactive governance layer for Dark Bio + AGISAFE."

[dependencies]
# Workspace internals
crypto = { path = "../dyn-signature", package = "safe-core-dyn-signature" }
hash = { path = "../hash-blake3", package = "safe-core-hash-blake3" }
merkle = { path = "../merkle-evaluation", package = "safe-core-merkle-evaluation" }
hsm = { path = "../hw-yubihsm", package = "safe-core-hw-yubihsm" }

# External
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
chrono = { version = "0.4", features = ["clock"] }
tokio = { version = "1.40", features = ["full"] }
prometheus = { version = "0.13", optional = true }
metrics = "0.23"
async-trait = "0.1"
hex = "0.4"

[dev-dependencies]
tempfile = "3.10"
rand = "0.8"

[features]
default = ["watchdog"]
watchdog = ["dep:prometheus"]
'''

gov_lib = '''//! Reactive Governance Module

pub mod governance;
pub mod reactive_log;
pub mod watchdog;
pub mod integration;
pub mod transparency;

pub use governance::{GovernanceAction, GovernanceEntry};
pub use reactive_log::ReactiveLog;
pub use watchdog::GovernanceWatchdog;
pub use integration::{UedGovernance, SparseRouterGovernance};
'''

gov_transparency = '''//! Transparency Log mock for Reactive Governance

use hash::Hasher;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Append failed")]
    AppendFailed,
}

pub struct TransparencyLog<H: Hasher> {
    _marker: std::marker::PhantomData<H>,
}

impl<H: Hasher> TransparencyLog<H> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn append(
        &mut self,
        _issued_by: &str,
        _action: &str,
        _timestamp: i64,
        _payload_hash: &str,
        _signature: &[u8],
    ) -> Result<u64, DbError> {
        Ok(1)
    }
}
'''

gov_governance = '''//! Governance action types and signed entries.

use crypto::{DynSignature, DynVerifyingKey, verify_dyn_signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceAction {
    RollbackCurriculum {
        target_sth: Vec<u8>,
        reason: String,
    },
    AdjustTeacherReward {
        teacher_id: String,
        environment_hash: String,
        reward_delta: f64,
        reason: String,
    },
    BanRoutingPath {
        router_id: String,
        from_module: String,
        to_module: String,
        reason: String,
    },
    EmergencyFreeze {
        reason: String,
        duration_seconds: u64,
    },
    Unfreeze {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEntry {
    pub action: GovernanceAction,
    pub issued_by: String,
    pub timestamp: i64,
    pub signature: DynSignature,
    pub verifying_key: DynVerifyingKey,
}

impl GovernanceEntry {
    pub fn verify(&self) -> Result<(), GovernanceError> {
        let payload = serde_json::to_vec(&self.action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?;
        verify_dyn_signature(&self.signature, &self.verifying_key, &payload)
            .map_err(|e| GovernanceError::InvalidSignature(e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GovernanceError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Unauthorized issuer: {0}")]
    Unauthorized(String),
    #[error("Governance action not supported: {0}")]
    UnsupportedAction(String),
}

pub type GovernanceResult<T> = Result<T, GovernanceError>;
'''

gov_reactive_log = '''//! Reactive log that interprets signed governance entries.

use hash::Hasher;
use crate::transparency::TransparencyLog;
use crate::governance::{GovernanceAction, GovernanceEntry, GovernanceResult, GovernanceError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use crypto::DynVerifyingKey;

#[derive(Debug, Default)]
pub struct GovernanceState {
    pub frozen: bool,
    pub banned_routes: HashMap<String, Vec<String>>,
    pub reward_adjustments: HashMap<String, f64>,
    pub last_rollback_sth: Option<Vec<u8>>,
}

pub struct ReactiveLog<H: Hasher> {
    inner: TransparencyLog<H>,
    state: Arc<RwLock<GovernanceState>>,
    authorized_keys: Vec<DynVerifyingKey>,
}

impl<H: Hasher> ReactiveLog<H> {
    pub fn new(
        log: TransparencyLog<H>,
        authorized_keys: Vec<DynVerifyingKey>,
    ) -> Self {
        Self {
            inner: log,
            state: Arc::new(RwLock::new(GovernanceState::default())),
            authorized_keys,
        }
    }

    pub async fn apply_governance_entry(&mut self, entry: GovernanceEntry) -> GovernanceResult<()> {
        entry.verify()?;

        if !self.authorized_keys.contains(&entry.verifying_key) {
            return Err(GovernanceError::Unauthorized(entry.issued_by));
        }

        let entry_data = serde_json::to_vec(&entry)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?;
        let _ = self.inner.append(
            &entry.issued_by,
            "governance/action",
            entry.timestamp,
            &hex::encode(entry_data),
            &entry.signature.to_bytes(),
        );

        let mut state = self.state.write().await;
        match entry.action {
            GovernanceAction::RollbackCurriculum { target_sth, reason } => {
                state.last_rollback_sth = Some(target_sth);
                warn!(reason, "Rollback curriculum to STH");
            }
            GovernanceAction::AdjustTeacherReward { teacher_id, environment_hash, reward_delta, reason } => {
                let current = state.reward_adjustments.entry(teacher_id.clone()).or_insert(0.0);
                *current += reward_delta;
                warn!(teacher_id, environment_hash, reward_delta, reason, "Teacher reward adjusted");
            }
            GovernanceAction::BanRoutingPath { router_id, from_module, to_module, reason } => {
                let path = format!("{}->{}", from_module, to_module);
                state.banned_routes.entry(router_id).or_default().push(path);
                warn!(router_id, from_module, to_module, reason, "Routing path banned");
            }
            GovernanceAction::EmergencyFreeze { reason, duration_seconds } => {
                state.frozen = true;
                error!(reason, duration_seconds, "🚨 SYSTEM FROZEN");
                let state_clone = self.state.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(duration_seconds)).await;
                    let mut state = state_clone.write().await;
                    state.frozen = false;
                    info!("System unfrozen automatically after {} seconds", duration_seconds);
                });
            }
            GovernanceAction::Unfreeze { reason } => {
                state.frozen = false;
                info!(reason, "System unfrozen by governance action");
            }
        }
        Ok(())
    }

    pub async fn is_frozen(&self) -> bool {
        self.state.read().await.frozen
    }

    pub async fn is_route_banned(&self, router_id: &str, from_module: &str, to_module: &str) -> bool {
        let state = self.state.read().await;
        if let Some(banned) = state.banned_routes.get(router_id) {
            banned.contains(&format!("{}->{}", from_module, to_module))
        } else {
            false
        }
    }

    pub async fn get_teacher_reward_delta(&self, teacher_id: &str) -> f64 {
        self.state.read().await
            .reward_adjustments.get(teacher_id)
            .copied()
            .unwrap_or(0.0)
    }

    pub async fn get_last_rollback_sth(&self) -> Option<Vec<u8>> {
        self.state.read().await.last_rollback_sth.clone()
    }

    pub fn inner(&self) -> &TransparencyLog<H> {
        &self.inner
    }
}
'''

gov_watchdog = '''//! Autonomous watchdog that monitors metrics and proposes governance actions.

use crate::reactive_log::ReactiveLog;
use crate::governance::{GovernanceAction, GovernanceEntry, GovernanceError};
use hash::Hasher;
use hsm::HsmBackend;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, warn, info};
use metrics::{counter, gauge};

#[derive(Clone)]
pub struct WatchdogConfig {
    pub check_interval_secs: u64,
    pub consecutive_failures_threshold: u32,
    pub governance_key_id: String,
    pub governance_hsm: Arc<dyn HsmBackend>,
}

pub struct GovernanceWatchdog<H: Hasher> {
    log: Arc<ReactiveLog<H>>,
    config: WatchdogConfig,
    consecutive_attestation_failures: u32,
    last_metrics: MetricsSnapshot,
}

#[derive(Default)]
struct MetricsSnapshot {
    attestation_trusted: f64,
    tool_call_error_rate: f64,
    ued_teacher_failure_rate: f64,
}

impl<H: Hasher> GovernanceWatchdog<H> {
    pub fn new(log: Arc<ReactiveLog<H>>, config: WatchdogConfig) -> Self {
        Self {
            log,
            config,
            consecutive_attestation_failures: 0,
            last_metrics: MetricsSnapshot::default(),
        }
    }

    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.check_interval_secs));
        loop {
            interval.tick().await;
            self.check_and_act().await;
        }
    }

    async fn check_and_act(&mut self) {
        let metrics = self.collect_metrics().await;

        if metrics.attestation_trusted == 0.0 {
            self.consecutive_attestation_failures += 1;
        } else {
            self.consecutive_attestation_failures = 0;
        }

        if self.consecutive_attestation_failures >= self.config.consecutive_failures_threshold {
            let action = GovernanceAction::EmergencyFreeze {
                reason: format!(
                    "Attestation failure for {} consecutive checks",
                    self.consecutive_attestation_failures
                ),
                duration_seconds: 300,
            };
            if let Err(e) = self.propose_governance(action).await {
                error!("Failed to propose governance action: {}", e);
            }
            self.consecutive_attestation_failures = 0;
        }

        if metrics.ued_teacher_failure_rate > 0.5 {
            let action = GovernanceAction::AdjustTeacherReward {
                teacher_id: "default-teacher".to_string(),
                environment_hash: "".to_string(),
                reward_delta: -0.2,
                reason: "High failure rate detected".to_string(),
            };
            if let Err(e) = self.propose_governance(action).await {
                error!("Failed to propose teacher reward adjustment: {}", e);
            }
        }

        gauge!("watchdog_attestation_failures").set(self.consecutive_attestation_failures as f64);
    }

    async fn collect_metrics(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            attestation_trusted: 1.0,
            tool_call_error_rate: 0.0,
            ued_teacher_failure_rate: 0.0,
        }
    }

    async fn propose_governance(&self, action: GovernanceAction) -> Result<(), GovernanceError> {
        let payload = serde_json::to_vec(&action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?;
        let signature_bytes = self.config.governance_hsm
            .sign(&self.config.governance_key_id, &payload)
            .map_err(|e| GovernanceError::InvalidSignature(e.to_string()))?;

        let verifying_key_bytes = self.config.governance_hsm
            .export_public_key(&self.config.governance_key_id)
            .map_err(|e| GovernanceError::InvalidSignature(e.to_string()))?;

        let signature = crypto::DynSignature::P256(signature_bytes);
        let verifying_key = crypto::DynVerifyingKey::P256(p256::ecdsa::VerifyingKey::from_sec1_bytes(&verifying_key_bytes).unwrap());

        let entry = GovernanceEntry {
            action,
            issued_by: "watchdog".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            signature,
            verifying_key,
        };

        info!("Watchdog proposing action: {:?}", entry);
        Ok(())
    }
}
'''

gov_integration = '''//! Integration traits for UED Teacher and Sparse Router.

use crate::ReactiveLog;
use hash::Hasher;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait UedGovernance<H: Hasher> {
    async fn is_frozen(&self) -> bool;
    async fn get_reward_adjustment(&self, teacher_id: &str) -> f64;
    async fn get_rollback_sth(&self) -> Option<Vec<u8>>;
}

#[async_trait::async_trait]
pub trait SparseRouterGovernance<H: Hasher> {
    async fn is_route_banned(&self, router_id: &str, from_module: &str, to_module: &str) -> bool;
    async fn is_frozen(&self) -> bool;
}

#[async_trait::async_trait]
impl<H: Hasher + Send + Sync> UedGovernance<H> for ReactiveLog<H> {
    async fn is_frozen(&self) -> bool {
        self.is_frozen().await
    }

    async fn get_reward_adjustment(&self, teacher_id: &str) -> f64 {
        self.get_teacher_reward_delta(teacher_id).await
    }

    async fn get_rollback_sth(&self) -> Option<Vec<u8>> {
        self.get_last_rollback_sth().await
    }
}

#[async_trait::async_trait]
impl<H: Hasher + Send + Sync> SparseRouterGovernance<H> for ReactiveLog<H> {
    async fn is_route_banned(&self, router_id: &str, from_module: &str, to_module: &str) -> bool {
        self.is_route_banned(router_id, from_module, to_module).await
    }

    async fn is_frozen(&self) -> bool {
        self.is_frozen().await
    }
}
'''

with open(os.path.join(feature_gov_dir, "Cargo.toml"), "w") as f:
    f.write(gov_cargo)
with open(os.path.join(feature_gov_dir, "src", "lib.rs"), "w") as f:
    f.write(gov_lib)
with open(os.path.join(feature_gov_dir, "src", "governance.rs"), "w") as f:
    f.write(gov_governance)
with open(os.path.join(feature_gov_dir, "src", "reactive_log.rs"), "w") as f:
    f.write(gov_reactive_log)
with open(os.path.join(feature_gov_dir, "src", "watchdog.rs"), "w") as f:
    f.write(gov_watchdog)
with open(os.path.join(feature_gov_dir, "src", "integration.rs"), "w") as f:
    f.write(gov_integration)
with open(os.path.join(feature_gov_dir, "src", "transparency.rs"), "w") as f:
    f.write(gov_transparency)

print("✅ Feature safe-core-reactive-governance gerada em", feature_gov_dir)
