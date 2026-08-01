# Testing Strategy

This document outlines the testing strategy, tools, and requirements for the project.

## Testing Pyramid
- **Unit Tests:** `cargo test` for Rust, Lean theorem proofs for formalized segments.
- **Integration Tests:** Cross-boundary interactions (e.g., gRPC, external APIs).
- **End-to-End (E2E) Tests:** Verifying the full flow, often integrating with UI or full system pipelines.

## Tooling
- `cargo test` for Rust code.
- `criterion` for benchmarking.
- Specific linters and formatters.

## Requirements
- Maintain high code coverage.
- All new features must be accompanied by relevant tests.
