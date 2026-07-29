use arkhe_core::hash::Hasher;

pub struct Blake3Hasher;
impl Hasher for Blake3Hasher {
    fn hash(&self, content: &[u8]) -> [u8; 32] {
        blake3::hash(content).into()
    }
}
