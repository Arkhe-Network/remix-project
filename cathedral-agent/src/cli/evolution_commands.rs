use crate::swarm::second_self::SecondSelfOrchestrator;

#[derive(Debug, Clone)]
pub enum EvolutionCommand {
    QVACInfer {
        prompt: String,
        model_hash: Option<String>,
    },
    LoRAFineTune {
        skill: String,
        goal: String,
        rank: u32,
        adapter_name: String,
    },
}

impl EvolutionCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "/qvac-infer" | "qvac-infer" => {
                if parts.len() >= 2 {
                    let mut prompt_parts = Vec::new();
                    let mut model_hash = None;
                    let mut i = 1;
                    while i < parts.len() {
                        if parts[i] == "--model" && i + 1 < parts.len() {
                            model_hash = Some(parts[i + 1].to_string());
                            i += 2;
                        } else {
                            prompt_parts.push(parts[i]);
                            i += 1;
                        }
                    }
                    Some(Self::QVACInfer { prompt: prompt_parts.join(" "), model_hash })
                } else {
                    None
                }
            }
            "/lora-finetune" | "lora-finetune" => {
                if parts.len() >= 4 {
                    Some(Self::LoRAFineTune {
                        skill: parts[1].to_string(),
                        goal: parts[2].to_string(),
                        rank: parts[3].parse().unwrap_or(16),
                        adapter_name: parts.get(4).map(|s| s.to_string()).unwrap_or_else(|| format!("lora-{}", parts[1])),
                    })
                } else { None }
            }
            _ => None,
        }
    }

    pub async fn execute(&self, orchestrator: &mut SecondSelfOrchestrator) -> Result<String, String> {
        match self {
            Self::QVACInfer { prompt, model_hash } => {
                let executor = orchestrator.qvac_executor.as_ref()
                    .ok_or("QVAC não inicializado. Use --init-evolution com --qvac.")?;
                let result = executor.infer(
                    prompt,
                    model_hash.as_deref(),
                    None, // trace_id opcional
                ).await?;
                Ok(format!("🧠 QVAC Result:\n{}", result))
            }
            Self::LoRAFineTune { skill, goal: _, rank: _, adapter_name } => {
                // Not fully implemented in orchestrator yet
                Ok(format!("✅ (Mock) Skill '{}' evoluída com LoRA (adaptador: {})", skill, adapter_name))
            }
        }
    }
}
