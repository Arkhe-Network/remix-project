//! /to-prd — Converte contexto em PRD estruturado

use crate::skill::types::{Skill, SkillType, SkillStep};

pub fn to_prd_skill() -> Skill {
    Skill {
        name: "to-prd".to_string(),
        description: "Turn conversation context into a structured PRD".to_string(),
        skill_type: SkillType::UserInvoked,
        version: "1.0.0".to_string(),
        author: Some("mattpocock".to_string()),
        tags: vec!["product".to_string(), "documentation".to_string()],
        triggers: vec!["prd".to_string(), "spec".to_string(), "product".to_string()],
        instructions: r#"Use when the user says `/to-prd` or needs to convert a conversation into a Product Requirements Document.

1. Extract the problem statement
2. Define user stories
3. Specify functional requirements
4. Specify non-functional requirements
5. Define acceptance criteria
6. Generate the PRD"#.to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Extract the problem statement".to_string(), expected_output: "Problem statement".to_string(), validation: None },
            SkillStep { order: 2, description: "Define user stories".to_string(), expected_output: "User stories".to_string(), validation: None },
            SkillStep { order: 3, description: "Specify functional requirements".to_string(), expected_output: "Functional requirements".to_string(), validation: None },
            SkillStep { order: 4, description: "Specify non-functional requirements".to_string(), expected_output: "Non-functional requirements".to_string(), validation: None },
            SkillStep { order: 5, description: "Define acceptance criteria".to_string(), expected_output: "Acceptance criteria".to_string(), validation: None },
            SkillStep { order: 6, description: "Generate the PRD".to_string(), expected_output: "PRD document".to_string(), validation: None },
        ],
        examples: vec![
            "/to-prd we discussed building X".to_string(),
        ],
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
        okf_bundle_id: None,
    }
}
