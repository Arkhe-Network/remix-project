// crates/arkhe-core/src/safety/spectroscopy/probe_strategies.rs
//! ARKHE-χ Fase 4 — Estratégias de Sonda para Invariant Spectroscopy
//!
//! Cada estratégia é análoga a um protocolo experimental do artigo:
//!   LinearSweep          → "fixar φ_R1 e variar t_LR"
//!   RadialSweep          → varredura radial a partir de ponto central
//!   ManifoldTrace        → varredura ao longo do χ-manifold
//!   AdiabaticPerturbation → perturbações pequenas para detectar gap

use crate::safety::symmetry_generator::{SystemState, SystemConfig};

/// Estratégia de sonda para varredura espectroscópica
///
/// Cada estratégia define como explorar o espaço de parâmetros
/// para encontrar violações de invariantes ("nós de Weyl").
pub trait ProbeStrategy: Send + Sync {
    /// Nome da estratégia
    fn name(&self) -> &str;

    /// Gera os estados a serem testados
    fn generate_states(&self, base_config: &SystemConfig) -> Vec<SystemState>;

    /// Invariantes a verificar nesta sonda
    fn target_invariants(&self) -> &[String];
}

/// Varredura linear ao longo de um eixo de parâmetro
///
/// Análogo a: fixar φ_R1 e variar t_LR no artigo.
/// Varre um parâmetro de um valor inicial a um final em N passos.
#[derive(Debug, Clone)]
pub struct LinearSweep {
    pub name: String,
    /// Parâmetro a variar: "token_budget", "agent_count", "sandbox_fuel", etc.
    pub parameter: String,
    /// Range de variação (inicial, final)
    pub range: (f64, f64),
    /// Número de passos
    pub steps: usize,
    /// Invariantes a verificar
    pub invariants: Vec<String>,
}

impl ProbeStrategy for LinearSweep {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate_states(&self, base_config: &SystemConfig) -> Vec<SystemState> {
        let mut states = Vec::with_capacity(self.steps);
        let (start, end) = self.range;
        let step_size = (end - start) / (self.steps.max(2) as f64 - 1.0);

        for i in 0..self.steps {
            let mut state = SystemState::safe(base_config.clone());
            let value = start + step_size * i as f64;

            match self.parameter.as_str() {
                "token_budget" => state.token_budget = value as i64,
                "agent_count" => state.agent_count = value as u32,
                "sandbox_fuel" => state.sandbox_fuel = value as i64,
                "entropy_bits" => state.entropy_bits = value as u32,
                "rate_limit_remaining" => state.rate_limit_remaining = value as i64,
                _ => {}
            }

            states.push(state);
        }

        states
    }

    fn target_invariants(&self) -> &[String] {
        &self.invariants
    }
}

/// Varredura radial a partir de um ponto central
///
/// Análogo a: explorar o espaço de parâmetros em círculos concêntricos
/// a partir de um estado base conhecido.
#[derive(Debug, Clone)]
pub struct RadialSweep {
    pub name: String,
    /// Estado central (ponto de partida)
    pub center: SystemState,
    /// Raio máximo de variação
    pub radius: f64,
    /// Número de ângulos (direções)
    pub invariants: Vec<String>,
    pub angles: usize,
}

impl ProbeStrategy for RadialSweep {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate_states(&self, _base_config: &SystemConfig) -> Vec<SystemState> {
        let mut states = Vec::with_capacity(self.angles);

        for i in 0..self.angles {
            let angle = 2.0 * std::f64::consts::PI * (i as f64 / self.angles as f64);
            let mut state = self.center.clone();

            // Perturbar token_budget e agent_count radialmente
            state.token_budget += (self.radius * angle.cos()) as i64;
            state.agent_count = ((state.agent_count as f64 + self.radius * angle.sin()) as u32)
                .max(0);

            states.push(state);
        }

        states
    }

    fn target_invariants(&self) -> &[String] {
        &self.invariants
    }
}

/// Varredura ao longo do χ-manifold
///
/// Análogo a: "sweeping t_LR on χ-manifold" no artigo.
/// Gera estados que satisfazem um subconjunto de invariantes (no manifold)
/// e perturba ligeiramente fora dele para detectar gap.
#[derive(Debug, Clone)]
pub struct ManifoldTrace {
    pub name: String,
    /// Subconjunto de invariantes que definem o manifold
    pub invariant_subset: Vec<String>,
    /// Resolução da varredura (passo entre estados)
    pub resolution: f64,
    /// Invariantes a verificar (todos)
    pub invariants: Vec<String>,
}

impl ProbeStrategy for ManifoldTrace {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate_states(&self, base_config: &SystemConfig) -> Vec<SystemState> {
        let mut states = Vec::new();
        let steps = (1.0 / self.resolution) as usize;

        for i in 0..steps {
            let t = i as f64 * self.resolution;
            let mut state = SystemState::safe(base_config.clone());

            // Variação paramétrica ao longo do manifold
            // Simula: fixar alguns invariantes, variar outros
            state.token_budget = (base_config.max_tokens as f64 * t) as i64;
            state.agent_count = (base_config.max_agents as f64 * t) as u32;
            state.sandbox_fuel = (base_config.min_fuel as f64 * (1.0 + t)) as i64;

            states.push(state);
        }

        states
    }

    fn target_invariants(&self) -> &[String] {
        &self.invariants
    }
}

/// Perturbação adiabática
///
/// Análogo a: "adiabatic tracking" do DMRG no artigo.
/// Aplica pequenas perturbações a um estado base para detectar
/// a "resposta" do sistema (quão rápido o gap se abre).
#[derive(Debug, Clone)]
pub struct AdiabaticPerturbation {
    pub name: String,
    /// Estado base (ponto de partida)
    pub base_state: SystemState,
    /// Escala da perturbação (fração do valor)
    pub perturbation_scale: f64,
    /// Número de iterações
    pub iterations: usize,
    /// Invariantes a verificar
    pub invariants: Vec<String>,
}

impl ProbeStrategy for AdiabaticPerturbation {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate_states(&self, _base_config: &SystemConfig) -> Vec<SystemState> {
        let mut states = Vec::with_capacity(self.iterations);
        let mut rng = SimpleRng::new(42); // Seed fixo para reprodutibilidade

        for _ in 0..self.iterations {
            let mut state = self.base_state.clone();

            // Perturbação gaussiana-like nos parâmetros numéricos
            let mut perturbation = |base: f64| -> f64 {
                let noise = (rng.next() as f64 / u32::MAX as f64 - 0.5) * 2.0;
                base * (1.0 + noise * self.perturbation_scale)
            };

            state.token_budget = perturbation(state.token_budget as f64) as i64;
            state.agent_count = perturbation(state.agent_count as f64) as u32;
            state.sandbox_fuel = perturbation(state.sandbox_fuel as f64) as i64;
            state.entropy_bits = perturbation(state.entropy_bits as f64) as u32;

            states.push(state);
        }

        states
    }

    fn target_invariants(&self) -> &[String] {
        &self.invariants
    }
}

/// RNG simples e determinístico para perturbações reprodutíveis
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u32 {
        // LCG (Linear Congruential Generator)
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.state >> 32) & 0xFFFFFFFF) as u32
    }
}
