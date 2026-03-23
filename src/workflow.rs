//! # PVWF Workflow DSL
//!
//! Declarative workflow definitions for composing PVOS syscalls.
//! Workflows are serializable data structures, not code — enabling
//! persistence, replay, and inspection.
//!
//! ## Primitive: σ (Sequence)
//!
//! A workflow is fundamentally an ordered sequence of steps.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

/// Workflow identifier.
/// Tier: T2-P (N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub u64);

impl GroundsTo for WorkflowId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
    }
}

/// Step identifier within a workflow.
/// Tier: T2-P (N + σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(pub usize);

impl GroundsTo for StepId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Sequence])
    }
}

/// PVOS syscall kinds (maps to Pvos methods).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyscallKind {
    /// detect(drug, event, algo, contingency)
    Detect,
    /// compare(observed, expected, threshold)
    Compare,
    /// ingest(source, raw)
    Ingest,
    /// route(case, rules)
    Route,
    /// prioritize(cases)
    Prioritize,
    /// store(artifact)
    Store,
    /// query(filter)
    Query,
    /// feedback(signal, outcome)
    Feedback,
    /// retrain()
    Retrain,
}

/// Branch condition — serializable predicate on step output.
/// Uses enum variants instead of closures to enable serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BranchCondition {
    /// Continue if previous step detected a signal.
    SignalDetected,
    /// Continue if case has seriousness criteria.
    IsSerious,
    /// Continue if statistic exceeds threshold.
    StatisticAbove(f64),
    /// Always continue (unconditional).
    Always,
    /// Never continue (skip).
    Never,
}

impl BranchCondition {
    /// Evaluates the condition against a step output description.
    #[must_use]
    pub fn evaluate(&self, output: &StepOutput) -> bool {
        match self {
            Self::SignalDetected => output.is_signal(),
            Self::IsSerious => output.is_serious(),
            Self::StatisticAbove(thresh) => output.statistic().map_or(false, |s| s >= *thresh),
            Self::Always => true,
            Self::Never => false,
        }
    }
}

/// A single step in a workflow.
/// Tier: T2-P (σ + →)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Step {
    /// Call a PVOS syscall.
    Syscall { name: String, kind: SyscallKind },
    /// Conditional branch: evaluate condition on previous output.
    /// If false, skip `skip_count` subsequent steps.
    Branch {
        name: String,
        condition: BranchCondition,
        skip_count: usize,
    },
    /// Execute multiple steps (conceptually in parallel).
    Parallel { name: String, steps: Vec<Step> },
    /// Wait for human approval.
    AwaitHuman {
        name: String,
        timeout_secs: Option<u64>,
    },
    /// Retry loop: execute body up to max_iterations times.
    Loop {
        name: String,
        body: Vec<Step>,
        max_iterations: usize,
    },
}

impl Step {
    /// Creates a syscall step.
    #[must_use]
    pub fn syscall(name: &str, kind: SyscallKind) -> Self {
        Self::Syscall {
            name: name.to_string(),
            kind,
        }
    }

    /// Creates a branch step.
    #[must_use]
    pub fn branch(name: &str, condition: BranchCondition, skip_count: usize) -> Self {
        Self::Branch {
            name: name.to_string(),
            condition,
            skip_count,
        }
    }

    /// Creates a human-await step.
    #[must_use]
    pub fn await_human(name: &str, timeout_secs: Option<u64>) -> Self {
        Self::AwaitHuman {
            name: name.to_string(),
            timeout_secs,
        }
    }

    /// Creates a loop step.
    #[must_use]
    pub fn loop_step(name: &str, body: Vec<Step>, max_iterations: usize) -> Self {
        Self::Loop {
            name: name.to_string(),
            body,
            max_iterations,
        }
    }

    /// Creates a parallel step.
    #[must_use]
    pub fn parallel(name: &str, steps: Vec<Step>) -> Self {
        Self::Parallel {
            name: name.to_string(),
            steps,
        }
    }

    /// Returns the step name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Syscall { name, .. }
            | Self::Branch { name, .. }
            | Self::Parallel { name, .. }
            | Self::AwaitHuman { name, .. }
            | Self::Loop { name, .. } => name,
        }
    }
}

impl GroundsTo for Step {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Causality])
    }
}

/// Output produced by a step during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepOutput {
    /// Signal detection result.
    Signal {
        detected: bool,
        statistic: f64,
        drug: String,
        event: String,
    },
    /// Comparison result.
    Comparison { exceeded: bool, delta: f64 },
    /// Case ingested.
    CaseIngested { case_id: u64, serious: bool },
    /// Case routed.
    Routed { destination: String },
    /// Artifact stored.
    Stored { artifact_id: u64 },
    /// Human approval received.
    HumanApproval(bool),
    /// Step completed with no specific output.
    Completed,
    /// Step was skipped (branch condition false).
    Skipped,
}

impl StepOutput {
    /// Returns true if this output indicates a signal was detected.
    #[must_use]
    pub fn is_signal(&self) -> bool {
        matches!(self, Self::Signal { detected: true, .. })
    }

    /// Returns true if this output indicates seriousness.
    #[must_use]
    pub fn is_serious(&self) -> bool {
        matches!(self, Self::CaseIngested { serious: true, .. })
    }

    /// Returns the statistic value if available.
    #[must_use]
    pub fn statistic(&self) -> Option<f64> {
        match self {
            Self::Signal { statistic, .. } => Some(*statistic),
            Self::Comparison { delta, .. } => Some(*delta),
            _ => None,
        }
    }
}

/// A complete workflow definition.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name.
    pub name: String,
    /// Ordered steps.
    pub steps: Vec<Step>,
    /// Description of what this workflow does.
    pub description: String,
}

impl Workflow {
    /// Creates a builder for fluent workflow construction.
    #[must_use]
    pub fn builder(name: &str) -> WorkflowBuilder {
        WorkflowBuilder {
            name: name.to_string(),
            steps: Vec::new(),
            description: String::new(),
        }
    }

    /// Number of steps.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Returns step names in order.
    #[must_use]
    pub fn step_names(&self) -> Vec<&str> {
        self.steps.iter().map(|s| s.name()).collect()
    }
}

impl GroundsTo for Workflow {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

/// Fluent builder for workflow construction.
pub struct WorkflowBuilder {
    name: String,
    steps: Vec<Step>,
    description: String,
}

impl WorkflowBuilder {
    /// Adds a description.
    #[must_use]
    pub fn description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Adds a step.
    #[must_use]
    pub fn step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    /// Adds a syscall step (convenience).
    #[must_use]
    pub fn syscall(self, name: &str, kind: SyscallKind) -> Self {
        self.step(Step::syscall(name, kind))
    }

    /// Adds a branch step (convenience).
    #[must_use]
    pub fn branch(self, name: &str, condition: BranchCondition, skip: usize) -> Self {
        self.step(Step::branch(name, condition, skip))
    }

    /// Adds a human-await step (convenience).
    #[must_use]
    pub fn await_human(self, name: &str, timeout_secs: Option<u64>) -> Self {
        self.step(Step::await_human(name, timeout_secs))
    }

    /// Builds the workflow.
    #[must_use]
    pub fn build(self) -> Workflow {
        Workflow {
            name: self.name,
            steps: self.steps,
            description: self.description,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_workflow_builder() {
        let wf = Workflow::builder("test")
            .description("A test workflow")
            .syscall("detect", SyscallKind::Detect)
            .syscall("store", SyscallKind::Store)
            .build();

        assert_eq!(wf.name, "test");
        assert_eq!(wf.step_count(), 2);
        assert_eq!(wf.step_names(), vec!["detect", "store"]);
    }

    #[test]
    fn test_workflow_with_branch() {
        let wf = Workflow::builder("branching")
            .syscall("detect", SyscallKind::Detect)
            .branch("check_signal", BranchCondition::SignalDetected, 1)
            .syscall("alert", SyscallKind::Route)
            .syscall("audit", SyscallKind::Store)
            .build();

        assert_eq!(wf.step_count(), 4);
    }

    #[test]
    fn test_workflow_with_human_await() {
        let wf = Workflow::builder("human_review")
            .syscall("detect", SyscallKind::Detect)
            .await_human("review", Some(300))
            .syscall("store", SyscallKind::Store)
            .build();

        assert_eq!(wf.step_count(), 3);
    }

    #[test]
    fn test_workflow_serialization() {
        let wf = Workflow::builder("serial")
            .syscall("detect", SyscallKind::Detect)
            .branch("check", BranchCondition::StatisticAbove(2.0), 1)
            .build();

        let json = serde_json::to_string(&wf);
        assert!(json.is_ok());
        if let Ok(j) = json {
            let deserialized: Result<Workflow, _> = serde_json::from_str(&j);
            assert!(deserialized.is_ok());
            if let Ok(d) = deserialized {
                assert_eq!(d.name, "serial");
                assert_eq!(d.step_count(), 2);
            }
        }
    }

    #[test]
    fn test_step_output_predicates() {
        let signal = StepOutput::Signal {
            detected: true,
            statistic: 3.5,
            drug: "aspirin".into(),
            event: "headache".into(),
        };
        assert!(signal.is_signal());
        assert!(!signal.is_serious());
        assert_eq!(signal.statistic(), Some(3.5));

        let no_signal = StepOutput::Signal {
            detected: false,
            statistic: 1.0,
            drug: "x".into(),
            event: "y".into(),
        };
        assert!(!no_signal.is_signal());

        let case = StepOutput::CaseIngested {
            case_id: 1,
            serious: true,
        };
        assert!(case.is_serious());
        assert!(!case.is_signal());
    }

    #[test]
    fn test_branch_condition_evaluate() {
        let signal_output = StepOutput::Signal {
            detected: true,
            statistic: 3.5,
            drug: "x".into(),
            event: "y".into(),
        };

        assert!(BranchCondition::SignalDetected.evaluate(&signal_output));
        assert!(BranchCondition::StatisticAbove(2.0).evaluate(&signal_output));
        assert!(!BranchCondition::StatisticAbove(4.0).evaluate(&signal_output));
        assert!(BranchCondition::Always.evaluate(&signal_output));
        assert!(!BranchCondition::Never.evaluate(&signal_output));
    }

    #[test]
    fn test_workflow_grounding() {
        let comp = Workflow::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sequence));
    }

    #[test]
    fn test_step_grounding() {
        let comp = Step::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_loop_step() {
        let body = vec![
            Step::syscall("detect", SyscallKind::Detect),
            Step::syscall("feedback", SyscallKind::Feedback),
        ];
        let step = Step::loop_step("retrain_loop", body, 5);
        assert_eq!(step.name(), "retrain_loop");
        if let Step::Loop {
            max_iterations,
            body,
            ..
        } = &step
        {
            assert_eq!(*max_iterations, 5);
            assert_eq!(body.len(), 2);
        }
    }

    #[test]
    fn test_parallel_step() {
        let steps = vec![
            Step::syscall("prr", SyscallKind::Detect),
            Step::syscall("ror", SyscallKind::Detect),
            Step::syscall("chi2", SyscallKind::Detect),
        ];
        let step = Step::parallel("multi_detect", steps);
        assert_eq!(step.name(), "multi_detect");
        if let Step::Parallel { steps, .. } = &step {
            assert_eq!(steps.len(), 3);
        }
    }
}
