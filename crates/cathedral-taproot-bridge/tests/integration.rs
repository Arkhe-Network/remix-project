// tests/integration.rs
use cathedral_taproot_bridge::TaprootClient;
use std::path::Path;
use tokio::time::{sleep, Duration};

#[cfg(test)]
mod integration_tests {
    use super::*;

    const ALICE_ADDR: &str = "http://localhost:10029";
    const BOB_ADDR: &str = "http://localhost:10030";

    /// Testa a verificação de provas
    #[tokio::test]
    async fn test_proof_verification() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var("RUN_INTEGRATION_TESTS").is_err() {
            return Ok(());
        }

        let mut alice = TaprootClient::connect(
            ALICE_ADDR,
            None,
            None, // Macaroon removed for simplicity in dummy regtest
        ).await?;

        // 1. Criar ativo - Mock for actual tests since full mint capability wasn't exported in our current subset
        // In a real environment, you'd use AssetWallet::MintAsset

        let proof_bytes = vec![0, 1, 2, 3];
        // 2. Verificar prova
        let verified = alice
            .verify_proof(proof_bytes)
            .await?;
        assert!(verified.valid, "Prova inválida");
        println!("✅ Prova verificada com sucesso");

        Ok(())
    }

    /// Testa a Universe Federation
    #[tokio::test]
    async fn test_universe_sync() -> Result<(), Box<dyn std::error::Error>> {
        if std::env::var("RUN_INTEGRATION_TESTS").is_err() {
            return Ok(());
        }

        let mut bob = TaprootClient::connect(BOB_ADDR, None, None).await?;

        // Bob consulta Universe
        let result = bob
            .query_universe(vec![1, 2, 3], None)
            .await?;
        assert!(result.issuance_root.is_some() || result.transfer_root.is_some(), "Universe sync falhou");

        println!("✅ Universe Federation sincronizada");

        Ok(())
    }
}
