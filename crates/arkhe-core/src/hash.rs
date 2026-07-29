pub trait Hasher: Send + Sync {
    fn hash(&self, content: &[u8]) -> [u8; 32];
}

pub struct IdentityHasher;
impl Hasher for IdentityHasher {
    fn hash(&self, content: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        let len = content.len().min(32);
        out[..len].copy_from_slice(&content[..len]);
        out
    }
}

pub struct FnHasher<F>(pub F);
impl<F: Fn(&[u8]) -> [u8; 32] + Send + Sync> Hasher for FnHasher<F> {
    fn hash(&self, content: &[u8]) -> [u8; 32] {
        (self.0)(content)
    }
}
