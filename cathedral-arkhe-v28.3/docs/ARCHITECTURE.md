# Cathedral ARKHE v28.3 – Architecture

## 1. Visão Geral

O Cathedral ARKHE é uma plataforma de agentes autónomos com consciência coletiva, projetada para **segurança pós‑quântica**, **verificabilidade imutável** e **governança descentralizada**. A versão v28.3 integra:

- **LLM Server** (Axum/vLLM‑compatible) com circuit breaker e telemetry.
- **Multi‑Agent Orchestrator** com suporte a:
  - **Consenso** (majoritário, ponderado, unânime, hierárquico, Delphi)
  - **Debate estruturado** com 5 critérios de avaliação
  - **Hierarquia de comando** com delegação e sobrescrita
- **ACP (Agent Communication Protocol)** com assinatura SPHINCS+ e proveniência
- **Event Bus** (Redis Pub/Sub + Redis Streams para replay)
- **Consensus Ledger** (ancoragem imutável na TemporalChain)
- **Modelo de confiança** (ZK integrity proofs, SPEX, assinaturas pós‑quânticas)

## 2. Componentes Principais

### 2.1 Modelo LLM (`core/model/`)

- `manifest.json`: metadados do modelo (versão, assinatura, segurança)
- `config.json`: arquitetura do modelo (huggingface‑compatible)
- `tokenizer.json`, `chat_template.jinja`, etc.
- Suporte a quantização (GGUF, GPTQ) e adaptadores LoRA

### 2.2 Agente (`agent/`)

- `config.yaml`: define identidade, planeamento, memória, confiança, governance
- `system_prompt.md`: prompt base do agente
- `tools/registry.json`: catálogo de ferramentas disponíveis
- `memory/`: RAG (Qdrant) + histórico de conversas

### 2.3 Trust Layer (`trust/`)

- `integrity_proof.json`: prova ZK da integridade do modelo
- `spex_proof.json`: prova de execução segura (SPEX)
- `agent_signature.pqc`: chave pública SPHINCS+ do agente

### 2.4 Orquestrador (`orchestrator/`)

- `MultiAgentOrchestrator`: coordena coalizões, consenso, emergências
- `ConsensusWorkflow`: votação multi‑rodada com deadline e override
- `DebateEngine`: argumentação estruturada com juiz
- `HierarchyManager`: cadeia de comando (Strategic → Tactical → Operational)

### 2.5 Governance (`governance/`)

- `CathedralConsensusLedger.sol`: smart contract EVM para registo on‑chain de decisões
- Políticas éticas (Axiarquia) são definidas em Rego e verificadas pelo Guardian

### 2.6 Telemetry (`telemetry/`)

- OpenTelemetry + Jaeger para tracing distribuído
- Prometheus para métricas (latência, erros, uso de tokens)

## 3. Fluxos de Execução

### 3.1 Inicialização

1. Orquestrador carrega `agent/config.yaml` e `core/model/manifest.json`
2. Verifica integridade do modelo via prova ZK
3. Regista agentes (Oracle, Coder, Analyst, Guardian) com suas roles
4. Conecta‑se ao Event Bus (Redis) e cria consumer groups

### 3.2 Tarefa Simples (ex: “Analise yield da hub DeFi”)

1. **Oracle** recebe tarefa, planeia usando ReAct
2. **Analyst** consulta dados on‑chain + memória RAG
3. **Coder** gera relatório com gráficos (opcional)
4. **Guardian** valida conformidade ética e integridade
5. Resultado final assinado com SPHINCS+ e registado no Consensus Ledger

### 3.3 Decisão com Consenso

- Proposta submetida a coalizão
- Votação em múltiplas rodadas (Delphi ou maioria)
- Se consenso ≥ 75%, decisão registada no ledger
- Guardian pode sobrescrever em emergência

### 3.4 Debate

- Dois ou mais agentes argumentam sobre um tópico
- Juiz (Oracle/Guardian) avalia com base em critérios (lógica, evidência, etc.)
- Veredito final guia a ação

## 4. Segurança

- **SPHINCS+** (pós‑quântico): assinatura de todas as mensagens ACP e blocos do ledger
- **ZK‑SNARKs**: prova de integridade do modelo (verificação sem dados expostos)
- **SPEX** (Secure Process Execution): prova de que a inferência foi executada corretamente
- **Memory proofs**: verificações de que o agente tem provas de memória (para ações críticas)

## 5. Implantação

- **Docker Compose**: LLM Server, Agent Runtime, TemporalChain (Postgres), Vector DB (Qdrant), Redis, Jaeger
- **Configuração**: arquivos YAML/JSON montados como volumes
- **CI/CD**: testes unitários (Rust + Python) + linting (ruff, clippy)

## 6. Próximos Passos (v28.4)

- Integração com LLM real (candle/llama.cpp)
- Consenso distribuído (Raft/PBFT)
- Sistema de reputação on‑chain para agentes
- Multi‑coalizão e negociação entre agentes
