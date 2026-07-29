// src/integrations/opencut_render_queue.rs
//! Fila de renderização OpenCut

use serde::{Serialize, Deserialize};

// Mock struct to avoid dependencies not available in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCutProject {}

pub struct OpenCutRenderQueue {}

impl Default for OpenCutRenderQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenCutRenderQueue {
    pub fn new() -> Self {
        Self {}
    }
}
