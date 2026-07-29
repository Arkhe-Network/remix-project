//! /triage — Triagem de issues

use crate::skill::types::{Skill, SkillType, SkillStep};

pub fn triage_skill() -> Skill {
    Skill {
        name: "triage".to_string(),
        description: "Triage issues through a state machine of triage roles".to_string(),
        skill_type: SkillType::UserInvoked,
        version: "1.0.0".to_string(),
        author: Some("mattpocock".to_string()),
        tags: vec!["product".to_string(), "management".to_string()],
        triggers: vec!["triage".to_string(), "issue".to_string()],
        instructions: r#"Use when the user says `/triage` to classify and route issues.

1. Classify the issue type
2. Assess priority
3. Assign to the right person
4. Estimate effort
5. Route to appropriate workflow"#.to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Classify the issue type".to_string(), expected_output: "Issue classification".to_string(), validation: None },
            SkillStep { order: 2, description: "Assess priority".to_string(), expected_output: "Priority assessment".to_string(), validation: None },
            SkillStep { order: 3, description: "Assign to the right person".to_string(), expected_output: "Assignee".to_string(), validation: None },
            SkillStep { order: 4, description: "Estimate effort".to_string(), expected_output: "Effort estimate".to_string(), validation: None },
            SkillStep { order: 5, description: "Route to appropriate workflow".to_string(), expected_output: "Workflow routing".to_string(), validation: None },
        ],
        examples: vec![
            "/triage this bug report".to_string(),
        ],
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
        okf_bundle_id: None,
    }
}
