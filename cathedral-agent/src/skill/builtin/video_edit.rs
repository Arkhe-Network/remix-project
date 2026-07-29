// src/skill/builtin/video_edit.rs
//! Skill para edição de vídeos usando OpenCut

use crate::skill::types::{Skill, SkillType, SkillStep};
use std::collections::HashMap;

pub fn video_edit_skill() -> Skill {
    Skill {
        name: "video-edit".to_string(),
        description: "Edita e gera vídeos programaticamente usando OpenCut".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("Cathedral ARKHE".to_string()),
        tags: vec!["video".to_string(), "editing".to_string(), "opencut".to_string()],
        triggers: vec!["video".to_string(), "editar".to_string(), "renderizar".to_string()],
        instructions: "".to_string(),
        steps: vec![
            SkillStep {
                order: 1,
                description: "Analisa o comando e extrai parâmetros".to_string(),
                expected_output: "Parâmetros validados".to_string(),
                validation: Some("Parâmetros completos".to_string()),
            },
            SkillStep {
                order: 2,
                description: "Executa a ação no OpenCut (CLI ou MCP)".to_string(),
                expected_output: "Vídeo gerado".to_string(),
                validation: Some("Arquivo de saída existe".to_string()),
            },
            SkillStep {
                order: 3,
                description: "Armazena o resultado no HashTree".to_string(),
                expected_output: "ContentHash do vídeo".to_string(),
                validation: None,
            },
        ],
        examples: vec![
            "renderizar projeto /projetos/meu-video.opencut".to_string(),
            "cortar video.mp4 dos 10s aos 30s".to_string(),
        ],
        dependencies: vec!["opencut-cli".to_string(), "ffmpeg".to_string()],
        metadata: HashMap::new(),
        okf_bundle_id: None,
    }
}
