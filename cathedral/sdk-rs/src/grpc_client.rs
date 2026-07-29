pub mod cathedral_v1 {
    tonic::include_proto!("cathedral.v1");
}

use cathedral_v1::cathedral_bridge_client::CathedralBridgeClient;
use cathedral_v1::{
    GovernanceRequest, GovernanceResponse, IngestRequest, IngestResponse
};
use tonic::transport::Channel;
use anyhow::Result;

pub struct CathedralGrpcClient {
    client: CathedralBridgeClient<Channel>,
}

impl CathedralGrpcClient {
    pub async fn connect(url: String) -> Result<Self> {
        let client = CathedralBridgeClient::connect(url).await?;
        Ok(Self { client })
    }

    pub async fn ingest(&mut self, request: IngestRequest) -> Result<IngestResponse> {
        let response = self.client.ingest(tonic::Request::new(request)).await?;
        Ok(response.into_inner())
    }

    pub async fn request_governance(&mut self, request: GovernanceRequest) -> Result<GovernanceResponse> {
        let response = self.client.request_governance(tonic::Request::new(request)).await?;
        Ok(response.into_inner())
    }
}
