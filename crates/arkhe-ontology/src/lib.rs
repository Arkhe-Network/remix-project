use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyNode {
    pub id: String,
    pub label: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyRelation {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveOntology {
    pub nodes: Vec<OntologyNode>,
    pub relations: Vec<OntologyRelation>,
    pub domains: Vec<String>,
}

impl Default for CognitiveOntology {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveOntology {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relations: Vec::new(),
            domains: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: OntologyNode) {
        self.nodes.push(node);
    }

    pub fn add_relation(&mut self, relation: OntologyRelation) {
        self.relations.push(relation);
    }

    pub fn query_relevant(&self, query: &str) -> Vec<OntologyNode> {
        self.nodes
            .iter()
            .filter(|n| {
                n.label.contains(query)
                    || n.properties.values().any(|v| v.contains(query))
            })
            .cloned()
            .collect()
    }

    pub fn get_domain_nodes(&self, domain: &str) -> Vec<OntologyNode> {
        self.nodes
            .iter()
            .filter(|n| n.properties.get("domain") == Some(&domain.to_string()))
            .cloned()
            .collect()
    }

    pub fn merge_inferences(&mut self, facts: &[String]) -> Result<(), String> {
        for fact in facts {
            let mut p = HashMap::new();
            p.insert("source".to_string(), "inference".to_string());
            let node = OntologyNode {
                id: format!("inferred-{}", self.nodes.len()),
                label: fact.clone(),
                properties: p,
            };
            self.nodes.push(node);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub query: String,
    pub new_facts: Vec<String>,
    pub contradictions: Vec<String>,
    pub confidence: f64,
    pub iteration: u32,
}

impl InferenceResult {
    pub fn converged_with(&self, other: &InferenceResult) -> bool {
        self.new_facts == other.new_facts && self.contradictions == other.contradictions
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceEngine;

impl InferenceEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn infer(
        &self,
        ontology: &CognitiveOntology,
        query: &str,
    ) -> Result<InferenceResult, String> {
        let relevant = ontology.query_relevant(query);

        let new_facts: Vec<String> = relevant
            .iter()
            .map(|n| {
                format!(
                    "{}: {}",
                    n.label,
                    n.properties.get("definition").unwrap_or(&"".to_string())
                )
            })
            .collect();

        Ok(InferenceResult {
            query: query.to_string(),
            new_facts,
            contradictions: Vec::new(),
            confidence: 0.8,
            iteration: 0,
        })
    }

    pub fn verify_consistency(&self, _ontology: &CognitiveOntology) -> Result<bool, String> {
        Ok(true)
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}
