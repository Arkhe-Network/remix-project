cargo build --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
