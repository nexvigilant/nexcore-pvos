//! # TypesafeWorkflow — Compile-Time Workflow Lifecycle
//!
//! Workflow lifecycle with compile-time state enforcement:
//! `Pending → Running → Completed | Failed`
//! With retry: `Failed → Running` (non-terminal failed state)
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role      | Weight |
//! |--------|-----------|--------|
//! | ς      | State     | 0.80 (dominant) |
//! | ∂      | Boundary  | 0.10   |
//! | ρ      | Recursion | 0.05   |
//! | →      | Causality | 0.05   |
//!
//! ## ToV Axiom Mapping
//!
//! - **A1**: 4 states form finite decomposition
//! - **A2**: Pending < Running < {Completed, Failed}, Failed → Running (cycle)
//! - **A4**: Only Completed is terminal (absorbing boundary)

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use super::LifecycleState;
use crate::state::StateContext;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// STATE MARKERS
// ═══════════════════════════════════════════════════════════

/// Workflow is queued but not started.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkflowPending;

impl LifecycleState for WorkflowPending {
    fn name() -> &'static str {
        "pending"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        true
    }
}

/// Workflow is actively executing.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkflowRunning;

impl LifecycleState for WorkflowRunning {
    fn name() -> &'static str {
        "running"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Workflow completed successfully (terminal).
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkflowCompleted;

impl LifecycleState for WorkflowCompleted {
    fn name() -> &'static str {
        "completed"
    }
    fn is_terminal() -> bool {
        true
    }
    fn is_initial() -> bool {
        false
    }
}

/// Workflow failed (NOT terminal — can retry).
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkflowFailed;

impl LifecycleState for WorkflowFailed {
    fn name() -> &'static str {
        "failed"
    }
    fn is_terminal() -> bool {
        false
    } // Can retry!
    fn is_initial() -> bool {
        false
    }
}

// ═══════════════════════════════════════════════════════════
// TYPESAFE WORKFLOW
// ═══════════════════════════════════════════════════════════

/// Workflow lifecycle wrapper with compile-time state enforcement.
///
/// Tier: T2-C (ς + ∂ + ρ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesafeWorkflow<S: LifecycleState> {
    /// Entity identifier.
    pub entity_id: u64,
    /// Workflow name.
    pub name: String,
    /// Context data.
    pub context: StateContext,
    /// Number of transitions applied.
    pub transition_count: u64,
    /// Number of retry attempts.
    pub retry_count: u64,
    /// State marker.
    #[serde(skip)]
    _state: PhantomData<S>,
}

impl<S: LifecycleState> TypesafeWorkflow<S> {
    /// Returns the current state name.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        S::name()
    }

    /// Returns whether the workflow is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        S::is_terminal()
    }

    /// Returns the entity ID.
    #[must_use]
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }

    /// Returns the workflow name.
    #[must_use]
    pub fn workflow_name(&self) -> &str {
        &self.name
    }

    /// Returns the retry count.
    #[must_use]
    pub fn retry_count(&self) -> u64 {
        self.retry_count
    }
}

impl TypesafeWorkflow<WorkflowPending> {
    /// Creates a new workflow in the Pending state.
    #[must_use]
    pub fn new(entity_id: u64, name: &str, timestamp: u64) -> Self {
        Self {
            entity_id,
            name: name.to_string(),
            context: StateContext::new(entity_id, timestamp),
            transition_count: 0,
            retry_count: 0,
            _state: PhantomData,
        }
    }

    /// Start the workflow → transitions to Running.
    #[must_use]
    pub fn start(self, timestamp: u64) -> TypesafeWorkflow<WorkflowRunning> {
        TypesafeWorkflow {
            entity_id: self.entity_id,
            name: self.name,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            retry_count: self.retry_count,
            _state: PhantomData,
        }
    }
}

impl TypesafeWorkflow<WorkflowRunning> {
    /// Complete the workflow → transitions to Completed (terminal).
    #[must_use]
    pub fn complete(self, timestamp: u64) -> TypesafeWorkflow<WorkflowCompleted> {
        TypesafeWorkflow {
            entity_id: self.entity_id,
            name: self.name,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            retry_count: self.retry_count,
            _state: PhantomData,
        }
    }

    /// Fail the workflow → transitions to Failed (can retry).
    #[must_use]
    pub fn fail(self, timestamp: u64) -> TypesafeWorkflow<WorkflowFailed> {
        TypesafeWorkflow {
            entity_id: self.entity_id,
            name: self.name,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            retry_count: self.retry_count,
            _state: PhantomData,
        }
    }
}

impl TypesafeWorkflow<WorkflowFailed> {
    /// Retry the workflow → transitions back to Running.
    ///
    /// This demonstrates the ρ (recursion) primitive: cycles allowed.
    #[must_use]
    pub fn retry(self, timestamp: u64) -> TypesafeWorkflow<WorkflowRunning> {
        TypesafeWorkflow {
            entity_id: self.entity_id,
            name: self.name,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            retry_count: self.retry_count + 1,
            _state: PhantomData,
        }
    }
}

// No transition methods on WorkflowCompleted — true terminal state.

impl<S: LifecycleState> GroundsTo for TypesafeWorkflow<S> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,
            LexPrimitiva::Boundary,
            LexPrimitiva::Recursion, // ρ — retry loop
            LexPrimitiva::Causality,
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_typesafe_workflow_grounding() {
        let comp = TypesafeWorkflow::<WorkflowPending>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_workflow_complete_path() {
        let wf = TypesafeWorkflow::<WorkflowPending>::new(300, "signal_detection", 1000);
        assert_eq!(wf.state_name(), "pending");

        let wf = wf.start(2000);
        assert_eq!(wf.state_name(), "running");

        let wf = wf.complete(3000);
        assert_eq!(wf.state_name(), "completed");
        assert!(wf.is_terminal());
    }

    #[test]
    fn test_workflow_fail_retry() {
        let wf = TypesafeWorkflow::<WorkflowPending>::new(301, "case_processing", 1000);
        let wf = wf.start(2000);
        let wf = wf.fail(3000);

        assert_eq!(wf.state_name(), "failed");
        assert!(!wf.is_terminal()); // Failed is NOT terminal
        assert_eq!(wf.retry_count(), 0);

        let wf = wf.retry(4000);
        assert_eq!(wf.state_name(), "running");
        assert_eq!(wf.retry_count(), 1);

        let wf = wf.complete(5000);
        assert!(wf.is_terminal());
        assert_eq!(wf.transition_count, 4);
    }

    #[test]
    fn test_multiple_retries() {
        let wf = TypesafeWorkflow::<WorkflowPending>::new(302, "flaky_job", 1000);
        let wf = wf.start(2000);
        let wf = wf.fail(3000);
        let wf = wf.retry(4000);
        let wf = wf.fail(5000);
        let wf = wf.retry(6000);
        let wf = wf.complete(7000);

        assert_eq!(wf.retry_count(), 2);
        assert_eq!(wf.transition_count, 6);
    }

    #[test]
    fn test_state_markers() {
        assert!(WorkflowPending::is_initial());
        assert!(!WorkflowFailed::is_terminal()); // Can retry
        assert!(WorkflowCompleted::is_terminal());
    }
}
