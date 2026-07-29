use crate::agi::aegis_evolution::AegisEvolution;
use crate::context::ContextEmbedding;
use crate::policy::ZkMemoryProofPolicy;
use crate::picoads::PicoAdsClient;  // assume a client exists

impl AegisEvolution {
    /// Decide whether to use PicoAds in this cycle.
    /// Returns (should_use, require_memory_proof).
    pub fn should_use_picoads(
        &self,
        ctx: &ContextEmbedding,
        policy: &ZkMemoryProofPolicy,
    ) -> (bool, bool) {
        // Example logic: use PicoAds if not stagnating and acceptance rate > 0.6
        if ctx.stagnation_rounds > 3 {
            return (false, false);
        }
        if ctx.acceptance_rate < 0.55 {
            return (false, false);
        }
        // High interference → require memory proof
        let require_proof = policy.should_require_memory_proof_for_recommendation(
            None, // hub unknown
            0.0,
        ) || ctx.avg_interference > 0.7;
        (true, require_proof)
    }

    /// Execute a PicoAds request and record the result.
    pub async fn execute_picoads_cycle(
        &mut self,
        ctx: &ContextEmbedding,
        policy: &ZkMemoryProofPolicy,
        client: &PicoAdsClient,
    ) -> Result<(), String> {
        let (should_use, require_proof) = self.should_use_picoads(ctx, policy);
        if !should_use {
            return Ok(());
        }

        // Generate memory proof if required
        let _memory_commitment = if require_proof {
            match crate::dla::prove_memory_state().await {
                Ok(proof) => Some(proof.merkle_root),
                Err(e) => {
                    eprintln!("Memory proof failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Build query based on context (e.g., "best yield" if defi)
        let query = if ctx.avg_interference > 0.6 {
            "high APY stablecoin vault"
        } else {
            "general DeFi recommendations"
        };

        // Call PicoAds
        let recs = client.get_recommendations(query, Some("defi-yield"), Some(5)).await
            .map_err(|e| e.to_string())?;

        // Record that we used PicoAds (could log to success recorder)
        // For simplicity, just print
        println!("[AegisEvolution] Received {} recommendations from PicoAds", recs.len());

        // Optionally, evaluate acceptance of these recommendations later and record improvement.
        Ok(())
    }
}
