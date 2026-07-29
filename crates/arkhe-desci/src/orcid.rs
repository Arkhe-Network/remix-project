//! ORCID ↔ DIDArkhe Bridge
//!
//! Conecta identidades ORCID ao ecossistema DID do ARKHE:
//! - Verificação de ORCID via API pública
//! - Derivação de DID a partir do ORCID iD
//! - Resolução DID → ORCID profile
//! - Attestation: prova de que um DID controla um ORCID

use serde::{Deserialize, Serialize};


use crate::error::{DesciError, Result};

/// Prefixo DID para ORCID no ARKHE
pub const DID_ORCID_PREFIX: &str = "did:arkhe:orcid";

/// Perfil ORCID simplificado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcidProfile {
    pub orcid_id: String,
    pub given_names: String,
    pub family_name: String,
    pub email: Option<String>,
    pub institution: Option<String>,
    pub country: Option<String>,
    pub works_count: u32,
    pub keywords: Vec<String>,
}

/// DID derivado de ORCID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcidDID {
    pub did: String,
    pub orcid_id: String,
    pub did_document: DidDocument,
    pub verified: bool,
    pub verified_at: Option<String>,
}

/// DID Document simplificado (W3C DID Core)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    pub controller: Option<String>,
    #[serde(rename = "verificationMethod")]
    pub verification_methods: Vec<VerificationMethod>,
    pub service: Vec<DidService>,
    #[serde(rename = "alsoKnownAs")] pub also_known_as: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub vm_type: String,
    pub controller: String,
    pub public_key_multibase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidService {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    pub service_endpoint: String,
}

/// Attestation: prova criptográfica de vínculo DID↔ORCID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcidAttestation {
    pub attester_did: String,
    pub subject_did: String,
    pub orcid_id: String,
    pub claim_type: String,
    pub issued_at: String,
    pub expires_at: String,
    pub proof_hash: String,
}

/// Cliente ORCID (requer feature `orcid` para HTTP real)
#[cfg(feature = "orcid")]
pub struct OrcidClient {
    base_url: String,
    http: reqwest::Client,
    // Em produção: client_id + client_secret para OAuth
}

#[cfg(feature = "orcid")]
impl OrcidClient {
    /// API pública (sem auth, dados limitados)
    pub fn public() -> Self {
        Self {
            base_url: "https://pub.orcid.org/v3.0".into(),
            http: reqwest::Client::builder()
                .default_headers({
                    let mut h = reqwest::header::HeaderMap::new();
                    h.insert("Accept", "application/json".parse().unwrap());
                    h
                })
                .build()
                .unwrap(),
        }
    }

    /// Busca perfil público pelo ORCID iD
    pub async fn get_profile(&self, orcid_id: &str) -> Result<OrcidProfile> {
        let clean_id = orcid_id.trim_start_matches("https://orcid.org/");
        let url = format!("{}/{}/record", self.base_url, clean_id);

        let resp = self.http.get(&url)
            .send().await
            .map_err(|e| DesciError::OrcidError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(DesciError::OrcidNotFound { orcid_id: orcid_id.into() });
        }
        let resp = resp.error_for_status()
            .map_err(|e| DesciError::OrcidError(e.to_string()))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| DesciError::OrcidError(e.to_string()))?;

        let name = &data["person"]["name"];
        Ok(OrcidProfile {
            orcid_id: clean_id.into(),
            given_names: name["given-names"]["value"].as_str().unwrap_or("").into(),
            family_name: name["family-name"]["value"].as_str().unwrap_or("").into(),
            email: None, // Requer OAuth para acesso
            institution: data["employment-summary"]
                .get(0).and_then(|v| v.get("organization")).and_then(|v| v.get("name")).and_then(|v| v.as_str()).map(String::from),
            country: data["address"]["country"]["value"].as_str().map(String::from),
            works_count: data["activities-summary"]["works"]["group"].as_array()
                .map(|a| a.len() as u32).unwrap_or(0),
            keywords: data["keywords"]["keyword"].as_array()
                .map(|a| a.iter().filter_map(|k| k["content"].as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
    }

    pub fn base_url(&self) -> &str { &self.base_url }
}

/// Deriva DID ARKHE a partir de ORCID iD
pub fn derive_did(orcid_id: &str) -> String {
    let clean = orcid_id
        .trim_start_matches("https://orcid.org/")
        .replace('-', "");
    let hash = blake3::hash(clean.as_bytes()).to_string()[..16].to_string();
    format!("{}:{}", DID_ORCID_PREFIX, hash)
}

/// Gera DID Document para um ORCID
pub fn build_did_document(orcid_id: &str) -> OrcidDID {
    let did = derive_did(orcid_id);
    let vm_id = format!("{}#key-1", did);

    OrcidDID {
        did: did.clone(),
        orcid_id: orcid_id.trim_start_matches("https://orcid.org/").into(),
        did_document: DidDocument {
            id: did.clone(),
            controller: Some(did.clone()),
            verification_methods: vec![VerificationMethod {
                id: vm_id,
                vm_type: "Ed25519VerificationKey2020".into(),
                controller: did.clone(),
                public_key_multibase: None, // Em produção: chave real
            }],
            service: vec![
                DidService {
                    id: format!("{}#orcid", did),
                    service_type: "OrcidProfile".into(),
                    service_endpoint: format!("https://orcid.org/{}", orcid_id),
                },
                DidService {
                    id: format!("{}#desci", did),
                    service_type: "DesciNode".into(),
                    service_endpoint: "https://nodes.desci.com".into(),
                },
            ],
            also_known_as: vec![format!("https://orcid.org/{}", orcid_id)],
        },
        verified: false,
        verified_at: None,
    }
}

/// Cria attestation de vínculo
pub fn create_attestation(
    attester_did: &str,
    subject_did: &str,
    orcid_id: &str,
    valid_hours: u64,
) -> OrcidAttestation {
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(valid_hours as i64);

    let claim = format!("{}:{}:{}:{}",
        attester_did, subject_did, orcid_id, now.timestamp()
    );
    let proof_hash = blake3::hash(claim.as_bytes()).to_string();

    OrcidAttestation {
        attester_did: attester_did.into(),
        subject_did: subject_did.into(),
        orcid_id: orcid_id.into(),
        claim_type: "OrcidOwnership".into(),
        issued_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
        proof_hash,
    }
}

/// Verifica uma attestation
pub fn verify_attestation(att: &OrcidAttestation) -> bool {
    let claim = format!("{}:{}:{}:{}",
        att.attester_did, att.subject_did, att.orcid_id,
        chrono::DateTime::parse_from_rfc3339(&att.issued_at)
            .map(|dt| dt.timestamp()).unwrap_or(0)
    );
    let expected = blake3::hash(claim.as_bytes()).to_string();
    if att.proof_hash != expected {
        return false;
    }
    // Verificar expiração
    if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&att.expires_at) {
        if chrono::Utc::now() > exp.with_timezone(&chrono::Utc) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ORCID: &str = "0000-0001-2345-6789";

    #[test]
    fn test_derive_did_deterministic() {
        let d1 = derive_did(TEST_ORCID);
        let d2 = derive_did(TEST_ORCID);
        assert_eq!(d1, d2);
        assert!(d1.starts_with(DID_ORCID_PREFIX));
    }

    #[test]
    fn test_derive_did_different_orcid() {
        let d1 = derive_did("0000-0001-2345-6789");
        let d2 = derive_did("0000-0002-9876-5432");
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_derive_did_strips_url_prefix() {
        let d1 = derive_did(TEST_ORCID);
        let d2 = derive_did(&format!("https://orcid.org/{}", TEST_ORCID));
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_build_did_document() {
        let odid = build_did_document(TEST_ORCID);
        assert!(odid.did.starts_with(DID_ORCID_PREFIX));
        assert_eq!(odid.orcid_id, TEST_ORCID);
        assert_eq!(odid.did_document.id, odid.did);
        assert_eq!(odid.did_document.verification_methods.len(), 1);
        assert_eq!(odid.did_document.service.len(), 2);
        assert!(!odid.verified);
    }

    #[test]
    fn test_did_document_serialization() {
        let odid = build_did_document(TEST_ORCID);
        let json = serde_json::to_string_pretty(&odid).unwrap();
        assert!(json.contains("did:arkhe:orcid:"));
        assert!(json.contains("Ed25519VerificationKey2020"));
        assert!(json.contains("OrcidProfile"));
        assert!(json.contains("DesciNode"));
        // Round-trip
        let odid2: OrcidDID = serde_json::from_str(&json).unwrap();
        assert_eq!(odid.did, odid2.did);
    }

    #[test]
    fn test_attestation_roundtrip() {
        let att = create_attestation(
            "did:arkhe:authority-01",
            "did:arkhe:orcid:abc123",
            TEST_ORCID,
            24,
        );
        assert_eq!(att.attester_did, "did:arkhe:authority-01");
        assert_eq!(att.claim_type, "OrcidOwnership");
        assert!(verify_attestation(&att));
    }

    #[test]
    fn test_attestation_tampered_fails() {
        let mut att = create_attestation("did:a", "did:b", TEST_ORCID, 24);
        att.proof_hash = "tampered".into();
        assert!(!verify_attestation(&att));
    }

    #[test]
    fn test_orcid_profile_serialization() {
        let p = OrcidProfile {
            orcid_id: TEST_ORCID.into(),
            given_names: "João".into(),
            family_name: "Silva".into(),
            email: Some("joao@uni.br".into()),
            institution: Some("USP".into()),
            country: Some("BR".into()),
            works_count: 42,
            keywords: vec!["genomics".into(), "crispr".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let p2: OrcidProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p.works_count, p2.works_count);
        assert_eq!(p.keywords, p2.keywords);
    }
}
