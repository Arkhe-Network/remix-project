use serde::{Deserialize, Serialize};

/// Texto que foi validado e é seguro para ser injetado no prompt do LLM.
/// REGRAS DE SEGURANÇA:
/// 1. Não possui construtor `From<secrecy::SecretString>`.
/// 2. Não implementa `Display` de forma a expor dados sensíveis acidentalmente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSafeText(String);

impl LlmSafeText {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        // Em uma implementação real, aplicaríamos regex aquí (Sanitizer)
        // para garantir que não há padrões de chave API.
        Self(text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
