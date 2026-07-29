// Cathedral ARKHE v30.3 — Crawler Integrated Demo
// examples/crawler_demo.rs
//
// Demonstra o fluxo completo:
//   1. Crawling soberano de uma URL
//   2. Processamento em RAG com zVEC
//   3. Atestado criptográfico do resultado
//   4. Monitoramento de reputação
//   5. Recuperação de contexto para query
//
// Selo: CATHEDRAL-ARKHE-v30.3-CRAWLER-DEMO-2026-06-17
// Arquiteto ORCID 0009-0005-2697-4668

use cathedral_arkhe_crawler::crawler::{
    types::{CrawlRequest, CrawlTarget, ContentType, CrawlFilters, CrawlPurpose, RetentionPolicy, ReputationMonitorConfig, ReputationSource},
    agents::sovereign_crawler::{SovereignCrawler, SovereignCrawlerConfig},
    pipeline::rag_pipeline::{RagPipeline, RagPipelineConfig},
    attestation::crawl_attestation::{CrawlAttestationAgent, ReputationMonitor},
};
use cathedral_arkhe_crawler::attestation::AttestationBackend;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🏛️ Cathedral ARKHE v30.3 — Sovereign Crawler Demo");
    info!("═══════════════════════════════════════════════════");

    // ── 1. Configurar crawler ──
    let crawler_config = SovereignCrawlerConfig::default();
    let mut crawler = SovereignCrawler::new(crawler_config);

    // ── 2. Criar requisição de crawl ──
    let request = CrawlRequest {
        request_id: format!("crawl-demo-{}", chrono::Utc::now().timestamp()),
        target: CrawlTarget::SingleUrl(
            "https://docs.firecrawl.dev/introduction".to_string()
        ),
        content_types: vec![ContentType::Markdown, ContentType::Metadata],
        max_depth: 1,
        max_pages: 5,
        filters: CrawlFilters::default(),
        consent_token: None,
        timestamp: chrono::Utc::now(),
        requesting_agent: [0xAAu8; 32],
        purpose: CrawlPurpose::RagContext,
        retention_policy: RetentionPolicy::Temporary(30),
    };

    info!("Starting crawl | Target: {:?} | Purpose: {:?}", request.target, request.purpose);

    // ── 3. Executar crawl ──
    let crawl_result = crawler.crawl(request).await?;
    info!(
        "✅ Crawl completed | Pages: {} | Success: {} | Duration: {:.2}s",
        crawl_result.stats.total_pages,
        crawl_result.stats.successful_pages,
        crawl_result.stats.duration_seconds
    );

    // ── 4. Processar em RAG ──
    info!("Processing into RAG pipeline...");
    let rag_config = RagPipelineConfig::default();
    let rag = RagPipeline::new(rag_config).await?;
    let documents = rag.process_crawl_result(&crawl_result).await?;

    info!(
        "✅ RAG pipeline completed | Documents: {} | Total chunks: {}",
        documents.len(),
        documents.iter().map(|d| d.chunks.len()).sum::<usize>()
    );

    // ── 5. Gerar atestado ──
    info!("Generating crawl attestation...");
    let backend = AttestationBackend::new_memory();
    let attestation_agent = CrawlAttestationAgent::new(
        std::sync::Arc::new(backend),
        [0xBBu8; 32],
        cathedral_arkhe_crawler::attestation::Ed25519Signer::new()?
    );

    let attestation = attestation_agent.attest_crawl(
        &crawl_result,
        cathedral_arkhe_crawler::crawler::types::CrawlAttestationType::IntegrityValidated
    ).await?;

    info!(
        "✅ Attestation generated | ID: {} | Quality: {:.2}",
        attestation.attestation_id, attestation.quality_score
    );

    // ── 6. Recuperar contexto RAG ──
    info!("Retrieving RAG context for query...");
    let query = "What is Firecrawl and how does it work?";
    let context = rag.retrieve_context(query, 3).await?;

    info!("✅ Retrieved {} chunks for query", context.len());
    for (i, chunk) in context.iter().enumerate() {
        info!("  Chunk {}: {} chars", i, chunk.text.len());
    }

    // ── 7. Monitoramento de reputação (async) ──
    info!("Starting reputation monitor...");
    let rep_config = ReputationMonitorConfig {
        target_name: "Cathedral ARKHE".to_string(),
        keywords: vec!["Cathedral ARKHE".to_string(), "AGI Soberana".to_string()],
        sources: vec![
            ReputationSource::HackerNews,
            ReputationSource::Twitter,
            ReputationSource::GitHub,
        ],
        check_interval_secs: 3600,
        alert_threshold: -0.3,
    };

    let rep_backend = AttestationBackend::new_memory();
    let mut monitor = ReputationMonitor::new(
        rep_config,
        crawler,
        std::sync::Arc::new(rep_backend),
    );

    // Executar uma verificação única (em produção: start_monitoring)
    let snapshot = monitor.check_reputation().await?;
    info!(
        "Reputation snapshot | Mentions: {} | Sentiment: {:.2} | Alert: {}",
        snapshot.total_mentions,
        snapshot.sentiment_score,
        if snapshot.alert_triggered { "⚠️ YES" } else { "✅ NO" }
    );

    // ── Resumo ──
    info!("═══════════════════════════════════════════════════");
    info!("🏁 Sovereign Crawler Demo completed!");
    info!("   Crawled: {} pages", crawl_result.stats.successful_pages);
    info!("   RAG Docs: {} | Chunks: {}",
          documents.len(),
          documents.iter().map(|d| d.chunks.len()).sum::<usize>());
    info!("   Attestation: {}", attestation.attestation_id);
    info!("   Context retrieved: {} chunks", context.len());
    info!("═══════════════════════════════════════════════════");

    Ok(())
}
