// src/thread/schema.rs
//! Schema unificado para threads de conversas de IA

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ─── Tipos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ThreadSource {
    Claude,
    Cursor,
    ChatGPT,
    Codex,
    ContinueDev,
    Aider,
    Windsurf,
    Custom(String),
}

impl std::fmt::Display for ThreadSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Cursor => write!(f, "cursor"),
            Self::ChatGPT => write!(f, "chatgpt"),
            Self::Codex => write!(f, "codex"),
            Self::ContinueDev => write!(f, "continue"),
            Self::Aider => write!(f, "aider"),
            Self::Windsurf => write!(f, "windsurf"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl ThreadSource {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude" => Self::Claude,
            "cursor" => Self::Cursor,
            "chatgpt" => Self::ChatGPT,
            "codex" => Self::Codex,
            "continue" => Self::ContinueDev,
            "aider" => Self::Aider,
            "windsurf" => Self::Windsurf,
            _ => Self::Custom(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,                     // Blake3 hash do conteúdo normalizado (hex)
    pub source: ThreadSource,
    pub source_id: String,              // ID original na ferramenta
    pub title: Option<String>,
    pub created_at: u64,                // Unix timestamp
    pub updated_at: u64,                // Unix timestamp
    pub project_context: Option<String>,
    pub model: Option<String>,
    pub messages: Vec<Message>,
    pub metadata: HashMap<String, String>,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,    // Embedding do resumo/título
    pub content_hash: String,           // Hash do conteúdo completo (para dedup)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: u64,
    pub tokens: Option<u32>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    pub content_hash: Option<String>,    // Referência ao HashTree
    pub content: Option<Vec<u8>>,        // Inline (para pequenos)
}

// ─── Normalização e Hashing ────────────────────────────────────────

impl Thread {
    /// Gera um ID determinístico a partir do conteúdo normalizado
    pub fn compute_id(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.source.to_string().as_bytes());
        hasher.update(self.source_id.as_bytes());
        hasher.update(&self.created_at.to_le_bytes());
        if let Some(proj) = &self.project_context {
            hasher.update(proj.as_bytes());
        }
        // Primeira mensagem como âncora
        if let Some(first) = self.messages.first() {
            hasher.update(first.content.as_bytes());
            hasher.update(&first.timestamp.to_le_bytes());
        }
        // Última mensagem também
        if let Some(last) = self.messages.last() {
            hasher.update(last.content.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    /// Calcula o hash do conteúdo completo (para deduplicação)
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        for msg in &self.messages {
            hasher.update(msg.content.as_bytes());
            hasher.update(&msg.timestamp.to_le_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    /// Normaliza uma thread: computa hashes e prepara para armazenamento
    pub fn normalize(&mut self) {
        self.content_hash = self.compute_content_hash();
        self.id = self.compute_id();
        self.updated_at = self.messages.last()
            .map(|m| m.timestamp)
            .unwrap_or(self.created_at);
    }
}

// ─── Construção ────────────────────────────────────────────────────

pub struct ThreadBuilder {
    pub thread: Thread,
}

impl ThreadBuilder {
    pub fn new(source: ThreadSource, source_id: &str) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            thread: Thread {
                id: String::new(),
                source,
                source_id: source_id.to_string(),
                title: None,
                created_at: now,
                updated_at: now,
                project_context: None,
                model: None,
                messages: Vec::new(),
                metadata: HashMap::new(),
                summary: None,
                embedding: None,
                content_hash: String::new(),
            },
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.thread.title = Some(title.to_string());
        self
    }

    pub fn project(mut self, project: &str) -> Self {
        self.thread.project_context = Some(project.to_string());
        self
    }

    pub fn model(mut self, model: &str) -> Self {
        self.thread.model = Some(model.to_string());
        self
    }

    pub fn created_at(mut self, ts: u64) -> Self {
        self.thread.created_at = ts;
        self
    }

    pub fn add_message(mut self, role: MessageRole, content: &str, timestamp: u64) -> Self {
        self.thread.messages.push(Message {
            role,
            content: content.to_string(),
            timestamp,
            tokens: None,
            attachments: Vec::new(),
        });
        self
    }

    pub fn add_message_with_tokens(mut self, role: MessageRole, content: &str, timestamp: u64, tokens: u32) -> Self {
        self.thread.messages.push(Message {
            role,
            content: content.to_string(),
            timestamp,
            tokens: Some(tokens),
            attachments: Vec::new(),
        });
        self
    }

    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.thread.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(mut self) -> Thread {
        self.thread.normalize();
        self.thread
    }

    pub fn add_message_with_attachments(
        mut self,
        role: MessageRole,
        content: &str,
        timestamp: u64,
        attachments: Vec<Attachment>,
    ) -> Self {
        self.thread.messages.push(Message {
            role,
            content: content.to_string(),
            timestamp,
            tokens: None,
            attachments,
        });
        self
    }
}

// ─── Filtros para busca ────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub source: Option<ThreadSource>,
    pub project: Option<String>,
    pub from_date: Option<u64>,
    pub to_date: Option<u64>,
    pub min_messages: Option<usize>,
    pub max_messages: Option<usize>,
    pub tags: Vec<String>,
}

impl SearchFilters {
    pub fn with_source(mut self, source: ThreadSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_project(mut self, project: &str) -> Self {
        self.project = Some(project.to_string());
        self
    }

    pub fn with_date_range(mut self, from: u64, to: u64) -> Self {
        self.from_date = Some(from);
        self.to_date = Some(to);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}
