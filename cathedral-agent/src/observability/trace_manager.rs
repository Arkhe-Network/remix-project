
pub struct TraceManager {}

impl Default for TraceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceManager {
    pub fn new() -> Self { Self {} }

    pub async fn start_trace(&self, _resource_id: &str) -> Result<String, String> {
        Ok(uuid::Uuid::new_v4().to_string())
    }

    pub async fn add_artifact(
        &self,
        _trace_id: &str,
        _name: &str,
        _data: Vec<u8>,
        _content_type: &str,
        _description: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}
