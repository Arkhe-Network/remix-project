
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PicoAdsRecommendation {
    pub hub: String,
    pub url: String,
    pub title: String,
}

pub struct HubPerformance {
    pub acceptance_rate: f32,
    pub recommendation_volume: u32,
    pub roas: f32,
}

pub struct PicoAdsClient {
    pub api_key: String,
    pub backend_url: Option<String>,
}

impl PicoAdsClient {
    pub fn new(api_key: String, backend_url: Option<String>) -> Self {
        Self { api_key, backend_url }
    }
    pub async fn get_recommendations(&self, _query: &str, _hub: Option<&str>, _limit: Option<u32>) -> Result<Vec<PicoAdsRecommendation>, String> {
        Ok(vec![])
    }
}
