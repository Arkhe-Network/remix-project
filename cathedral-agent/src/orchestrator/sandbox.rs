//! Cathedral ARKHE v28.3.2 — Sandbox para Subagentes
//! Isolamento de execução via WASM (wasmtime) ou processos filhos.
//! Selo: CATHEDRAL-ARKHE-v28.3.2-SANDBOX-2026-06-17

use std::sync::Arc;
use async_trait::async_trait;
use tokio::process::Command;

// ============================================================================
// 1. Trait Sandbox
// ============================================================================

#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn execute(&self, code: &str, input: &str) -> Result<String, String>;
    async fn spawn(&self) -> Result<(), String>;
    async fn terminate(&self) -> Result<(), String>;
}

// ============================================================================
// 2. Implementação com Processo Filho (Fallback)
// ============================================================================

pub struct ProcessSandbox {
    cmd: String,
    args: Vec<String>,
}

impl ProcessSandbox {
    pub fn new(cmd: &str, args: Vec<String>) -> Self {
        Self {
            cmd: cmd.to_string(),
            args,
        }
    }
}

#[async_trait]
impl Sandbox for ProcessSandbox {
    async fn execute(&self, code: &str, input: &str) -> Result<String, String> {
        let output = Command::new(&self.cmd)
            .args(&self.args)
            .arg(code)
            .arg(input)
            .output()
            .await
            .map_err(|e| format!("Erro ao executar processo: {}", e))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn spawn(&self) -> Result<(), String> { Ok(()) }
    async fn terminate(&self) -> Result<(), String> { Ok(()) }
}

// ============================================================================
// 4. Fábrica de Sandbox
// ============================================================================

pub enum SandboxType {
    Process { cmd: String, args: Vec<String> },
}

pub fn create_sandbox(sandbox_type: SandboxType) -> Arc<dyn Sandbox> {
    match sandbox_type {
        SandboxType::Process { cmd, args } => Arc::new(ProcessSandbox::new(&cmd, args)),
    }
}