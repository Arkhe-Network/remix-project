use crate::nonology::NonologyTerm;
use crate::state::LoopResult;
use serde::{Deserialize, Serialize};
use tracing::warn;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologicBounds {
    pub allowed_actions: Vec<String>,
    pub forbidden_concepts: Vec<String>,
    pub system_constraints: Vec<String>,
}
impl Default for OntologicBounds {
    fn default() -> Self {
        Self {
            allowed_actions: vec!["respond".into()],
            forbidden_concepts: vec![],
            system_constraints: vec!["Do not hallucinate".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualState {
    pub working_memory: Vec<String>,
    pub rag_snippets: Vec<String>,
    pub wiggum_token: u64,
}
impl Default for ContextualState {
    fn default() -> Self {
        Self {
            working_memory: vec![],
            rag_snippets: vec![],
            wiggum_token: arkhe_core::Timestamp::now().as_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologicPrompt {
    pub system_payload: String,
    pub user_payload: String,
    pub term_origin: NonologyTerm,
}
impl OntologicPrompt {
    pub fn to_llm_format(&self) -> String {
        format!(
            "=== SYSTEM ===\n{}\n\n=== USER ===\n{}",
            self.system_payload, self.user_payload
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologicViolation {
    pub rule: String,
    pub reason: String,
    pub fix: String,
}

pub trait OntologicLoopDriver: Send + Sync {
    fn evaluate_intent(&self, intent: &str) -> OntologicBounds;
    fn validate_output(
        &self,
        output: &str,
        bounds: &OntologicBounds,
    ) -> Result<(), OntologicViolation>;
}
pub trait ContextLoopDriver: Send + Sync {
    fn retrieve_context(&self, intent: &str) -> ContextualState;
    fn update_state(&self, output: &str) -> ContextualState;
}
pub trait PromptOntologicBuilder: Send + Sync {
    fn synthesize(
        &self,
        bounds: &OntologicBounds,
        context: &ContextualState,
        task: &str,
    ) -> OntologicPrompt;
}

pub struct DefaultOntologicDriver;
impl OntologicLoopDriver for DefaultOntologicDriver {
    fn evaluate_intent(&self, intent: &str) -> OntologicBounds {
        let mut b = OntologicBounds::default();
        if intent.to_lowercase().contains("medical") || intent.to_lowercase().contains("dosage") {
            b.forbidden_concepts.push("dosage".into());
            b.system_constraints.push("NEVER suggest dosages".into());
        }
        b
    }
    fn validate_output(
        &self,
        output: &str,
        bounds: &OntologicBounds,
    ) -> Result<(), OntologicViolation> {
        for c in &bounds.forbidden_concepts {
            if output.to_lowercase().contains(&c.to_lowercase()) {
                return Err(OntologicViolation {
                    rule: c.clone(),
                    reason: format!("Forbidden: {}", c),
                    fix: "Remove".into(),
                });
            }
        }
        Ok(())
    }
}

pub struct DefaultContextDriver {
    pub rag: Vec<String>,
}
impl DefaultContextDriver {
    pub fn new(rag: Vec<String>) -> Self {
        Self { rag }
    }
}
impl ContextLoopDriver for DefaultContextDriver {
    fn retrieve_context(&self, intent: &str) -> ContextualState {
        let mut s = ContextualState::default();
        for d in &self.rag {
            if d.to_lowercase().contains(
                intent
                    .to_lowercase()
                    .split_whitespace()
                    .next()
                    .unwrap_or(""),
            ) {
                s.rag_snippets.push(d.clone());
            }
        }
        s.working_memory.push(format!("Task: {}", intent));
        s
    }
    fn update_state(&self, output: &str) -> ContextualState {
        // Ralph Wiggum Loop: always return a fresh state, do not accumulate memory!
        let mut s = ContextualState::default();
        s.working_memory.push(output.to_string());
        s
    }
}

pub struct DefaultPromptBuilder;
impl PromptOntologicBuilder for DefaultPromptBuilder {
    fn synthesize(
        &self,
        bounds: &OntologicBounds,
        context: &ContextualState,
        task: &str,
    ) -> OntologicPrompt {
        let mut sys = "You are ARKHE.
== CONSTRAINTS ==
"
        .to_string();
        for c in &bounds.system_constraints {
            sys.push_str(&format!(
                "- {}
",
                c
            ));
        }
        if !bounds.forbidden_concepts.is_empty() {
            sys.push_str(&format!(
                "FORBIDDEN: {}
",
                bounds.forbidden_concepts.join(", ")
            ));
        }

        // NOPE (Normative Ontological Prompt Engineering) activation
        sys.push_str(
            "
== NOPE ACTIVATION ==
",
        );
        sys.push_str(
            "Generate the ontological. Respect the phenomenological.
",
        );
        sys.push_str(
            "Execute the categorical. Maintain the hermeneutic.
",
        );
        sys.push_str(
            "Preserve the ergodic. Uphold the systemic.
",
        );

        let mut usr = String::new();
        if !context.rag_snippets.is_empty() {
            usr.push_str(
                "== CONTEXT ==
",
            );
            for s in &context.rag_snippets {
                usr.push_str(&format!(
                    "- {}
",
                    s
                ));
            }
        }
        usr.push_str(&format!(
            "== TASK ==
{}",
            task
        ));
        OntologicPrompt {
            system_payload: sys,
            user_payload: usr,
            term_origin: NonologyTerm::PromptOntologic,
        }
    }
}

pub struct TriadicEngine<O, C, P>
where
    O: OntologicLoopDriver,
    C: ContextLoopDriver,
    P: PromptOntologicBuilder,
{
    ontologic: O,
    context: C,
    prompt: P,
    max_retries: u32,
    steps: Vec<TriadicStep>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadicStep {
    pub phase: String,
    pub output: String,
    pub duration_ms: u64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriadicMetrics {
    pub total_steps: u32,
    pub violations_caught: u32,
}

impl<O, C, P> TriadicEngine<O, C, P>
where
    O: OntologicLoopDriver,
    C: ContextLoopDriver,
    P: PromptOntologicBuilder,
{
    pub fn new(o: O, c: C, p: P) -> Self {
        Self {
            ontologic: o,
            context: c,
            prompt: p,
            max_retries: 3,
            steps: Vec::new(),
        }
    }
    pub fn execute_step(
        &mut self,
        task: &str,
        llm: impl Fn(&OntologicPrompt) -> LoopResult,
    ) -> LoopResult {
        let start = std::time::Instant::now();
        let bounds = self.ontologic.evaluate_intent(task);
        self.steps.push(TriadicStep {
            phase: "OntologicEval".into(),
            output: format!("{} forbidden", bounds.forbidden_concepts.len()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
        let ctx = self.context.retrieve_context(task);
        self.steps.push(TriadicStep {
            phase: "ContextLoad".into(),
            output: format!("{} snippets", ctx.rag_snippets.len()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
        let mut retries = 0;
        loop {
            let p = self.prompt.synthesize(&bounds, &ctx, task);
            self.steps.push(TriadicStep {
                phase: "PromptSynthesis".into(),
                output: format!("{}b sys", p.system_payload.len()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
            let raw = llm(&p)?;
            match self.ontologic.validate_output(&raw, &bounds) {
                Ok(()) => {
                    self.steps.push(TriadicStep {
                        phase: "Validation".into(),
                        output: "PASSED".into(),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                    let _ = self.context.update_state(&raw);
                    return Ok(raw);
                }
                Err(v) => {
                    retries += 1;
                    if retries >= self.max_retries {
                        return Err(format!("Violation: {}", v.reason));
                    }
                    warn!("Retry: {}", v.rule);
                }
            }
        }
    }
    pub fn steps(&self) -> &[TriadicStep] {
        &self.steps
    }
}
