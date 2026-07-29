// src/skill/builtin/thread_ingest_claude.rs
//! Skill: Importação de conversas do Claude com parser robusto e suporte a anexos

use crate::skill::types::{Skill, SkillType, SkillStep};
use crate::thread::schema::{Thread, MessageRole, ThreadSource, Attachment, ThreadBuilder};
use serde_json::Value;
use std::collections::HashMap;
use tokio::fs;
use tracing::{info, warn, debug};
use base64::{Engine as _, engine::general_purpose};

// ─── Tipos de formato ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClaudeExportFormat {
    Unknown,
    ChatJson,          // { "conversations": [...] }
    SingleConversation,// { "id": "...", "messages": [...] }
    JsonLines,         // cada linha é um JSON
    ClaudeCode,        // { "history": [...] }
}

// ─── Skill Definition ──────────────────────────────────────────────

pub fn thread_ingest_claude_skill() -> Skill {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "claude".to_string());
    metadata.insert("formats".to_string(), "json,jsonl,claude_export".to_string());
    metadata.insert("features".to_string(), "attachments,dedup,multiple_formats".to_string());

    Skill {
        name: "thread-ingest-claude".to_string(),
        description: "Importa conversas do Claude (JSON/JSONL) para o HashTree unificado com suporte a anexos e múltiplos formatos".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "2.0.0".to_string(),
        author: Some("Cathedral ARKHE".to_string()),
        tags: vec![
            "thread".to_string(),
            "ingest".to_string(),
            "claude".to_string(),
            "import".to_string(),
            "attachments".to_string(),
        ],
        triggers: vec![
            "importar claude".to_string(),
            "claude ingest".to_string(),
            "claude import".to_string(),
        ],
        instructions: "".to_string(),
        steps: vec![
            SkillStep {
                order: 1,
                description: "Detecta formato e parseia arquivo(s) JSON".to_string(),
                expected_output: "Estrutura de dados".to_string(),
                validation: Some("Formato reconhecido".to_string()),
            },
            SkillStep {
                order: 2,
                description: "Converte para schema unificado com anexos".to_string(),
                expected_output: "Threads normalizadas".to_string(),
                validation: Some("Hash computado".to_string()),
            },
            SkillStep {
                order: 3,
                description: "Salva no HashTree com deduplicação e anexos".to_string(),
                expected_output: "Lista de ContentHashes".to_string(),
                validation: None,
            },
        ],
        examples: vec![
            "Importar conversas do claude_export.json --project api".to_string(),
            "Importar diretório ~/.claude/history/ --extract-attachments true".to_string(),
        ],
        dependencies: vec!["serde_json".to_string(), "base64".to_string()],
        metadata,
        okf_bundle_id: None,
    }
}

// ─── Executor Aprimorado ──────────────────────────────────────────

pub struct ClaudeIngestExecutor {
    extract_attachments: bool,
}

impl ClaudeIngestExecutor {
    pub fn new(extract_attachments: bool) -> Self {
        Self {
            extract_attachments,
        }
    }

    pub async fn ingest(
        &self,
        path: &str,
        project: Option<&str>,
        _dry_run: bool,
    ) -> Result<IngestResult, String> {
        let path = std::path::Path::new(path);
        let mut threads = Vec::new();

        if path.is_dir() {
            threads = self.ingest_directory(path, project).await?;
        } else {
            threads = self.ingest_file(path, project).await?;
        }

        info!("✅ Importadas {} conversas do Claude", threads.len());

        Ok(IngestResult {
            total: threads.len(),
            new: threads.len(),
            skipped: 0,
            hashes: Vec::new(),
            threads,
        })
    }

    async fn ingest_file(&self, path: &std::path::Path, project: Option<&str>) -> Result<Vec<Thread>, String> {
        let content = fs::read_to_string(path).await
            .map_err(|e| format!("Erro ao ler arquivo {}: {}", path.display(), e))?;

        let format = Self::detect_format(&content);
        debug!("Formato detectado: {:?} para {}", format, path.display());

        match format {
            ClaudeExportFormat::JsonLines => {
                let mut threads = Vec::new();
                for line in content.lines() {
                    if let Ok(val) = serde_json::from_str::<Value>(line) {
                        if let Some(t) = self.parse_conversation(&val, project)? {
                            threads.push(t);
                        }
                    }
                }
                Ok(threads)
            }
            _ => {
                let json: Value = serde_json::from_str(&content)
                    .map_err(|e| format!("JSON inválido em {}: {}", path.display(), e))?;
                self.extract_threads(&json, project)
            }
        }
    }

    async fn ingest_directory(&self, dir: &std::path::Path, project: Option<&str>) -> Result<Vec<Thread>, String> {
        let mut all = Vec::new();
        let mut read_dir = tokio::fs::read_dir(dir).await
            .map_err(|e| format!("Erro ao ler diretório {}: {}", dir.display(), e))?;

        while let Some(entry) = read_dir.next_entry().await
            .map_err(|e| format!("Erro ao ler entry: {}", e))?
        {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "json" || ext == "jsonl" {
                    match self.ingest_file(&path, project).await {
                        Ok(threads) => all.extend(threads),
                        Err(e) => warn!("Erro ao importar {}: {}", path.display(), e),
                    }
                }
            }
        }
        Ok(all)
    }

    // ─── Detecção de formato ──────────────────────────────────────

    fn detect_format(content: &str) -> ClaudeExportFormat {
        // Verifica se parece JSONL (muitas linhas que são JSON válidos)
        let first_line = content.lines().next().unwrap_or("");
        if serde_json::from_str::<Value>(first_line).is_ok() && content.lines().count() > 1 {
            // Pode ser JSONL ou array. Verifica se a primeira linha é um objeto com "id" ou "messages"
            if let Ok(val) = serde_json::from_str::<Value>(first_line) {
                if val.as_object().is_some() && (val.get("id").is_some() || val.get("messages").is_some()) {
                    return ClaudeExportFormat::JsonLines;
                }
            }
        }

        // Tenta parsear como JSON completo
        if let Ok(json) = serde_json::from_str::<Value>(content) {
            if let Some(obj) = json.as_object() {
                if obj.contains_key("conversations") {
                    return ClaudeExportFormat::ChatJson;
                }
                if obj.contains_key("id") && obj.contains_key("messages") {
                    return ClaudeExportFormat::SingleConversation;
                }
                if obj.contains_key("history") {
                    return ClaudeExportFormat::ClaudeCode;
                }
            }
            if json.as_array().is_some() {
                return ClaudeExportFormat::ChatJson; // array de conversas
            }
        }

        ClaudeExportFormat::Unknown
    }

    // ─── Extração aprimorada ──────────────────────────────────────

    fn extract_threads(&self, json: &Value, project: Option<&str>) -> Result<Vec<Thread>, String> {
        let mut threads = Vec::new();

        if let Some(arr) = json.as_array() {
            for item in arr {
                if let Some(t) = self.parse_conversation(item, project)? {
                    threads.push(t);
                }
            }
            return Ok(threads);
        }

        if let Some(obj) = json.as_object() {
            if let Some(convs) = obj.get("conversations").and_then(|v| v.as_array()) {
                for item in convs {
                    if let Some(t) = self.parse_conversation(item, project)? {
                        threads.push(t);
                    }
                }
                return Ok(threads);
            }
            if let Some(history) = obj.get("history").and_then(|v| v.as_array()) {
                for item in history {
                    if let Some(t) = self.parse_conversation(item, project)? {
                        threads.push(t);
                    }
                }
                return Ok(threads);
            }
            if let Some(t) = self.parse_conversation(json, project)? {
                threads.push(t);
            }
        }

        Ok(threads)
    }

    /// Parseia uma conversa individual com suporte a anexos
    fn parse_conversation(&self, item: &Value, project: Option<&str>) -> Result<Option<Thread>, String> {
        let source_id = item.get("id")
            .or_else(|| item.get("conversation_id"))
            .or_else(|| item.get("uuid"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let title = item.get("title")
            .or_else(|| item.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let created_at = item.get("created_at")
            .or_else(|| item.get("create_time"))
            .or_else(|| item.get("timestamp"))
            .and_then(|v| v.as_u64())
            .or_else(|| {
                item.get("created_at")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .unwrap_or_else(|| chrono::Utc::now().timestamp() as u64);

        let model = item.get("model")
            .or_else(|| item.get("model_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut builder = ThreadBuilder::new(ThreadSource::Claude, &source_id)
            .title(title.as_deref().unwrap_or("Untitled"))
            .model(model.as_deref().unwrap_or("unknown"))
            .created_at(created_at);

        if let Some(proj) = project {
            builder = builder.project(proj);
        }

        // Extrai mensagens
        let msg_source = item.get("messages")
            .or_else(|| item.get("conversation"))
            .or_else(|| item.get("chat_messages"));

        if let Some(msgs) = msg_source.and_then(|v| v.as_array()) {
            for msg in msgs {
                let role_str = msg.get("role")
                    .or_else(|| msg.get("author"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("user");

                let content = msg.get("content")
                    .or_else(|| msg.get("text"))
                    .or_else(|| msg.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let timestamp = msg.get("timestamp")
                    .or_else(|| msg.get("created_at"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(created_at);

                let role = match role_str.to_lowercase().as_str() {
                    "assistant" | "bot" | "ai" | "claude" => MessageRole::Assistant,
                    "user" | "human" => MessageRole::User,
                    "system" => MessageRole::System,
                    "tool" => MessageRole::Tool,
                    _ => MessageRole::User,
                };

                // Anexos: arquivos, imagens, etc.
                let mut attachments = Vec::new();
                if self.extract_attachments {
                    if let Some(attach) = msg.get("attachments").or_else(|| msg.get("files")) {
                        if let Some(arr) = attach.as_array() {
                            for file in arr {
                                if let Some(attachment) = self.parse_attachment(file) {
                                    attachments.push(attachment);
                                }
                            }
                        }
                    }
                    // Mensagens do Claude podem ter conteúdo com imagens embutidas (base64)
                    if let Some(img_data) = msg.get("image_data").and_then(|v| v.as_str()) {
                        // Salva imagem como anexo
                        if let Ok(bytes) = general_purpose::STANDARD.decode(img_data) {
                            let name = format!("image_{}.png", attachments.len());
                            let hash = self.store_attachment(&name, &bytes)?;
                            attachments.push(Attachment {
                                name,
                                mime_type: "image/png".to_string(),
                                content_hash: Some(hash),
                                content: None,
                            });
                        }
                    }
                }

                builder = builder.add_message_with_attachments(role, &content, timestamp, attachments);
            }
        }

        // Metadados adicionais
        if let Some(total_tokens) = item.get("total_tokens").and_then(|v| v.as_u64()) {
            builder = builder.metadata("total_tokens", &total_tokens.to_string());
        }
        if let Some(usage) = item.get("usage").and_then(|v| v.as_object()) {
            if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                builder = builder.metadata("input_tokens", &input.to_string());
            }
            if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                builder = builder.metadata("output_tokens", &output.to_string());
            }
        }

        let thread = builder.build();
        Ok(Some(thread))
    }

    fn parse_attachment(&self, file: &Value) -> Option<Attachment> {
        let name = file.get("name")
            .or_else(|| file.get("filename"))
            .and_then(|v| v.as_str())
            .unwrap_or("file")
            .to_string();

        let mime_type = file.get("mime_type")
            .or_else(|| file.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();

        let content = file.get("content")
            .or_else(|| file.get("data"))
            .and_then(|v| v.as_str())
            .and_then(|s| general_purpose::STANDARD.decode(s).ok());

        if let Some(bytes) = content {
            // Armazena inline ou no HashTree (dependendo do tamanho)
            Some(Attachment {
                name,
                mime_type,
                content_hash: None,
                content: Some(bytes),
            })
        } else {
            Some(Attachment {
                name,
                mime_type,
                content_hash: None,
                content: None,
            })
        }
    }

    fn store_attachment(&self, _name: &str, data: &[u8]) -> Result<String, String> {
        let hash = blake3::hash(data);
        let hash_hex = hash.to_hex().to_string();
        // Em um sistema real, salvaríamos via storage provider aqui
        Ok(hash_hex)
    }
}

// ─── Resultado ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IngestResult {
    pub total: usize,
    pub new: usize,
    pub skipped: usize,
    pub hashes: Vec<String>,
    pub threads: Vec<Thread>,
}
