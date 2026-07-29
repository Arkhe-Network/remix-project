// src/skill/builtin/opencut_skill.rs
//! Skill completa para OpenCut — operações de edição de vídeo

use crate::skill::types::{Skill, SkillType, SkillStep};
use std::collections::HashMap;

pub fn opencut_skill() -> Skill {
    Skill {
        name: "opencut".to_string(),
        description: "Edição de vídeo profissional com OpenCut".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("Cathedral ARKHE".to_string()),
        tags: vec![
            "video".to_string(),
            "editing".to_string(),
            "opencut".to_string(),
            "rendering".to_string(),
        ],
        triggers: vec![
            "video".to_string(),
            "editar".to_string(),
            "renderizar".to_string(),
            "cortar".to_string(),
            "transição".to_string(),
            "legenda".to_string(),
        ],
        instructions: "".to_string(),
        steps: vec![
            SkillStep {
                order: 1,
                description: "Parseia o comando e extrai parâmetros".to_string(),
                expected_output: "Estrutura OpenCutParams validada".to_string(),
                validation: Some("Todos os campos obrigatórios preenchidos".to_string()),
            },
            SkillStep {
                order: 2,
                description: "Valida arquivos de entrada (existência e formato)".to_string(),
                expected_output: "Arquivos validados".to_string(),
                validation: Some("Arquivos existem".to_string()),
            },
            SkillStep {
                order: 3,
                description: "Executa a operação via OpenCut CLI ou MCP".to_string(),
                expected_output: "Vídeo gerado".to_string(),
                validation: Some("Arquivo de saída criado".to_string()),
            },
            SkillStep {
                order: 4,
                description: "Armazena resultado no HashTree como OKF bundle".to_string(),
                expected_output: "ContentHash do bundle".to_string(),
                validation: None,
            },
        ],
        examples: vec![
            "renderizar projeto /projetos/meu-video.opencut --res 1920x1080".to_string(),
            "cortar video.mp4 --start 10 --duration 30".to_string(),
        ],
        dependencies: vec![
            "opencut-cli".to_string(),
            "ffmpeg".to_string(),
        ],
        metadata: {
            let mut m = HashMap::new();
            m.insert("operations".to_string(), "8".to_string());
            m.insert("mcp_supported".to_string(), "true".to_string());
            m
        },
        okf_bundle_id: None,
    }
}
