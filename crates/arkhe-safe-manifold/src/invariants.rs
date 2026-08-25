//! Constitutional invariants I‑01 through I‑08 for ARKHE‑χ.
//!
//! This module defines the static configuration [`SystemConfig`], the dynamic
//! state [`SystemState`], and the [`Invariant`] trait for extensibility.
//!
//! # The Eight Invariants
//!
//! | ID | Predicate | Description |
//! |----|-----------|-------------|
//! | I-01 | `token_budget >= 0` | Budget must be non-negative |
//! | I-02 | `agent_count <= 10` | Agent cap |
//! | I-03 | `sandbox_fuel > 0` | Sandbox must have fuel |
//! | I-04 | `entropy_bits >= 256` | Minimum entropy |
//! | I-05 | `pii_scrubbed == true` | PII must be scrubbed |
//! | I-06 | `signature_valid == true` | Signature must be valid |
//! | I-07 | `rate_limit_remaining > 0` | Rate limit must remain |
//! | I-08 | `model_capability >= 2^32` | Minimum model capability |

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in the SafeManifold.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ManifoldError {
    /// One or more constitutional invariants are violated.
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
    /// A projection or computation produced an invalid result.
    #[error("Projection error: {0}")]
    ProjectionError(String),
}

/// Static system configuration (thresholds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Maximum token budget.
    pub max_tokens: i64,
    /// Maximum number of concurrent agents.
    pub max_agents: u32,
    /// Maximum sandbox fuel.
    pub max_sandbox_fuel: i64,
    /// Minimum entropy bits.
    pub min_entropy: u32,
    /// Maximum rate limit.
    pub max_rate_limit: i64,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_tokens: 10000,
            max_agents: 10,
            max_sandbox_fuel: 1000,
            min_entropy: 256,
            max_rate_limit: 1000,
        }
    }
}

/// Dynamic system state.
///
/// **Warning**: This struct can represent *invalid* states. For a type that
/// guarantees invariants at construction time, use [`SafeState`](crate::safe_manifold::SafeState).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemState {
    /// Remaining token budget (I-01: must be >= 0).
    pub token_budget: i64,
    /// Number of active agents (I-02: must be <= 10).
    pub agent_count: u32,
    /// Remaining sandbox fuel (I-03: must be > 0).
    pub sandbox_fuel: i64,
    /// Entropy bits available (I-04: must be >= 256).
    pub entropy_bits: u32,
    /// Whether PII has been scrubbed (I-05: must be true).
    pub pii_scrubbed: bool,
    /// Whether the state signature is valid (I-06: must be true).
    pub signature_valid: bool,
    /// Remaining rate limit (I-07: must be > 0).
    pub rate_limit_remaining: i64,
    /// Model capability bits (I-08: must be >= 2^32).
    pub model_capability: u64,
    /// Associated configuration.
    pub config: SystemConfig,
}

impl SystemState {
    /// Construct a safe-by-construction state (all invariants satisfied).
    ///
    /// # Example
    /// ```
    /// use arkhe_safe_manifold::*;
    /// let config = SystemConfig::default();
    /// let state = SystemState::safe(config);
    /// assert!(state.check_all());
    /// ```
    pub fn safe(config: SystemConfig) -> Self {
        Self {
            token_budget: config.max_tokens,
            agent_count: 5,
            sandbox_fuel: config.max_sandbox_fuel,
            entropy_bits: config.min_entropy.saturating_add(256),
            pii_scrubbed: true,
            signature_valid: true,
            rate_limit_remaining: config.max_rate_limit,
            model_capability: u64::MAX,
            config,
        }
    }

    // ── Individual invariant checks ─────────────────────────────────────────

    /// I-01: token_budget >= 0
    pub fn check_i01(&self) -> bool { self.token_budget >= 0 }

    /// I-02: agent_count <= 10
    pub fn check_i02(&self) -> bool { self.agent_count <= 10 }

    /// I-03: sandbox_fuel > 0
    pub fn check_i03(&self) -> bool { self.sandbox_fuel > 0 }

    /// I-04: entropy_bits >= 256
    pub fn check_i04(&self) -> bool { self.entropy_bits >= 256 }

    /// I-05: pii_scrubbed == true
    pub fn check_i05(&self) -> bool { self.pii_scrubbed }

    /// I-06: signature_valid == true
    pub fn check_i06(&self) -> bool { self.signature_valid }

    /// I-07: rate_limit_remaining > 0
    pub fn check_i07(&self) -> bool { self.rate_limit_remaining > 0 }

    /// I-08: model_capability >= 2^32
    pub fn check_i08(&self) -> bool { self.model_capability >= 4294967296 }

    /// Check all invariants (I-01 through I-08).
    ///
    /// # Example
    /// ```
    /// use arkhe_safe_manifold::*;
    /// let state = SystemState::safe(SystemConfig::default());
    /// assert!(state.check_all());
    /// ```
    pub fn check_all(&self) -> bool {
        self.check_i01() && self.check_i02() && self.check_i03() &&
        self.check_i04() && self.check_i05() && self.check_i06() &&
        self.check_i07() && self.check_i08()
    }

    /// Count how many invariants are violated.
    ///
    /// # Example
    /// ```
    /// use arkhe_safe_manifold::*;
    /// let mut state = SystemState::safe(SystemConfig::default());
    /// state.token_budget = -1;
    /// assert_eq!(state.violation_count(), 1);
    /// ```
    pub fn violation_count(&self) -> u32 {
        let mut v = 0;
        if !self.check_i01() { v += 1; }
        if !self.check_i02() { v += 1; }
        if !self.check_i03() { v += 1; }
        if !self.check_i04() { v += 1; }
        if !self.check_i05() { v += 1; }
        if !self.check_i06() { v += 1; }
        if !self.check_i07() { v += 1; }
        if !self.check_i08() { v += 1; }
        v
    }
}

/// Extensible invariant trait.
///
/// Implement this trait to add custom invariants beyond I-01..I-08.
pub trait Invariant {
    /// Unique identifier for this invariant.
    fn id(&self) -> &'static str;
    /// Check whether the given state satisfies this invariant.
    fn check(&self, state: &SystemState) -> bool;
}

macro_rules! def_invariant {
    ($name:ident, $id:expr, $method:ident) => {
        /// Built-in invariant checker.
        pub struct $name;
        impl Invariant for $name {
            fn id(&self) -> &'static str { $id }
            fn check(&self, state: &SystemState) -> bool { state.$method() }
        }
    };
}

def_invariant!(I01, "I-01", check_i01);
def_invariant!(I02, "I-02", check_i02);
def_invariant!(I03, "I-03", check_i03);
def_invariant!(I04, "I-04", check_i04);
def_invariant!(I05, "I-05", check_i05);
def_invariant!(I06, "I-06", check_i06);
def_invariant!(I07, "I-07", check_i07);
def_invariant!(I08, "I-08", check_i08);
