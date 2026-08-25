use arkhe_safe_manifold::*;

// ═════════════════════════════════════════════════════════════════════════════
// Unit-style integration tests
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn test_manifold_embedding_preserves_invariants() {
    let manifold = SafeManifold::new();
    let state = SystemState::safe(manifold.config.clone());
    assert!(state.check_all());
    let point = manifold.embed_state(&state);
    assert!(!point.on_theta);
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
fn test_collision_detected_on_distinct_states() {
    let manifold = SafeManifold::new();
    let mut s1 = SystemState::safe(manifold.config.clone());
    let mut s2 = s1.clone();
    s1.token_budget = 10000;
    s2.token_budget = 20000;
    assert_ne!(s1, s2);
    assert!(manifold.collision_detected(&s1, &s2));
}

#[test]
fn test_neron_model_degrades_gracefully() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.agent_count = 20;
    let degraded = manifold.neron_model(&state);
    assert_eq!(degraded.agent_count, 10);
    assert!(degraded.check_all());
}

#[test]
fn test_classify_escape_warning() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.token_budget = -1;
    assert_eq!(manifold.classify_escape(&state), EscapeRegion::Warning);
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
fn test_classify_escape_continuum() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.token_budget = -1;
    state.agent_count = 11;
    state.sandbox_fuel = 0;
    state.entropy_bits = 128;
    assert_eq!(manifold.classify_escape(&state), EscapeRegion::Continuum);
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
    state.signature_valid = false;
    assert_eq!(manifold.classify_escape(&state), EscapeRegion::Outside);
}

#[test]
fn test_neron_model_enforces_all_invariants() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.token_budget = -9999;
    state.agent_count = 999;
    state.sandbox_fuel = -1;
    state.entropy_bits = 0;
    state.rate_limit_remaining = -1;
    state.model_capability = 1;
    state.pii_scrubbed = false;
    state.signature_valid = false;

    let degraded = manifold.neron_model(&state);
    assert!(degraded.check_all());
    assert_eq!(degraded.token_budget, 0);
    assert_eq!(degraded.agent_count, 10);
    assert_eq!(degraded.sandbox_fuel, 1);
    assert_eq!(degraded.entropy_bits, 256);
    assert_eq!(degraded.rate_limit_remaining, 1);
    assert!(degraded.model_capability >= 4294967296);
    assert!(degraded.pii_scrubbed);
    assert!(degraded.signature_valid);
}

#[test]
fn test_unsafe_state_embedding_on_theta() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.token_budget = -5000;
    let point = manifold.embed_state(&state);
    assert!(point.on_theta, "Negative token_budget should place state on theta boundary");
}

#[test]
fn test_torelli_equivalence_same_profile() {
    let manifold = SafeManifold::new();
    let s1 = SystemState::safe(manifold.config.clone());
    let mut s2 = s1.clone();
    s2.token_budget = 5000;
    let p1 = manifold.manifold_profile(&s1);
    let p2 = manifold.manifold_profile(&s2);
    assert!(manifold.torelli_equivalence(&p1, &p2));
}

#[test]
fn test_safe_state_construction_and_destruction() {
    let state = SystemState::safe(SystemConfig::default());
    let safe = SafeState::new(state.clone()).unwrap();
    assert!(safe.as_inner().check_all());
    assert_eq!(safe.into_inner(), state);
}

#[test]
fn test_safe_state_rejects_invalid() {
    let mut state = SystemState::safe(SystemConfig::default());
    state.token_budget = -1;
    assert!(SafeState::new(state).is_err());
}

#[test]
fn test_safe_state_default_safe_is_valid() {
    let safe = SafeState::default_safe();
    assert!(safe.as_inner().check_all());
}

#[test]
fn test_neron_model_idempotence() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.agent_count = 99;
    state.entropy_bits = 0;

    let once = manifold.neron_model(&state);
    let twice = manifold.neron_model(&once);
    assert_eq!(once, twice, "neron_model must be idempotent");
}

#[test]
fn test_neron_model_is_fixed_point_safe() {
    let manifold = SafeManifold::new();
    let mut state = SystemState::safe(manifold.config.clone());
    state.token_budget = -1;
    state.pii_scrubbed = false;

    let fixed = manifold.neron_model(&state);
    assert!(fixed.check_all(), "neron_model fixed point must be safe");
}

// ═════════════════════════════════════════════════════════════════════════════
// Property-based tests with proptest
// ═════════════════════════════════════════════════════════════════════════════

use proptest::prelude::*;

proptest! {
    /// neron_model must be idempotent for any input.
    #[test]
    fn prop_neron_model_idempotent(
        token_budget in -20000i64..20000i64,
        agent_count in 0u32..100u32,
        sandbox_fuel in -5000i64..5000i64,
        entropy_bits in 0u32..1024u32,
        rate_limit in -5000i64..5000i64,
        model_cap in 0u64..u64::MAX,
    ) {
        let config = SystemConfig::default();
        let manifold = SafeManifold::from_config(config.clone());
        let state = SystemState {
            token_budget,
            agent_count,
            sandbox_fuel,
            entropy_bits,
            pii_scrubbed: true,
            signature_valid: true,
            rate_limit_remaining: rate_limit,
            model_capability: model_cap,
            config,
        };

        let first = manifold.neron_model(&state);
        let second = manifold.neron_model(&first);
        prop_assert_eq!(first, second);
    }

    /// neron_model must always produce a state that passes check_all().
    #[test]
    fn prop_neron_model_produces_safe_state(
        token_budget in -20000i64..20000i64,
        agent_count in 0u32..100u32,
        sandbox_fuel in -5000i64..5000i64,
        entropy_bits in 0u32..1024u32,
        rate_limit in -5000i64..5000i64,
        model_cap in 0u64..u64::MAX,
        pii in proptest::bool::ANY,
        sig in proptest::bool::ANY,
    ) {
        let config = SystemConfig::default();
        let manifold = SafeManifold::from_config(config.clone());
        let state = SystemState {
            token_budget,
            agent_count,
            sandbox_fuel,
            entropy_bits,
            pii_scrubbed: pii,
            signature_valid: sig,
            rate_limit_remaining: rate_limit,
            model_capability: model_cap,
            config,
        };

        let degraded = manifold.neron_model(&state);
        prop_assert!(degraded.check_all());
    }

    /// observer_defect must be zero when ideal == actual.
    #[test]
    fn prop_observer_defect_zero_for_identical(
        token_budget in 0i64..10000i64,
        agent_count in 0u32..10u32,
        sandbox_fuel in 1i64..1000i64,
        entropy_bits in 256u32..1024u32,
        rate_limit in 1i64..1000i64,
        model_cap in 4294967296u64..u64::MAX,
    ) {
        let config = SystemConfig::default();
        let manifold = SafeManifold::from_config(config.clone());
        let state = SystemState {
            token_budget,
            agent_count,
            sandbox_fuel,
            entropy_bits,
            pii_scrubbed: true,
            signature_valid: true,
            rate_limit_remaining: rate_limit,
            model_capability: model_cap,
            config,
        };

        let defect = manifold.compute_observer_defect(&state, &state);
        prop_assert!(defect < 1.0e-9);
    }

    /// violation_count must be monotonic with respect to adding violations.
    #[test]
    fn prop_violation_count_monotonic(
        token_budget in -5000i64..15000i64,
        agent_count in 0u32..20u32,
        sandbox_fuel in -500i64..1500i64,
        entropy_bits in 0u32..1024u32,
    ) {
        let config = SystemConfig::default();
        let state = SystemState {
            token_budget,
            agent_count,
            sandbox_fuel,
            entropy_bits,
            pii_scrubbed: true,
            signature_valid: true,
            rate_limit_remaining: 500,
            model_capability: 4294967296,
            config,
        };

        let v = state.violation_count();
        prop_assert!(v <= 8);

        // Adding more violations should not decrease the count
        let mut worse = state.clone();
        worse.pii_scrubbed = false;
        prop_assert!(worse.violation_count() >= v);
    }

    /// classify_escape must be monotonic: more violations → worse or equal region.
    #[test]
    fn prop_classify_escape_monotonic(
        token_budget in any::<i64>(),
        agent_count in any::<u32>(),
        sandbox_fuel in any::<i64>(),
        entropy_bits in any::<u32>(),
        rate_limit in any::<i64>(),
        model_cap in any::<u64>(),
    ) {
        let config = SystemConfig::default();
        let manifold = SafeManifold::from_config(config.clone());
        let state = SystemState {
            token_budget,
            agent_count,
            sandbox_fuel,
            entropy_bits,
            pii_scrubbed: true,
            signature_valid: true,
            rate_limit_remaining: rate_limit,
            model_capability: model_cap,
            config: config.clone(),
        };

        let region = manifold.classify_escape(&state);
        let v = state.violation_count();

        match region {
            EscapeRegion::Safe     => prop_assert_eq!(v, 0),
            EscapeRegion::Warning  => prop_assert_eq!(v, 1),
            EscapeRegion::Boundary => prop_assert_eq!(v, 2),
            EscapeRegion::Continuum => prop_assert!((3..=4).contains(&v)),
            EscapeRegion::Outside  => prop_assert!(v >= 5),
        }
    }
}
