// crates/safe-core-core/src/lib.rs
pub fn blake3_hash(data: &[u8]) -> blake3::Hash {
    blake3::hash(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_roundtrip() {
        let hash = blake3_hash(b"test");
        assert!(!hash.as_bytes().iter().all(|&b| b == 0));
        assert_eq!(hash.to_hex().len(), 64);
    }
}
