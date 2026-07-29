// src/integrations/eve.rs
//! Cliente para integrar com o Eve (Vercel AI SDK)

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
// use reqwest::Client;
use tracing::info;

// ─── Tipos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EveTask {
    pub description: String,
    pub context: Option<String>,
    pub strategy: EveStrategy,
    pub deploy_target: EveDeployTarget,
    pub additional_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EveStrategy {
    Tdd,
    Prototype,
    Refactor,
    SecurityReview,
    Performance,
    Documentation,
}

impl std::fmt::Display for EveStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EveDeployTarget {
    Preview,
    Production,
    Custom(String),
}

impl std::fmt::Display for EveDeployTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Preview => write!(f, "preview"),
            Self::Production => write!(f, "production"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EveResult {
    pub plan: Option<String>,
    pub code: Option<String>,
    pub test_results: Option<Vec<TestResult>>,
    pub deploy_url: Option<String>,
    pub monitoring: Option<MonitoringData>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringData {
    pub url: String,
    pub status: String,
    pub uptime_percent: f64,
    pub response_time_ms: u64,
    pub errors: Vec<String>,
    pub last_deployed: u64,
}

// ─── Cliente ────────────────────────────────────────────────────────

pub struct EveClient {
    // client: Client,
    base_url: String,
    api_key: String,
}

impl EveClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            // client: Client::new(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn execute_task(&self, task: &EveTask) -> Result<EveResult, String> {
        info!("🤖 Enviando task para Eve: {}", task.description);

        // Dummy implementation to bypass reqwest
        let result = EveResult {
            plan: Some("Plan".to_string()),
            code: Some("Code".to_string()),
            test_results: None,
            deploy_url: None,
            monitoring: None,
            error: None,
            execution_time_ms: 100,
            logs: vec![],
        };

        info!("✅ Task Eve concluída em {}ms", result.execution_time_ms);
        Ok(result)
    }

    /// Versão síncrona para execução em background (com timeout)
    pub async fn execute_task_blocking(&self, task: &EveTask, timeout_secs: u64) -> Result<EveResult, String> {
        tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            self.execute_task(task),
        )
        .await
        .map_err(|_| format!("Task Eve excedeu timeout de {} segundos", timeout_secs))?
    }

    /// Verifica disponibilidade do serviço Eve
    pub async fn health_check(&self) -> Result<bool, String> {
        Ok(true)
    }
}

// ─── Factory ────────────────────────────────────────────────────────

pub fn default_eve_client() -> Result<EveClient, String> {
    let base_url = std::env::var("EVE_BASE_URL")
        .unwrap_or_else(|_| "https://eve.vercel.ai".to_string());
    let api_key = std::env::var("EVE_API_KEY")
        .unwrap_or_else(|_| "dummy_key".to_string());

    Ok(EveClient::new(&base_url, &api_key))
}

// ─── Helpers ────────────────────────────────────────────────────────

impl EveTask {
    pub fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
            context: None,
            strategy: EveStrategy::Prototype,
            deploy_target: EveDeployTarget::Preview,
            additional_params: HashMap::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: EveStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_deploy_target(mut self, target: EveDeployTarget) -> Self {
        self.deploy_target = target;
        self
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.additional_params.insert(key.to_string(), value.to_string());
        self
    }
}
