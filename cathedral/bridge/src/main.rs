#![allow(warnings)]
mod grpc_service;
mod signature_verifier;

use std::sync::Arc;
use tonic::transport::Server;
use grpc_service::{CathedralBridgeImpl, cathedral_v1::cathedral_bridge_server::CathedralBridgeServer};
use common::crypto_config::crypto_config_from_env;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let crypto_config = crypto_config_from_env();
    info!("Configuração criptográfica: {:?}", crypto_config);

    let registry = Arc::new(signature_verifier::PublicKeyRegistry::new(crypto_config.clone()));
    let _verifier = Arc::new(signature_verifier::SignatureVerifier::new(registry.clone(), crypto_config.clone()));

    let addr = "[::1]:50051".parse()?;
    let bridge = CathedralBridgeImpl::default();

    println!("CathedralBridgeServer listening on {}", addr);

    Server::builder()
        .add_service(CathedralBridgeServer::new(bridge))
        .serve(addr)
        .await?;

    Ok(())
}
