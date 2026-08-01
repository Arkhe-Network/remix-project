# Architecture

This document describes the high-level architecture of the project, including technological choices, system constraints, and the overall vision.

## High-Level Vision
- Modular structure
- Use of Rust and Lean 4
- AI and Agentic development orientation
- High constraints on data privacy and memory leakage

## Technological Choices
- **Languages:** Rust (performance/safety), Lean 4 (formal verification)
- **Tooling:** Cargo, Lake (Lean), Make
- **AI Tooling:** Integration with various LLMs (Claude, Gemini, etc.) and agent frameworks.

## Constraints
- **Security:** Zero Token Architecture (ZTA), stringent isolation of credentials.
- **Performance:** Optimized loops, avoidance of deadlocks, deterministic execution.
- **Verification:** Formal methods used for critical substrate properties.
