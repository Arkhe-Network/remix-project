// src/skill/builtin/eve.rs
//! Skill: Eve Developer Agent (Vercel)

use crate::skill::types::{Skill, SkillType, SkillStep};
use std::collections::HashMap;

pub fn eve_skill() -> Skill {
    let mut metadata = HashMap::new();
    metadata.insert("provider".to_string(), "vercel".to_string());
    metadata.insert("version".to_string(), "1.0.0".to_string());
    metadata.insert("strategies".to_string(), "tdd,prototype,refactor,security_review,performance,documentation".to_string());

    Skill {
        name: "eve-dev".to_string(),
        description: "Agente de desenvolvimento Eve da Vercel — entende requisitos, escreve código, testa, deploy, monitora".to_string(),
        skill_type: SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("Cathedral ARKHE / Vercel".to_string()),
        tags: vec![
            "development".to_string(),
            "coding".to_string(),
            "deploy".to_string(),
            "testing".to_string(),
            "ai".to_string(),
        ],
        triggers: vec![
            "desenvolver".to_string(),
            "codificar".to_string(),
            "deploy".to_string(),
            "testar".to_string(),
            "refatorar".to_string(),
            "eve".to_string(),
        ],
        instructions: r#"# 🤖 Eve Developer Agent

Esta skill invoca o Eve da Vercel, um agente de IA especializado em desenvolvimento de software.

## Operações

- **plan**: Gera um plano de desenvolvimento detalhado
- **code**: Escreve código para uma tarefa específica
- **test**: Gera e executa testes automatizados
- **deploy**: Faz deploy em Vercel (preview/production)
- **monitor**: Monitora aplicação em produção
- **iterate**: Itera com base em feedback humano ou métricas

## Estratégias Suportadas

- **tdd**: Desenvolvimento orientado a testes
- **prototype**: Prototipagem rápida
- **refactor**: Refatoração de código existente
- **security_review**: Revisão de segurança
- **performance**: Otimização de performance
- **documentation**: Geração de documentação

## Exemplo

```
/run eve-dev "Criar uma API REST para gerenciar produtos com autenticação JWT"
```

## Parâmetros Opcionais

- `--strategy tdd|prototype|refactor|security_review|performance|documentation`
- `--deploy preview|production`
- `--context <path>` (caminho para contexto do projeto)
- `--timeout <segundos>`
"#.to_string(),
        steps: vec![
            SkillStep {
                order: 1,
                description: "Entende requisitos e gera plano de desenvolvimento".to_string(),
                expected_output: "Plano detalhado".to_string(),
                validation: Some("Plano contém etapas claras".to_string()),
            },
            SkillStep {
                order: 2,
                description: "Escreve código conforme o plano".to_string(),
                expected_output: "Código-fonte gerado".to_string(),
                validation: Some("Código compila".to_string()),
            },
            SkillStep {
                order: 3,
                description: "Executa testes automatizados".to_string(),
                expected_output: "Resultados dos testes".to_string(),
                validation: Some("Todos os testes passam".to_string()),
            },
            SkillStep {
                order: 4,
                description: "Faz deploy (se solicitado)".to_string(),
                expected_output: "URL do deploy".to_string(),
                validation: Some("Deploy bem-sucedido".to_string()),
            },
            SkillStep {
                order: 5,
                description: "Monitora aplicação em produção".to_string(),
                expected_output: "Dados de monitoramento".to_string(),
                validation: Some("Aplicação está saudável".to_string()),
            },
        ],
        examples: vec![
            "Desenvolver uma API de autenticação com JWT".to_string(),
            "Refatorar o módulo de pagamentos".to_string(),
            "Fazer deploy da aplicação em produção".to_string(),
        ],
        dependencies: vec![
            "vercel-cli".to_string(),
            "eve-client".to_string(),
        ],
        metadata,
        okf_bundle_id: None,
    }
}
