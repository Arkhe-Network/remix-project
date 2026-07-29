//! # Arkhe BT Core
//!
//! Fundamental types for the Constitutional Behavior Tree architecture.
//! Replaces Python pseudocode with zero-cost Rust abstractions.

pub mod blackboard;
pub mod node;
pub mod status;

pub use blackboard::*;
pub use node::*;
pub use status::*;

/// Maximum constitutional tree depth (OA-BT-001 mitigation).
/// Bounds tick latency by limiting traversal depth.
pub const MAX_CONSTITUTIONAL_DEPTH: usize = 32;

/// Maximum tick duration in nanoseconds before violation logging.
pub const MAX_TICK_NS: u64 = 1_000_000; // 1ms
