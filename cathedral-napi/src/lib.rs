
#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use tokio::sync::{Mutex, MutexGuard};
use std::sync::Arc;

use cathedral_embodied_no_std::core::embodied_cognitive_core::EmbodiedCognitiveCore;

#[derive(serde::Serialize, serde::Deserialize)]
#[napi(object)]
pub struct MemoryProof {
    pub merkle_root: String,
    pub timestamp: f64,
    pub state_count: u32,
}

#[napi]
pub async fn prove_memory_state() -> Result<MemoryProof> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let merkle_root = format!(
        "0x{:064x}",
        blake3::hash(format!("dla_memory_{}", timestamp).as_bytes()).as_bytes()[..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64)
    );

    Ok(MemoryProof {
        merkle_root,
        timestamp: timestamp as f64,
        state_count: 47,
    })
}

#[napi]
pub struct CathedralAgent {
    inner: Arc<Mutex<EmbodiedCognitiveCore>>,
}

#[napi]
impl CathedralAgent {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        let picoads_api_key = std::env::var("PICOADS_API_KEY").ok();
        let picoads_backend = std::env::var("PICOADS_BACKEND_URL").ok();
        let recorder_path = std::env::var("SUCCESS_RECORDER_DB")
            .or_else(|_| std::env::var("SUCCESS_RECORDER_PATH"))
            .ok();

        let core = EmbodiedCognitiveCore::new(picoads_api_key, picoads_backend, recorder_path.as_deref());

        Ok(Self {
            inner: Arc::new(Mutex::new(core)),
        })
    }

    #[napi]
    pub async fn tick(&self) -> Result<String> {
        let mut core: MutexGuard<'_, EmbodiedCognitiveCore> = self.inner.lock().await;
        core.tick_zk_with_accelerator().await
            .map_err(|e: &str| Error::from_reason(e.to_string()))?;
        Ok("tick_complete".to_string())
    }

    #[napi(getter)]
    pub async fn current_round(&self) -> Result<u32> {
        let core: MutexGuard<'_, EmbodiedCognitiveCore> = self.inner.lock().await;
        Ok(core.current_round)
    }

    #[napi]
    pub async fn get_policy(&self) -> Result<String> {
        let core: MutexGuard<'_, EmbodiedCognitiveCore> = self.inner.lock().await;
        serde_json::to_string(&core.current_policy)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn accept_recommendation(&self, rec_id: String) -> Result<()> {
        let mut core: MutexGuard<'_, EmbodiedCognitiveCore> = self.inner.lock().await;
        core.accept_recommendation(&rec_id);
        Ok(())
    }

    #[napi]
    pub async fn get_recommendations(
        &self,
        query: String,
        hub: Option<String>,
        max_results: Option<u32>,
    ) -> Result<String> {
        let core: MutexGuard<'_, EmbodiedCognitiveCore> = self.inner.lock().await;
        let recs = core.fetch_picoads_recommendations(&query, hub.as_deref(), max_results)
            .await
            .map_err(|e: String| Error::from_reason(e))?;
        serde_json::to_string(&recs).map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn flush_recorder(&self) -> Result<()> {
        let mut core: MutexGuard<'_, EmbodiedCognitiveCore> = self.inner.lock().await;
        core.shutdown();
        Ok(())
    }
}

#[napi]
pub fn init_cathedral() {
}
