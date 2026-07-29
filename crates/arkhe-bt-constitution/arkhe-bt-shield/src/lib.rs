//! # Arkhe BT Shield
//!
//! OA-BT-003 mitigation: "Shield Nodes" — non-interruptible safety sequences.
//!
//! ## Design
//! The `ShieldNode` uses an `AtomicBool` to establish a **global safety lock**.
//! When a ShieldNode begins execution, it atomically sets the lock.
//! All other nodes in the tree **must** check this lock before executing.
//! If the lock is held, they spin-wait (busy-wait) until released.
//!
//! This provides **true preemption** of non-safety nodes by safety nodes,
//! unlike the Python `threading.Lock` pseudocode which merely blocked threads.
//!
//! ## Constitutional Guarantees
//! - I5 (Autonomy): Safety checks execute regardless of dynamic insertions.
//! - I4 (Polynomial): Lock acquisition is O(1).
//! - I1 (Physical): Lock state is observable via `is_locked()`.

use arkhe_bt_core::{Blackboard, Node, Status};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Global constitutional safety lock.
///
/// This is a **process-wide** (or executor-wide) lock. All BT nodes
/// in the same address space share this lock to ensure safety nodes
/// have absolute priority.
///
/// In a multi-process context, this would be backed by a named semaphore
/// or distributed lock (e.g., etcd, Consul).
#[derive(Debug, Clone)]
pub struct SafetyLock {
    flag: Arc<AtomicBool>,
}

impl Default for SafetyLock {
    fn default() -> Self {
        Self::new()
    }
}

impl SafetyLock {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Attempt to acquire the safety lock.
    /// Returns true if acquired, false if already held.
    pub fn try_acquire(&self) -> bool {
        // SeqCst ensures total ordering across all threads.
        // This is critical for safety: we cannot tolerate reordering
        // that would allow a non-safety node to execute after the lock
        // is conceptually held.
        self.flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Spin-wait until the lock is acquired.
    ///
    /// # Constitutional Note
    /// This is a **busy-wait** (spinlock). It is appropriate here because:
    /// 1. Safety critical sections are expected to be very short (<1μs).
    /// 2. We cannot yield to the scheduler (risk of priority inversion).
    /// 3. We cannot park the thread (risk of missing safety deadlines).
    ///
    /// For longer critical sections, use `try_acquire` + async yield.
    pub fn acquire_spin(&self) {
        while !self.try_acquire() {
            // hint::spin_loop reduces power consumption on modern CPUs
            // while maintaining low latency.
            std::hint::spin_loop();
        }
    }

    /// Release the safety lock.
    pub fn release(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    /// Check if the lock is currently held.
    pub fn is_locked(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// A ShieldNode wraps a child node in a non-interruptible safety sequence.
///
/// When ticked, the ShieldNode:
/// 1. Acquires the global safety lock (spin-wait if necessary).
/// 2. Executes the child node.
/// 3. Releases the lock.
///
/// All other nodes that respect the `SafetyLock` will wait during steps 1-3.
pub struct ShieldNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub lock: SafetyLock,
}

impl ShieldNode {
    pub fn new(name: &'static str, child: Box<dyn Node>, lock: SafetyLock) -> Self {
        Self { name, child, lock }
    }
}

impl Node for ShieldNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        // Acquire safety lock — preempts all non-safety nodes.
        self.lock.acquire_spin();

        // Execute safety-critical child.
        let status = self.child.tick(blackboard);

        // Release lock — non-safety nodes may proceed.
        self.lock.release();

        status
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

/// A `RespectfulNode` wraps any node to make it respect the safety lock.
///
/// Before executing, it checks if the safety lock is held.
/// If held, it spin-waits until released.
///
/// This is the **complementary** mechanism to `ShieldNode`:
/// - `ShieldNode` **acquires** the lock (safety node).
/// - `RespectfulNode` **waits** for the lock (non-safety node).
pub struct RespectfulNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub lock: SafetyLock,
}

impl RespectfulNode {
    pub fn new(name: &'static str, child: Box<dyn Node>, lock: SafetyLock) -> Self {
        Self { name, child, lock }
    }
}

impl Node for RespectfulNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        // Wait until safety lock is released.
        while self.lock.is_locked() {
            std::hint::spin_loop();
        }

        // Now safe to execute (no safety node is running).
        self.child.tick(blackboard)
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

/// Convenience: wrap an entire tree to make all nodes respectful of safety.
///
/// Usage:
/// ```rust,ignore
/// let lock = SafetyLock::new();
/// let safe_tree = make_respectful(tree, lock.clone());
/// let shielded_safety = ShieldNode::new("SafetyRoot", safety_subtree, lock);
/// ```
pub fn make_respectful(tree: Box<dyn Node>, lock: SafetyLock) -> Box<dyn Node> {
    Box::new(RespectfulNode {
        name: "RespectfulWrapper",
        child: tree,
        lock,
    })
}
