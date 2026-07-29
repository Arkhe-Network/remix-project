//! Definição de Skills — inspirado em mattpocock/skills

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillType {
    /// Invocada pelo usuário (ex: /grill-me, /to-prd)
    UserInvoked,
    /// Invocada pelo modelo automaticamente (ex: /tdd, /diagnose)
    ModelInvoked,
    /// Executada em background (ex: /improve-codebase-architecture)
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub triggers: Vec<String>,
    pub instructions: String,
    pub steps: Vec<SkillStep>,
    pub examples: Vec<String>,
    pub dependencies: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub okf_bundle_id: Option<String>, // referência ao OKF bundle
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    pub order: usize,
    pub description: String,
    pub expected_output: String,
    pub validation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecution {
    pub skill_name: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub status: ExecutionStatus,
    pub output: Option<Vec<u8>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Skill {
    pub fn from_markdown(content: &str, source: &str) -> Result<Self, String> {
        let (frontmatter, body) = Self::split_frontmatter(content)?;
        let metadata = Self::parse_yaml(&frontmatter);
        let steps = Self::extract_steps(&body);

        let name = metadata.get("name")
            .cloned()
            .unwrap_or_else(|| source.to_string());

        let skill_type = match metadata.get("type").map(|s| s.as_str()) {
            Some("model-invoked") | Some("model") => SkillType::ModelInvoked,
            Some("background") => SkillType::Background,
            _ => SkillType::UserInvoked,
        };

        let triggers = metadata.get("triggers")
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();

        let dependencies = metadata.get("dependencies")
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Self {
            name,
            description: metadata.get("description").cloned().unwrap_or_default(),
            skill_type,
            version: metadata.get("version").cloned().unwrap_or_else(|| "1.0.0".to_string()),
            author: metadata.get("author").cloned(),
            tags: metadata.get("tags").map(|s| s.split(',').map(|t| t.trim().to_string()).collect()).unwrap_or_default(),
            triggers,
            instructions: body,
            steps,
            examples: metadata.get("examples").map(|s| s.split(',').map(|t| t.trim().to_string()).collect()).unwrap_or_default(),
            dependencies,
            metadata,
            okf_bundle_id: None,
        })
    }

    // ─── Helpers ──────────────────────────────────────────────────────

    fn split_frontmatter(content: &str) -> Result<(String, String), String> {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() { return Ok((String::new(), String::new())); }
        if lines[0].trim() != "---" {
            return Ok((String::new(), content.to_string()));
        }
        let mut front = Vec::new();
        let mut body = Vec::new();
        let mut in_front = true;
        let mut found_end = false;
        for (i, line) in lines.iter().enumerate() {
            if i == 0 { continue; }
            if in_front && line.trim() == "---" {
                in_front = false;
                found_end = true;
                continue;
            }
            if in_front { front.push(*line); }
            else { body.push(*line); }
        }
        if !found_end { return Err("Frontmatter não fechado".to_string()); }
        Ok((front.join("\n"), body.join("\n")))
    }

    fn parse_yaml(yaml: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for line in yaml.lines() {
            if let Some((key, value)) = line.split_once(':') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        map
    }

    fn extract_steps(body: &str) -> Vec<SkillStep> {
        let mut steps = Vec::new();
        let mut order = 0;
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("1.") || trimmed.starts_with("2.") || trimmed.starts_with("- ") ||
               trimmed.starts_with("**Step") || trimmed.starts_with("Step") {
                let description = trimmed
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == ' ' || c == '*' || c == 'S' || c == 't' || c == 'e' || c == 'p')
                    .trim()
                    .to_string();
                if !description.is_empty() {
                    order += 1;
                    steps.push(SkillStep {
                        order,
                        description,
                        expected_output: String::new(),
                        validation: None,
                    });
                }
            }
        }
        steps
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            description: String::new(),
            skill_type: SkillType::UserInvoked,
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            triggers: vec![],
            instructions: String::new(),
            steps: vec![],
            examples: vec![],
            dependencies: vec![],
            metadata: HashMap::new(),
            okf_bundle_id: None,
        }
    }
}
