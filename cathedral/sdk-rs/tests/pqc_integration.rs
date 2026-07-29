use cathedral_sdk_rs::crypto::{SigningKeyWrapper, VerifyingKeyWrapper, CryptoFactory};
use common::crypto_config::{CryptoConfig, SignatureAlgorithm};

#[test]
fn test_mldsa_integration() {
    let mut config = CryptoConfig::default();
    config.signature_algorithm = SignatureAlgorithm::MlDsa;
    config.dual_stack_mode = true;
    config.fallback_signature_algorithm = Some(SignatureAlgorithm::Ed25519);

    let factory = CryptoFactory::new(config);

    // Geração de chave
    let sk = factory.generate_signing_key().expect("Falha ao gerar chave ML-DSA");
    assert_eq!(sk.algorithm(), SignatureAlgorithm::MlDsa);

    // Assinatura
    let msg = b"test message for integration";
    let sig = factory.sign(&sk, msg).expect("Falha ao assinar com ML-DSA");
    assert!(!sig.is_empty());

    // Verificação (a chave de verificação teria que ser derivada ou armazenada, vamos simular isso usando o teste de dual_stack)
}
