use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Clone)]
pub struct HashTreeStorage {
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Default for HashTreeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl HashTreeStorage {
    pub fn new() -> Self {
        Self { storage: Arc::new(Mutex::new(HashMap::new())) }
    }
    pub async fn get_by_path(&self, path: &str) -> Result<Vec<u8>, String> {
        let storage = self.storage.lock().await;
        storage.get(path).cloned().ok_or_else(|| format!("Path not found: {}", path))
    }
    pub async fn put(&self, data: &[u8]) -> Result<String, String> {
        let hash = format!("{:x}", md5::compute(data));
        self.storage.lock().await.insert(hash.clone(), data.to_vec());
        Ok(hash)
    }
}
