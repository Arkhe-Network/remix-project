// src/client.rs
use tonic::transport::{Channel, Certificate, ClientTlsConfig};
use tonic::{Request, Response, Status, metadata::MetadataValue};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use tracing::{info, warn, error};

pub mod proto {
    pub mod taprpc {
        tonic::include_proto!("taprpc");
    }

    // Export taprpc types to the current scope so we don't break our client.rs code
    pub use taprpc::*;

    pub mod assetwallet {
        tonic::include_proto!("assetwalletrpc");
    }

    pub mod universe {
        tonic::include_proto!("universerpc");
    }
}

use proto::{
    taproot_assets_client::TaprootAssetsClient,
    GetInfoRequest, GetInfoResponse,
    ListAssetRequest, ListAssetResponse,
    ListBalancesRequest, ListBalancesResponse,
    NewAddrRequest, Addr,
    SendAssetRequest, SendAssetResponse,
    BurnAssetRequest, BurnAssetResponse,
    VerifyProofResponse, ProofFile,
};

use proto::assetwallet::asset_wallet_client::AssetWalletClient;
use proto::universe::universe_client::UniverseClient;
use proto::universe::{
    AssetRootQuery, QueryRootResponse,
};

use crate::error::BridgeError;
use crate::auth::Macaroon;

/// Cliente avançado para o Taproot Assets Daemon (tapd).
#[derive(Clone)]
pub struct TaprootClient {
    /// Cliente principal do serviço TaprootAssets
    pub taproot: TaprootAssetsClient<Channel>,
    /// Cliente do serviço AssetWallet
    pub asset_wallet: AssetWalletClient<Channel>,
    /// Cliente do serviço Universe
    pub universe: UniverseClient<Channel>,
    /// Macaroon de autenticação
    macaroon: Option<Macaroon>,
}

impl TaprootClient {
    /// Conecta a um nó tapd via gRPC com autenticação completa.
    ///
    /// # Arguments
    /// * `addr` - Endereço do nó (ex: "https://localhost:10029")
    /// * `tls_config` - Configuração TLS (opcional)
    /// * `macaroon_path` - Caminho para o arquivo macaroon
    pub async fn connect(
        addr: &str,
        tls_config: Option<ClientTlsConfig>,
        macaroon_path: Option<&Path>,
    ) -> Result<Self, BridgeError> {
        let mut endpoint = tonic::transport::Endpoint::from_shared(addr.to_string())?;

        if let Some(tls) = tls_config {
            endpoint = endpoint.tls_config(tls)?;
        } else {
            // Se não for TLS, usa HTTP (apenas para desenvolvimento)
            warn!("Connecting without TLS - insecure!");
        }

        let channel = endpoint.connect().await?;

        // Carrega macaroon
        let macaroon = if let Some(path) = macaroon_path {
            let bytes = fs::read(path)?;
            Some(Macaroon::from_bytes(bytes)?)
        } else {
            None
        };

        Ok(Self {
            taproot: TaprootAssetsClient::new(channel.clone()),
            asset_wallet: AssetWalletClient::new(channel.clone()),
            universe: UniverseClient::new(channel.clone()),
            macaroon,
        })
    }

    /// Adiciona macaroon aos metadados da requisição
    fn add_auth<T>(&self, mut req: Request<T>) -> Request<T> {
        if let Some(mac) = &self.macaroon {
            let mac_hex = hex::encode(mac.bytes());
            if let Ok(val) = MetadataValue::from_str(&mac_hex) {
                req.metadata_mut().insert("macaroon", val);
            }
        }
        req
    }

    // --- Operações principais ---

    /// Obtém informações do nó.
    pub async fn get_info(&mut self) -> Result<GetInfoResponse, BridgeError> {
        let req = GetInfoRequest {};
        let request = self.add_auth(Request::new(req));
        let response = self.taproot.get_info(request).await?;
        Ok(response.into_inner())
    }

    /// Lista ativos da carteira.
    pub async fn list_assets(
        &mut self,
        with_witness: bool,
        include_spent: bool,
    ) -> Result<ListAssetResponse, BridgeError> {
        let req = ListAssetRequest {
            with_witness,
            include_spent,
            ..Default::default()
        };
        let request = self.add_auth(Request::new(req));
        let response = self.taproot.list_assets(request).await?;
        Ok(response.into_inner())
    }

    /// Lista balanços por ativo.
    pub async fn list_balances(
        &mut self,
        asset_id: Option<Vec<u8>>,
        group_key: Option<Vec<u8>>,
    ) -> Result<ListBalancesResponse, BridgeError> {
        let mut req = ListBalancesRequest {
            ..Default::default()
        };
        if let Some(asset_filter) = asset_id {
            req.asset_filter = asset_filter;
        }
        if let Some(group_filter) = group_key {
            req.group_key_filter = group_filter;
        }

        let request = self.add_auth(Request::new(req));
        let response = self.taproot.list_balances(request).await?;
        Ok(response.into_inner())
    }

    /// Cria um novo endereço para receber ativos.
    pub async fn new_address(
        &mut self,
        asset_id: Vec<u8>,
        amount: u64,
    ) -> Result<Addr, BridgeError> {
        let req = NewAddrRequest {
            asset_id,
            amt: amount,
            ..Default::default()
        };
        let request = self.add_auth(Request::new(req));
        let response = self.taproot.new_addr(request).await?;
        Ok(response.into_inner())
    }

    /// Envia ativos para um endereço.
    pub async fn send_asset(
        &mut self,
        addr: String,
        fee_rate: Option<u64>,
    ) -> Result<SendAssetResponse, BridgeError> {
        let req = SendAssetRequest {
            tap_addrs: vec![addr],
            fee_rate: fee_rate.unwrap_or(0) as u32,
            ..Default::default()
        };
        let request = self.add_auth(Request::new(req));
        let response = self.taproot.send_asset(request).await?;
        Ok(response.into_inner())
    }

    /// Verifica uma prova de ativo.
    pub async fn verify_proof(
        &mut self,
        proof_file: Vec<u8>,
    ) -> Result<VerifyProofResponse, BridgeError> {
        let req = ProofFile {
            raw_proof_file: proof_file,
            ..Default::default()
        };
        let request = self.add_auth(Request::new(req));
        let response: Response<VerifyProofResponse> = self.taproot.verify_proof(request).await?;
        Ok(response.into_inner())
    }

    /// Consulta Universe para um ativo.
    pub async fn query_universe(
        &mut self,
        asset_id: Vec<u8>,
        group_key: Option<Vec<u8>>,
    ) -> Result<QueryRootResponse, BridgeError> {
        use proto::universe::id::Id as InnerId;
        let inner_id = if let Some(key) = group_key {
            Some(InnerId::GroupKey(key))
        } else {
            Some(InnerId::AssetId(asset_id))
        };
        let id = proto::universe::Id {
            id: inner_id,
            proof_type: 0,
        };
        let req = AssetRootQuery {
            id: Some(id),
            ..Default::default()
        };
        let request = self.add_auth(Request::new(req));
        let response: Response<QueryRootResponse> = self.universe.query_asset_roots(request).await?;
        Ok(response.into_inner())
    }

}
