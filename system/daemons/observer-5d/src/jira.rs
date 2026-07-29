//! Observer 5D — Jira Client
//! Cliente para integração com o Atlassian Jira com suporte a backoff

use reqwest::Client;
use std::time::Duration;

pub struct JiraClient {
    pub url: String,
    client: Client,
}

impl JiraClient {
    pub fn new(url: &str) -> Self {
        JiraClient {
            url: url.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_issue(&self, issue_id: &str) -> Result<String, reqwest::Error> {
        let endpoint = format!("{}/rest/api/2/issue/{}", self.url, issue_id);

        let mut retries = 0;
        let mut delay = 1;

        loop {
            match self.client.get(&endpoint).send().await {
                Ok(response) => {
                    return response.text().await;
                },
                Err(e) => {
                    if retries >= 3 {
                        return Err(e);
                    }
                    retries += 1;
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    delay *= 2;
                }
            }
        }
    }
}
