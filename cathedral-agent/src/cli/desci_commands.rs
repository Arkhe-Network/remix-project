use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum DesciCommands {
    /// Creates a new DeSci node
    CreateNode(CreateNodeArgs),
    /// Publishes a DeSci node
    Publish(PublishArgs),
    /// Evolves a DeSci node
    Evolve(EvolveArgs),
    /// Views DeSci profile
    Profile,
}

#[derive(Debug, Args)]
pub struct CreateNodeArgs {
    pub title: String,
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    pub node_id: String,
    #[arg(long)]
    pub dpid: Option<String>,
}

#[derive(Debug, Args)]
pub struct EvolveArgs {
    pub node_id: String,
    pub target_metric: String,
}
