import grpc
import sys
import os

# Add generated directory to path so imports work
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../generated')))

import cathedral.v1.bridge_pb2 as pb2
import cathedral.v1.bridge_pb2_grpc as pb2_grpc
from google.protobuf.timestamp_pb2 import Timestamp
import time
import uuid
import json

class CathedralGrpcClient:
    def __init__(self, endpoint: str = "localhost:50051"):
        self.channel = grpc.insecure_channel(endpoint)
        self.stub = pb2_grpc.CathedralBridgeStub(self.channel)

    def ingest(self, project_id: str, agent_id: str, events: list, agent_signature: bytes = None, batch_hash: bytes = None, signature_algorithm: int = None) -> pb2.IngestResponse:
        request = pb2.IngestRequest(
            project_id=project_id,
            agent_id=agent_id,
            events=events,
            agent_signature=agent_signature,
            batch_hash=batch_hash,
            signature_algorithm=signature_algorithm
        )
        return self.stub.Ingest(request)

    def request_governance(self, project_id: str, agent_id: str, event_type: int, proposed_state: dict) -> pb2.GovernanceResponse:
        request = pb2.GovernanceRequest(
            request_id=str(uuid.uuid4()),
            project_id=project_id,
            agent_id=agent_id,
            event_type=event_type,
            proposed_state_json=json.dumps(proposed_state)
        )
        return self.stub.RequestGovernance(request)

class CathedralSdk:
    def __init__(self, endpoint: str = "localhost:50051", project_id: str = "default", agent_id: str = "agent-1"):
        self.client = CathedralGrpcClient(endpoint)
        self.project_id = project_id
        self.agent_id = agent_id

    def emit_design_proposed(self, design_hash: str, parent_hashes: list, parameters: dict, rationale: str):
        payload = {
            "parameters": parameters,
            "rationale": rationale
        }

        ts = Timestamp()
        ts.GetCurrentTime()

        event = pb2.Event(
            event_id=str(uuid.uuid4()),
            timestamp=ts,
            event_type=pb2.EventType.DESIGN_PROPOSED,
            design_hash=design_hash,
            parent_hashes=parent_hashes,
            payload_json=json.dumps(payload),
            metadata=pb2.EventMetadata(
                domain="aerospace",
                confidence=0.5,
                compute_cost_usd=0.0
            )
        )

        return self.client.ingest(self.project_id, self.agent_id, [event])
