pub struct PrivacyGuard;

impl PrivacyGuard {
    pub fn redact(&self, text: &str, _threshold: f32) -> Result<String, String> {
        Ok(text.to_string())
    }

    pub fn load(_path: &str, _config: Option<()>) -> Result<Self, String> {
        Ok(Self)
    }
}
