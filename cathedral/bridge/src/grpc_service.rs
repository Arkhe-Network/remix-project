pub mod cathedral_v1 {
    tonic::include_proto!("cathedral.v1");
}

use cathedral_v1::cathedral_bridge_server::CathedralBridge;
use cathedral_v1::{
    GovernanceRequest, GovernanceResponse, IngestRequest, IngestResponse,
    QueryProvenanceRequest, QueryProvenanceResponse, GovernanceVerdict
};
use tonic::{Request, Response, Status};

use cathedral_v1::SignatureAlgorithm;


#[derive(Debug, Default)]
pub struct CathedralBridgeImpl {}

#[tonic::async_trait]
impl CathedralBridge for CathedralBridgeImpl {
async fn ingest(
        &self,
        request: Request<IngestRequest>,
    ) -> Result<Response<IngestResponse>, Status> {
        let req = request.into_inner();
        let events_count = req.events.len() as u32;

        if let (Some(_batch_hash), Some(_sig), Some(sig_alg)) = (&req.batch_hash, &req.agent_signature, req.signature_algorithm) {
            match SignatureAlgorithm::try_from(sig_alg) {
                Ok(SignatureAlgorithm::Ed25519) => {
                    // Logic to retrieve agent's Ed25519 public key.
                    // For this stub, we just pretend it succeeds.
                    println!("Verifying Ed25519 signature for agent {}", req.agent_id);
                }
                Ok(SignatureAlgorithm::MlDsa65) => {
                    // Logic to retrieve agent's ML-DSA-65 public key.
                    println!("Verifying ML-DSA-65 signature for agent {}", req.agent_id);
                }
                _ => {
                    return Err(Status::invalid_argument("Unsupported signature algorithm"));
                }
            }
        }

        let response = IngestResponse {
            success: true,
            message: "Events ingested successfully".to_string(),
            events_accepted: events_count,
            rejected_event_ids: vec![],
        };

        Ok(Response::new(response))
    }

    async fn request_governance(
        &self,
        request: Request<GovernanceRequest>,
    ) -> Result<Response<GovernanceResponse>, Status> {
        let req = request.into_inner();

        let response = GovernanceResponse {
            request_id: req.request_id,
            verdict: GovernanceVerdict::Approved.into(),
            rationale: "Approved by Cathedral Bridge Stub".to_string(),
            conditions: vec![],
            evaluated_by: "cathedral_bridge_stub".to_string(),
            evaluated_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: chrono::Utc::now().timestamp_subsec_nanos() as i32,
            }),
        };

        Ok(Response::new(response))
    }

    async fn query_provenance(
        &self,
        _request: Request<QueryProvenanceRequest>,
    ) -> Result<Response<QueryProvenanceResponse>, Status> {
        let response = QueryProvenanceResponse {
            entries: vec![],
            has_more: false,
        };

        Ok(Response::new(response))
    }
}
