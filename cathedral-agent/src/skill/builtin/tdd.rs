//! /tdd — Test-Driven Development

use crate::skill::types::{Skill, SkillType, SkillStep};

pub fn tdd_skill() -> Skill {
    Skill {
        name: "tdd".to_string(),
        description: "Test-Driven Development loop: red → green → refactor".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("mattpocock".to_string()),
        tags: vec!["testing".to_string(), "engineering".to_string()],
        triggers: vec!["test".to_string(), "tdd".to_string(), "red-green-refactor".to_string()],
        instructions: r#"Apply this skill automatically when writing code.

1. Write a failing test (Red)
2. Write minimal code to make it pass (Green)
3. Refactor (Refactor)
4. Repeat"#.to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Write a failing test (Red)".to_string(), expected_output: "Failing test".to_string(), validation: Some("Test must fail".to_string()) },
            SkillStep { order: 2, description: "Write minimal code to make it pass (Green)".to_string(), expected_output: "Passing code".to_string(), validation: Some("Code must pass the test".to_string()) },
            SkillStep { order: 3, description: "Refactor (Refactor)".to_string(), expected_output: "Refactored code".to_string(), validation: Some("Tests must still pass".to_string()) },
            SkillStep { order: 4, description: "Repeat".to_string(), expected_output: "Next cycle".to_string(), validation: None },
        ],
        examples: vec![],
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
        okf_bundle_id: None,
    }
}
