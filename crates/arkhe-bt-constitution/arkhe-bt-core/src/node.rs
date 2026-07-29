//! Constitutional Behavior Tree nodes.
//!
//! Design principles:
//! - I3 (Substrate): Nodes are generic over Blackboard type.
//! - I4 (Polynomial): Traversal is O(N) where N = node count, bounded by MAX_CONSTITUTIONAL_DEPTH.
//! - I6 (Self-reference): Composite nodes can reference subtrees via `Box<dyn Node>`.

use crate::{Blackboard, Status};

/// A node in the Constitutional Behavior Tree.
///
/// I1 (Physical): `tick` returns a measurable status.
/// I2 (Falsifiability): Each node is independently testable.
pub trait Node: Send + Sync {
    /// Execute one tick of this node.
    ///
    /// # Arguments
    /// * `blackboard` - Shared mutable state (I6: Self-reference via shared state).
    ///
    /// # Returns
    /// * `Status` - The observable result of this tick (I1).
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status;

    /// Returns the number of direct children (for traversal metrics).
    fn child_count(&self) -> usize {
        0
    }
}

/// Action node: executes a specific task.
/// I1 (Physical): Performs a measurable action.
pub struct ActionNode<F>
where
    F: Fn(&mut dyn Blackboard) -> Status + Send + Sync,
{
    pub name: &'static str,
    pub action: F,
}

impl<F> Node for ActionNode<F>
where
    F: Fn(&mut dyn Blackboard) -> Status + Send + Sync,
{
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        (self.action)(blackboard)
    }
}

/// Condition node: checks a Boolean predicate.
/// I2 (Falsifiability): Returns true/false, directly testable.
pub struct ConditionNode<F>
where
    F: Fn(&dyn Blackboard) -> bool + Send + Sync,
{
    pub name: &'static str,
    pub predicate: F,
}

impl<F> Node for ConditionNode<F>
where
    F: Fn(&dyn Blackboard) -> bool + Send + Sync,
{
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        if (self.predicate)(blackboard) {
            Status::Success
        } else {
            Status::Failure
        }
    }
}

/// Sequence node (AND): all children must succeed.
/// I4 (Polynomial): Linear execution, O(children) per tick.
pub struct SequenceNode {
    pub name: &'static str,
    pub children: Vec<Box<dyn Node>>,
}

impl SequenceNode {
    pub fn new(name: &'static str, children: Vec<Box<dyn Node>>) -> Self {
        Self { name, children }
    }
}

impl Node for SequenceNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        for child in &self.children {
            match child.tick(blackboard) {
                Status::Success => continue,
                Status::Failure => return Status::Failure,
                Status::Running => return Status::Running,
            }
        }
        Status::Success
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
}

/// Fallback node (OR / Selector): succeeds if any child succeeds.
/// I5 (Autonomy): Self-recovery from failure via fallback paths.
pub struct FallbackNode {
    pub name: &'static str,
    pub children: Vec<Box<dyn Node>>,
}

impl FallbackNode {
    pub fn new(name: &'static str, children: Vec<Box<dyn Node>>) -> Self {
        Self { name, children }
    }
}

impl Node for FallbackNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        for child in &self.children {
            match child.tick(blackboard) {
                Status::Success => return Status::Success,
                Status::Failure => continue,
                Status::Running => return Status::Running,
            }
        }
        Status::Failure
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
}

/// Parallel node: all children execute conceptually in parallel.
/// I3 (Substrate): Concurrent execution abstraction.
///
/// Note: This is a logical parallel node. True parallelism requires
/// an async executor (see arkhe-bt-shield for priority-aware execution).
pub struct ParallelNode {
    pub name: &'static str,
    pub children: Vec<Box<dyn Node>>,
    pub success_threshold: usize, // Minimum children that must succeed
}

impl ParallelNode {
    pub fn new(name: &'static str, children: Vec<Box<dyn Node>>, success_threshold: usize) -> Self {
        Self {
            name,
            children,
            success_threshold,
        }
    }
}

impl Node for ParallelNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        let mut success_count = 0;

        for child in &self.children {
            match child.tick(blackboard) {
                Status::Success => success_count += 1,
                Status::Failure => {} // Count could be used if needed
                Status::Running => return Status::Running,
            }
        }

        if success_count >= self.success_threshold {
            Status::Success
        } else {
            Status::Failure
        }
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }
}

/// Decorator node: modifies child behavior.
/// I6 (Self-reference): Meta-behavior over child execution.
pub struct DecoratorNode<F>
where
    F: Fn(Status) -> Status + Send + Sync,
{
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub transform: F,
}

impl<F> Node for DecoratorNode<F>
where
    F: Fn(Status) -> Status + Send + Sync,
{
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        let child_status = self.child.tick(blackboard);
        (self.transform)(child_status)
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}

/// Inverter decorator: flips Success <-> Failure.
pub fn inverter(child: Box<dyn Node>) -> DecoratorNode<impl Fn(Status) -> Status + Send + Sync> {
    DecoratorNode {
        name: "Inverter",
        child,
        transform: |status| match status {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Running => Status::Running,
        },
    }
}

/// Retry decorator: retries child up to N times on Failure.
pub struct RetryNode {
    pub name: &'static str,
    pub child: Box<dyn Node>,
    pub max_attempts: usize,
}

impl RetryNode {
    pub fn new(name: &'static str, child: Box<dyn Node>, max_attempts: usize) -> Self {
        Self {
            name,
            child,
            max_attempts,
        }
    }
}

impl Node for RetryNode {
    fn tick(&self, blackboard: &mut dyn Blackboard) -> Status {
        for _ in 0..self.max_attempts {
            match self.child.tick(blackboard) {
                Status::Success => return Status::Success,
                Status::Running => return Status::Running,
                Status::Failure => continue,
            }
        }
        Status::Failure
    }

    fn child_count(&self) -> usize {
        1 + self.child.child_count()
    }
}
