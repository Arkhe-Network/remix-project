# Taproot Assets Authentication

## Model
To connect to `tapd` via gRPC, you need:
1. **TLS/SSL Certificate**: For encrypted communication.
2. **Macaroon**: For scoped authentication.

## Default Paths
- Linux: `~/.taproot-assets`
- macOS: `~/Library/Application Support/TaprootAssets`

## Rust Integration
The `cathedral-taproot-bridge` implements Macaroon loading and embeds it inside the gRPC request headers as `metadata::MetadataValue`.
