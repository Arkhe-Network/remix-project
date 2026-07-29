//! Constitutional Blackboard pattern.
//!
//! I6 (Self-reference): Shared state across the tree.
//! I1 (Physical): All reads/writes are observable and measurable.
//!
//! The blackboard is the substrate-independent shared memory of the BT.
//! Concrete implementations provide thread-safety guarantees.

use std::any::Any;
use std::collections::HashMap;

/// Trait for constitutional blackboard implementations.
///
/// I3 (Substrate): This trait abstracts over the storage backend,
/// allowing the same BT to run with different blackboard implementations
/// (in-memory, double-buffered, persistent, etc.).
pub trait Blackboard {
    /// Retrieve a value by key.
    fn get(&self, key: &str) -> Option<&(dyn Any + Send)>;

    /// Store a value by key.
    fn set(&mut self, key: &str, value: Box<dyn Any + Send>);

    /// Check if a key exists.
    fn has(&self, key: &str) -> bool;

    /// Remove a key.
    fn remove(&mut self, key: &str) -> Option<Box<dyn Any + Send>>;
}

/// Standard in-memory blackboard.
///
/// Suitable for single-threaded or externally-synchronized contexts.
/// For concurrent contexts, use `arkhe_bt_atomic::DoubleBufferBlackboard`.
#[derive(Default)]
pub struct StandardBlackboard {
    data: HashMap<String, Box<dyn Any + Send>>,
}

impl StandardBlackboard {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl Blackboard for StandardBlackboard {
    fn get(&self, key: &str) -> Option<&(dyn Any + Send)> {
        self.data.get(key).map(|v| v.as_ref())
    }

    fn set(&mut self, key: &str, value: Box<dyn Any + Send>) {
        self.data.insert(key.to_string(), value);
    }

    fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    fn remove(&mut self, key: &str) -> Option<Box<dyn Any + Send>> {
        self.data.remove(key)
    }
}

/// Typed convenience methods for blackboards.
pub trait TypedBlackboard {
    fn get_typed<T: Any + Clone>(&self, key: &str) -> Option<T>;
    fn set_typed<T: Any + Send>(&mut self, key: &str, value: T);
}

impl<B: Blackboard + ?Sized> TypedBlackboard for B {
    fn get_typed<T: Any + Clone>(&self, key: &str) -> Option<T> {
        self.get(key)?.downcast_ref::<T>().cloned()
    }

    fn set_typed<T: Any + Send>(&mut self, key: &str, value: T) {
        self.set(key, Box::new(value));
    }
}
