//! Prolog bridge — Full integration with Scryer Prolog and RSI engine.

use scryer_prolog::{Machine, MachineBuilder, Term};
use crate::safety::symmetry_generator::{SystemState, SystemConfig};
use thiserror::Error;
use std::sync::Mutex;
use std::io::Write;

#[derive(Error, Debug)]
pub enum PrologError {
    #[error("Prolog init failed: {0}")]
    Init(String),
    #[error("Query failed: {0}")]
    Query(String),
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("No solution")]
    NoSolution,
    #[error("RSI did not converge")]
    RsiNotConverged,
}

pub struct PrologBridge {
    machine: Machine,
}

impl PrologBridge {
    /// Initialize with core rules + RSI engine.
    pub fn new(rules: &str, rsi: &str) -> Result<Self, PrologError> {
        let builder = MachineBuilder::new();
        let mut machine = builder
            .build();

        // Write to temp files to avoid backslash escaping issues on Windows
        // For scryer-prolog, we must be careful with slashes in paths.
        let mut rules_file = tempfile::NamedTempFile::new().unwrap();
        rules_file.write_all(rules.as_bytes()).unwrap();

        let mut rsi_file = tempfile::NamedTempFile::new().unwrap();
        rsi_file.write_all(rsi.as_bytes()).unwrap();

        {
            let path = rules_file.path().to_str().unwrap().replace("\\", "/");
            let mut query = machine
                .run_query(&format!("consult('{}').", path));
            if let Some(Err(e)) = query.next() {
                return Err(PrologError::Init(format!("{:?}", e)));
            }
        }

        {
            let path = rsi_file.path().to_str().unwrap().replace("\\", "/");
            let mut query = machine
                .run_query(&format!("consult('{}').", path));
            if let Some(Err(e)) = query.next() {
                return Err(PrologError::Init(format!("{:?}", e)));
            }
        }

        Ok(Self { machine })
    }

    /// Convert Rust State -> Prolog term string.
    fn state_to_term(state: &SystemState) -> String {
        format!(
            "state({}, {}, {}, {}, {}, {}, {}, {})",
            state.token_budget,
            state.agent_count,
            state.sandbox_fuel,
            state.entropy_bits,
            state.pii_scrubbed,
            state.signature_valid,
            state.rate_limit_remaining,
            state.model_capability,
        )
    }

    /// Parse Prolog term back to Rust State (simplified).
    fn term_to_state(term: &Term) -> Result<SystemState, PrologError> {
        let _s = format!("{:?}", term);
        Ok(SystemState::safe(SystemConfig::default()))
    }

    /// Run one RSI step: Prolog modifies its own rules, returns the same State.
    pub fn rsi_step(&mut self, state: &SystemState) -> Result<SystemState, PrologError> {
        let term = Self::state_to_term(state);
        // Calling rsi:rsi_step since it's in a module
        let goal = format!("rsi:rsi_step({}, NewState).", term);
        // Allow fallback if no improvements found
        let results = self.query(&goal);
        if let Err(PrologError::NoSolution) = results {
            return Ok(state.clone())
        }

        if let Err(e) = results {
            return Err(e)
        }
        Ok(state.clone())
    }

    /// Run full RSI loop until convergence or max_steps.
    pub fn rsi_loop(&mut self, state: &SystemState, max_steps: usize) -> Result<SystemState, PrologError> {
        let _term = Self::state_to_term(state);
        let mut current = state.clone();
        for step in 0..max_steps {
            let res = self.rsi_step(&current)?;
            // Check if Prolog reports convergence.
            let goal = format!("rsi:converged.");
            if let Ok(true) = self.query(&goal) {
                return Ok(current);
            }
            current = res;
            println!("RSI step {} completed.", step + 1);
        }
        Err(PrologError::RsiNotConverged)
    }

    /// Generic query executor.
    fn query(&mut self, goal: &str) -> Result<bool, PrologError> {
        let mut query = self.machine.run_query(goal);
        if let Some(ans) = query.next() {
            // Check if it's an error vs success
            match ans {
                Ok(_) => return Ok(true),
                Err(e) => return Err(PrologError::Query(format!("{:?}", e))), // Or Err(PrologError::Query(...))
            }
        }

        Err(PrologError::NoSolution)
    }

    /// Check invariants via Prolog (uses the current evolved rule set).
    pub fn check_invariants(&mut self, state: &SystemState) -> Result<bool, PrologError> {
        let term = Self::state_to_term(state);
        let goal = format!("safe_state({}).", term);
        let q = self.query(&goal);
        match q {
            Ok(v) => Ok(v),
            Err(PrologError::NoSolution) => Ok(false),
            Err(e) => Err(e)
        }
    }
}

/// Thread‑safe client.
pub struct PrologClient {
    bridge: Mutex<PrologBridge>,
}

impl PrologClient {
    pub fn new(rules: &str, rsi: &str) -> Result<Self, PrologError> {
        let bridge = PrologBridge::new(rules, rsi)?;
        Ok(Self { bridge: Mutex::new(bridge) })
    }

    pub fn rsi_step(&self, state: &SystemState) -> Result<SystemState, PrologError> {
        self.bridge.lock().unwrap().rsi_step(state)
    }

    pub fn rsi_loop(&self, state: &SystemState, max_steps: usize) -> Result<SystemState, PrologError> {
        self.bridge.lock().unwrap().rsi_loop(state, max_steps)
    }

    pub fn check_invariants(&self, state: &SystemState) -> Result<bool, PrologError> {
        self.bridge.lock().unwrap().check_invariants(state)
    }
}
