//! Escape regions for the ARKHE‑χ security manifold.
//!
//! Regions classify the severity of invariant violations using a monotonic
//! scale based on [`SystemState::violation_count`].

use serde::{Deserialize, Serialize};

/// Severity region of a system state relative to the safe envelope.
///
/// The classification is monotonic: more violations → more severe region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscapeRegion {
    /// All invariants satisfied — no threshold violated.
    Safe,
    /// Exactly 1 invariant violated — early warning.
    Warning,
    /// Exactly 2 invariants violated — degraded but recoverable.
    Boundary,
    /// 3–4 invariants violated — critical, likely unsafe.
    Continuum,
    /// 5+ invariants violated — outside safe envelope.
    Outside,
}
