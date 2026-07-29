# ARKHE v7.0 — Cognitive Substrate

This directory contains the formal specifications and verification models for the ARKHE v7.0 Cognitive Substrate.

## Specifications

- **TLA+**: Models state, transitions, and invariants (IC1–IC10) in `specification/tla/`.
- **Coq**: Formal proofs of invariants in `specification/coq/`.

## Verification

- **Kani**: Model checking for Rust implementations in `verification/kani/`.
- **Proptest**: Property-based testing in `verification/proptest/`.

## Documentation

- `docs/invariants.md`: Description of IC1–IC10.
- `docs/verification_matrix.md`: Status of each invariant across verification methods.