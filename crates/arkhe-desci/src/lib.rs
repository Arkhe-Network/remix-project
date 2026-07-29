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

pub mod error;
pub mod plugin_governance;
pub mod assistant_guardrails;
pub mod workflow_traceability;
pub mod publishing;
pub mod nodes_desci;
pub mod orcid;
pub mod sei_giga;

// Re-exports principais
pub use error::{DesciError, Result};
pub use plugin_governance::{PluginValidator, PluginManifest, ValidationResult, ValidationCheck};
pub use assistant_guardrails::{
    DeSciAssistantGuardrails, AssistantContext, GuardrailConfig,
    GuardrailCheckResult, GuardrailCategory, PiiMasker, PiiCheckResult,
    Redaction, PiiType,
};
pub use workflow_traceability::{
    ScientificWorkflowTrace, WorkflowStep, WorkflowType, StepId, StepStatus,
};
pub use publishing::{
    DatasetMetadata, IpfsPublishResult, PublishResult,
    IpfsClient, WormGraphNotifier, DeSciPublisher,
};
pub use nodes_desci::{
    NodeInfo, NodeStatus, NodeDataset, NodeSearchResult,
    NodesDesciClient, NodeRegistry,
};
pub use orcid::{
    OrcidProfile, OrcidDID, DidDocument, OrcidAttestation,
    OrcidClient, derive_did, build_did_document,
    create_attestation, verify_attestation, DID_ORCID_PREFIX,
};
pub use sei_giga::{
    AnchorMsg, RegisterIdentityMsg, AnchorInfo, IdentityInfo,
    AnchorEvent, compute_anchor_hash,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
