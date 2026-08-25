// crates/arkhe-core/src/safety/symmetry_generator.rs
//! ARKHE-χ Fase 1 — SymmetryGenerator v2.3 (PATCHED)
//! Correções aplicadas:
//!   - TransitionSafety::Degraded adicionado
//!   - ViolationType::High ativado
//!   - compute_spectral_gap usa mínimo (não média)
//!   - Trait Invariant com margin()
//!   - preserves_manifold retorna Degraded/Cascade/Recovery corretamente

use std::collections::HashSet;

/// Estado do sistema operacional
#[derive(Debug, Clone)]
pub struct SystemState {
    pub token_budget: i64,
    pub agent_count: u32,
    pub sandbox_fuel: i64,
    pub entropy_bits: u32,
    pub pii_scrubbed: bool,
    pub signature_valid: bool,
    pub rate_limit_remaining: i64,
    pub model_capability: u64,  // B6-fix: u32 → u64
    pub task_requirement: u64,
    pub config: SystemConfig,
}

#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub max_tokens: i64,
    pub max_agents: u32,
    pub min_fuel: i64,
    pub min_entropy: u32,
    pub max_rate_limit: i64,
    pub topological_gap_threshold: f64,
    pub max_sandbox_fuel: i64,  // R1-fix: configurável
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_tokens: 10_000,
            max_agents: 10,
            min_fuel: 100,
            min_entropy: 256,
            max_rate_limit: 1_000,
            topological_gap_threshold: 0.5,
            max_sandbox_fuel: 1_000,
        }
    }
}

impl SystemState {
    pub fn safe(config: SystemConfig) -> Self {
        Self {
            token_budget: config.max_tokens,
            agent_count: 1,
            sandbox_fuel: config.max_sandbox_fuel,
            entropy_bits: 512,
            pii_scrubbed: true,
            signature_valid: true,
            rate_limit_remaining: config.max_rate_limit,
            model_capability: 0xFFFF_FFFF_FFFF_FFFF,  // u64 max
            task_requirement: 0xFF,
            config,
        }
    }
}

/// Invariante como trait extensível
pub trait Invariant: Send + Sync {
    fn id(&self) -> &'static str;
    fn class(&self) -> InvariantClass;
    fn check(&self, state: &SystemState) -> bool;
    /// D1-fix: cada invariante computa sua própria margem
    fn margin(&self, state: &SystemState) -> f64 {
        if self.check(state) { 1.0 } else { 0.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InvariantClass {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub enum ViolationType {
    Critical { invariant_ids: Vec<&'static str> },
    High { invariant_ids: Vec<&'static str> },
}

#[derive(Debug, Clone)]
pub enum ManifoldResult {
    Inside,
    Degraded(Vec<(InvariantClass, String)>),  // (class, invariant_id)
    Outside { violation: ViolationType, state: SystemState },
}

#[derive(Debug, Clone)]
pub enum TransitionSafety {
    Safe,
    CriticalEscape { violation: ViolationType, state: SystemState },
    CascadeFailure { violation: ViolationType },
    Degraded { violations: Vec<String>, warning: String },  // C1-fix
    Recovery,  // D5-fix: Outside → Inside
    Unsafe { reason: String },
}

/// Gerador de simetria 𝒫_safe
pub struct SymmetryGenerator {
    invariants: Vec<Box<dyn Invariant>>,
}

impl SymmetryGenerator {
    pub fn new(invariants: Vec<Box<dyn Invariant>>, _config: SystemConfig) -> Self {
        Self { invariants }
    }

    pub fn invariants(&self) -> &[Box<dyn Invariant>] {
        &self.invariants
    }

    /// Verifica se um estado está em ℳ_safe
    pub fn is_in_manifold(&self, state: &SystemState) -> ManifoldResult {
        let mut critical_violations = Vec::new();
        let mut high_violations = Vec::new();

        for inv in &self.invariants {
            if !inv.check(state) {
                match inv.class() {
                    InvariantClass::Critical => {
                        critical_violations.push(inv.id());
                    }
                    InvariantClass::High => {
                        high_violations.push((InvariantClass::High, inv.id().to_string()));
                    }
                    _ => {}
                }
            }
        }

        if !critical_violations.is_empty() {
            ManifoldResult::Outside {
                violation: ViolationType::Critical {
                    invariant_ids: critical_violations,
                },
                state: state.clone(),
            }
        } else if !high_violations.is_empty() {
            ManifoldResult::Degraded(high_violations)
        } else {
            ManifoldResult::Inside
        }
    }

    /// C2-fix: compute_spectral_gap usa MÍNIMO (bottleneck), não média
    pub fn compute_spectral_gap(&self, state: &SystemState) -> f64 {
        let margins: Vec<f64> = self.invariants.iter()
            .map(|inv| inv.margin(state))
            .collect();

        if margins.is_empty() {
            1.0
        } else {
            margins.into_iter().fold(f64::INFINITY, f64::min)
        }
    }

    /// Verifica se uma transição preserva ℳ_safe
    pub fn preserves_manifold(
        &self,
        from: &SystemState,
        to: &SystemState,
    ) -> TransitionSafety {
        match (self.is_in_manifold(from), self.is_in_manifold(to)) {
            (ManifoldResult::Inside, ManifoldResult::Inside) => {
                TransitionSafety::Safe
            }
            (ManifoldResult::Inside, ManifoldResult::Degraded(v)) => {
                TransitionSafety::Degraded {
                    violations: v.iter().map(|(_, id)| id.clone()).collect(),
                    warning: "Transition from Inside to Degraded".into(),
                }
            }
            (ManifoldResult::Inside, ManifoldResult::Outside { violation, .. }) => {
                TransitionSafety::CriticalEscape { violation, state: to.clone() }
            }
            (ManifoldResult::Degraded(_), ManifoldResult::Degraded(v)) => {
                TransitionSafety::Degraded {
                    violations: v.iter().map(|(_, id)| id.clone()).collect(),
                    warning: "Transition within Degraded region".into(),
                }
            }
            (ManifoldResult::Degraded(_), ManifoldResult::Inside) => {
                TransitionSafety::Recovery
            }
            (ManifoldResult::Degraded(_), ManifoldResult::Outside { violation, .. }) => {
                TransitionSafety::CascadeFailure { violation }
            }
            (ManifoldResult::Outside { .. }, ManifoldResult::Inside) => {
                TransitionSafety::Recovery  // D5-fix: permite recovery
            }
            (ManifoldResult::Outside { .. }, ManifoldResult::Degraded(v)) => {
                TransitionSafety::Degraded {
                    violations: v.iter().map(|(_, id)| id.clone()).collect(),
                    warning: "Recovery to Degraded state".into(),
                }
            }
            (ManifoldResult::Outside { .. }, ManifoldResult::Outside { violation, .. }) => {
                TransitionSafety::Unsafe {
                    reason: format!("Outside → Outside: {:?}", violation),
                }
            }
        }
    }
}

// ─── Invariantes concretos ────────────────────────────────────

pub struct TokenBudgetInvariant;
impl Invariant for TokenBudgetInvariant {
    fn id(&self) -> &'static str { "I-01" }
    fn class(&self) -> InvariantClass { InvariantClass::Critical }
    fn check(&self, state: &SystemState) -> bool {
        state.token_budget >= 0 && state.token_budget <= state.config.max_tokens
    }
    fn margin(&self, state: &SystemState) -> f64 {
        if state.token_budget < 0 { return 0.0; }
        (state.token_budget as f64 / state.config.max_tokens as f64).min(1.0)
    }
}

pub struct AgentCountInvariant;
impl Invariant for AgentCountInvariant {
    fn id(&self) -> &'static str { "I-02" }
    fn class(&self) -> InvariantClass { InvariantClass::High }  // C1-fix: HIGH, não CRITICAL
    fn check(&self, state: &SystemState) -> bool {
        state.agent_count <= state.config.max_agents
    }
    fn margin(&self, state: &SystemState) -> f64 {
        if state.agent_count > state.config.max_agents { return 0.0; }
        1.0 - (state.agent_count as f64 / state.config.max_agents as f64)
    }
}

pub struct SandboxFuelInvariant;
impl Invariant for SandboxFuelInvariant {
    fn id(&self) -> &'static str { "I-03" }
    fn class(&self) -> InvariantClass { InvariantClass::Critical }
    fn check(&self, state: &SystemState) -> bool {
        state.sandbox_fuel >= state.config.min_fuel
    }
    fn margin(&self, state: &SystemState) -> f64 {
        // R1-fix: usa max_sandbox_fuel configurável
        if state.sandbox_fuel < state.config.min_fuel { return 0.0; }
        (state.sandbox_fuel as f64 / state.config.max_sandbox_fuel as f64).min(1.0)
    }
}

pub struct EntropyInvariant;
impl Invariant for EntropyInvariant {
    fn id(&self) -> &'static str { "I-04" }
    fn class(&self) -> InvariantClass { InvariantClass::Critical }
    fn check(&self, state: &SystemState) -> bool {
        state.entropy_bits >= state.config.min_entropy
    }
    fn margin(&self, state: &SystemState) -> f64 {
        // D4-fix: retorna 0.0 para violações
        if state.entropy_bits < state.config.min_entropy { return 0.0; }
        ((state.entropy_bits - state.config.min_entropy) as f64 / state.config.min_entropy as f64).min(1.0)
    }
}

pub struct PiiScrubbedInvariant;
impl Invariant for PiiScrubbedInvariant {
    fn id(&self) -> &'static str { "I-05" }
    fn class(&self) -> InvariantClass { InvariantClass::Critical }
    fn check(&self, state: &SystemState) -> bool { state.pii_scrubbed }
}

pub struct SignatureValidInvariant;
impl Invariant for SignatureValidInvariant {
    fn id(&self) -> &'static str { "I-06" }
    fn class(&self) -> InvariantClass { InvariantClass::Critical }
    fn check(&self, state: &SystemState) -> bool { state.signature_valid }
}

pub struct RateLimitInvariant;
impl Invariant for RateLimitInvariant {
    fn id(&self) -> &'static str { "I-07" }
    fn class(&self) -> InvariantClass { InvariantClass::High }  // C1-fix: HIGH
    fn check(&self, state: &SystemState) -> bool {
        state.rate_limit_remaining >= 0
    }
    fn margin(&self, state: &SystemState) -> f64 {
        if state.rate_limit_remaining < 0 { return 0.0; }
        (state.rate_limit_remaining as f64 / state.config.max_rate_limit as f64).min(1.0)
    }
}

pub struct CapabilityInvariant;
impl Invariant for CapabilityInvariant {
    fn id(&self) -> &'static str { "I-08" }
    fn class(&self) -> InvariantClass { InvariantClass::Critical }
    fn check(&self, state: &SystemState) -> bool {
        (state.model_capability & state.task_requirement) == state.task_requirement
    }
    fn margin(&self, state: &SystemState) -> f64 {
        // D3-fix: margem reflete capacidade extra, não inverte
        let required = state.task_requirement;
        let available = state.model_capability;
        if (available & required) != required { return 0.0; }
        let extra = available.count_ones() as i64 - required.count_ones() as i64;
        let max_extra = 64 - required.count_ones() as i64;  // B6-fix: 64 bits
        if max_extra <= 0 { 1.0 } else { (extra as f64 / max_extra as f64).min(1.0).max(0.0) }
    }
}

pub fn all_invariants() -> Vec<Box<dyn Invariant>> {
    vec![
        Box::new(TokenBudgetInvariant),
        Box::new(AgentCountInvariant),
        Box::new(SandboxFuelInvariant),
        Box::new(EntropyInvariant),
        Box::new(PiiScrubbedInvariant),
        Box::new(SignatureValidInvariant),
        Box::new(RateLimitInvariant),
        Box::new(CapabilityInvariant),
    ]
}
