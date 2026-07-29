// src/skill/builtin/thread_search.rs
//! Skill: Busca unificada em threads

use crate::skill::types::{Skill, SkillType, SkillStep};
use crate::thread::index::ThreadIndex;
use crate::thread::schema::{Thread, SearchFilters};
use std::collections::HashMap;
use tracing::info;

pub fn thread_search_skill() -> Skill {
    Skill {
        name: "thread-search".to_string(),
        description: "Busca unificada em conversas de IA (Claude, Cursor, ChatGPT, etc.)".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("Cathedral ARKHE".to_string()),
        tags: vec!["thread".to_string(), "search".to_string(), "ai".to_string()],
        triggers: vec!["buscar thread".to_string(), "thread search".to_string(), "pesquisar".to_string()],
        instructions: "".to_string(),
        steps: vec![
            SkillStep {
                order: 1,
                description: "Interpreta a consulta e filtros".to_string(),
                expected_output: "Query estruturada".to_string(),
                validation: None,
            },
            SkillStep {
                order: 2,
                description: "Executa busca no índice híbrido".to_string(),
                expected_output: "Lista de resultados".to_string(),
                validation: None,
            },
            SkillStep {
                order: 3,
                description: "Formata e retorna resultados".to_string(),
                expected_output: "Resultados formatados".to_string(),
                validation: None,
            },
        ],
        examples: vec![
            "Buscar 'WebAssembly' no projeto engine".to_string(),
            "Mostrar conversas do Claude sobre Rust".to_string(),
        ],
        dependencies: vec!["sqlite".to_string(), "lance-db".to_string()],
        metadata: {
            let mut m = HashMap::new();
            m.insert("search_type".to_string(), "hybrid".to_string());
            m
        },
        okf_bundle_id: None,
    }
}

// ─── Executor ──────────────────────────────────────────────────────

pub struct ThreadSearchExecutor {
    index: ThreadIndex,
}

impl ThreadSearchExecutor {
    pub fn new(index: ThreadIndex) -> Self {
        Self { index }
    }

    pub async fn search(
        &self,
        query: &str,
        filters: SearchFilters,
        limit: usize,
        hybrid: bool,
    ) -> Result<Vec<Thread>, String> {
        info!("🔍 Buscando: '{}' (híbrida: {})", query, hybrid);

        if hybrid {
            // Placeholder
            self.index.search_hybrid(query, limit, &filters, 60).await
        } else {
            self.index.search_textual(query, limit, &filters).await
        }
    }
}
