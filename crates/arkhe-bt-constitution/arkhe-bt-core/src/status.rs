//! Constitutional execution status.
//!
//! I1 (Physical): Every tick returns a measurable, observable status.

/// The result of a single BT tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Status {
    /// Node completed successfully.
    Success,
    /// Node failed (may trigger fallback).
    Failure,
    /// Node is still executing (async/reentrant contexts).
    Running,
}

impl Status {
    /// Returns true if the status represents completion (Success or Failure).
    /// I2 (Falsifiability): Terminal states are unambiguous.
    pub fn is_terminal(self) -> bool {
        matches!(self, Status::Success | Status::Failure)
    }
}
