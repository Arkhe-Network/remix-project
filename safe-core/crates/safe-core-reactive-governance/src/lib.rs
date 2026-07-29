//! Reactive Governance Module

pub mod governance;
pub mod reactive_log;
pub mod watchdog;
pub mod integration;
pub mod transparency;

pub use governance::{GovernanceAction, GovernanceEntry};
pub use reactive_log::ReactiveLog;
pub use watchdog::GovernanceWatchdog;
pub use integration::{UedGovernance, SparseRouterGovernance};

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::{DynSignature, DynVerifyingKey, SignatureAlgorithm};
    use crate::transparency::TransparencyLog;
    use crate::governance::{GovernanceAction, GovernanceEntry};
    use crate::reactive_log::ReactiveLog;
    use hash::Blake3Hasher;
    use p256::ecdsa::SigningKey;

    use rand::rngs::OsRng;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn setup() -> (
        DynVerifyingKey,
        ReactiveLog<Blake3Hasher>,
        SigningKey,
    ) {
        let gov_sk = SigningKey::random(&mut OsRng);
        let gov_vk = DynVerifyingKey::P256(*gov_sk.verifying_key());

        let inner_log = TransparencyLog::<Blake3Hasher>::new();

        let reactive_log = ReactiveLog::new(inner_log, vec![gov_vk.clone()]);

        (gov_vk, reactive_log, gov_sk)
    }

    #[tokio::test]
    async fn test_reactive_freeze_blocks_operations() {
        let (gov_vk, mut reactive_log, gov_sk) = setup();

        assert!(!reactive_log.is_frozen().await);

        let action = GovernanceAction::EmergencyFreeze {
            reason: "Test freeze".into(),
            duration_seconds: 60,
        };

        let payload = serde_jcs::to_string(&action).unwrap().into_bytes();
        use p256::ecdsa::signature::Signer;
        let signature: p256::ecdsa::Signature = gov_sk.sign(&payload);
        let sig_bytes = signature.to_vec();
        let sig = DynSignature::P256(sig_bytes);
        let entry = GovernanceEntry { action, issued_by: "test-authority".into(), verifying_key: gov_vk, signature: sig, timestamp: 1234 };

        reactive_log.apply_governance_entry(entry).await.unwrap();

        assert!(reactive_log.is_frozen().await, "System should be frozen");
    }
}