// Cathedral ARKHE v30.3 — Crawler Errors
// src/crawler/error.rs
//
// Selo: CATHEDRAL-ARKHE-v30.3-CRAWLER-ERROR-2026-06-17

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("Attestation error: {0}")]
    Attestation(String),

    #[error("Signature error: {0}")]
    Signature(String),

    #[error("zVEC error: {0}")]
    ZvecError(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Robots.txt disallowed")]
    RobotsTxtDisallowed,

    #[error("Rate limited")]
    RateLimited,

    #[error("Max pages reached")]
    MaxPagesReached,
}

pub type Result<T> = std::result::Result<T, CrawlerError>;
