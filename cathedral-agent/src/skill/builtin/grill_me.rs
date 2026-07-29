//! /grill-me — Entrevista detalhada para alinhamento antes de agir

use crate::skill::types::{Skill, SkillType, SkillStep};

pub fn grill_me_skill() -> Skill {
    Skill {
        name: "grill-me".to_string(),
        description: "Get relentlessly grilled about a plan until every decision branch is resolved".to_string(),
        skill_type: SkillType::UserInvoked,
        version: "1.0.0".to_string(),
        author: Some("mattpocock".to_string()),
        tags: vec!["alignment".to_string(), "planning".to_string()],
        triggers: vec!["grill".to_string(), "plan".to_string(), "align".to_string()],
        instructions: r#"Use this skill when the user says `/grill-me` or when a plan needs thorough questioning.

1. Ask clarifying questions about the goal
2. Identify hidden assumptions
3. Challenge the approach
4. Explore edge cases
5. Suggest alternatives
6. Confirm understanding before proceeding"#.to_string(),
        steps: vec![
            SkillStep { order: 1, description: "Ask clarifying questions about the goal".to_string(), expected_output: "List of clarifications".to_string(), validation: None },
            SkillStep { order: 2, description: "Identify hidden assumptions".to_string(), expected_output: "List of assumptions".to_string(), validation: None },
            SkillStep { order: 3, description: "Challenge the approach".to_string(), expected_output: "Alternative approaches".to_string(), validation: None },
            SkillStep { order: 4, description: "Explore edge cases".to_string(), expected_output: "Edge case analysis".to_string(), validation: None },
            SkillStep { order: 5, description: "Suggest alternatives".to_string(), expected_output: "Alternative proposals".to_string(), validation: None },
            SkillStep { order: 6, description: "Confirm understanding before proceeding".to_string(), expected_output: "Confirmation summary".to_string(), validation: None },
        ],
        examples: vec![
            "/grill-me I want to build a new feature X".to_string(),
        ],
        dependencies: vec![],
        metadata: std::collections::HashMap::new(),
        okf_bundle_id: None,
    }
}
