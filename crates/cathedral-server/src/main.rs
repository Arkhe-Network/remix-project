use cathedral_remix_bridge::client::RemixClient;
use cathedral_server::api::create_routes;
use cathedral_server::orchestration::orchestrator::Orchestrator;
use cathedral_wormgraph::WormGraphClient;
use cathedral_zk::ZKGateway;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let remix_client = Arc::new(RemixClient::new("http://localhost:3000".to_string()));
    let wormgraph = Arc::new(WormGraphClient::new());
    let zk = Arc::new(ZKGateway::new());

    let orchestrator = Arc::new(Orchestrator::new(remix_client, wormgraph, zk));

    let app = create_routes(orchestrator);

    let addr = "0.0.0.0:8000";
    info!("Cathedral Server listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
