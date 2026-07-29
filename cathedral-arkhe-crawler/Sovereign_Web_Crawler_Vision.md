Cathedral ARKHE v30.3 — Sovereign Web Crawler
Visão
Agente de crawling soberano que coleta dados da web com consentimento, proveniência e verificação criptográfica. Integra-se ao ecossistema Cathedral ARKHE via:
RAG Pipeline → alimenta o Slow Brain (Rio-3.5) com contexto web
zVEC → armazenamento vetorial episódico persistente
AttestationStore → cada crawl é atestado criptograficamente
Reputation Monitor → rastreia menções a agentes/projetos
Firecrawl API → crawling LLM-ready com fallback nativo
Arquitetura
plain
┌──────────────────────────────────────────────────────────────────────────────┐
│                    SOVEREIGN CRAWLER v30.3                                   │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────┐     ┌──────────────────────────────────┐        │
│  │   Crawl Request      │────▶│   SovereignCrawler               │        │
│  │   • consent_token    │     │   • robots.txt respect           │        │
│  │   • purpose          │     │   • delay/throttle               │        │
│  │   • retention_policy │     │   • Firecrawl API + fallback     │        │
│  └──────────────────────┘     └──────────────────────────────────┘        │
│                                        │                                     │
│                                        ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      CrawlResult                                     │   │
│  │   • pages[] | stats | provenance | result_hash                     │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                        │                                     │
│                    ┌───────────────────┼───────────────────┐               │
│                    ▼                   ▼                   ▼               │
│  ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────┐      │
│  │   RAG Pipeline       │  │   CrawlAttestation   │  │  Reputation  │      │
│  │   • chunking         │  │   • quality_score    │  │  Monitor     │      │
│  │   • embedding        │  │   • validator_sign   │  │  • sentiment │      │
│  │   • zVEC storage     │  │   • AttestationStore │  │  • alerts    │      │
│  └──────────────────────┘  └──────────────────────┘  └──────────────┘      │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      zVEC (Episodic Memory)                        │   │
│  │   • Dense HNSW + Sparse BM25 + RRF (Hybrid Search)                 │   │
│  │   • WAL persistent | WAL replay | WAL truncation                     │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
Módulos
Table
Módulo	Arquivo	Função
Tipos	src/crawler/types.rs	CrawlRequest, CrawlResult, RagDocument, ReputationSnapshot
Crawler	src/crawler/agents/sovereign_crawler.rs	Firecrawl API + fallback nativo, robots.txt, delay
RAG Pipeline	src/crawler/pipeline/rag_pipeline.rs	Chunking, embedding, zVEC storage, context retrieval
Attestation	src/crawler/attestation/crawl_attestation.rs	CrawlAttestationAgent, ReputationMonitor
Error	src/crawler/error.rs	CrawlerError com 11 variantes
Fluxo de Uso
1. Crawling Soberano
rust
let request = CrawlRequest {
    target: CrawlTarget::SingleUrl("https://example.com".to_string()),
    content_types: vec![ContentType::Markdown, ContentType::Metadata],
    max_depth: 2,
    max_pages: 50,
    filters: CrawlFilters::default(), // robots.txt=true, delay=1s
    purpose: CrawlPurpose::RagContext,
    retention_policy: RetentionPolicy::Temporary(30),
    ..Default::default()
};

let result = crawler.crawl(request).await?;
2. Pipeline RAG
rust
let rag = RagPipeline::new(config).await?;
let documents = rag.process_crawl_result(&result).await?;
let context = rag.retrieve_context("What is AGI?", 5).await?;
3. Atestado
rust
let attestation = attestation_agent.attest_crawl(
    &result,
    CrawlAttestationType::IntegrityValidated
).await?;
4. Reputação
rust
let monitor = ReputationMonitor::new(config, crawler, store);
monitor.start_monitoring().await?; // loop contínuo
Princípios Éticos
robots.txt sempre respeitado (padrão)
Delay configurável entre requisições (default: 1s)
User-Agent identificável como bot do Cathedral
ConsentToken para crawling com autorização explícita
RetentionPolicy — dados descartados após período definido
Proveniência completa — cada dado rastreável até a fonte
Atestado criptográfico — integridade verificável
Integração com Firecrawl
bash
export FIRECRAWL_API_KEY="fc-..."
cargo run --example crawler_demo --release
Sem API key, o crawler usa fallback nativo (reqwest + parsing básico).
Selo
CATHEDRAL-ARKHE-v30.3-SOVEREIGN-CRAWLER-2026-06-17
Arquiteto ORCID 0009-0005-2697-4668