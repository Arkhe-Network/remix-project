with open("safe-core/crates/dyn-signature/src/lib.rs", "r") as f:
    data = f.read()

bad_string = """pub fn verify_dyn_signature(sig: &DynSignature, key: &DynVerifyingKey, payload: &[u8]) -> Result<(), SignatureError> {
    Ok(())
}"""
good_string = """pub fn verify_dyn_signature(sig: &DynSignature, key: &DynVerifyingKey, payload: &[u8]) -> Result<(), SignatureError> {
    match (sig, key) {
        #[cfg(feature = "p256")]
        (DynSignature::P256(sig_bytes), DynVerifyingKey::P256(vk)) => {
            use p256::ecdsa::signature::Verifier;
            let signature = p256::ecdsa::Signature::from_slice(sig_bytes)
                .map_err(|e| SignatureError::InvalidKey(e.to_string()))?;
            vk.verify(payload, &signature).map_err(|e| SignatureError::VerificationFailed(e.to_string()))
        }
        #[cfg(feature = "ed25519")]
        (DynSignature::Ed25519(sig_bytes), DynVerifyingKey::Ed25519(vk)) => {
            use ed25519_dalek::Verifier;
            let bytes_arr: [u8; 64] = sig_bytes.as_slice().try_into()
                .map_err(|_| SignatureError::InvalidKey("Ed25519 sig must be 64 bytes".into()))?;
            let signature = ed25519_dalek::Signature::from_bytes(&bytes_arr);
            vk.verify(payload, &signature).map_err(|e| SignatureError::VerificationFailed(e.to_string()))
        }
        _ => Err(SignatureError::AlgorithmMismatch { expected: "Matching algorithms".into(), actual: "Mismatched algorithms".into() }),
    }
}"""
data = data.replace(bad_string, good_string)
with open("safe-core/crates/dyn-signature/src/lib.rs", "w") as f:
    f.write(data)

with open("safe-core/crates/safe-core-reactive-governance/src/governance.rs", "r") as f:
    data2 = f.read()
bad_string2 = """#[serde(skip)] pub verifying_key_opt: Option<DynVerifyingKey>,"""
good_string2 = """pub verifying_key: DynVerifyingKey,"""
data2 = data2.replace(bad_string2, good_string2)
bad_string3 = """if let Some(vk) = &self.verifying_key_opt { verify_dyn_signature(&self.signature, vk, &payload).map_err(|e| GovernanceError::InvalidSignature(e.to_string())) } else { Err(GovernanceError::InvalidSignature("No verifying key".into())) }"""
good_string3 = """
        let payload = serde_jcs::to_string(&self.action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?.into_bytes();
        verify_dyn_signature(&self.signature, &self.verifying_key, &payload)
            .map_err(|e| GovernanceError::InvalidSignature(e.to_string()))
"""
data2 = data2.replace(bad_string3, good_string3)
# We also have a previous line defining payload using serde_json which we need to remove
bad_string4 = """let payload = serde_json::to_vec(&self.action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?;"""
data2 = data2.replace(bad_string4, "")
with open("safe-core/crates/safe-core-reactive-governance/src/governance.rs", "w") as f:
    f.write(data2)

with open("safe-core/crates/safe-core-reactive-governance/src/reactive_log.rs", "r") as f:
    data3 = f.read()
bad_string5 = """if let Some(vk) = &entry.verifying_key_opt {
            if !self.authorized_keys.contains(vk) {
                return Err(GovernanceError::Unauthorized(entry.issued_by));
            }
        } else {
            return Err(GovernanceError::Unauthorized(entry.issued_by));
        }"""
good_string5 = """if !self.authorized_keys.contains(&entry.verifying_key) { return Err(GovernanceError::Unauthorized(entry.issued_by)); }"""
data3 = data3.replace(bad_string5, good_string5)

bad_string6 = """pub banned_routes: HashMap<String, Vec<String>>,"""
good_string6 = """pub banned_routes: HashMap<String, std::collections::HashSet<String>>,"""
data3 = data3.replace(bad_string6, good_string6)

bad_string7 = """state.banned_routes.entry(router_id.clone()).or_default().push(path);"""
good_string7 = """state.banned_routes.entry(router_id.clone()).or_default().insert(path);"""
data3 = data3.replace(bad_string7, good_string7)

with open("safe-core/crates/safe-core-reactive-governance/src/reactive_log.rs", "w") as f:
    f.write(data3)

with open("safe-core/crates/safe-core-reactive-governance/src/watchdog.rs", "r") as f:
    data4 = f.read()

bad_string8 = """verifying_key_opt: Some(verifying_key), """
good_string8 = """verifying_key, """
data4 = data4.replace(bad_string8, good_string8)

bad_string9 = """let payload = serde_json::to_vec(&action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?;"""
good_string9 = """let payload = serde_jcs::to_string(&action)
            .map_err(|e| GovernanceError::Serialization(e.to_string()))?.into_bytes();"""
data4 = data4.replace(bad_string9, good_string9)

with open("safe-core/crates/safe-core-reactive-governance/src/watchdog.rs", "w") as f:
    f.write(data4)


with open("safe-core/crates/safe-core-reactive-governance/src/lib.rs", "a") as f:
    f.write('''
#[cfg(test)]
mod tests {
    use super::*;
    use crypto::{DynSignature, DynVerifyingKey, SignatureAlgorithm};
    use crate::transparency::TransparencyLog;
    use crate::governance::{GovernanceAction, GovernanceEntry};
    use crate::reactive_log::ReactiveLog;
    use hash::Blake3Hasher;
    use p256::ecdsa::SigningKey;
    use ecdsa::signature::Signer;
    use rand_core::OsRng;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn setup() -> (
        DynVerifyingKey,
        ReactiveLog<Blake3Hasher>,
        SigningKey,
    ) {
        let gov_sk = SigningKey::random(&mut OsRng);
        let gov_vk = DynVerifyingKey::P256(*gov_sk.verifying_key());

        let inner_log = TransparencyLog::<Blake3Hasher>::new();

        let reactive_log = ReactiveLog::new(inner_log, vec![gov_vk.clone()]);

        (gov_vk, reactive_log, gov_sk)
    }

    #[tokio::test]
    async fn test_reactive_freeze_blocks_operations() {
        let (gov_vk, mut reactive_log, gov_sk) = setup();

        assert!(!reactive_log.is_frozen().await);

        let action = GovernanceAction::EmergencyFreeze {
            reason: "Test freeze".into(),
            duration_seconds: 60,
        };

        let payload = serde_jcs::to_string(&action).unwrap().into_bytes();
        let sig_bytes = gov_sk.sign(&payload).to_vec();
        let sig = DynSignature::P256(sig_bytes);
        let entry = GovernanceEntry { action, issued_by: "test-authority".into(), verifying_key: gov_vk, signature: sig, timestamp: 1234 };

        reactive_log.apply_governance_entry(entry).await.unwrap();

        assert!(reactive_log.is_frozen().await, "System should be frozen");
    }
}''')
