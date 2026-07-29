use crate::tree::MerkleHash;

pub struct Proof {
    pub siblings: Vec<MerkleHash>,
}
