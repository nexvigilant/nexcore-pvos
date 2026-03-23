//! # PVOS System Calls
//!
//! The API surface of the Pharmacovigilance Operating System.
//! System calls are grouped into 5 trait families:
//!
//! - **Detection** (κ): Signal detection and comparison
//! - **Case** (σ + μ): Case ingestion, routing, prioritization
//! - **Persistence** (π): Audited storage and retrieval
//! - **Workflow** (→ + σ): Process lifecycle management
//! - **Learning** (ρ): Feedback and model improvement
//!
//! These traits define what a PVOS provides. The kernel implements them.
//! User-space applications program against these traits.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::PvosError;

// ═══════════════════════════════════════════════════════════
// TYPES USED IN SYSTEM CALLS
// ═══════════════════════════════════════════════════════════

/// Detection algorithm selector.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Algorithm {
    /// Proportional Reporting Ratio
    Prr,
    /// Reporting Odds Ratio
    Ror,
    /// Information Component (Bayesian)
    Ic,
    /// Empirical Bayesian Geometric Mean
    Ebgm,
    /// Chi-Squared test
    ChiSquared,
    /// Fisher's Exact Test
    Fisher,
    /// Custom algorithm by name
    Custom(String),
}

/// Signal detection result.
/// Tier: T2-C (κ + ∂ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalResult {
    /// Drug-event pair tested.
    pub drug: String,
    pub event: String,
    /// Algorithm used.
    pub algorithm: Algorithm,
    /// Computed statistic value.
    pub statistic: f64,
    /// Whether signal threshold was exceeded.
    pub signal_detected: bool,
    /// Confidence interval lower bound (if applicable).
    pub ci_lower: Option<f64>,
    /// Confidence interval upper bound (if applicable).
    pub ci_upper: Option<f64>,
}

impl GroundsTo for SignalResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison,
            LexPrimitiva::Boundary,
            LexPrimitiva::Existence,
        ])
        .with_dominant(LexPrimitiva::Comparison, 0.90)
    }
}

/// Comparison result (observed vs expected).
/// Tier: T1 (κ)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub observed: f64,
    pub expected: f64,
    pub delta: f64,
    pub exceeded: bool,
}

impl GroundsTo for ComparisonResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
    }
}

/// Reference to an ingested case.
/// Tier: T2-P (N + π)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaseRef(pub u64);

impl GroundsTo for CaseRef {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Persistence])
    }
}

/// Routing destination for a case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Destination {
    /// Route to automated processing.
    Auto(String),
    /// Route to human reviewer.
    Human(String),
    /// Route to external system.
    External(String),
    /// Archive (no action needed).
    Archive,
}

/// Routing rules for case triage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRules {
    /// Seriousness criteria that trigger human review.
    pub serious_criteria: Vec<String>,
    /// Domains requiring automated processing.
    pub auto_domains: Vec<String>,
    /// Default destination when no rule matches.
    pub default: Destination,
}

impl Default for RoutingRules {
    fn default() -> Self {
        Self {
            serious_criteria: vec![
                "death".into(),
                "hospitalization".into(),
                "life-threatening".into(),
                "disability".into(),
                "congenital-anomaly".into(),
            ],
            auto_domains: Vec::new(),
            default: Destination::Auto("general".into()),
        }
    }
}

/// Priority level for scheduling.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Background processing.
    Low = 0,
    /// Standard processing.
    Normal = 1,
    /// Expedited processing.
    High = 2,
    /// Immediate processing (regulatory deadline).
    Critical = 3,
}

impl GroundsTo for Priority {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

/// Artifact for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Artifact kind.
    pub kind: ArtifactKind,
    /// Serialized content.
    pub content: String,
    /// Metadata tags.
    pub tags: Vec<String>,
}

/// Kinds of storable artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    Case,
    Signal,
    Report,
    Assessment,
    Decision,
    Custom(String),
}

/// Reference to a stored, audited artifact.
/// Tier: T2-P (π + N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditedRef {
    /// Unique artifact ID.
    pub id: u64,
    /// Integrity hash.
    pub hash: u64,
}

impl GroundsTo for AuditedRef {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Persistence, LexPrimitiva::Quantity])
    }
}

/// Filter for querying artifacts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Filter by kind.
    pub kind: Option<ArtifactKind>,
    /// Filter by tag.
    pub tags: Vec<String>,
    /// Maximum results.
    pub limit: Option<usize>,
}

/// Process reference for workflow management.
/// Tier: T2-P (σ + N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessRef(pub u64);

impl GroundsTo for ProcessRef {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Quantity])
    }
}

/// Workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDef {
    /// Workflow name.
    pub name: String,
    /// Steps in the workflow.
    pub steps: Vec<WorkflowStep>,
    /// Priority.
    pub priority: Priority,
}

/// A single step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step name.
    pub name: String,
    /// System call to invoke.
    pub syscall: String,
    /// Whether this step requires human approval.
    pub requires_human: bool,
}

/// Process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    /// Waiting to be scheduled.
    Pending,
    /// Currently executing.
    Running,
    /// Waiting for human input.
    AwaitingHuman,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
}

/// Outcome for learning feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningOutcome {
    /// Signal was real (confirmed by investigation).
    Confirmed,
    /// Signal was not real (refuted by investigation).
    Refuted,
    /// Insufficient evidence to determine.
    Indeterminate,
}

// ═══════════════════════════════════════════════════════════
// SYSTEM CALL TRAITS
// ═══════════════════════════════════════════════════════════

/// Detection system calls (κ).
/// Provides signal detection and metric comparison.
pub trait DetectionSyscall {
    /// Detect a signal for a drug-event pair using specified algorithm.
    fn detect(
        &self,
        drug: &str,
        event: &str,
        algo: Algorithm,
        contingency: [u64; 4],
    ) -> Result<SignalResult, PvosError>;

    /// Compare observed vs expected with threshold.
    fn compare(&self, observed: f64, expected: f64, threshold: f64) -> ComparisonResult;
}

/// Case management system calls (σ + μ).
/// Handles case lifecycle from ingestion to routing.
pub trait CaseSyscall {
    /// Ingest a case from a data source.
    fn ingest(&mut self, source: &str, data: &str) -> Result<CaseRef, PvosError>;

    /// Route a case based on rules.
    fn route(&self, case: CaseRef, rules: &RoutingRules) -> Result<Destination, PvosError>;

    /// Prioritize a set of cases.
    fn prioritize(&self, cases: &[CaseRef]) -> Vec<CaseRef>;
}

/// Persistence system calls (π).
/// Audited storage and retrieval.
pub trait PersistenceSyscall {
    /// Store an artifact with automatic audit.
    fn store(&mut self, artifact: Artifact) -> Result<AuditedRef, PvosError>;

    /// Query artifacts by filter.
    fn query(&self, filter: &Filter) -> Vec<Artifact>;
}

/// Workflow system calls (→ + σ).
/// Process lifecycle management.
pub trait WorkflowSyscall {
    /// Spawn a new workflow process.
    fn spawn(&mut self, workflow: WorkflowDef) -> Result<ProcessRef, PvosError>;

    /// Schedule a process at given priority.
    fn schedule(&mut self, process: ProcessRef, priority: Priority) -> Result<(), PvosError>;

    /// Get the current state of a process.
    fn process_state(&self, process: ProcessRef) -> Result<ProcessState, PvosError>;
}

/// Learning system calls (ρ).
/// Feedback collection and model improvement.
pub trait LearningSyscall {
    /// Record feedback on a detection result.
    fn feedback(&mut self, signal: &SignalResult, outcome: LearningOutcome);

    /// Trigger model retraining from accumulated feedback.
    fn retrain(&mut self) -> Result<(), PvosError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_signal_result_grounding() {
        let comp = SignalResult::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Comparison));
    }

    #[test]
    fn test_comparison_result_grounding() {
        let comp = ComparisonResult::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_default_routing_rules() {
        let rules = RoutingRules::default();
        assert_eq!(rules.serious_criteria.len(), 5);
        assert_eq!(rules.default, Destination::Auto("general".into()));
    }

    #[test]
    fn test_audited_ref_grounding() {
        let comp = AuditedRef::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_process_ref_grounding() {
        let comp = ProcessRef::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
