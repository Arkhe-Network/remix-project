pub struct SecurityMetrics {
    pub evidence_rate: f64,
    pub impact: f64,
}

impl SecurityMetrics {
    pub fn new(evidence_rate: f64, impact: f64) -> Self {
        Self {
            evidence_rate,
            impact,
        }
    }

    #[cfg_attr(kani, kani::ensures(|result: &f64| *result >= 0.0 && *result <= 1.0))]
    pub fn pressure(&self) -> f64 {
        self.evidence_rate * self.impact
    }
}

pub struct PolicyCapacity {
    pub max_pressure: f64,
}

impl PolicyCapacity {
    pub fn new(max_pressure: f64) -> Self {
        Self { max_pressure }
    }

    #[cfg_attr(kani, kani::ensures(|result: &bool| *result == (pressure >= 0.0 && pressure <= self.max_pressure)))]
    pub fn is_safe(&self, pressure: f64) -> bool {
        pressure >= 0.0 && pressure <= self.max_pressure
    }
}

pub struct PolicyEngine {
    max_pressure: f64,
    current_pressure: f64,
}

impl PolicyEngine {
    pub fn new(max_pressure: f64) -> Self {
        Self {
            max_pressure,
            current_pressure: 0.0,
        }
    }

    pub fn current_pressure(&self) -> f64 {
        self.current_pressure
    }

    pub fn max_pressure(&self) -> f64 {
        self.max_pressure
    }

    pub fn process_metrics(&mut self, metrics: &SecurityMetrics) -> Result<(), ()> {
        let p = metrics.pressure();
        if p <= self.max_pressure {
            self.current_pressure = p;
            Ok(())
        } else {
            Err(())
        }
    }
}

pub fn check_pressure(metrics: &SecurityMetrics, capacity: &PolicyCapacity) -> bool {
    capacity.is_safe(metrics.pressure())
}

#[cfg(kani)]
mod verification {
    use super::*;
    use kani::{any, assume};

    // ============================================================
    // S1-S5: Propriedades Fundamentais de Pressão
    // ============================================================

    /// S1: Pressão máxima — ∀ e,i ∈ [0,1]: e·i ≤ 1.0
    #[kani::proof]
    fn s1_pressure_bounded_by_one() {
        let e: f64 = any();
        let i: f64 = any();
        assume((0.0..=1.0).contains(&e));
        assume((0.0..=1.0).contains(&i));
        let metrics = SecurityMetrics::new(e, i);
        assert!(metrics.pressure() <= 1.0);
    }

    /// S2: Condição de fronteira — check_pressure ↔ pressure ≤ max_pressure
    #[kani::proof]
    fn s2_boundary_condition() {
        let e: f64 = any();
        let i: f64 = any();
        let max_p: f64 = any();
        assume((0.0..=1.0).contains(&e));
        assume((0.0..=1.0).contains(&i));
        assume(max_p >= 0.0 && max_p <= 1.0);
        let metrics = SecurityMetrics::new(e, i);
        let capacity = PolicyCapacity::new(max_p);
        let is_safe = check_pressure(&metrics, &capacity);
        assert_eq!(is_safe, metrics.pressure() <= max_p);
    }

    /// S3: Monotonicidade — e₁≤e₂ ∧ i₁≤i₂ ⟹ p₁≤p₂
    #[kani::proof]
    fn s3_pressure_monotonic() {
        let e1: f64 = any();
        let e2: f64 = any();
        let i1: f64 = any();
        let i2: f64 = any();
        assume((0.0..=1.0).contains(&e1) && (0.0..=1.0).contains(&e2));
        assume((0.0..=1.0).contains(&i1) && (0.0..=1.0).contains(&i2));
        assume(e1 <= e2 && i1 <= i2);
        let p1 = SecurityMetrics::new(e1, i1).pressure();
        let p2 = SecurityMetrics::new(e2, i2).pressure();
        assert!(p1 <= p2);
    }

    /// S4: Não-negatividade — pressure ≥ 0
    #[kani::proof]
    fn s4_pressure_non_negative() {
        let e: f64 = any();
        let i: f64 = any();
        assume((0.0..=1.0).contains(&e));
        assume((0.0..=1.0).contains(&i));
        let metrics = SecurityMetrics::new(e, i);
        assert!(metrics.pressure() >= 0.0);
    }

    /// S5: Estabilidade temporal — múltiplas atualizações nunca excedem a capacidade
    #[kani::proof]
    #[kani::unwind(4)]
    fn s5_temporal_stability() {
        let capacity = 0.8;
        let mut engine = PolicyEngine::new(capacity);
        for _ in 0..3 {
            let e: f64 = any();
            let i: f64 = any();
            assume((0.0..=1.0).contains(&e));
            assume((0.0..=1.0).contains(&i));
            let metrics = SecurityMetrics::new(e, i);
            let pressure = metrics.pressure();
            if pressure <= capacity {
                assert!(engine.process_metrics(&metrics).is_ok());
            } else {
                assert!(engine.process_metrics(&metrics).is_err());
            }
            assert!(engine.current_pressure() <= capacity);
        }
    }

    // ============================================================
    // S6-S8: Propriedades de Segurança Adaptativa
    // ============================================================

    /// S6: A redução da taxa de evidência nunca aumenta a pressão
    #[kani::proof]
    fn s6_rate_reduction_never_increases_pressure() {
        let e_high: f64 = any();
        let e_low: f64 = any();
        let i: f64 = any();
        assume((0.0..=1.0).contains(&e_high) && (0.0..=1.0).contains(&e_low));
        assume((0.0..=1.0).contains(&i));
        assume(e_low <= e_high);
        let p_high = SecurityMetrics::new(e_high, i).pressure();
        let p_low = SecurityMetrics::new(e_low, i).pressure();
        assert!(p_low <= p_high);
    }

    /// S7: Pressão zero é sempre segura (independente da capacidade)
    #[kani::proof]
    fn s7_zero_pressure_always_safe() {
        let max_p: f64 = any();
        assume(max_p >= 0.0 && max_p <= 1.0);
        let metrics = SecurityMetrics::new(0.0, 0.5);
        let capacity = PolicyCapacity::new(max_p);
        assert!(check_pressure(&metrics, &capacity));
    }

    /// S8: Capacidade máxima (1.0) sempre segura para qualquer pressão válida
    #[kani::proof]
    fn s8_max_capacity_always_safe() {
        let e: f64 = any();
        let i: f64 = any();
        assume((0.0..=1.0).contains(&e));
        assume((0.0..=1.0).contains(&i));
        let metrics = SecurityMetrics::new(e, i);
        let capacity = PolicyCapacity::new(1.0);
        assert!(check_pressure(&metrics, &capacity));
    }

    // ============================================================
    // S9-S11: Propriedades de Integridade Numérica
    // ============================================================

    /// S9: Ausência de overflow em PolicyCapacity::new
    #[kani::proof]
    fn s9_capacity_construction_no_overflow() {
        let max_p: f64 = any();
        assume(max_p >= 0.0 && max_p <= f64::MAX);
        let capacity = PolicyCapacity::new(max_p);
        assert!(capacity.max_pressure.is_finite());
        assert!(capacity.max_pressure >= 0.0);
    }

    /// S10: Ausência de panic em SecurityMetrics::new para entradas válidas
    #[kani::proof]
    fn s10_metrics_construction_no_panic() {
        let e: f64 = any();
        let i: f64 = any();
        assume((0.0..=1.0).contains(&e));
        assume((0.0..=1.0).contains(&i));
        let metrics = SecurityMetrics::new(e, i);
        assert!(metrics.evidence_rate.is_finite());
        assert!(metrics.impact.is_finite());
    }

    /// S11: Processamento de múltiplas métricas mantém invariantes
    #[kani::proof]
    #[kani::unwind(6)]
    fn s11_multiple_metrics_invariants() {
        let mut engine = PolicyEngine::new(0.7);
        let mut all_safe = true;
        for _ in 0..5 {
            let e: f64 = any();
            let i: f64 = any();
            assume((0.0..=1.0).contains(&e));
            assume((0.0..=1.0).contains(&i));
            let metrics = SecurityMetrics::new(e, i);
            if metrics.pressure() <= engine.max_pressure() {
                assert!(engine.process_metrics(&metrics).is_ok());
            } else {
                assert!(engine.process_metrics(&metrics).is_err());
                all_safe = false;
            }
            assert!(engine.current_pressure() <= engine.max_pressure());
        }
        // Se todas as métricas foram seguras, a pressão final também é segura
        if all_safe {
            assert!(engine.current_pressure() <= engine.max_pressure());
        }
    }

    #[kani::proof_for_contract(SecurityMetrics::pressure)]
    fn check_pressure_contract() {
        let e: f64 = any();
        let i: f64 = any();
        assume((0.0..=1.0).contains(&e));
        assume((0.0..=1.0).contains(&i));
        let metrics = SecurityMetrics::new(e, i);
        let _ = metrics.pressure(); // O contrato é verificado automaticamente
    }
}
