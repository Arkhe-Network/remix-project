pub mod string_safe;

#[derive(Debug)]
pub struct ArkheError(pub String);

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Timestamp {
    millis: u64,
}

impl Timestamp {
    pub fn now() -> Self {
        Self {
            millis: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    pub fn as_millis(&self) -> u64 {
        self.millis
    }
}
