// src/cli/thread_commands.rs
//! Comandos CLI para o sistema de threads

use crate::swarm::second_self::SecondSelfOrchestrator;
use crate::thread::index::ThreadIndex;
use crate::thread::schema::{SearchFilters, ThreadSource};
use crate::skill::builtin::thread_ingest_claude::ClaudeIngestExecutor;
use crate::skill::builtin::thread_search::ThreadSearchExecutor;

#[derive(Debug, Clone)]
pub enum ThreadCommand {
    IngestClaude { path: String, project: Option<String>, dry_run: bool },
    Search { query: String, source: Option<String>, project: Option<String>, limit: usize, hybrid: bool },
    Sync { force: bool },
    Stats,
}

impl ThreadCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { return None; }

        match parts[0] {
            "/thread-ingest" | "thread-ingest" => {
                if parts.len() >= 3 {
                    let source = parts[1];
                    let path = parts[2].to_string();
                    let mut project = None;
                    let mut dry_run = false;
                    let mut i = 3;
                    while i < parts.len() {
                        match parts[i] {
                            "--project" => {
                                if i + 1 < parts.len() {
                                    project = Some(parts[i + 1].to_string());
                                    i += 2;
                                } else { i += 1; }
                            }
                            "--dry-run" => {
                                dry_run = true;
                                i += 1;
                            }
                            _ => i += 1,
                        }
                    }
                    if source == "claude" {
                        Some(Self::IngestClaude { path, project, dry_run })
                    } else {
                        None
                    }
                } else { None }
            }
            "/thread-search" | "thread-search" => {
                if parts.len() >= 2 {
                    let query = parts[1..].join(" ");
                    let source = None;
                    let project = None;
                    let limit = 10;
                    let hybrid = false;
                    Some(Self::Search { query, source, project, limit, hybrid })
                } else { None }
            }
            "/thread-sync" | "thread-sync" => {
                Some(Self::Sync { force: true })
            }
            _ => None,
        }
    }

    pub async fn execute(
        &self,
        _orchestrator: &SecondSelfOrchestrator,
        _index: &ThreadIndex,
    ) -> Result<String, String> {
        match self {
            Self::IngestClaude { path, project, dry_run } => {
                let executor = ClaudeIngestExecutor::new(true);
                let result = executor.ingest(path, project.as_deref(), *dry_run).await?;

                let output = format!(
                    "📥 Importação do Claude concluída!\n   Total: {}\n   Novas: {}\n   Ignoradas: {}\n",
                    result.total, result.new, result.skipped
                );
                Ok(output)
            }
            Self::Search { query, source, project, limit, hybrid } => {
                let mut filters = SearchFilters::default();
                if let Some(src) = source {
                    filters = filters.with_source(ThreadSource::from_str(src));
                }
                if let Some(proj) = project {
                    filters = filters.with_project(proj);
                }

                let searcher = ThreadSearchExecutor::new(ThreadIndex::new().unwrap());
                let results: Vec<crate::thread::schema::Thread> = searcher.search(query, filters, *limit, *hybrid).await?;

                if results.is_empty() {
                    return Ok("🔍 Nenhum resultado encontrado.".to_string());
                }

                let mut output = format!("🔍 Resultados para '{}' ({} encontrados):\n\n", query, results.len());
                for (i, thread) in results.iter().enumerate() {
                    output.push_str(&format!(
                        "{}. [{}] {}\n   Fonte: {:?} | Projeto: {:?} | {} mensagens\n   ID: {}\n\n",
                        i + 1,
                        thread.source,
                        thread.title.as_deref().unwrap_or("Sem título"),
                        thread.source,
                        thread.project_context,
                        thread.messages.len(),
                        thread.id
                    ));
                }
                Ok(output)
            }
            Self::Sync { force: _ } => {
                Ok("🔄 Sincronização manual iniciada. Verifique os logs para detalhes.".to_string())
            }
            Self::Stats => {
                Ok("📊 Estatísticas do índice de threads (em breve)".to_string())
            }
        }
    }
}
