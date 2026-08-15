// crates/safe-core-identity/src/lib.rs
#[derive(Clone)]
pub struct ArkheDid {
    pub namespace: String,
    pub id: String,
}

impl ArkheDid {
    pub fn new(namespace: &str, id: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            id: id.to_string(),
        }
    }
}
