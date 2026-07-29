//! Cathedral ARKHE — Observer 5D Daemon
//! Inicializa a governança meta-nível e observabilidade

mod tree;
mod jira;

use tree::{TreeManager, AgentNode};
use jira::JiraClient;

#[tokio::main]
async fn main() {
    println!("Inicializando Observer-5D...");

    let mut tree_manager = TreeManager::new("Observer-5D-Tree");
    tree_manager.add_node(AgentNode {
        id: "root-agent".to_string(),
        active: true,
        reputation: 100,
        parent_id: None,
    });

    let jira_client = JiraClient::new("https://jira.arkhe.network");

    println!("Observer-5D daemon running with tree: {} and connected to Jira: {}", tree_manager.name, jira_client.url);
}
