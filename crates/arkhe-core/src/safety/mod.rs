// crates/arkhe-core/src/safety/mod.rs
//! ARKHE-χ Safety Module — Fase 1 + Fase 2 + Fase 3 + Fase 4 (PATCHED)

pub mod symmetry_generator;

// Fase 3
pub mod escape_detector;
pub mod topological_circuit_breaker;

pub use symmetry_generator::*;
pub use escape_detector::*;
pub use topological_circuit_breaker::*;
