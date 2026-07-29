#[derive(Debug)]
pub struct InvariantError;
impl std::fmt::Display for InvariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "InvariantError")
    }
}
impl std::error::Error for InvariantError {}

#[allow(clippy::new_without_default)]
pub struct InvariantEngine;
impl InvariantEngine {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
    pub fn validate_goal(&self, _ctx: &str) -> Result<(), InvariantError> {
        Ok(())
    }
}
