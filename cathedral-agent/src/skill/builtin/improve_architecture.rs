//! /improve-codebase-architecture — Resgate de código

use crate::skill::types::{Skill, SkillType, SkillStep};

pub fn improve_architecture_skill() -> Skill {
    Skill {
        name: "improve-codebase-architecture".to_string(),
        description: "Regularly rescue codebases from accelerating entropy".to_string(),
        skill_type: SkillType::Background,
        version: "1.0.0".to_string(),
        author: Some("mattpocock".to_string()),
        tags: vec!["architecture".to_string(), "refactoring".to_string()],
        triggers: vec!["architecture".to_string(), "refactor".to_string(), "rescue".to_string()],
        instructions: r#"Run this skill periodically (e.g., weekly) to improve codebase architecture.

1. Identify areas of concern
2. Propose improvements
3. Implement changes incrementally
4. Validate with tests
5. Document decisions"#.to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Identify areas of concern".to_string(), expected_output: "List of concerns".to_string(), validation: None },
            SkillStep { order: 2, description: "Propose improvements".to_string(), expected_output: "Improvement proposals".to_string(), validation: None },
            SkillStep { order: 3, description: "Implement changes incrementally".to_string(), expected_output: "Code changes".to_string(), validation: None },
            SkillStep { order: 4, description: "Validate with tests".to_string(), expected_output: "Test results".to_string(), validation: None },
            SkillStep { order: 5, description: "Document decisions".to_string(), expected_output: "ADRs".to_string(), validation: None },
        ],
        examples: vec![],
        dependencies: vec!["diagnose".to_string()],
        metadata: std::collections::HashMap::new(),
        okf_bundle_id: None,
    }
}
