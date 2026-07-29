//! /diagnose — Diagnóstico disciplinado de bugs

use crate::skill::types::{Skill, SkillType, SkillStep};

pub fn diagnose_skill() -> Skill {
    Skill {
        name: "diagnose".to_string(),
        description: "Disciplined diagnosis loop: reproduce → minimise → hypothesise → instrument → fix → regression-test".to_string(),
        skill_type: SkillType::UserInvoked,
        version: "1.0.0".to_string(),
        author: Some("mattpocock".to_string()),
        tags: vec!["debug".to_string(), "engineering".to_string()],
        triggers: vec!["diagnose".to_string(), "debug".to_string(), "fix".to_string()],
        instructions: r#"Use when the user says `/diagnose` or needs to debug an issue.

1. Reproduce the issue
2. Minimise the test case
3. Formulate a hypothesis
4. Instrument the code
5. Fix the issue
6. Write a regression test"#.to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Reproduce the issue".to_string(), expected_output: "Reproduction steps".to_string(), validation: None },
            SkillStep { order: 2, description: "Minimise the test case".to_string(), expected_output: "Minimal test case".to_string(), validation: None },
            SkillStep { order: 3, description: "Formulate a hypothesis".to_string(), expected_output: "Hypothesis".to_string(), validation: None },
            SkillStep { order: 4, description: "Instrument the code".to_string(), expected_output: "Instrumentation logs".to_string(), validation: None },
            SkillStep { order: 5, description: "Fix the issue".to_string(), expected_output: "Fix description".to_string(), validation: None },
            SkillStep { order: 6, description: "Write a regression test".to_string(), expected_output: "Regression test".to_string(), validation: None },
        ],
        examples: vec![
            "/diagnose the build is failing".to_string(),
        ],
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
        okf_bundle_id: None,
    }
}
