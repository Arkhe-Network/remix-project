//! Arkhe Hubble ML
//!
//! ML utilities for the Arkhe Hubble project.

pub mod phonon;

use thiserror::Error;

/// Error type for Hubble ML operations
#[derive(Debug, Error)]
pub enum HubbleError {
    #[error("Phonon error: {0}")]
    Phonon(String),
}

/// Result type for Hubble ML operations
pub type HubbleResult<T> = Result<T, HubbleError>;
