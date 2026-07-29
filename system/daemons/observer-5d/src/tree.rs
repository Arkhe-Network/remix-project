//! Observer 5D — Tree Manager
//! Gerencia a representação local e verificação da árvore de agentes

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentNode {
    pub id: String,
    pub active: bool,
    pub reputation: u32,
    pub parent_id: Option<String>,
}

pub struct TreeManager {
    pub name: String,
    nodes: HashMap<String, AgentNode>,
}

impl TreeManager {
    pub fn new(name: &str) -> Self {
        TreeManager {
            name: name.to_string(),
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: AgentNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn get_node(&self, id: &str) -> Option<&AgentNode> {
        self.nodes.get(id)
    }
}
