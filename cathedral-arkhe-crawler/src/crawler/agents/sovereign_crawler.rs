// Cathedral ARKHE v30.3 — Sovereign Crawler Agent
// src/crawler/agents/sovereign_crawler.rs
//
// Agente de crawling que respeita robots.txt, consentimento e proveniência.
// Integra com Firecrawl API (ou equivalente) para crawling LLM-ready.
//
// Selo: CATHEDRAL-ARKHE-v30.3-SOVEREIGN-CRAWLER-2026-06-17
// Arquiteto ORCID 0009-0005-2697-4668

use reqwest;
use serde_json::json;
use sha2::{Sha256, Digest};
use tracing::{info, debug, warn, error};
use std::collections::HashSet;
use std::time::Duration;
use chrono::Utc;

use crate::crawler::{
    types::*,
    error::CrawlerError,
};
use crate::attestation::Ed25519Signer;

// ───────────────────────────────────────────────────────────
// Configuração do Crawler
// ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SovereignCrawlerConfig {
    /// API key do Firecrawl (ou serviço equivalente)
    pub firecrawl_api_key: Option<String>,
    /// Endpoint do serviço de crawling
    pub crawl_endpoint: String,
    /// Timeout por requisição
    pub request_timeout_secs: u64,
    /// Número máximo de retries
    pub max_retries: u32,
    /// Diretório de cache local
    pub cache_dir: Option<String>,
    /// Usar cache quando disponível
    pub use_cache: bool,
    /// Assinador do agente crawler
    pub signer: Ed25519Signer,
}

impl Default for SovereignCrawlerConfig {
    fn default() -> Self {
        Self {
            firecrawl_api_key: std::env::var("FIRECRAWL_API_KEY").ok(),
            crawl_endpoint: "https://api.firecrawl.dev/v1".to_string(),
            request_timeout_secs: 60,
            max_retries: 3,
            cache_dir: Some(".cathedral/crawler_cache".to_string()),
            use_cache: true,
            signer: Ed25519Signer::new().expect("Failed to create signer"),
        }
    }
}

// ───────────────────────────────────────────────────────────
// SovereignCrawler
// ───────────────────────────────────────────────────────────

pub struct SovereignCrawler {
    config: SovereignCrawlerConfig,
    http: reqwest::Client,
    /// URLs já visitadas (para evitar duplicatas)
    visited_urls: HashSet<String>,
    /// Domínios respeitados (robots.txt)
    respected_domains: HashSet<String>,
}

impl SovereignCrawler {
    pub fn new(config: SovereignCrawlerConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .user_agent(&config.signer.public_key_hex())
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            http,
            visited_urls: HashSet::new(),
            respected_domains: HashSet::new(),
        }
    }

    /// Executa um crawling soberano completo.
    pub async fn crawl(&mut self, request: CrawlRequest) -> Result<CrawlResult, CrawlerError> {
        let started_at = Utc::now();
        info!(
            "Starting sovereign crawl | Request: {} | Target: {:?} | Purpose: {:?}",
            request.request_id, request.target, request.purpose
        );

        let mut pages = Vec::new();
        let mut stats = CrawlStats {
            total_pages: 0,
            successful_pages: 0,
            failed_pages: 0,
            total_bytes: 0,
            avg_page_size: 0,
            total_links_found: 0,
            unique_domains: 0,
            duration_seconds: 0.0,
        };

        let urls_to_crawl = self.resolve_target(&request.target).await?;
        let mut unique_domains = HashSet::new();

        for url in urls_to_crawl.iter().take(request.max_pages as usize) {
            if self.visited_urls.contains(url) {
                debug!("Skipping already visited URL: {}", url);
                continue;
            }

            // Verificar robots.txt
            if request.filters.respect_robots_txt {
                let domain = extract_domain(url);
                if !self.respected_domains.contains(&domain) {
                    if !self.check_robots_txt(&domain, &request.filters.user_agent).await {
                        warn!("robots.txt disallows crawling for domain: {}", domain);
                        continue;
                    }
                    self.respected_domains.insert(domain.clone());
                }
                unique_domains.insert(domain);
            }

            // Delay entre requisições
            tokio::time::sleep(Duration::from_millis(request.filters.delay_ms)).await;

            // Executar crawl da página
            match self.crawl_page(url, &request.content_types).await {
                Ok(page) => {
                    stats.successful_pages += 1;
                    stats.total_bytes += page.markdown.as_ref().map(|m| m.len() as u64).unwrap_or(0);
                    stats.total_links_found += page.links.len() as u32;
                    pages.push(page);
                }
                Err(e) => {
                    warn!("Failed to crawl {}: {}", url, e);
                    stats.failed_pages += 1;
                }
            }

            self.visited_urls.insert(url.clone());
            stats.total_pages += 1;
        }

        stats.unique_domains = unique_domains.len() as u32;
        stats.avg_page_size = if stats.successful_pages > 0 {
            stats.total_bytes / stats.successful_pages as u64
        } else { 0 };

        let completed_at = Utc::now();
        let duration = (completed_at - started_at).num_milliseconds() as f64 / 1000.0;
        stats.duration_seconds = duration;

        // Computar hash do resultado
        let result_hash = self.compute_result_hash(&pages);

        // Construir proveniência
        let provenance = CrawlProvenance {
            crawler_agent_id: self.config.signer.public_key(),
            crawler_version: "30.3.0".to_string(),
            crawl_method: if self.config.firecrawl_api_key.is_some() {
                "firecrawl-api".to_string()
            } else {
                "native-scraper".to_string()
            },
            consent_policy: format!("{:?}", request.retention_policy),
            ethical_log: vec![
                format!("robots.txt respected: {}", request.filters.respect_robots_txt),
                format!("User-Agent: {}", request.filters.user_agent),
                format!("Delay: {}ms", request.filters.delay_ms),
                format!("Pages crawled: {}/{}", stats.successful_pages, request.max_pages),
            ],
            crawler_signature: None, // preenchido após
        };

        let mut result = CrawlResult {
            request_id: request.request_id,
            pages,
            stats,
            started_at,
            completed_at,
            result_hash,
            provenance,
        };

        // Assinar resultado
        let result_bytes = bincode::serialize(&result)
            .map_err(|e| CrawlerError::Serialization(e))?;
        let signature = self.config.signer.sign(&result_bytes)
            .map_err(|e| CrawlerError::Signature(e.to_string()))?;
        result.provenance.crawler_signature = Some(signature.to_vec());

        info!(
            "Crawl completed | Pages: {}/{} | Success: {} | Failed: {} | Duration: {:.2}s | Hash: {:x?}",
            result.stats.total_pages,
            request.max_pages,
            result.stats.successful_pages,
            result.stats.failed_pages,
            result.stats.duration_seconds,
            &result.result_hash[..8]
        );

        Ok(result)
    }

    /// Crawl de uma única página via Firecrawl API.
    async fn crawl_page(
        &self,
        url: &str,
        content_types: &[ContentType],
    ) -> Result<CrawledPage, CrawlerError> {
        let formats: Vec<String> = content_types.iter()
            .map(|ct| match ct {
                ContentType::Markdown => "markdown".to_string(),
                ContentType::Html => "html".to_string(),
                ContentType::StructuredJson => "extract".to_string(),
                ContentType::Screenshot => "screenshot".to_string(),
                ContentType::Metadata => "metadata".to_string(),
                ContentType::Links => "links".to_string(),
            })
            .collect();

        // Tentar Firecrawl API primeiro
        if let Some(ref api_key) = self.config.firecrawl_api_key {
            match self.crawl_firecrawl(url, &formats).await {
                Ok(page) => return Ok(page),
                Err(e) => {
                    warn!("Firecrawl failed for {}: {}. Falling back to native.", url, e);
                }
            }
        }

        // Fallback: crawling nativo
        self.crawl_native(url, content_types).await
    }

    /// Crawl via Firecrawl API.
    async fn crawl_firecrawl(
        &self,
        url: &str,
        formats: &[String],
    ) -> Result<CrawledPage, CrawlerError> {
        let api_key = self.config.firecrawl_api_key.as_ref()
            .ok_or_else(|| CrawlerError::Config("No Firecrawl API key".to_string()))?;

        let payload = json!({
            "url": url,
            "formats": formats,
            "onlyMainContent": true,
        });

        let response = self.http
            .post(format!("{}/scrape", self.config.crawl_endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CrawlerError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CrawlerError::Http(
                format!("Firecrawl API error: {}", response.status())
            ));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| CrawlerError::Serialization(std::boxed::Box::new(bincode::ErrorKind::Custom(e.to_string()))))?;

        let markdown = data["data"]["markdown"].as_str().map(|s| s.to_string());
        let html = data["data"]["html"].as_str().map(|s| s.to_string());
        let metadata = data["data"]["metadata"].clone();

        let title = metadata["title"].as_str().map(|s| s.to_string());
        let description = metadata["description"].as_str().map(|s| s.to_string());
        let language = metadata["language"].as_str().map(|s| s.to_string());

        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(markdown.as_ref().unwrap_or(&"".to_string()).as_bytes());
            let r = hasher.finalize();
            let mut out = [0u8; 32];
            out.copy_from_slice(&r);
            out
        };

        Ok(CrawledPage {
            url: url.to_string(),
            title,
            markdown,
            html,
            structured_data: None,
            metadata: PageMetadata {
                language,
                description,
                keywords: vec![],
                author: None,
                publish_date: None,
                source_url: url.to_string(),
                status_code: 200,
                content_type: "text/html".to_string(),
                robots_meta: None,
            },
            links: vec![],
            screenshot: None,
            crawled_at: Utc::now(),
            content_hash,
        })
    }

    /// Crawl nativo (fallback quando Firecrawl não disponível).
    async fn crawl_native(
        &self,
        url: &str,
        _content_types: &[ContentType],
    ) -> Result<CrawledPage, CrawlerError> {
        let response = self.http
            .get(url)
            .send()
            .await
            .map_err(|e| CrawlerError::Http(e.to_string()))?;

        let status = response.status().as_u16();
        let html = response.text().await
            .map_err(|e| CrawlerError::Http(e.to_string()))?;

        // Extrair markdown simples (em produção: usar html2md ou similar)
        let markdown = Some(format!("# Content from {} \n\n{}", url, html.chars().take(5000).collect::<String>()));

        let content_hash = {
            let mut hasher = Sha256::new();
            hasher.update(html.as_bytes());
            let r = hasher.finalize();
            let mut out = [0u8; 32];
            out.copy_from_slice(&r);
            out
        };

        Ok(CrawledPage {
            url: url.to_string(),
            title: None,
            markdown,
            html: Some(html),
            structured_data: None,
            metadata: PageMetadata {
                language: None,
                description: None,
                keywords: vec![],
                author: None,
                publish_date: None,
                source_url: url.to_string(),
                status_code: status,
                content_type: "text/html".to_string(),
                robots_meta: None,
            },
            links: vec![],
            screenshot: None,
            crawled_at: Utc::now(),
            content_hash,
        })
    }

    /// Resolve o target em URLs concretas.
    async fn resolve_target(&self, target: &CrawlTarget) -> Result<Vec<String>, CrawlerError> {
        match target {
            CrawlTarget::SingleUrl(url) => Ok(vec![url.clone()]),
            CrawlTarget::Domain(domain) => {
                // Em produção: usar sitemap ou descoberta
                Ok(vec![format!("https://{}", domain)])
            }
            CrawlTarget::UrlPattern(pattern) => {
                // Em produção: expandir padrão
                Ok(vec![pattern.clone()])
            }
            CrawlTarget::Sitemap(url) => {
                self.parse_sitemap(url).await
            }
        }
    }

    /// Parse de sitemap XML.
    async fn parse_sitemap(&self, url: &str) -> Result<Vec<String>, CrawlerError> {
        let response = self.http
            .get(url)
            .send()
            .await
            .map_err(|e| CrawlerError::Http(e.to_string()))?;

        let xml = response.text().await
            .map_err(|e| CrawlerError::Http(e.to_string()))?;

        // Parse simples de URLs em sitemap
        let mut urls = Vec::new();
        for line in xml.lines() {
            if line.trim().starts_with("<loc>") {
                let url = line.trim()
                    .replace("<loc>", "")
                    .replace("</loc>", "");
                urls.push(url);
            }
        }

        Ok(urls)
    }

    /// Verifica robots.txt para um domínio.
    async fn check_robots_txt(&self, domain: &str, user_agent: &str) -> bool {
        let robots_url = format!("https://{}/robots.txt", domain);

        match self.http.get(&robots_url).send().await {
            Ok(response) => {
                if let Ok(text) = response.text().await {
                    // Parse simplificado de robots.txt
                    // Em produção: usar crate robots_txt
                    !text.contains(&format!("User-agent: {}\nDisallow: /", user_agent))
                } else {
                    true // Se não conseguir ler, assume permitido
                }
            }
            Err(_) => true,
        }
    }

    /// Computa hash do resultado completo.
    fn compute_result_hash(&self, pages: &[CrawledPage]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for page in pages {
            hasher.update(&page.content_hash);
            hasher.update(page.url.as_bytes());
        }
        let r = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&r);
        out
    }
}

fn extract_domain(url: &str) -> String {
    url.split("//").nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or(url)
        .to_string()
}
