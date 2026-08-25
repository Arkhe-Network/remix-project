//! ARKHE‑χ SafeManifold — Security State Projection
//!
//! This crate projects system security states into a canonical equivalence-class
//! space ("manifold"). Mathematical names (Abel-Jacobi, Torelli, Néron) are
//! **metaphors** — they evoke structural intuitions from algebraic geometry but
//! do NOT imply rigorous implementation of those mathematical objects.
//!
//! # Quick Start
//!
//! ```
//! use arkhe_safe_manifold::*;
//!
//! let config = SystemConfig::default();
//! let manifold = SafeManifold::from_config(config.clone());
//! let state = SystemState::safe(config);
//!
//! // Project onto the manifold
//! let point = manifold.embed_state(&state);
//! assert!(!point.on_theta);
//!
//! // Enforce invariants via graceful degradation
//! let degraded = manifold.neron_model(&state);
//! assert!(degraded.check_all());
//! ```
//!
//! # SafeState — Parse, Don't Validate
//!
//! For production code, prefer [`SafeState`] which guarantees invariants at
//! construction time:
//!
//! ```
//! use arkhe_safe_manifold::*;
//!
//! let config = SystemConfig::default();
//! let state = SystemState::safe(config);
//! let safe = SafeState::new(state).unwrap();
//! // `safe` is guaranteed to satisfy all invariants I-01..I-08
//! ```

pub mod safe_manifold;
pub mod abel_jacobi;
pub mod invariants;
pub mod prolog_bridge;
pub mod escape_region;

pub use safe_manifold::{SafeManifold, ManifoldPoint, EscapeThresholds, ManifoldProfile, SafeState};
pub use abel_jacobi::{embed_state, is_within_manifold, observer_defect, collision_detected, torelli_equivalence};
pub use invariants::{SystemState, SystemConfig, Invariant, I01, I02, I03, I04, I05, I06, I07, I08, ManifoldError};
pub use escape_region::EscapeRegion;
