with open("safe-core/crates/dyn-signature/src/lib.rs", "r") as f:
    data = f.read()

# Custom serialization for DynVerifyingKey
bad_string = """#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynVerifyingKey {"""
good_string = """#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynVerifyingKey {"""
data = data.replace(bad_string, good_string)

add_serialization = """
impl serde::Serialize for DynVerifyingKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            #[cfg(feature = "p256")]
            DynVerifyingKey::P256(vk) => {
                let bytes = vk.to_sec1_bytes();
                let mut state = serializer.serialize_struct("DynVerifyingKey", 2)?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "type", "P256")?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "data", bytes.as_ref())?;
                serde::ser::SerializeStruct::end(state)
            }
            #[cfg(feature = "ed25519")]
            DynVerifyingKey::Ed25519(vk) => {
                let bytes = vk.as_bytes();
                let mut state = serializer.serialize_struct("DynVerifyingKey", 2)?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "type", "Ed25519")?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "data", bytes.as_ref())?;
                serde::ser::SerializeStruct::end(state)
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for DynVerifyingKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct KeyData {
            #[serde(rename = "type")]
            key_type: String,
            data: Vec<u8>,
        }

        let key_data = KeyData::deserialize(deserializer)?;
        match key_data.key_type.as_str() {
            #[cfg(feature = "p256")]
            "P256" => {
                let vk = p256::ecdsa::VerifyingKey::from_sec1_bytes(&key_data.data)
                    .map_err(serde::de::Error::custom)?;
                Ok(DynVerifyingKey::P256(vk))
            }
            #[cfg(feature = "ed25519")]
            "Ed25519" => {
                let arr: [u8; 32] = key_data.data.try_into().map_err(|_| serde::de::Error::custom("Invalid Ed25519 length"))?;
                let vk = ed25519_dalek::VerifyingKey::from_bytes(&arr)
                    .map_err(serde::de::Error::custom)?;
                Ok(DynVerifyingKey::Ed25519(vk))
            }
            _ => Err(serde::de::Error::custom("Unknown key type")),
        }
    }
}
"""
data = data + add_serialization

with open("safe-core/crates/dyn-signature/src/lib.rs", "w") as f:
    f.write(data)
