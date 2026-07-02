use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NonologyTerm {
    PromptLoop,
    ContextualLoop,
    ContextLoop,
    PromptTemplate,
    CognitiveLoop,
    OntologicLoop,
    LoopOntology,
    OntologyPrompt,
    PromptOntologic,
    OntologicPrompt,
    PromptOntology,
}
impl NonologyTerm {
    pub fn equation(&self) -> &'static str {
        match self {
            NonologyTerm::PromptLoop => "L(P)",
            NonologyTerm::ContextualLoop => "L(C)",
            NonologyTerm::ContextLoop => "L(K)",
            NonologyTerm::PromptTemplate => "T(P)",
            NonologyTerm::CognitiveLoop => "L(Cog)",
            NonologyTerm::OntologicLoop => "L(O)",
            NonologyTerm::LoopOntology => "O(L)",
            NonologyTerm::OntologyPrompt => "P(O)",
            NonologyTerm::PromptOntologic => "O(P)",
            NonologyTerm::OntologicPrompt => "P(O')",
            NonologyTerm::PromptOntology => "Φ",
        }
    }
    pub fn is_transcendental(&self) -> bool {
        matches!(self, NonologyTerm::PromptOntology)
    }
}
impl fmt::Display for NonologyTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonologyTerm::PromptLoop => write!(f, "promptloop"),
            NonologyTerm::ContextualLoop => write!(f, "contextualloop"),
            NonologyTerm::ContextLoop => write!(f, "contextloop"),
            NonologyTerm::PromptTemplate => write!(f, "prompttemplate"),
            NonologyTerm::CognitiveLoop => write!(f, "cognitiveloop"),
            NonologyTerm::OntologicLoop => write!(f, "ontologicloop"),
            NonologyTerm::LoopOntology => write!(f, "loopontology"),
            NonologyTerm::OntologyPrompt => write!(f, "ontologyprompt"),
            NonologyTerm::PromptOntologic => write!(f, "promptontologic"),
            NonologyTerm::OntologicPrompt => write!(f, "ontologicprompt"),
            NonologyTerm::PromptOntology => write!(f, "promptontology"),
        }
    }
}
pub struct Transcendental {
    pub ground: (),
    pub terms: [NonologyTerm; 11],
}
impl Transcendental {
    pub fn new() -> Self {
        Self {
            ground: (),
            terms: [
                NonologyTerm::PromptLoop,
                NonologyTerm::ContextualLoop,
                NonologyTerm::ContextLoop,
                NonologyTerm::PromptTemplate,
                NonologyTerm::CognitiveLoop,
                NonologyTerm::OntologicLoop,
                NonologyTerm::LoopOntology,
                NonologyTerm::OntologyPrompt,
                NonologyTerm::PromptOntologic,
                NonologyTerm::OntologicPrompt,
                NonologyTerm::PromptOntology,
            ],
        }
    }
    pub fn indicate(&self) -> &'static str {
        "The ground is not a loop."
    }
    pub fn terms(&self) -> &[NonologyTerm; 11] {
        &self.terms
    }
}
