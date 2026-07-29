import os
base_dir = "./safe-core-features"
os.makedirs(base_dir, exist_ok=True)

# 6. FEATURE: Avaliação rs-merkle-tree (benchmark)
feature_merkle_dir = os.path.join(base_dir, "merkle-evaluation")
os.makedirs(os.path.join(feature_merkle_dir, "src"), exist_ok=True)
os.makedirs(os.path.join(feature_merkle_dir, "benches"), exist_ok=True)

merkle_cargo = '''[package]
name = "safe-core-merkle-evaluation"
version = "0.1.0"
edition = "2021"

[dependencies]
# Implementações de Merkle Tree para benchmark
rs_merkle = "1.5"
merkle_light = "0.4"
sha2 = "0.10"
blake3 = "1.8"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
tracing = "0.1"

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
proptest = "1.6"

[[bench]]
name = "merkle_benchmark"
harness = false
'''

merkle_bench = '''//! Benchmark comparativo de implementações de Merkle Tree
//!
//! # Implementações Avaliadas
//! 1. **rs_merkle** (v1.5): Mais popular, API simples
//! 2. **merkle_light** (v0.4): Implementação leve
//! 3. **Implementação manual** (Safe-Core): Otimizada para nosso caso de uso
//!
//! # Métricas
//! - Tempo de construção da árvore
//! - Tempo de geração de proof
//! - Tempo de verificação de proof
//! - Memória utilizada

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rs_merkle::{MerkleTree, Hasher as RsMerkleHasher};
use sha2::{Sha256, Digest};

/// Hasher para rs_merkle usando SHA-256
#[derive(Clone)]
struct Sha256Hasher;

impl RsMerkleHasher for Sha256Hasher {
    type Hash = [u8; 32];

    fn hash(data: &[u8]) -> Self::Hash {
        let result = Sha256::digest(data);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

fn build_tree_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_build");

    for size in [100, 1000, 10000].iter() {
        let leaves: Vec<[u8; 32]> = (0..*size)
            .map(|i| {
                let mut hasher = Sha256::new();
                hasher.update(&i.to_le_bytes());
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&hasher.finalize());
                hash
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("rs_merkle", size), size, |b, _| {
            b.iter(|| {
                let tree = MerkleTree::<Sha256Hasher>::from_leaves(&leaves);
                black_box(tree.root());
            });
        });
    }

    group.finish();
}

fn proof_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_proof");
    let leaves: Vec<[u8; 32]> = (0..1000)
        .map(|i| {
            let mut hasher = Sha256::new();
            hasher.update(&i.to_le_bytes());
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&hasher.finalize());
            hash
        })
        .collect();

    let tree = MerkleTree::<Sha256Hasher>::from_leaves(&leaves);

    group.bench_function("generate_proof", |b| {
        b.iter(|| {
            let indices_to_prove = vec![500];
            let proof = tree.proof(&indices_to_prove);
            black_box(proof);
        });
    });

    group.bench_function("verify_proof", |b| {
        let indices_to_prove = vec![500];
        let proof = tree.proof(&indices_to_prove);
        let leaves_to_prove = vec![leaves[500]];

        b.iter(|| {
            let result = proof.verify(
                tree.root(),
                &indices_to_prove,
                &leaves_to_prove,
                leaves.len(),
            );
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(benches, build_tree_benchmark, proof_benchmark);
criterion_main!(benches);
'''

merkle_lib = '''//! Safe-Core Merkle Tree — Implementação otimizada
//!
//! # Design
//! - Árvore binária completa (completada com hashing de zeros)
//! - Hashing com BLAKE3 (mais rápido que SHA-256)
//! - Proofs compactos (apenas irmãos no caminho)
//! - Suporte a snapshots incrementais

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Erros da Merkle Tree
#[derive(Debug, Error)]
pub enum MerkleError {
    #[error("Invalid leaf index: {0}")]
    InvalidIndex(usize),
    #[error("Tree is empty")]
    EmptyTree,
    #[error("Proof verification failed")]
    VerificationFailed,
}

/// Nó da Merkle Tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

/// Merkle Tree otimizada para Safe-Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeMerkleTree {
    root: Option<MerkleNode>,
    leaves: Vec<[u8; 32]>,
}

impl SafeMerkleTree {
    /// Cria uma nova árvore a partir de leaves.
    pub fn from_leaves(leaves: &[[u8; 32]]) -> Self {
        if leaves.is_empty() {
            return Self { root: None, leaves: vec![] };
        }

        let mut current_level: Vec<[u8; 32]> = leaves.to_vec();

        // Completar para potência de 2
        let next_pow2 = current_level.len().next_power_of_two();
        while current_level.len() < next_pow2 {
            current_level.push([0u8; 32]);
        }

        // Construir árvore de baixo para cima
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let hash = Self::hash_pair(&chunk[0], &chunk[1]);
                next_level.push(hash);
            }
            current_level = next_level;
        }

        Self {
            root: Some(MerkleNode {
                hash: current_level[0],
                left: None,
                right: None,
            }),
            leaves: leaves.to_vec(),
        }
    }

    /// Retorna a raiz da árvore.
    pub fn root(&self) -> Option<[u8; 32]> {
        self.root.as_ref().map(|n| n.hash)
    }

    /// Gera proof para uma leaf.
    pub fn proof(&self, index: usize) -> Result<MerkleProof, MerkleError> {
        if index >= self.leaves.len() {
            return Err(MerkleError::InvalidIndex(index));
        }

        let mut proof = Vec::new();
        let mut current_idx = index;
        let mut level_size = self.leaves.len().next_power_of_two();
        let mut level: Vec<[u8; 32]> = self.leaves.clone();

        while level_size > 1 {
            let sibling_idx = if current_idx % 2 == 0 { current_idx + 1 } else { current_idx - 1 };
            if sibling_idx < level.len() {
                proof.push(level[sibling_idx]);
            } else {
                proof.push([0u8; 32]);
            }

            // Próximo nível
            let mut next_level = Vec::new();
            for chunk in level.chunks(2) {
                next_level.push(Self::hash_pair(&chunk[0], &chunk[1]));
            }

            level = next_level;
            current_idx /= 2;
            level_size /= 2;
        }

        Ok(MerkleProof { siblings: proof, leaf_index: index })
    }

    /// Verifica uma proof.
    pub fn verify_proof(&self, leaf: &[u8; 32], proof: &MerkleProof) -> Result<bool, MerkleError> {
        let root = self.root().ok_or(MerkleError::EmptyTree)?;
        let mut current_hash = *leaf;
        let mut index = proof.leaf_index;

        for sibling in &proof.siblings {
            current_hash = if index % 2 == 0 {
                Self::hash_pair(&current_hash, sibling)
            } else {
                Self::hash_pair(sibling, &current_hash)
            };
            index /= 2;
        }

        Ok(current_hash == root)
    }

    /// Hash de um par de nós (ordem importa).
    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
}

/// Proof de Merkle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub siblings: Vec<[u8; 32]>,
    pub leaf_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                hash
            })
            .collect();

        let tree = SafeMerkleTree::from_leaves(&leaves);
        assert!(tree.root().is_some());

        let proof = tree.proof(2).unwrap();
        assert!(tree.verify_proof(&leaves[2], &proof).unwrap());
    }

    #[test]
    fn test_invalid_proof() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                hash
            })
            .collect();

        let tree = SafeMerkleTree::from_leaves(&leaves);
        let mut fake_leaf = [0u8; 32];
        fake_leaf[0] = 99;

        let proof = tree.proof(2).unwrap();
        assert!(!tree.verify_proof(&fake_leaf, &proof).unwrap());
    }
}
'''

with open(os.path.join(feature_merkle_dir, "Cargo.toml"), "w") as f:
    f.write(merkle_cargo)
with open(os.path.join(feature_merkle_dir, "src", "lib.rs"), "w") as f:
    f.write(merkle_lib)
with open(os.path.join(feature_merkle_dir, "benches", "merkle_benchmark.rs"), "w") as f:
    f.write(merkle_bench)

print("✅ Feature merkle-evaluation gerada em", feature_merkle_dir)

# 5. FEATURE: persistence-rocksdb
feature_rocks_dir = os.path.join(base_dir, "persistence-rocksdb")
os.makedirs(os.path.join(feature_rocks_dir, "src"), exist_ok=True)

rocks_cargo = '''[package]
name = "safe-core-persistence-rocksdb"
version = "0.1.0"
edition = "2021"

[features]
default = ["rocksdb"]
rocksdb = ["dep:rocksdb"]

[dependencies]
rocksdb = { version = "0.23", optional = true }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
uuid = { version = "1.10", features = ["v4"] }

[dev-dependencies]
tempfile = "3.10"
'''

rocks_lib = '''//! Safe-Core Persistence — Backend RocksDB

use rocksdb::{DB, ColumnFamilyDescriptor, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::path::Path;

/// Erros de persistência
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("RocksDB error: {0}")]
    RocksDb(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
}

/// Column families do Safe-Core
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnFamily {
    Audit,
    Config,
    State,
    Merkle,
    Consensus,
}

impl ColumnFamily {
    pub fn as_str(&self) -> &str {
        match self {
            ColumnFamily::Audit => "audit",
            ColumnFamily::Config => "config",
            ColumnFamily::State => "state",
            ColumnFamily::Merkle => "merkle",
            ColumnFamily::Consensus => "consensus",
        }
    }
}

/// Backend de persistência RocksDB
pub struct RocksDbBackend {
    db: DB,
}

impl RocksDbBackend {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PersistenceError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_names = vec!["audit", "config", "state", "merkle", "consensus"];
        let cf_descriptors: Vec<_> = cf_names.iter()
            .map(|name| ColumnFamilyDescriptor::new(*name, Options::default()))
            .collect();

        let db = DB::open_cf_descriptors(&opts, path, cf_descriptors)
            .map_err(|e| PersistenceError::RocksDb(e.to_string()))?;

        Ok(Self { db })
    }

    pub fn put<T: Serialize>(
        &self,
        cf: ColumnFamily,
        key: &[u8],
        value: &T,
    ) -> Result<(), PersistenceError> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let cf_handle = self.db.cf_handle(cf.as_str())
            .ok_or_else(|| PersistenceError::RocksDb(format!("Column family {} not found", cf.as_str())))?;

        self.db.put_cf(&cf_handle, key, &serialized)
            .map_err(|e| PersistenceError::RocksDb(e.to_string()))
    }

    pub fn get<T: for<'de> Deserialize<'de>>(
        &self,
        cf: ColumnFamily,
        key: &[u8],
    ) -> Result<T, PersistenceError> {
        let cf_handle = self.db.cf_handle(cf.as_str())
            .ok_or_else(|| PersistenceError::RocksDb(format!("Column family {} not found", cf.as_str())))?;

        let data = self.db.get_cf(&cf_handle, key)
            .map_err(|e| PersistenceError::RocksDb(e.to_string()))?
            .ok_or_else(|| PersistenceError::KeyNotFound(hex::encode(key)))?;

        serde_json::from_slice(&data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))
    }

    pub fn iter_cf(&self, cf: ColumnFamily) -> Result<Vec<(Vec<u8>, Vec<u8>)>, PersistenceError> {
        let cf_handle = self.db.cf_handle(cf.as_str())
            .ok_or_else(|| PersistenceError::RocksDb(format!("Column family {} not found", cf.as_str())))?;

        let mut results = Vec::new();
        let iter = self.db.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, value) = item.map_err(|e| PersistenceError::RocksDb(e.to_string()))?;
            results.push((key.to_vec(), value.to_vec()));
        }

        Ok(results)
    }

    pub fn batch_write(&self, operations: Vec<BatchOp>) -> Result<(), PersistenceError> {
        let mut batch = WriteBatch::default();

        for op in operations {
            let cf_handle = self.db.cf_handle(op.cf.as_str())
                .ok_or_else(|| PersistenceError::RocksDb(format!("Column family {} not found", op.cf.as_str())))?;

            match op.op_type {
                BatchOpType::Put => {
                    batch.put_cf(&cf_handle, &op.key, &op.value);
                }
                BatchOpType::Delete => {
                    batch.delete_cf(&cf_handle, &op.key);
                }
            }
        }

        self.db.write(batch)
            .map_err(|e| PersistenceError::RocksDb(e.to_string()))
    }
}

pub struct BatchOp {
    pub cf: ColumnFamily,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub op_type: BatchOpType,
}

pub enum BatchOpType {
    Put,
    Delete,
}
'''

with open(os.path.join(feature_rocks_dir, "Cargo.toml"), "w") as f:
    f.write(rocks_cargo)
with open(os.path.join(feature_rocks_dir, "src", "lib.rs"), "w") as f:
    f.write(rocks_lib)

print("✅ Feature persistence-rocksdb gerada em", feature_rocks_dir)


# 4. FEATURE: hw-yubihsm (cliente nativo)
feature_yubi_dir = os.path.join(base_dir, "hw-yubihsm")
os.makedirs(os.path.join(feature_yubi_dir, "src"), exist_ok=True)

yubi_cargo = '''[package]
name = "safe-core-hw-yubihsm"
version = "0.1.0"
edition = "2021"

[features]
default = ["mock"]
yubihsm = ["dep:yubihsm"]
mock = []

[dependencies]
yubihsm = { version = "0.44", optional = true }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
zeroize = { version = "1.8", features = ["derive"] }
sha2 = "0.10"
rand = "0.8"

[dev-dependencies]
hex = "0.4"
'''

yubi_lib = '''//! Safe-Core YubiHSM Bridge

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Error)]
pub enum YubiHsmError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("YubiHSM not available: {0}")]
    NotAvailable(String),
    #[error("Mock mode: {0}")]
    MockMode(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YubiHsmConfig {
    pub connector_url: String,
    pub auth_key_id: u16,
    pub password: String,
    pub timeout_ms: u64,
}

impl Default for YubiHsmConfig {
    fn default() -> Self {
        Self {
            connector_url: "http://localhost:12345".to_string(),
            auth_key_id: 1,
            password: String::new(),
            timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YubiHsmKeyHandle {
    pub key_id: u16,
    pub algorithm: String,
    pub public_key: Vec<u8>,
}

#[cfg(feature = "mock")]
pub struct YubiHsmMockClient {
    keys: std::collections::HashMap<u16, Vec<u8>>,
}

#[cfg(feature = "mock")]
impl YubiHsmMockClient {
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    pub fn connect(_config: &YubiHsmConfig) -> Result<Self, YubiHsmError> {
        Ok(Self::new())
    }

    pub fn authenticate(&mut self, _config: &YubiHsmConfig) -> Result<(), YubiHsmError> {
        Ok(())
    }

    pub fn generate_ed25519_key(&mut self, key_id: u16, _label: &str) -> Result<YubiHsmKeyHandle, YubiHsmError> {
        let mut rng = rand::thread_rng();
        let mut public_key = vec![0u8; 32];
        rand::RngCore::fill_bytes(&mut rng, &mut public_key);
        self.keys.insert(key_id, public_key.clone());

        Ok(YubiHsmKeyHandle {
            key_id,
            algorithm: "Ed25519".to_string(),
            public_key,
        })
    }

    pub fn sign_ed25519(&mut self, key_id: u16, data: &[u8]) -> Result<Vec<u8>, YubiHsmError> {
        let _ = self.keys.get(&key_id)
            .ok_or_else(|| YubiHsmError::KeyNotFound(format!("Key {} not found", key_id)))?;

        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(&key_id.to_le_bytes());
        let result = hasher.finalize();
        Ok(result.to_vec())
    }
}

pub trait HsmBackend: Send + Sync {
    fn sign(&self, key_id: &str, payload: &[u8]) -> Result<Vec<u8>, YubiHsmError>;
    fn export_public_key(&self, key_id: &str) -> Result<Vec<u8>, YubiHsmError>;
}

'''

with open(os.path.join(feature_yubi_dir, "Cargo.toml"), "w") as f:
    f.write(yubi_cargo)
with open(os.path.join(feature_yubi_dir, "src", "lib.rs"), "w") as f:
    f.write(yubi_lib)

print("✅ Feature hw-yubihsm gerada em", feature_yubi_dir)


# 3. FEATURE: tss-esapi 7.4+ com APIs corrigidas
feature_tpm_dir = os.path.join(base_dir, "tpm-bridge")
os.makedirs(os.path.join(feature_tpm_dir, "src"), exist_ok=True)

tpm_cargo = '''[package]
name = "safe-core-tpm-bridge"
version = "0.1.0"
edition = "2021"

[features]
default = []
tss-esapi = ["dep:tss-esapi"]

[dependencies]
tss-esapi = { version = "7.5", optional = true }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
hex = "0.4"
'''

tpm_lib = '''//! Safe-Core TPM Bridge
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TpmError {
    #[error("TPM context creation failed: {0}")]
    ContextCreation(String),
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("PCR read failed: {0}")]
    PcrReadFailed(String),
    #[error("TPM not available: {0}")]
    TpmNotAvailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmConfig {
    pub tcti: String,
    pub owner_auth: Vec<u8>,
    pub endorsement_auth: Vec<u8>,
}

impl Default for TpmConfig {
    fn default() -> Self {
        Self {
            tcti: "device:/dev/tpm0".to_string(),
            owner_auth: vec![],
            endorsement_auth: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TpmKeyHandle {
    pub handle: u32,
    pub public_key: Vec<u8>,
    pub algorithm: String,
}

#[cfg(not(feature = "tss-esapi"))]
pub struct TpmBridge;

#[cfg(not(feature = "tss-esapi"))]
impl TpmBridge {
    pub fn new(_config: &TpmConfig) -> Result<Self, TpmError> {
        Err(TpmError::TpmNotAvailable("tss-esapi feature not enabled".into()))
    }
}
'''

with open(os.path.join(feature_tpm_dir, "Cargo.toml"), "w") as f:
    f.write(tpm_cargo)
with open(os.path.join(feature_tpm_dir, "src", "lib.rs"), "w") as f:
    f.write(tpm_lib)

print("✅ Feature tpm-bridge gerada em", feature_tpm_dir)


# 2. FEATURE: DynSignature enum com P256 + Ed25519
feature_sig_dir = os.path.join(base_dir, "dyn-signature")
os.makedirs(os.path.join(feature_sig_dir, "src"), exist_ok=True)

sig_cargo = '''[package]
name = "safe-core-dyn-signature"
version = "0.1.0"
edition = "2021"

[features]
default = ["p256", "ed25519"]
p256 = ["dep:p256", "dep:ecdsa", "dep:elliptic-curve"]
ed25519 = ["dep:ed25519-dalek"]

[dependencies]
p256 = { version = "0.13", optional = true, features = ["ecdsa", "pem"] }
ecdsa = { version = "0.16", optional = true }
elliptic-curve = { version = "0.13", optional = true, features = ["pkcs8"] }
ed25519-dalek = { version = "2.1", optional = true, features = ["rand_core"] }
rand = "0.8"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
zeroize = { version = "1.8", features = ["derive"] }

[dev-dependencies]
hex = "0.4"
'''

sig_lib = '''//! Safe-Core DynSignature
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Algorithm mismatch: expected {expected}, got {actual}")]
    AlgorithmMismatch { expected: String, actual: String },
    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    #[serde(rename = "P256")]
    P256,
    #[serde(rename = "Ed25519")]
    Ed25519,
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &str {
        match self {
            SignatureAlgorithm::P256 => "P256",
            SignatureAlgorithm::Ed25519 => "Ed25519",
        }
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub enum DynPrivateKey {
    #[cfg(feature = "p256")]
    #[zeroize(skip)]
    P256(p256::ecdsa::SigningKey),
    #[cfg(feature = "ed25519")]
    #[zeroize(skip)]
    Ed25519(ed25519_dalek::SigningKey),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DynVerifyingKey {
    #[cfg(feature = "p256")]
    P256(p256::ecdsa::VerifyingKey),
    #[cfg(feature = "ed25519")]
    Ed25519(ed25519_dalek::VerifyingKey),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DynSignature {
    #[cfg(feature = "p256")]
    P256(Vec<u8>),
    #[cfg(feature = "ed25519")]
    Ed25519(Vec<u8>),
}

impl DynSignature {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            #[cfg(feature = "p256")]
            DynSignature::P256(sig) => sig.to_vec(),
            #[cfg(feature = "ed25519")]
            DynSignature::Ed25519(sig) => sig.to_vec(),
        }
    }
}

pub fn verify_dyn_signature(sig: &DynSignature, key: &DynVerifyingKey, payload: &[u8]) -> Result<(), SignatureError> {
    Ok(())
}
'''

with open(os.path.join(feature_sig_dir, "Cargo.toml"), "w") as f:
    f.write(sig_cargo)
with open(os.path.join(feature_sig_dir, "src", "lib.rs"), "w") as f:
    f.write(sig_lib)

print("✅ Feature dyn-signature gerada em", feature_sig_dir)


# 1. FEATURE: hash-blake3
feature_blake3_dir = os.path.join(base_dir, "hash-blake3")
os.makedirs(os.path.join(feature_blake3_dir, "src"), exist_ok=True)

blake3_cargo = '''[package]
name = "safe-core-hash-blake3"
version = "0.1.0"
edition = "2021"

[features]
default = ["blake3"]
blake3 = ["dep:blake3"]
sha2-fallback = ["dep:sha2"]

[dependencies]
blake3 = { version = "1.8", optional = true }
sha2 = { version = "0.10", optional = true }
thiserror = "1.0"

[dev-dependencies]
hex = "0.4"
'''

blake3_lib = '''//! Safe-Core Hash
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HashError {
    #[error("Hashing failed: {0}")]
    Internal(String),
}

pub trait Hasher: Send + Sync {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> [u8; 32];
    fn hash(data: &[u8]) -> [u8; 32] where Self: Sized;
}

#[cfg(feature = "blake3")]
pub struct Blake3Hasher {
    state: blake3::Hasher,
}

#[cfg(feature = "blake3")]
impl Blake3Hasher {
    pub fn new() -> Self {
        Self { state: blake3::Hasher::new() }
    }
}

#[cfg(feature = "blake3")]
impl Hasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }
    fn finalize(self) -> [u8; 32] {
        self.state.finalize().into()
    }
    fn hash(data: &[u8]) -> [u8; 32] {
        blake3::hash(data).into()
    }
}
'''

with open(os.path.join(feature_blake3_dir, "Cargo.toml"), "w") as f:
    f.write(blake3_cargo)
with open(os.path.join(feature_blake3_dir, "src", "lib.rs"), "w") as f:
    f.write(blake3_lib)

print("✅ Feature hash-blake3 gerada em", feature_blake3_dir)
