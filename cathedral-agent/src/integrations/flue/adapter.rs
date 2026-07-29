// src/integrations/flue/adapter.rs
//! Adapter para implantar Skills do Cathedral ARKHE como Workers Flue (Cloudflare)

use crate::skill::types::Skill;
// use crate::swarm::types::SwarmSpec;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

// ─── Tipos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlueWorkerConfig {
    pub name: String,
    pub description: String,
    pub entrypoint: String,
    pub tools: Vec<FlueTool>,
    pub env_vars: HashMap<String, String>,
    pub resources: FlueResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlueTool {
    pub name: String,
    pub description: String,
    pub handler: String, // Nome da função no Worker
    pub parameters: FlueParameterSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlueParameterSchema {
    pub r#type: String,
    pub properties: HashMap<String, FlueParameter>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlueParameter {
    pub r#type: String,
    pub description: String,
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlueResources {
    pub cpu: String,
    pub memory: String,
    pub timeout: u64,
}

// ─── Adapter ──────────────────────────────────────────────────────────

pub struct FlueAdapter {
    output_dir: PathBuf,
    worker_name_prefix: String,
    cloudflare_account_id: Option<String>,
    cloudflare_api_token: Option<String>,
}

impl FlueAdapter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            worker_name_prefix: "cathedral-skill-".to_string(),
            cloudflare_account_id: std::env::var("CLOUDFLARE_ACCOUNT_ID").ok(),
            cloudflare_api_token: std::env::var("CLOUDFLARE_API_TOKEN").ok(),
        }
    }

    /// Converte uma Skill para um Worker Flue (gerando código TypeScript)
    pub async fn skill_to_worker(&self, skill: &Skill) -> Result<FlueWorkerConfig, String> {
        info!("🔄 Convertendo skill '{}' para Worker Flue", skill.name);

        let mut tools = Vec::new();

        // Cria tools baseadas nos passos da skill
        for (i, step) in skill.steps.iter().enumerate() {
            let tool_name = format!("step_{}", i + 1);
            tools.push(FlueTool {
                name: tool_name.clone(),
                description: step.description.clone(),
                handler: format!("handle_{}", tool_name),
                parameters: FlueParameterSchema {
                    r#type: "object".to_string(),
                    properties: {
                        let mut props = HashMap::new();
                        props.insert("input".to_string(), FlueParameter {
                            r#type: "string".to_string(),
                            description: "Input para o passo".to_string(),
                            default: None,
                        });
                        props
                    },
                    required: vec!["input".to_string()],
                },
            });
        }

        // Adiciona tool de execução completa
        tools.push(FlueTool {
            name: "execute".to_string(),
            description: format!("Executa a skill '{}'", skill.name),
            handler: "handle_execute".to_string(),
            parameters: FlueParameterSchema {
                r#type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("input".to_string(), FlueParameter {
                        r#type: "string".to_string(),
                        description: "Descrição da tarefa".to_string(),
                        default: None,
                    });
                    props.insert("strategy".to_string(), FlueParameter {
                        r#type: "string".to_string(),
                        description: "Estratégia de execução".to_string(),
                        default: Some(serde_json::Value::String("default".to_string())),
                    });
                    props
                },
                required: vec!["input".to_string()],
            },
        });

        let config = FlueWorkerConfig {
            name: format!("{}{}", self.worker_name_prefix, skill.name.replace('-', "_")),
            description: skill.description.clone(),
            entrypoint: "src/index.ts".to_string(),
            tools,
            env_vars: {
                let mut env = HashMap::new();
                env.insert("SKILL_NAME".to_string(), skill.name.clone());
                env.insert("SKILL_VERSION".to_string(), skill.version.clone());
                env
            },
            resources: FlueResources {
                cpu: "1".to_string(),
                memory: "512 MB".to_string(),
                timeout: 60,
            },
        };

        Ok(config)
    }

    /// Gera o código TypeScript do Worker Flue
    pub async fn generate_worker_code(&self, skill: &Skill, config: &FlueWorkerConfig) -> Result<String, String> {
        let mut code = String::new();

        // Header
        code.push_str("// Cathedral ARKHE — Skill Worker (Flue SDK)\n");
        code.push_str(&format!("// Skill: {}\n", skill.name));
        code.push_str(&format!("// Version: {}\n", skill.version));
        code.push_str("// Generated by FlueAdapter\n\n");

        // Imports
        code.push_str("import { Agent, tool, run } from '@cloudflare/flue';\n");
        code.push_str("import { Hono } from 'hono';\n\n");

        // Constantes
        code.push_str(&format!("const SKILL_NAME = '{}';\n", skill.name));
        code.push_str(&format!("const SKILL_DESCRIPTION = '{}';\n", skill.description));
        code.push_str(&format!("const SKILL_VERSION = '{}';\n\n", skill.version));

        // Tools
        for tool in &config.tools {
            code.push_str(&format!(
                r#"
const {} = tool({{
    name: '{}',
    description: '{}',
    parameters: {},
    handler: async (input) => {{
        console.log('Tool {} called with input:', input);
        // Implementation generated from skill step
        return {{ result: 'Step completed', input }};
    }},
}});
"#,
                tool.handler,
                tool.name,
                tool.description,
                serde_json::to_string_pretty(&tool.parameters).unwrap_or_default(),
                tool.name
            ));
        }

        // Agent
        code.push_str(&format!(r#"
const agent = new Agent({{
    name: SKILL_NAME,
    description: SKILL_DESCRIPTION,
    version: SKILL_VERSION,
    tools: [{}],
    async run(context) {{
        console.log('Executing skill:', SKILL_NAME);
        // Execute steps in sequence
        let result = {{}};
        for (const step of context.steps || []) {{
            const output = await context.callTool(step.tool, step.input);
            result[step.name] = output;
        }}
        return {{ result, metadata: {{ skill: SKILL_NAME, version: SKILL_VERSION }} }};
    }},
}});
"#,
            config.tools.iter().map(|t| t.handler.clone()).collect::<Vec<_>>().join(", ")
        ));

        // Hono app
        code.push_str(r#"
const app = new Hono();

app.post('/execute', async (c) => {
    const body = await c.req.json();
    const result = await agent.run(body);
    return c.json({ success: true, result });
});

app.get('/health', (c) => c.json({ status: 'ok', skill: SKILL_NAME }));

export default app;
"#);

        Ok(code)
    }

    /// Escreve o worker em disco e prepara para deploy
    pub async fn write_worker_files(&self, skill: &Skill) -> Result<PathBuf, String> {
        let config = self.skill_to_worker(skill).await?;
        let code = self.generate_worker_code(skill, &config).await?;

        let skill_dir = self.output_dir.join(&skill.name);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| format!("Erro ao criar diretório: {}", e))?;

        // Escreve index.ts
        let index_path = skill_dir.join("index.ts");
        std::fs::write(&index_path, code)
            .map_err(|e| format!("Erro ao escrever index.ts: {}", e))?;

        // Escreve wrangler.toml
        let wrangler_toml = self.generate_wrangler_toml(&config);
        let wrangler_path = skill_dir.join("wrangler.toml");
        std::fs::write(&wrangler_path, wrangler_toml)
            .map_err(|e| format!("Erro ao escrever wrangler.toml: {}", e))?;

        // Escreve package.json
        let package_json = r#"{
            "name": "cathedral-skill",
            "version": "1.0.0",
            "scripts": {
                "deploy": "wrangler deploy",
                "dev": "wrangler dev"
            },
            "dependencies": {
                "@cloudflare/flue": "^0.1.0",
                "hono": "^4.0.0"
            },
            "devDependencies": {
                "@cloudflare/workers-types": "^4.0.0",
                "wrangler": "^3.0.0",
                "typescript": "^5.0.0"
            }
        }"#;
        let package_path = skill_dir.join("package.json");
        std::fs::write(&package_path, package_json)
            .map_err(|e| format!("Erro ao escrever package.json: {}", e))?;

        info!("✅ Worker Flue gerado em: {}", skill_dir.display());
        Ok(skill_dir)
    }

    /// Gera wrangler.toml
    fn generate_wrangler_toml(&self, config: &FlueWorkerConfig) -> String {
        let mut toml = String::new();

        toml.push_str(&format!("name = \"{}\"\n", config.name));
        toml.push_str("main = \"src/index.ts\"\n");
        toml.push_str("compatibility_date = \"2025-01-01\"\n");
        toml.push_str("\n[vars]\n");
        for (k, v) in &config.env_vars {
            toml.push_str(&format!("{} = \"{}\"\n", k, v));
        }

        if let Some(account_id) = &self.cloudflare_account_id {
            toml.push_str("\n[env.production]\n");
            toml.push_str(&format!("account_id = \"{}\"\n", account_id));
        }

        toml.push_str("\n[build]\n");
        toml.push_str("command = \"npm run build\"\n");
        toml.push_str("cwd = \".\"\n");

        toml
    }

    /// Faz deploy do worker usando wrangler
    pub async fn deploy_worker(&self, skill_name: &str) -> Result<String, String> {
        let skill_dir = self.output_dir.join(skill_name);
        if !skill_dir.exists() {
            return Err(format!("Skill '{}' não foi gerada como Worker", skill_name));
        }

        // Executa npm install e deploy via wrangler
        let install_output = std::process::Command::new("npm")
            .arg("install")
            .current_dir(&skill_dir)
            .output()
            .map_err(|e| format!("Erro ao executar npm install: {}", e))?;

        if !install_output.status.success() {
            let stderr = String::from_utf8_lossy(&install_output.stderr);
            return Err(format!("npm install falhou: {}", stderr));
        }

        let deploy_output = std::process::Command::new("npx")
            .args(["wrangler", "deploy", "--name", &format!("cathedral-skill-{}", skill_name)])
            .current_dir(&skill_dir)
            .output()
            .map_err(|e| format!("Erro ao executar wrangler deploy: {}", e))?;

        if !deploy_output.status.success() {
            let stderr = String::from_utf8_lossy(&deploy_output.stderr);
            return Err(format!("wrangler deploy falhou: {}", stderr));
        }

        // Extrai a URL do deploy
        let stdout = String::from_utf8_lossy(&deploy_output.stdout);
        let url = stdout.lines()
            .find(|line| line.contains("https://"))
            .map(|line| line.trim().to_string())
            .unwrap_or_else(|| format!("https://cathedral-skill-{}.workers.dev", skill_name));

        info!("✅ Worker '{}' implantado em: {}", skill_name, url);
        Ok(url)
    }
}
