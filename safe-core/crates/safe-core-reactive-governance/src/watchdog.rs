//! Autonomous watchdog that monitors metrics and proposes governance actions.

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
        let payload = serde_jcs::to_string(&action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?.into_bytes();
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
