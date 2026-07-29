with open("safe-core/crates/dyn-signature/src/lib.rs", "r") as f:
    data = f.read()

bad_string = """#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynVerifyingKey {"""
good_string = """#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynVerifyingKey {"""
data = data.replace(bad_string, good_string)

with open("safe-core/crates/dyn-signature/src/lib.rs", "w") as f:
    f.write(data)

with open("safe-core/crates/safe-core-reactive-governance/src/lib.rs", "r") as f:
    data2 = f.read()

bad_string2 = """let sig_bytes = gov_sk.sign(&payload).to_vec();"""
good_string2 = """use p256::ecdsa::signature::Signer;
        let sig_bytes = gov_sk.sign(&payload).to_vec();"""
data2 = data2.replace(bad_string2, good_string2)
with open("safe-core/crates/safe-core-reactive-governance/src/lib.rs", "w") as f:
    f.write(data2)
