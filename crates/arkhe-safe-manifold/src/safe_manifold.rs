//! SafeManifold — Security projection space.
//!
//! **Metaphor disclaimer**: Names like "Jacobiana", "Abel-Jacobi", "Theta",
//! "Néron", and "Torelli" are used as structural metaphors. The code does NOT
//! implement complex tori, holomorphic maps, or minimal models over DVRs.
//! It projects security states into equivalence classes using bounded arithmetic.

use serde::{Deserialize, Serialize};
use crate::invariants::{SystemState, SystemConfig, ManifoldError};
use crate::escape_region::EscapeRegion;

/// Thresholds defining the boundary between safe and unsafe regions.
///
/// These values are used by [`SafeManifold::is_on_theta`] to determine whether
/// a state lies on the metaphorical "Theta divisor".
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EscapeThresholds {
    /// Token budget threshold (default: 5000).
    pub token_theta: i64,
    /// Agent count threshold (default: 8).
    pub agent_theta: u32,
    /// Sandbox fuel threshold (default: 500).
    pub fuel_theta: i64,
    /// Entropy bits threshold (default: 384).
    pub entropy_theta: u32,
    /// Rate limit threshold (default: 500).
    pub rate_theta: i64,
}

impl Default for EscapeThresholds {
    fn default() -> Self {
        Self {
            token_theta: 5000,
            agent_theta: 8,
            fuel_theta: 500,
            entropy_theta: 384,
            rate_theta: 500,
        }
    }
}

/// A point on the SafeManifold — canonical coordinates (equivalence class).
///
/// Two distinct states may map to the same `ManifoldPoint` by design
/// (this is a many-to-one projection, analogous to a hash collision).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifoldPoint {
    /// Token budget equivalence class (modulo max_tokens).
    pub token_class: u64,
    /// Agent count equivalence class (clamped to max_agents).
    pub agent_class: u32,
    /// Sandbox fuel equivalence class (modulo max_fuel).
    pub fuel_class: u64,
    /// Entropy bits equivalence class (clamped to min_entropy).
    pub entropy_class: u32,
    /// Rate limit equivalence class (modulo max_rate).
    pub rate_class: u64,
    /// Whether the state lies on the Theta boundary (i.e., has violations).
    pub on_theta: bool,
}

/// Behavioural profile of an agent.
///
/// Used by [`SafeManifold::manifold_profile`] and
/// [`SafeManifold::torelli_equivalence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifoldProfile {
    /// Number of agents.
    pub agents: u32,
    /// Entropy bits.
    pub entropy: u32,
    /// Rate limit remaining.
    pub rate: i64,
}

/// A [`SystemState`] that is guaranteed to satisfy all invariants I-01..I-08.
///
/// This type follows the "Parse, Don't Validate" pattern: invariants are
/// checked at construction time, and downstream code can rely on them without
/// re-checking.
///
/// # Example
/// ```
/// use arkhe_safe_manifold::*;
///
/// let config = SystemConfig::default();
/// let state = SystemState::safe(config);
/// let safe = SafeState::new(state).unwrap();
///
/// // SafeState always satisfies check_all()
/// assert!(safe.as_inner().check_all());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeState(SystemState);

impl SafeState {
    /// Attempt to construct a `SafeState` from a raw [`SystemState`].
    ///
    /// Returns [`ManifoldError::InvariantViolation`] if any invariant is violated.
    ///
    /// # Example
    /// ```
    /// use arkhe_safe_manifold::*;
    ///
    /// let mut state = SystemState::safe(SystemConfig::default());
    /// state.token_budget = -1;
    ///
    /// assert!(SafeState::new(state).is_err());
    /// ```
    pub fn new(state: SystemState) -> Result<Self, ManifoldError> {
        if state.check_all() {
            Ok(Self(state))
        } else {
            Err(ManifoldError::InvariantViolation(format!(
                "State violates {} invariants",
                state.violation_count()
            )))
        }
    }

    /// Consume the `SafeState` and return the inner [`SystemState`].
    pub fn into_inner(self) -> SystemState {
        self.0
    }

    /// Borrow the inner [`SystemState`].
    pub fn as_inner(&self) -> &SystemState {
        &self.0
    }

    /// Create a `SafeState` from the default safe configuration.
    ///
    /// This is infallible because [`SystemState::safe`] satisfies all invariants.
    pub fn default_safe() -> Self {
        let config = SystemConfig::default();
        Self(SystemState::safe(config))
    }
}

impl AsRef<SystemState> for SafeState {
    fn as_ref(&self) -> &SystemState {
        &self.0
    }
}

/// The SafeManifold — space of all possible safe configurations.
///
/// # Example
/// ```
/// use arkhe_safe_manifold::*;
///
/// let config = SystemConfig::default();
/// let manifold = SafeManifold::from_config(config.clone());
/// let state = SystemState::safe(config);
/// let point = manifold.embed_state(&state);
///
/// assert!(!point.on_theta);
/// ```
#[derive(Debug, Clone)]
pub struct SafeManifold {
    /// Maximum token budget.
    pub max_tokens: i64,
    /// Maximum number of agents.
    pub max_agents: u32,
    /// Maximum sandbox fuel.
    pub max_fuel: i64,
    /// Minimum entropy bits.
    pub min_entropy: u32,
    /// Maximum rate limit.
    pub max_rate: i64,
    /// Thresholds for Theta-boundary detection.
    pub theta_thresholds: EscapeThresholds,
    /// Associated system configuration.
    pub config: SystemConfig,
}

impl Default for SafeManifold {
    fn default() -> Self { Self::new() }
}

impl SafeManifold {
    /// Create the default manifold with DLCMAI-4V thresholds.
    pub fn new() -> Self {
        Self {
            max_tokens: 10000,
            max_agents: 10,
            max_fuel: 1000,
            min_entropy: 256,
            max_rate: 1000,
            theta_thresholds: EscapeThresholds::default(),
            config: SystemConfig::default(),
        }
    }

    /// Create a manifold from an explicit configuration.
    ///
    /// # Example
    /// ```
    /// use arkhe_safe_manifold::*;
    /// let config = SystemConfig::default();
    /// let manifold = SafeManifold::from_config(config);
    /// assert_eq!(manifold.max_tokens, 10000);
    /// ```
    pub fn from_config(config: SystemConfig) -> Self {
        Self {
            max_tokens: config.max_tokens,
            max_agents: config.max_agents,
            max_fuel: config.max_sandbox_fuel,
            min_entropy: config.min_entropy,
            max_rate: config.max_rate_limit,
            theta_thresholds: EscapeThresholds::default(),
            config,
        }
    }

    /// Canonical projection of a state onto the manifold.
    ///
    /// **PRECONDITION**: `state.check_all()` should ideally be true. If invariants
    /// are violated, the projection still succeeds but `on_theta` will be set.
    ///
    /// # Example
    /// ```
    /// use arkhe_safe_manifold::*;
    ///
    /// let manifold = SafeManifold::new();
    /// let state = SystemState::safe(manifold.config.clone());
    /// let point = manifold.embed_state(&state);
    ///
    /// assert!(!point.on_theta);
    /// ```
    pub fn embed_state(&self, state: &SystemState) -> ManifoldPoint {
        let token_class = state.token_budget.rem_euclid(self.max_tokens) as u64;
        let agent_class = state.agent_count.min(self.max_agents);
        let fuel_class = state.sandbox_fuel.rem_euclid(self.max_fuel) as u64;
        let entropy_class = state.entropy_bits.max(self.min_entropy);
        let rate_class = state.rate_limit_remaining.rem_euclid(self.max_rate) as u64;
        let on_theta = self.is_on_theta(state);

        ManifoldPoint {
            token_class,
            agent_class,
            fuel_class,
            entropy_class,
            rate_class,
            on_theta,
        }
    }

    /// Check whether the state lies on the Theta boundary.
    ///
    /// A state is "on theta" if at least one threshold is violated.
    fn is_on_theta(&self, state: &SystemState) -> bool {
        let t = &self.theta_thresholds;
        let mut theta_count = 0;
        if state.token_budget < t.token_theta { theta_count += 1; }
        if state.agent_count > t.agent_theta  { theta_count += 1; }
        if state.sandbox_fuel < t.fuel_theta  { theta_count += 1; }
        if state.entropy_bits < t.entropy_theta { theta_count += 1; }
        if state.rate_limit_remaining < t.rate_theta { theta_count += 1; }
        theta_count > 0
    }

    /// Compute the **normalized** safety-distance score between ideal and actual.
    pub fn compute_observer_defect(&self, ideal: &SystemState, actual: &SystemState) -> f64 {
        let norm = |v: f64, max: f64| -> f64 {
            if max <= 0.0 { 0.0 } else { (v / max).clamp(-1.0, 1.0) }
        };

        // 6 dimensões contínuas normalizadas por range
        let d1 = norm((ideal.token_budget - actual.token_budget) as f64, self.max_tokens as f64);
        let d2 = norm((ideal.agent_count as i64 - actual.agent_count as i64) as f64, self.max_agents as f64);
        let d3 = norm((ideal.sandbox_fuel - actual.sandbox_fuel) as f64, self.max_fuel as f64);
        let d4 = norm((ideal.entropy_bits as i64 - actual.entropy_bits as i64) as f64,
                      (self.min_entropy * 4).max(1024) as f64);
        let d5 = norm((ideal.rate_limit_remaining - actual.rate_limit_remaining) as f64, self.max_rate as f64);

        // ✅ R3a: model_capability em escala log (sem overflow)
        let d6 = norm(
            (ideal.model_capability as f64).ln_1p() - (actual.model_capability as f64).ln_1p(),
            44.36  // ln(2^64) ≈ 44.36
        );

        // Booleanos: penalidade binária
        let d7: f64 = if ideal.pii_scrubbed != actual.pii_scrubbed { 1.0 } else { 0.0 };
        let d8: f64 = if ideal.signature_valid != actual.signature_valid { 1.0 } else { 0.0 };

        // ✅ N1: Stress simétrico (max de ambos os lados)
        let token_stress  = ideal.token_budget < (self.max_tokens as f64 * 0.2) as i64
                         || actual.token_budget < (self.max_tokens as f64 * 0.2) as i64;
        let agent_stress  = ideal.agent_count > (self.max_agents as f64 * 0.8) as u32
                         || actual.agent_count > (self.max_agents as f64 * 0.8) as u32;
        let fuel_stress   = ideal.sandbox_fuel < (self.max_fuel as f64 * 0.2) as i64
                         || actual.sandbox_fuel < (self.max_fuel as f64 * 0.2) as i64;
        let entropy_stress = ideal.entropy_bits < 512 || actual.entropy_bits < 512;
        let rate_stress   = ideal.rate_limit_remaining < (self.max_rate as f64 * 0.2) as i64
                         || actual.rate_limit_remaining < (self.max_rate as f64 * 0.2) as i64;

        // ✅ R3b+R3c: Pesos somam 1.0, booleanos têm prioridade sobre contínuos
        let token_weight   = if token_stress  { 0.10 } else { 0.08 };
        let agent_weight   = if agent_stress  { 0.12 } else { 0.08 };
        let fuel_weight    = if fuel_stress   { 0.10 } else { 0.08 };
        let entropy_weight = if entropy_stress { 0.12 } else { 0.08 };
        let rate_weight    = if rate_stress   { 0.05 } else { 0.03 };
        let model_weight   = 0.05;  // fixo, range logarítmico já comprime
        let pii_weight     = 0.20;  // ✅ MAIOR que qualquer contínuo
        let sig_weight     = 0.20;  // ✅ MAIOR que qualquer contínuo

        (token_weight * d1.powi(2)
         + agent_weight * d2.powi(2)
         + fuel_weight * d3.powi(2)
         + entropy_weight * d4.powi(2)
         + rate_weight * d5.powi(2)
         + model_weight * d6.powi(2)
         + pii_weight * d7.powi(2)
         + sig_weight * d8.powi(2))
        .sqrt()
    }

    /// True if the defect is effectively zero (state matches ideal).
    ///
    /// Uses a tolerance of `1.0e-10` to account for floating-point rounding.
    pub fn is_automorphism(&self, ideal: &SystemState, actual: &SystemState) -> bool {
        self.compute_observer_defect(ideal, actual) < 1.0e-10
    }

    /// Detect a **projection collision**: two distinct states mapping to the
    /// same equivalence class.
    ///
    /// **Important**: This is an EXPECTED property of the many-to-one
    /// projection (like a hash collision), NOT a security vulnerability.
    pub fn collision_detected(&self, s1: &SystemState, s2: &SystemState) -> bool {
        s1 != s2 && self.embed_state(s1) == self.embed_state(s2)
    }

    /// Classify the escape region using violation severity.
    pub fn classify_escape(&self, state: &SystemState) -> EscapeRegion {
        match state.violation_count() {
            0 => EscapeRegion::Safe,
            1 => EscapeRegion::Warning,
            2 => EscapeRegion::Boundary,
            3 | 4 => EscapeRegion::Continuum,
            _ => EscapeRegion::Outside,
        }
    }

    /// Graceful degradation: clamp fields and enforce invariants.
    pub fn neron_model(&self, state: &SystemState) -> SystemState {
        let mut degraded = state.clone();
        degraded.token_budget = degraded.token_budget.max(0).min(self.max_tokens);
        degraded.agent_count = degraded.agent_count.min(self.max_agents);
        degraded.sandbox_fuel = degraded.sandbox_fuel.max(1).min(self.max_fuel);
        degraded.entropy_bits = degraded.entropy_bits.max(self.min_entropy);
        degraded.rate_limit_remaining = degraded.rate_limit_remaining.max(1).min(self.max_rate);
        degraded.pii_scrubbed = true;        // enforce I-05
        degraded.signature_valid = true;     // enforce I-06
        degraded.model_capability = degraded.model_capability.max(4294967296);
        degraded.config = self.config.clone();
        degraded
    }

    /// neron_model_checked rejeita antes de clamarpar para campos booleanos duros
    pub fn neron_model_checked(&self, state: &SystemState) -> Result<SystemState, ManifoldError> {
        // Booleanos duros NÃO são degradáveis — rejeitar antes de clamarpar
        if !state.pii_scrubbed {
            return Err(ManifoldError::InvariantViolation(
                "I-05: pii_scrubbed=false — PII scrubbing cannot be 'degraded'. \
                 Either the data is scrubbed or it is not.".to_string(),
            ));
        }
        if !state.signature_valid {
            return Err(ManifoldError::InvariantViolation(
                "I-06: signature_valid=false — cryptographic signatures cannot be \
                 'degraded'. Either the signature is valid or it is not.".to_string(),
            ));
        }

        // Degradar apenas campos contínuos (seguro)
        let mut degraded = state.clone();
        degraded.token_budget = degraded.token_budget.max(0).min(self.max_tokens);
        degraded.agent_count = degraded.agent_count.min(self.max_agents);
        degraded.sandbox_fuel = degraded.sandbox_fuel.max(1).min(self.max_fuel);
        degraded.entropy_bits = degraded.entropy_bits.max(self.min_entropy);
        degraded.rate_limit_remaining = degraded.rate_limit_remaining.max(1).min(self.max_rate);
        degraded.model_capability = degraded.model_capability.max(4294967296);
        degraded.config = self.config.clone();

        // Garantir pós-condição
        debug_assert!(degraded.check_all(), "neron_model internal error");
        Ok(degraded)
    }

    /// Extract the behavioural profile (Torelli metaphor).
    pub fn manifold_profile(&self, state: &SystemState) -> ManifoldProfile {
        ManifoldProfile {
            agents: state.agent_count,
            entropy: state.entropy_bits,
            rate: state.rate_limit_remaining,
        }
    }

    /// Profile equality (Torelli equivalence metaphor).
    pub fn torelli_equivalence(&self, p1: &ManifoldProfile, p2: &ManifoldProfile) -> bool {
        p1 == p2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_state_safe() {
        let manifold = SafeManifold::new();
        let state = SystemState::safe(manifold.config.clone());
        assert!(state.check_all());
        let point = manifold.embed_state(&state);
        assert!(!point.on_theta);
    }

    #[test]
    fn test_embed_state_unsafe_detected() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.token_budget = -1;
        assert!(!state.check_all());
        let point = manifold.embed_state(&state);
        assert!(point.on_theta);
    }

    #[test]
    fn test_observer_defect_zero_when_safe() {
        let manifold = SafeManifold::new();
        let ideal = SystemState::safe(manifold.config.clone());
        let actual = ideal.clone();
        let defect = manifold.compute_observer_defect(&ideal, &actual);
        assert!(defect < 1.0e-10);
    }

    #[test]
    fn test_observer_defect_scaling() {
        let manifold = SafeManifold::new();
        let ideal = SystemState::safe(manifold.config.clone());

        let mut far = ideal.clone();
        far.token_budget = 0;
        let defect_far = manifold.compute_observer_defect(&ideal, &far);

        let mut near = ideal.clone();
        near.token_budget = 9000;
        let defect_near = manifold.compute_observer_defect(&ideal, &near);

        assert!(defect_far > defect_near);
    }

    #[test]
    fn test_neron_model_checked_rejects_pii() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.pii_scrubbed = false;
        assert!(matches!(
            manifold.neron_model_checked(&state),
            Err(ManifoldError::InvariantViolation(msg)) if msg.contains("I-05")
        ));
    }

    #[test]
    fn test_neron_model_checked_rejects_signature() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.signature_valid = false;
        assert!(manifold.neron_model_checked(&state).is_err());
    }

    #[test]
    fn test_neron_model_checked_accepts_continuous_degradation() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.token_budget = -5000;
        state.agent_count = 20;
        // pii e signature permanecem true
        let degraded = manifold.neron_model_checked(&state).unwrap();
        assert!(degraded.check_all());
        assert_eq!(degraded.token_budget, 0);
        assert_eq!(degraded.agent_count, 10);
    }

    #[test]
    fn test_dynamic_weights_increase_near_limits() {
        let manifold = SafeManifold::new();
        let ideal = SystemState::safe(manifold.config.clone());

        let mut near_limit = ideal.clone();
        near_limit.agent_count = 9;
        near_limit.entropy_bits = 300;

        let mut mid = ideal.clone();
        mid.agent_count = 5;
        mid.entropy_bits = 600;

        let defect_near = manifold.compute_observer_defect(&ideal, &near_limit);
        let defect_mid = manifold.compute_observer_defect(&ideal, &mid);

        assert!(defect_near >= 0.0);
        assert!(defect_mid >= 0.0);
    }

    #[test]
    fn test_collision_detection() {
        let manifold = SafeManifold::new();
        let mut s1 = SystemState::safe(manifold.config.clone());
        let mut s2 = s1.clone();
        s1.token_budget = 10000;
        s2.token_budget = 20000;
        assert_ne!(s1, s2);
        assert!(manifold.collision_detected(&s1, &s2));
    }

    #[test]
    fn test_classify_escape_boundary() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.token_budget = -1;
        state.agent_count = 11;
        assert_eq!(manifold.classify_escape(&state), EscapeRegion::Boundary);
    }

    #[test]
    fn test_classify_escape_outside() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.token_budget = -1;
        state.agent_count = 11;
        state.sandbox_fuel = 0;
        state.entropy_bits = 128;
        state.pii_scrubbed = false;
        assert_eq!(manifold.classify_escape(&state), EscapeRegion::Outside);
    }

    #[test]
    fn test_neron_model_enforces_invariants() {
        let manifold = SafeManifold::new();
        let mut state = SystemState::safe(manifold.config.clone());
        state.token_budget = -5000;
        state.agent_count = 20;
        state.sandbox_fuel = 0;
        state.entropy_bits = 128;
        state.rate_limit_remaining = -100;
        state.model_capability = 100;

        let degraded = manifold.neron_model(&state);
        assert!(degraded.check_all());
        assert_eq!(degraded.token_budget, 0);
        assert_eq!(degraded.agent_count, 10);
        assert_eq!(degraded.sandbox_fuel, 1);
        assert_eq!(degraded.entropy_bits, 256);
        assert_eq!(degraded.rate_limit_remaining, 1);
        assert!(degraded.model_capability >= 4294967296);
    }

    #[test]
    fn test_safe_state_construction_ok() {
        let state = SystemState::safe(SystemConfig::default());
        let safe = SafeState::new(state).unwrap();
        assert!(safe.as_inner().check_all());
    }

    #[test]
    fn test_safe_state_construction_fails_on_invalid() {
        let mut state = SystemState::safe(SystemConfig::default());
        state.token_budget = -1;
        assert!(SafeState::new(state).is_err());
    }

    #[test]
    fn test_safe_state_default_safe() {
        let safe = SafeState::default_safe();
        assert!(safe.as_inner().check_all());
    }
}
