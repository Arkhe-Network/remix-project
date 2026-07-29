//! ARKHE × DeSciOS — Integração para Ciência Descentralizada v0.2.0
//!
//! Módulos:
//! - `error` — Tipos de erro unificados
//! - `plugin_governance` — Validação de plugins contra invariantes
//! - `assistant_guardrails` — PII masking + content filtering + SSRF prevention
//! - `workflow_traceability` — Causal chains IC16 com blake3
//! - `publishing` — IPFS + WormGraph gRPC
//! - `nodes_desci` — Integração com nodes.desci
//! - `orcid` — ORCID ↔ DIDArkhe bridge
//! - `sei_giga` — SEI GigaChain on-chain anchoring
//!
//! # Features
//! - `ipfs` (default) — Habilita clientes HTTP para IPFS, ORCID, nodes.desci, SEI
//! - `orcid` (default) — Habilita cliente ORCID
//! - `sei-giga` — Habilita cliente SEI GigaChain

pub mod assistant_guardrails;
pub mod error;
pub mod nodes_desci;
pub mod orcid;
pub mod plugin_governance;
pub mod publishing;
pub mod sei_giga;
pub mod workflow_traceability;

// Re-exports principais
pub use assistant_guardrails::{
    AssistantContext, DeSciAssistantGuardrails, GuardrailCategory, GuardrailCheckResult,
    GuardrailConfig, PiiCheckResult, PiiMasker, PiiType, Redaction,
};
pub use error::{DesciError, Result};
pub use nodes_desci::{
    NodeDataset, NodeInfo, NodeRegistry, NodeSearchResult, NodeStatus, NodesDesciClient,
};
pub use orcid::{
    build_did_document, create_attestation, derive_did, verify_attestation, DidDocument,
    OrcidAttestation, OrcidClient, OrcidDID, OrcidProfile, DID_ORCID_PREFIX,
};
pub use plugin_governance::{PluginManifest, PluginValidator, ValidationCheck, ValidationResult};
pub use publishing::{
    DatasetMetadata, DeSciPublisher, IpfsClient, IpfsPublishResult, PublishResult,
    WormGraphNotifier,
};
pub use sei_giga::{
    compute_anchor_hash, AnchorEvent, AnchorInfo, AnchorMsg, IdentityInfo, RegisterIdentityMsg,
};
pub use workflow_traceability::{
    ScientificWorkflowTrace, StepId, StepStatus, WorkflowStep, WorkflowType,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
