use crate::PluralityClientTrait;
use crate::plurality_types::{MemoryBucket, SearchResult};
use anyhow::Result;

pub struct PluralityClient {
    endpoint: String,
}

impl PluralityClientTrait for PluralityClient {
    fn new() -> Self {
        PluralityClient {
            endpoint: "https://api.plurality.network".to_string(),
        }
    }
}

impl PluralityClient {
    pub async fn fetch_buckets(&self) -> Result<Vec<MemoryBucket>> {
        // Mock API call
        Ok(vec![
            MemoryBucket {
                id: "b1".to_string(),
                name: "Primary".to_string(),
                capacity: 1024,
            }
        ])
    }
}
