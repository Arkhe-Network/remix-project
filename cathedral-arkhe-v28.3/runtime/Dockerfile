FROM rust:1.78-slim-bookworm AS builder

WORKDIR /app
# Create a dummy project structure to build the orchestrator
RUN cargo new --bin orchestrator
WORKDIR /app/orchestrator
COPY orchestrator/src ./src
# Dummy Cargo.toml for building
RUN cat << 'TOML' > Cargo.toml
[package]
name = "orchestrator"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tokio = { version = "1.0", features = ["full"] }
TOML

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/orchestrator/target/release/orchestrator /usr/local/bin/cathedral-agent-runtime
COPY agent /app/agent
COPY core /app/core

EXPOSE 8001
CMD ["cathedral-agent-runtime", "--config", "/app/agent/config.yaml", "--manifest", "/app/core/model/manifest.json"]
