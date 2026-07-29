// Cathedral ARKHE v30.3 — Web Crawler Types
// src/crawler/types.rs
//
// Tipos fundamentais do agente de crawling soberano.
//
// Selo: CATHEDRAL-ARKHE-v30.3-CRAWLER-TYPES-2026-06-17
// Arquiteto ORCID 0009-0005-2697-4668

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use serde_big_array::BigArray;

// ───────────────────────────────────────────────────────────
// Crawl Request — requisição de crawling com consentimento
// ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlRequest {
    /// ID único da requisição
    pub request_id: String,
    /// URL ou padrão de URLs a serem crawleadas
    pub target: CrawlTarget,
    /// Tipo de conteúdo desejado
    pub content_types: Vec<ContentType>,
    /// Profundidade máxima de crawling
    pub max_depth: u8,
    /// Limite de páginas
    pub max_pages: u32,
    /// Filtros de exclusão (robots.txt, políticas, etc.)
    pub filters: CrawlFilters,
    /// ConsentToken do agente solicitante
    pub consent_token: Option<crate::private_bank::types::ConsentTokenV3>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Agente solicitante
    pub requesting_agent: [u8; 32],
    /// Propósito do crawling (treinamento, RAG, monitoramento, fact-checking)
    pub purpose: CrawlPurpose,
    /// Política de retenção dos dados
    pub retention_policy: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlTarget {
    SingleUrl(String),
    Domain(String),
    UrlPattern(String),
    Sitemap(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Markdown,
    Html,
    StructuredJson,
    Screenshot,
    Metadata,
    Links,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlFilters {
    /// Respeitar robots.txt
    pub respect_robots_txt: bool,
    /// User-Agent string
    pub user_agent: String,
    /// Delay entre requisições (ms)
    pub delay_ms: u64,
    /// Padrões de URL a excluir
    pub exclude_patterns: Vec<String>,
    /// Padrões de URL a incluir
    pub include_patterns: Vec<String>,
    /// Máximo de tamanho de página (bytes)
    pub max_page_size: usize,
}

impl Default for CrawlFilters {
    fn default() -> Self {
        Self {
            respect_robots_txt: true,
            user_agent: "Cathedral-Arkhe-Crawler/30.3 (Sovereign-Agent; +https://arkhe.cathedral.ai/bot)".to_string(),
            delay_ms: 1000,
            exclude_patterns: vec![
                "*.pdf".to_string(),
                "*.zip".to_string(),
                "/admin/*".to_string(),
            ],
            include_patterns: vec![],
            max_page_size: 10_000_000, // 10MB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlPurpose {
    /// Coleta para pipeline de treinamento (100T tokens)
    TrainingData,
    /// Alimentação de RAG para o Slow Brain
    RagContext,
    /// Monitoramento de reputação de agente/projeto
    ReputationMonitoring,
    /// Verificação de fontes para fact-checking
    FactChecking,
    /// Pesquisa acadêmica/técnica
    Research,
    /// Auditoria de conformidade
    ComplianceAudit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Dados descartados após processamento
    Ephemeral,
    /// Armazenados por período definido (dias)
    Temporary(u32),
    /// Armazenados permanentemente (com consentimento)
    Permanent,
    /// Armazenados até revogação explícita
    UntilRevoked,
}

// ───────────────────────────────────────────────────────────
// Crawl Result — resultado do crawling com proveniência
// ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    /// ID da requisição
    pub request_id: String,
    /// Páginas crawleadas
    pub pages: Vec<CrawledPage>,
    /// Estatísticas
    pub stats: CrawlStats,
    /// Timestamp de início e fim
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    /// Hash do resultado completo (para attestation)
    pub result_hash: [u8; 32],
    /// Proveniência do crawling
    pub provenance: CrawlProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPage {
    /// URL original
    pub url: String,
    /// Título da página
    pub title: Option<String>,
    /// Conteúdo em markdown
    pub markdown: Option<String>,
    /// HTML bruto (se solicitado)
    pub html: Option<String>,
    /// Dados estruturados (JSON)
    pub structured_data: Option<serde_json::Value>,
    /// Metadata
    pub metadata: PageMetadata,
    /// Links encontrados
    pub links: Vec<String>,
    /// Screenshot (URL ou base64)
    pub screenshot: Option<String>,
    /// Timestamp do crawling
    pub crawled_at: DateTime<Utc>,
    /// Hash do conteúdo (para integridade)
    pub content_hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    pub language: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub author: Option<String>,
    pub publish_date: Option<DateTime<Utc>>,
    pub source_url: String,
    pub status_code: u16,
    pub content_type: String,
    pub robots_meta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlStats {
    pub total_pages: u32,
    pub successful_pages: u32,
    pub failed_pages: u32,
    pub total_bytes: u64,
    pub avg_page_size: u64,
    pub total_links_found: u32,
    pub unique_domains: u32,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlProvenance {
    /// Agente que executou o crawl
    pub crawler_agent_id: [u8; 32],
    /// Versão do crawler
    pub crawler_version: String,
    /// Método de crawling (firecrawl, scrapy, playwright, etc.)
    pub crawl_method: String,
    /// Política de consentimento aplicada
    pub consent_policy: String,
    /// Registro de decisões éticas (ex: "robots.txt respeitado")
    pub ethical_log: Vec<String>,
    /// Assinatura do agente crawler sobre o resultado
    pub crawler_signature: Option<Vec<u8>>,
}

// ───────────────────────────────────────────────────────────
// Crawl Attestation — atestado criptográfico do crawling
// ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlAttestation {
    /// ID do atestado
    pub attestation_id: String,
    /// ID da requisição de crawl
    pub request_id: String,
    /// Hash do resultado
    pub result_hash: [u8; 32],
    /// Commitment do conteúdo (Merkle root das páginas)
    pub content_commitment: [u8; 32],
    /// Tipo de atestado
    pub attestation_type: CrawlAttestationType,
    /// Score de qualidade do crawl
    pub quality_score: f32, // 0.0 - 1.0
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Assinatura do agente validador
    pub validator_signature: Vec<u8>,
    /// Public key do validador
    pub validator_pubkey: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlAttestationType {
    /// Crawl validado por IntegrityTestAgent
    IntegrityValidated,
    /// Crawl validado por ComplianceTestAgent
    ComplianceValidated,
    /// Crawl verificado por ZK proof (RISC Zero)
    ZkVerified,
    /// Crawl auditado por AuditTestAgent
    AuditValidated,
}

// ───────────────────────────────────────────────────────────
// RAG Document — documento processado para RAG
// ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagDocument {
    /// ID do documento
    pub doc_id: String,
    /// URL fonte
    pub source_url: String,
    /// Título
    pub title: String,
    /// Conteúdo em chunks
    pub chunks: Vec<RagChunk>,
    /// Embedding do documento (se processado)
    pub embedding: Option<Vec<f32>>,
    /// Metadata
    pub metadata: RagMetadata,
    /// Proveniência
    pub provenance: CrawlProvenance,
    /// Atestado associado
    pub attestation: Option<CrawlAttestation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagChunk {
    /// Índice do chunk
    pub chunk_index: u32,
    /// Texto do chunk
    pub text: String,
    /// Embedding do chunk
    pub embedding: Option<Vec<f32>>,
    /// Token count
    pub token_count: u32,
    /// Overlap com chunk anterior
    pub overlap_chars: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagMetadata {
    pub source_url: String,
    pub crawl_timestamp: DateTime<Utc>,
    pub document_type: String,
    pub language: String,
    pub word_count: u32,
    pub confidence_score: f32,
    pub fact_check_status: Option<FactCheckStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactCheckStatus {
    pub checked: bool,
    pub verified_sources: Vec<String>,
    pub contradictions_found: Vec<String>,
    pub confidence: f32,
}

// ───────────────────────────────────────────────────────────
// Reputation Monitor — monitoramento de reputação
// ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationMonitorConfig {
    /// Agente ou projeto a monitorar
    pub target_name: String,
    /// Keywords para busca
    pub keywords: Vec<String>,
    /// Fontes a monitorar
    pub sources: Vec<ReputationSource>,
    /// Frequência de verificação (segundos)
    pub check_interval_secs: u64,
    /// Threshold de alerta (menções negativas)
    pub alert_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationSource {
    Twitter,
    Reddit,
    HackerNews,
    GitHub,
    NewsSites(Vec<String>),
    AcademicPapers,
    CustomApi(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationSnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_mentions: u32,
    pub positive_mentions: u32,
    pub negative_mentions: u32,
    pub neutral_mentions: u32,
    pub sentiment_score: f32, // -1.0 a 1.0
    pub top_sources: Vec<(String, u32)>,
    pub trending_keywords: Vec<(String, u32)>,
    pub alert_triggered: bool,
}
