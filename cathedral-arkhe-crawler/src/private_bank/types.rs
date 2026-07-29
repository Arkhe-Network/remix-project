use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentTokenV3 {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkBalanceProof {
    pub proof: Vec<u8>,
}
