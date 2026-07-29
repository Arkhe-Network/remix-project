use criterion::{criterion_group, criterion_main, Criterion};
use cathedral_sdk_rs::crypto::{SigningKeyWrapper, VerifyingKeyWrapper, CryptoFactory};
use common::crypto_config::{CryptoConfig, SignatureAlgorithm};
use pqcrypto_dilithium::dilithium3::keypair;

fn bench_pqc_signatures(c: &mut Criterion) {
    let config_ed25519 = CryptoConfig {
        signature_algorithm: SignatureAlgorithm::Ed25519,
        ..Default::default()
    };
    let config_mldsa = CryptoConfig {
        signature_algorithm: SignatureAlgorithm::MlDsa,
        ..Default::default()
    };

    let factory_ed25519 = CryptoFactory::new(config_ed25519);
    let factory_mldsa = CryptoFactory::new(config_mldsa);

    let sk_ed25519 = factory_ed25519.generate_signing_key().unwrap();
    let sk_mldsa = factory_mldsa.generate_signing_key().unwrap();

    let msg = vec![0u8; 1024]; // mensagem de 1KB

    let mut group = c.benchmark_group("signature_generation");
    group.bench_function("Ed25519", |b| {
        b.iter(|| sk_ed25519.sign(&msg).unwrap())
    });
    group.bench_function("ML-DSA", |b| {
        b.iter(|| sk_mldsa.sign(&msg).unwrap())
    });
    group.finish();

    // Verificação
    let vk_ed25519 = match &sk_ed25519 {
        SigningKeyWrapper::Ed25519(sk) => VerifyingKeyWrapper::Ed25519(sk.verifying_key()),
        _ => unreachable!(),
    };

    // We can't generate a new keypair and expect the signatures from the old keypair to match
    // So let's extract the public key correctly from the existing private key? We don't have an API to do this in dilithium directly easily
    // So let's generate a new keypair specifically for verification benchmarking to ensure it matches
    let (mldsa_pk, mldsa_sk) = keypair();
    let mldsa_verifying_key = VerifyingKeyWrapper::MlDsa(mldsa_pk);
    let mldsa_signing_key = SigningKeyWrapper::MlDsa(mldsa_sk);

    let sig_ed25519 = sk_ed25519.sign(&msg).unwrap();
    let sig_mldsa = mldsa_signing_key.sign(&msg).unwrap();

    let mut group = c.benchmark_group("signature_verification");
    group.bench_function("Ed25519", |b| {
        b.iter(|| vk_ed25519.verify(&msg, &sig_ed25519).unwrap())
    });
    group.bench_function("ML-DSA", |b| {
        b.iter(|| mldsa_verifying_key.verify(&msg, &sig_mldsa).unwrap())
    });
    group.finish();

    // Tamanho das chaves e assinaturas
    println!("Tamanhos:");
    println!("  Ed25519 chave pública: {} B", vk_ed25519.to_bytes().len());
    println!("  Ed25519 assinatura: {} B", sig_ed25519.len());
    println!("  ML-DSA chave pública: {} B", mldsa_verifying_key.to_bytes().len());
    println!("  ML-DSA assinatura: {} B", sig_mldsa.len());
}

criterion_group!(benches, bench_pqc_signatures);
criterion_main!(benches);
