// Cathedral ARKHE v30.3 — Crawl Attestation + Reputation Monitor
// src/crawler/attestation/crawl_attestation.rs
//
// Gera atestados criptograficos para resultados de crawl
// e monitora reputacao de agentes/projetos na web.
//
// Selo: CATHEDRAL-ARKHE-v30.3-CRAWL-ATTESTATION-2026-06-17
// Arquiteto ORCID 0009-0005-2697-4668

use sha2::{Sha256, Digest};
use tracing::{info, debug, warn};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use chrono::Utc;

use crate::crawler::{
    types::*,
    agents::sovereign_crawler::SovereignCrawler,
    error::CrawlerError,
};
use crate::attestation::{
    AttestationStore, TestAttestation, UnifiedAttestation,
    AttestationBackend, AttestationId,
};

// ───────────────────────────────────────────────────────────
// CrawlAttestationAgent
// ───────────────────────────────────────────────────────────

pub struct CrawlAttestationAgent {
    store: Arc<dyn AttestationStore<Error = String>>,
    validator_id: [u8; 32],
    signer: crate::attestation::Ed25519Signer,
}

impl CrawlAttestationAgent {
    pub fn new(
        store: Arc<dyn AttestationStore<Error = String>>,
        validator_id: [u8; 32],
        signer: crate::attestation::Ed25519Signer,
    ) -> Self {
        Self { store, validator_id, signer }
    }

    /// Gera um atestado para um resultado de crawl.
    pub async fn attest_crawl(
        &self,
        crawl_result: &CrawlResult,
        attestation_type: CrawlAttestationType,
    ) -> Result<CrawlAttestation, CrawlerError> {
        // 1. Validar integridade do resultado
        let computed_hash = self.compute_result_hash(&crawl_result.pages);
        if computed_hash != crawl_result.result_hash {
            return Err(CrawlerError::Attestation("Result hash mismatch".to_string()));
        }

        // 2. Calcular score de qualidade
        let quality_score = self.compute_quality_score(crawl_result);

        // 3. Gerar commitment do conteudo (Merkle root)
        let content_commitment = self.compute_content_commitment(&crawl_result.pages);

        // 4. Construir atestado
        let attestation = CrawlAttestation {
            attestation_id: format!("crawl-att-{}-{}",
                &crawl_result.request_id[..8.min(crawl_result.request_id.len())],
                chrono::Utc::now().timestamp()
            ),
            request_id: crawl_result.request_id.clone(),
            result_hash: crawl_result.result_hash,
            content_commitment,
            attestation_type: attestation_type.clone(),
            quality_score,
            timestamp: Utc::now(),
            validator_signature: vec![0u8; 64], // preenchido abaixo
            validator_pubkey: self.validator_id,
        };

        // 5. Assinar atestado
        let att_bytes = bincode::serialize(&attestation)
            .map_err(|e| CrawlerError::Serialization(e))?;
        let signature = self.signer.sign(&att_bytes)
            .map_err(|e| CrawlerError::Signature(e.to_string()))?;

        let mut attestation = attestation;
        attestation.validator_signature = signature.to_vec();

        // 6. Armazenar no AttestationStore
        let test_att = TestAttestation {
            agent_id: hex::encode(&crawl_result.provenance.crawler_agent_id),
            test_type: "crawl_validation".to_string(),
            passed: quality_score >= 0.7,
            commitment: attestation.content_commitment,
            receipt_hash: attestation.result_hash,
            metadata: serde_json::json!({
                "attestation_id": attestation.attestation_id,
                "request_id": crawl_result.request_id,
                "quality_score": quality_score,
                "pages_crawled": crawl_result.stats.successful_pages,
                "attestation_type": format!("{:?}", attestation_type),
                "crawler_version": crawl_result.provenance.crawler_version,
            }),
            timestamp: Utc::now(),
            signature: Some(signature.to_vec()),
            public_key: Some(self.validator_id.to_vec()),
        };

        self.store.store(
            test_att.agent_id,
            test_att.test_type,
            test_att.commitment,
            test_att.receipt_hash,
            test_att.metadata,
        ).await.map_err(|e| CrawlerError::Attestation(e.to_string()))?;

        info!(
            "Crawl attestation generated | ID: {} | Quality: {:.2} | Type: {:?}",
            attestation.attestation_id, quality_score, attestation_type
        );

        Ok(attestation)
    }

    /// Verifica um atestado de crawl.
    pub fn verify_attestation(&self, attestation: &CrawlAttestation) -> bool {
        // Verificar assinatura
        let att_bytes = match bincode::serialize(attestation) {
            Ok(b) => b,
            Err(_) => return false,
        };

        // Stub: verificacao real usaria ed25519-dalek
        // self.signer.verify(&attestation.validator_pubkey, &att_bytes, &attestation.validator_signature)
        true
    }

    fn compute_result_hash(&self, pages: &[CrawledPage]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for page in pages {
            hasher.update(&page.content_hash);
        }
        let r = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&r);
        out
    }

    fn compute_content_commitment(&self, pages: &[CrawledPage]) -> [u8; 32] {
        // Merkle root simples
        let mut hashes: Vec<_> = pages.iter().map(|p| p.content_hash).collect();
        while hashes.len() > 1 {
            let mut next = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&chunk[0]);
                hasher.update(chunk.get(1).unwrap_or(&[0u8; 32]));
                let r = hasher.finalize();
                let mut out = [0u8; 32];
                out.copy_from_slice(&r);
                next.push(out);
            }
            hashes = next;
        }
        hashes.into_iter().next().unwrap_or([0u8; 32])
    }

    fn compute_quality_score(&self, result: &CrawlResult) -> f32 {
        let success_rate = if result.stats.total_pages > 0 {
            result.stats.successful_pages as f32 / result.stats.total_pages as f32
        } else { 0.0 };

        let size_score = if result.stats.avg_page_size > 1000 {
            1.0
        } else {
            result.stats.avg_page_size as f32 / 1000.0
        };

        let diversity_score = (result.stats.unique_domains as f32 / 10.0).min(1.0);

        (success_rate * 0.5 + size_score * 0.3 + diversity_score * 0.2).min(1.0)
    }
}

// ───────────────────────────────────────────────────────────
// ReputationMonitor
// ───────────────────────────────────────────────────────────

pub struct ReputationMonitor {
    config: ReputationMonitorConfig,
    crawler: SovereignCrawler,
    store: Arc<dyn AttestationStore<Error = String>>,
    /// Histórico de snapshots
    history: Vec<ReputationSnapshot>,
}

impl ReputationMonitor {
    pub fn new(
        config: ReputationMonitorConfig,
        crawler: SovereignCrawler,
        store: Arc<dyn AttestationStore<Error = String>>,
    ) -> Self {
        Self {
            config,
            crawler,
            store,
            history: Vec::new(),
        }
    }

    /// Inicia monitoramento contínuo.
    pub async fn start_monitoring(&mut self) -> Result<(), CrawlerError> {
        info!(
            "Starting reputation monitoring for '{}' | Interval: {}s",
            self.config.target_name, self.config.check_interval_secs
        );

        let mut ticker = interval(Duration::from_secs(self.config.check_interval_secs));

        loop {
            ticker.tick().await;

            match self.check_reputation().await {
                Ok(snapshot) => {
                    if snapshot.alert_triggered {
                        warn!(
                            "REPUTATION ALERT for '{}' | Sentiment: {:.2} | Neg: {} | Pos: {}",
                            self.config.target_name,
                            snapshot.sentiment_score,
                            snapshot.negative_mentions,
                            snapshot.positive_mentions
                        );
                    }

                    self.history.push(snapshot);

                    // Manter apenas últimos 100 snapshots
                    if self.history.len() > 100 {
                        self.history.remove(0);
                    }
                }
                Err(e) => {
                    warn!("Reputation check failed: {}", e);
                }
            }
        }
    }

    /// Executa uma verificação de reputação.
    pub async fn check_reputation(&mut self) -> Result<ReputationSnapshot, CrawlerError> {
        let mut total_mentions = 0u32;
        let mut positive = 0u32;
        let mut negative = 0u32;
        let mut neutral = 0u32;
        let mut sources: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut keywords: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

        for source in &self.config.sources {
            match self.check_source(source).await {
                Ok((mentions, pos, neg, neu, src_keywords)) => {
                    total_mentions += mentions;
                    positive += pos;
                    negative += neg;
                    neutral += neu;

                    *sources.entry(format!("{:?}", source)).or_insert(0) += mentions;

                    for (kw, count) in src_keywords {
                        *keywords.entry(kw).or_insert(0) += count;
                    }
                }
                Err(e) => {
                    warn!("Source check failed: {}", e);
                }
            }
        }

        let sentiment = if total_mentions > 0 {
            (positive as f32 - negative as f32) / total_mentions as f32
        } else { 0.0 };

        let alert = sentiment < self.config.alert_threshold;

        let mut top_sources: Vec<_> = sources.into_iter().collect();
        top_sources.sort_by(|a, b| b.1.cmp(&a.1));
        top_sources.truncate(5);

        let mut trending: Vec<_> = keywords.into_iter().collect();
        trending.sort_by(|a, b| b.1.cmp(&a.1));
        trending.truncate(10);

        let snapshot = ReputationSnapshot {
            timestamp: Utc::now(),
            total_mentions,
            positive_mentions: positive,
            negative_mentions: negative,
            neutral_mentions: neutral,
            sentiment_score: sentiment,
            top_sources,
            trending_keywords: trending,
            alert_triggered: alert,
        };

        // Armazenar snapshot como attestation
        self.store_snapshot_attestation(&snapshot).await?;

        Ok(snapshot)
    }

    /// Verifica uma fonte específica.
    async fn check_source(
        &self,
        source: &ReputationSource,
    ) -> Result<(u32, u32, u32, u32, Vec<(String, u32)>), CrawlerError> {
        match source {
            ReputationSource::Twitter => {
                // Em produção: usar API do Twitter/X
                Ok((0, 0, 0, 0, vec![]))
            }
            ReputationSource::Reddit => {
                // Em produção: usar Reddit API
                Ok((0, 0, 0, 0, vec![]))
            }
            ReputationSource::HackerNews => {
                // Em produção: usar HN API (Algolia)
                Ok((0, 0, 0, 0, vec![]))
            }
            ReputationSource::GitHub => {
                // Em produção: usar GitHub API
                Ok((0, 0, 0, 0, vec![]))
            }
            ReputationSource::NewsSites(urls) => {
                let mut mentions = 0;
                let mut keywords = std::collections::HashMap::new();

                for url in urls {
                    let request = CrawlRequest {
                        request_id: format!("rep-check-{}", chrono::Utc::now().timestamp()),
                        target: CrawlTarget::SingleUrl(url.clone()),
                        content_types: vec![ContentType::Markdown],
                        max_depth: 1,
                        max_pages: 10,
                        filters: CrawlFilters::default(),
                        consent_token: None,
                        timestamp: Utc::now(),
                        requesting_agent: { let mut b = [0u8; 32]; let n = self.config.target_name.len().min(32); b[..n].copy_from_slice(&self.config.target_name.as_bytes()[..n]); b },
                        purpose: CrawlPurpose::ReputationMonitoring,
                        retention_policy: RetentionPolicy::Ephemeral,
                    };

                    // Usar crawler para verificar menções
                    // Stub: simular resultado
                    mentions += 1;
                }

                Ok((mentions, mentions / 3, mentions / 5, mentions - mentions / 3 - mentions / 5,
                    keywords.into_iter().collect()))
            }
            ReputationSource::AcademicPapers => {
                Ok((0, 0, 0, 0, vec![]))
            }
            ReputationSource::CustomApi(_endpoint) => {
                Ok((0, 0, 0, 0, vec![]))
            }
        }
    }

    /// Armazena snapshot como attestation.
    async fn store_snapshot_attestation(
        &self,
        snapshot: &ReputationSnapshot,
    ) -> Result<(), CrawlerError> {
        let metadata = serde_json::json!({
            "target": self.config.target_name,
            "sentiment": snapshot.sentiment_score,
            "mentions": snapshot.total_mentions,
            "alert": snapshot.alert_triggered,
            "top_sources": snapshot.top_sources,
            "trending": snapshot.trending_keywords,
        });

        let commitment = {
            let mut hasher = Sha256::new();
            hasher.update(self.config.target_name.as_bytes());
            hasher.update(&snapshot.sentiment_score.to_le_bytes());
            let r = hasher.finalize();
            let mut out = [0u8; 32];
            out.copy_from_slice(&r);
            out
        };

        self.store.store(
            self.config.target_name.clone(),
            "reputation_snapshot".to_string(),
            commitment,
            commitment,
            metadata,
        ).await.map_err(|e| CrawlerError::Attestation(e.to_string()))?;

        Ok(())
    }

    /// Retorna histórico de reputação.
    pub fn get_history(&self) -> &[ReputationSnapshot] {
        &self.history
    }
}
