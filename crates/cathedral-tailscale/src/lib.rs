pub mod client;
pub mod headscale;
pub mod psk;
pub mod derp;
pub mod metrics;
pub mod grants;

use cathedral_identity::Did;

// Stub for missing cathedral_identity::VerifiableCredential and cathedral_permissions::GrantPolicy
pub struct VerifiableCredential;
impl VerifiableCredential {
    pub fn verify(&self) -> Result<(), Error> { Ok(()) }
}

#[derive(Debug)]
pub struct Error;

pub struct TailnetConnection;

impl TailnetConnection {
    pub fn new(_wg_config: WireGuardConfig) -> Self { Self }
}

pub struct WireGuardConfig;
impl WireGuardConfig {
    pub fn new() -> Self { Self }
    pub fn with_psk(self, _psk: &psk::PreSharedKey) -> Self { self }
    pub fn with_identity(self, _identity: &()) -> Self { self }
}

pub struct HeadscaleClient;
impl HeadscaleClient {
    pub async fn authenticate(&self, _did: &Did, _credential: &VerifiableCredential) -> Result<(), Error> { Ok(()) }
}

pub struct PskManager;
impl PskManager {
    pub async fn get_or_create(&self, _did: &Did) -> Result<psk::PreSharedKey, Error> {
        Ok(psk::PreSharedKey::generate())
    }
}

/// Wrapper principal para integração Tailscale/Headscale
pub struct CathedralTailscale {
    headscale: HeadscaleClient,
    psk_manager: PskManager,
    metrics: metrics::TailscaleMetrics,
}

impl CathedralTailscale {
    /// Inicializa conexão com Headscale + autenticação DID
    pub async fn connect(
        &self,
        did: &Did,
        credential: &VerifiableCredential,
    ) -> Result<TailnetConnection, Error> {
        // 1. Verificar credencial DID
        credential.verify()?;

        // 2. Obter PSK para esta conexão
        let psk = self.psk_manager.get_or_create(did).await?;

        // 3. Autenticar no Headscale via OIDC bridge
        let identity = self.headscale.authenticate(did, credential).await?;

        // 4. Configurar WireGuard com PSK
        let wg_config = WireGuardConfig::new()
            .with_psk(&psk)
            .with_identity(&identity);

        // 5. Registrar métricas
        self.metrics.handshake_latency.observe(0.01); // Fake latency

        Ok(TailnetConnection::new(wg_config))
    }
}
