
#[derive(Debug)]
pub struct InvariantError;
impl std::fmt::Display for InvariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "InvariantError") }
}
impl std::error::Error for InvariantError {}

pub struct InvariantEngine;
impl InvariantEngine {
    pub fn new() -> Self { Self }
    pub fn validate_goal(&self, _ctx: &str) -> Result<(), InvariantError> { Ok(()) }
}
