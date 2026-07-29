use crate::skill::types::{Skill, SkillType, SkillStep};
use std::collections::HashMap;

pub fn desci_review_skill() -> Skill {
    Skill {
        name: "desci-review".to_string(),
        description: "Realiza revisão por pares de um Research Object no DeSci".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        instructions: "Review a DeSci node".to_string(),
        author: Some("Cathedral ARKHE".to_string()),
        tags: vec!["desci".to_string(), "review".to_string(), "peer-review".to_string()],
        triggers: vec!["revisar pesquisa".to_string(), "peer review".to_string()],
        steps: vec![
            SkillStep {
                order: 1,
                description: "Carrega Research Object do HashTree ou dPID".to_string(),
                expected_output: "Conteúdo carregado".to_string(),
                validation: None,
            },
            SkillStep {
                order: 2,
                description: "Analisa o Research Object com agentes especializados".to_string(),
                expected_output: "Review estruturado".to_string(),
                validation: None,
            },
            SkillStep {
                order: 3,
                description: "Publica review como OKF Bundle com assinatura".to_string(),
                expected_output: "Review publicado".to_string(),
                validation: None,
            },
        ],
        examples: vec!["Revisar research object sobre WebAssembly".to_string()],
        dependencies: vec!["desci-api".to_string()],
        metadata: {
            let mut m = HashMap::new();
            m.insert("platform".to_string(), "desci".to_string());
            m
        },
        okf_bundle_id: None,
    }
}
