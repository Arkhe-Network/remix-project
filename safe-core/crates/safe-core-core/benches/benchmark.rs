// crates/safe-core-core/benches/benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};
use safe_core_core::blake3_hash;

fn bench_hash(c: &mut Criterion) {
    let data = vec![0u8; 1024];
    c.bench_function("blake3 1KB", |b| b.iter(|| blake3_hash(&data)));
}

criterion_group!(benches, bench_hash);
criterion_main!(benches);
