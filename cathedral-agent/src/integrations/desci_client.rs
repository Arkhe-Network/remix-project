use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DesciClient {
    client: Client,
    api_url: String,
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRegistrationRequest {
    pub title: String,
    pub description: Option<String>,
    // other fields as needed
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRegistrationResponse {
    pub dpid: String,
    pub node_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DpidResolutionResponse {
    pub node_id: String,
    pub title: String,
    // other fields as needed
}

impl DesciClient {
    pub fn new(api_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_url,
            api_key,
        }
    }

    pub async fn register_node(&self, request: NodeRegistrationRequest) -> Result<NodeRegistrationResponse, String> {
        let url = format!("{}/api/v1/nodes", self.api_url);
        let res = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            let body = res.json::<NodeRegistrationResponse>().await.map_err(|e| e.to_string())?;
            Ok(body)
        } else {
            Err(format!("Failed to register node: {}", res.status()))
        }
    }

    pub async fn resolve_dpid(&self, dpid: &str) -> Result<DpidResolutionResponse, String> {
        let url = format!("{}/api/v1/dpid/{}", self.api_url, dpid);
        let res = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            let body = res.json::<DpidResolutionResponse>().await.map_err(|e| e.to_string())?;
            Ok(body)
        } else {
            Err(format!("Failed to resolve dpid {}: {}", dpid, res.status()))
        }
    }
}
