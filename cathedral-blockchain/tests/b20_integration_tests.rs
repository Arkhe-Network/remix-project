//! tests/b20_integration_tests.rs

use cathedral_blockchain::substrato_4004::mock::{Action, Address, U256, EthicalFilter, EventStore, B20Payment, CrossChainEmitterV2, X402XrplBridge, EscrowManager, Contract, Provider, Http, BatchSettlementEngine, HybridZkVerifier};
use cathedral_blockchain::substrato_4004::compliance_engine::{ComplianceEngine, EthicalCompliance, PolicyCompliance};
use cathedral_blockchain::substrato_4004::policy_adapter::PolicyRegistryClient;
use cathedral_blockchain::substrato_4004::b20_mapper::B20TokenMapper;
use cathedral_blockchain::substrato_4004::settlement_engine::B20SettlementEngine;
use cathedral_blockchain::substrato_4004::memo_tracer::MemoTracer;
use cathedral_blockchain::substrato_4004::cross_chain_bridge::B20XrplBridge;
use std::sync::Arc;
use std::collections::HashMap;

async fn setup_compliance_engine() -> ComplianceEngine {
    let ethical_filter = Arc::new(EthicalFilter);
    let policy_registry = Arc::new(PolicyRegistryClient { contract: Contract { _provider: Provider::default() }, b20_factory: Address([0; 20]) });
    let event_store = Arc::new(EventStore);

    let b20_mapper = Arc::new(B20TokenMapper {
        ethical_filter: ethical_filter.clone(),
        policy_registry: policy_registry.clone(),
    });

    ComplianceEngine {
        ethical_filter,
        policy_registry,
        b20_mapper,
        event_store,
        provider: Provider::default(),
    }
}

async fn setup_b20_xrpl_bridge() -> B20XrplBridge {
    let compliance_engine = Arc::new(setup_compliance_engine().await);
    let event_store = compliance_engine.event_store.clone();
    let cross_chain_emitter = Arc::new(CrossChainEmitterV2);

    let b20_settlement = Arc::new(B20SettlementEngine {
        b20_mapper: compliance_engine.b20_mapper.clone(),
        compliance_engine,
        batch_engine: Arc::new(BatchSettlementEngine),
        cross_chain_emitter: cross_chain_emitter.clone(),
        zk_prover: Arc::new(HybridZkVerifier),
        provider: Provider::default(),
    });

    let xrpl_bridge = Arc::new(X402XrplBridge {
        escrow_manager: EscrowManager,
    });

    let memo_tracer = Arc::new(MemoTracer {
        event_store,
        cross_chain_emitter: cross_chain_emitter.clone(),
    });

    B20XrplBridge {
        b20_settlement,
        xrpl_bridge,
        cross_chain_emitter,
        memo_tracer,
    }
}

#[tokio::test]
async fn test_b20_compliance_full_flow() {
    let engine = setup_compliance_engine().await;

    let mut metadata = HashMap::new();
    metadata.insert("affects_human_dignity".to_string(), "false".to_string());
    metadata.insert("auditable".to_string(), "true".to_string());

    let action = Action {
        id: "b20-payment-1".to_string(),
        action_type: "payment_b20".to_string(),
        payload: serde_json::json!({
            "token": "0xB200...",
            "from": "0x...",
            "to": "0x...",
            "amount": "1000000000000000000",
        }),
        metadata,
    };

    let verdict = engine.evaluate_compliance(&action).await.unwrap();
    assert!(verdict.overall);
    assert!(matches!(verdict.ethical, EthicalCompliance::Passed));
    assert!(matches!(verdict.policy, PolicyCompliance::Passed));
}

#[tokio::test]
async fn test_b20_freeze_and_seize() {
    let engine = setup_compliance_engine().await;

    let mut metadata = HashMap::new();
    metadata.insert("has_kill_switch".to_string(), "true".to_string());
    metadata.insert("respects_constitution".to_string(), "true".to_string());

    let action = Action {
        id: "freeze-1".to_string(),
        action_type: "freeze_and_seize".to_string(),
        payload: serde_json::json!({
            "token": "0xB200...",
            "target": "0x...",
            "amount": "1000000",
        }),
        metadata,
    };

    let verdict = engine.evaluate_compliance(&action).await.unwrap();
    assert!(verdict.overall);
}

#[tokio::test]
async fn test_b20_xrpl_bridge() {
    let bridge = setup_b20_xrpl_bridge().await;

    let payment = B20Payment {
        id: "payment_id".to_string(),
        token: Address([0; 20]),
        from: Address([0; 20]),
        to: Address([0; 20]),
        amount: U256([1000, 0, 0, 0]),
        memo: None,
    };

    let escrow_id = bridge.b20_to_xrpl_escrow(&payment).await.unwrap();
    assert!(!escrow_id.is_empty());

    // Simula liberação do escrow XRPL
    let release_tx = bridge.xrpl_to_b20_release(&escrow_id, payment.to).await.unwrap();
    assert!(!release_tx.is_empty());
}
