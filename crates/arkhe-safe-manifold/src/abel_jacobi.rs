//! Canonical projection functions ("Abel-Jacobi map" metaphor).

use crate::safe_manifold::{SafeManifold, ManifoldPoint, ManifoldProfile};
use crate::invariants::{SystemState, SystemConfig};

pub fn embed_state(state: &SystemState, config: &SystemConfig) -> ManifoldPoint {
    let manifold = SafeManifold::from_config(config.clone());
    manifold.embed_state(state)
}

pub fn is_within_manifold(point: &ManifoldPoint) -> bool {
    !point.on_theta
}

pub fn observer_defect(ideal: &SystemState, actual: &SystemState, config: &SystemConfig) -> f64 {
    let manifold = SafeManifold::from_config(config.clone());
    manifold.compute_observer_defect(ideal, actual)
}

pub fn collision_detected(s1: &SystemState, s2: &SystemState, config: &SystemConfig) -> bool {
    let manifold = SafeManifold::from_config(config.clone());
    manifold.collision_detected(s1, s2)
}

pub fn torelli_equivalence(p1: &ManifoldProfile, p2: &ManifoldProfile) -> bool {
    p1 == p2
}
