#![allow(warnings)]
pub mod orchestrator;
// pub mod mcp;
// pub mod testing;
pub mod attestation {
    pub struct AttestationManager {}
    impl AttestationManager {
        pub fn new(_store: Option<std::sync::Arc<crate::memory::TrajectoryStore>>) -> Self { Self {} }
    }
    pub trait AttestationProvider: Send + Sync {
        fn run<'a>(&'a self, task: &'a str, cost: Option<f64>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>;
    }
    pub trait AttestationVerifier: Send + Sync {
        fn verify(&self, data: &str, sig: &str) -> Result<bool, String>;
    }
    pub trait AttestationSigner: Send + Sync {
        fn sign(&self, data: &str) -> Result<String, String>;
    }
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
    pub struct ExecutionAttestation { pub id: String, pub cost_usd: f64 }
    impl ExecutionAttestation {
        pub fn new(_task: &str, _result: &str, _id: &str, _cost: f64, _tags: Vec<String>, _score: f64, _key: &str) -> Self { Self { id: "".to_string(), cost_usd: 0.0 } }
        pub fn sign(&mut self, _signer: &dyn AttestationSigner) -> Result<(), String> { Ok(()) }
        pub fn is_policy_compliant(&self) -> bool { true }
    }
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
    pub struct IdentityAttestation {
        pub id: String, pub timestamp: String, pub architect_id: String,
        pub voice_hash: String, pub biometric_score: f64, pub coercion_score: f64,
        pub blockchain_signature_id: Option<String>, pub hardware_fingerprint: Option<String>,
        pub confidence: f64, pub signature: Option<String>, pub signer_key_id: String, pub metadata: serde_json::Value
    }
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
    pub struct PolicyDescriptor { pub blocking: bool, pub name: String }
    pub trait Canonicalizable {}
    pub mod canonical {
        pub fn canonical_json<T: serde::Serialize>(_data: &T) -> Result<String, String> { Ok("".to_string()) }
    }
}
pub mod memory {
    #[derive(Debug)]
    pub struct TrajectoryStore {}
    impl Default for TrajectoryStore {
        fn default() -> Self {
            Self::new()
        }
    }

    impl TrajectoryStore {
        pub fn new() -> Self { Self {} }
        pub async fn record_trajectory(&self, _agent: &str, _goal: &str, _tags: Vec<String>, _result: &str, _in: Vec<String>, _out: Vec<String>) -> Result<String, String> { Ok("".to_string()) }
        pub async fn list_trajectories(&self) -> Vec<crate::testing::test_agent::Trajectory> { vec![] }
    }
}
pub mod testing {
    pub mod test_agent {
        #[derive(Clone)]
        pub struct Trajectory { pub agent_id: String, pub goal: String, pub final_result: String }
    }
}
pub mod identity_attestation {
    pub trait IdentityAttestationProvider: Send + Sync {}
}
pub mod voice {
    pub struct VoiceCore {}
}
pub mod security {
    pub mod blockchain_nervous_system {
        pub struct BlockchainNervousSystem {}
    }
}
pub mod governance {
    pub struct GeometricPolicyEngine {}
    impl Default for GeometricPolicyEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GeometricPolicyEngine {
        pub fn new() -> Self { Self {} }
        pub async fn list_active_policies(&self) -> Result<Vec<crate::attestation::PolicyDescriptor>, String> { Ok(vec![]) }
    }
}
pub mod integrations;
pub mod skill;
pub mod swarm;
pub mod cli;
pub mod thread;
pub mod evolution;
pub mod hashtree;
pub mod observability;
pub mod rfq;
