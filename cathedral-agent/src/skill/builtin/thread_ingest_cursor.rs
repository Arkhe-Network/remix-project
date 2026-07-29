// src/skill/builtin/thread_ingest_cursor.rs
//! Skill: Importação de histórico do Cursor IDE

use crate::skill::types::{Skill, SkillType, SkillStep};
use crate::thread::schema::{Thread, Message, MessageRole, ThreadSource, ThreadBuilder};
use serde_json::Value;
use std::collections::HashMap;
use tokio::fs;

pub fn thread_ingest_cursor_skill() -> Skill {
    Skill {
        name: "thread-ingest-cursor".to_string(),
        description: "Importa histórico de conversas do Cursor IDE para o HashTree".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("Cathedral ARKHE".to_string()),
        tags: vec!["thread".to_string(), "ingest".to_string(), "cursor".to_string()],
        triggers: vec!["importar cursor".to_string(), "cursor ingest".to_string()],
        instructions: "".to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Escaneia diretório de histórico do Cursor".to_string(), expected_output: "Arquivos .json".to_string(), validation: None },
            SkillStep { order: 2, description: "Parseia conversas do Cursor".to_string(), expected_output: "Threads".to_string(), validation: None },
            SkillStep { order: 3, description: "Salva no HashTree".to_string(), expected_output: "ContentHashes".to_string(), validation: None },
        ],
        examples: vec!["Importar histórico do Cursor ~/Library/Application Support/Cursor/User/workspaceStorage/".to_string()],
        dependencies: vec![],
        metadata: {
            let mut m = HashMap::new();
            m.insert("source".to_string(), "cursor".to_string());
            m
        },
        okf_bundle_id: None,
    }
}

pub struct CursorIngestExecutor {}

impl Default for CursorIngestExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorIngestExecutor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn ingest_from_dir(path: &str, project: Option<&str>) -> Result<Vec<Thread>, String> {
        let mut threads = Vec::new();
        let mut read_dir = tokio::fs::read_dir(path).await
            .map_err(|e| format!("Erro ao ler diretório {}: {}", path, e))?;

        while let Some(entry) = read_dir.next_entry().await
            .map_err(|e| format!("Erro ao ler entry: {}", e))?
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await
                    .map_err(|e| format!("Erro ao ler {}: {}", path.display(), e))?;
                if let Ok(json) = serde_json::from_str::<Value>(&content) {
                    if let Some(thread) = parse_cursor_file(&json, project)? {
                        threads.push(thread);
                    }
                }
            }
        }
        Ok(threads)
    }
}

fn parse_cursor_file(json: &Value, project: Option<&str>) -> Result<Option<Thread>, String> {
    let mut messages = Vec::new();
    let mut source_id = "unknown".to_string();
    let mut created_at = chrono::Utc::now().timestamp() as u64;

    if let Some(convs) = json.get("conversations").and_then(|v| v.as_object()) {
        if let Some((id, conv)) = convs.into_iter().next() {
            source_id = id.clone();
            if let Some(msgs) = conv.get("messages").and_then(|v| v.as_array()) {
                for msg in msgs {
                    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                    let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let timestamp = msg.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(created_at);
                    let role_enum = match role { "assistant" => MessageRole::Assistant, _ => MessageRole::User };
                    messages.push(Message { role: role_enum, content, timestamp, tokens: None, attachments: Vec::new() });
                }
            }
            if let Some(ts) = conv.get("created_at").and_then(|v| v.as_u64()) {
                created_at = ts;
            }

        }
    }

    if messages.is_empty() {
        return Ok(None);
    }

    let mut builder = ThreadBuilder::new(ThreadSource::Cursor, &source_id)
        .title("Cursor Conversation")
        .created_at(created_at);
    if let Some(proj) = project {
        builder = builder.project(proj);
    }
    for msg in messages {
        builder = builder.add_message(msg.role, &msg.content, msg.timestamp);
    }
    let thread = builder.build();
    Ok(Some(thread))
}
