//! # PVST — Typestate Wrappers
//!
//! Compile-time enforcement of state machine transitions using the
//! typestate pattern. Invalid transitions are compile errors, not
//! runtime failures.
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role      | Weight |
//! |--------|-----------|--------|
//! | ς      | State     | 0.80 (dominant) |
//! | ∂      | Boundary  | 0.10   |
//! | ∅      | Void      | 0.10   |
//!
//! Typestate wrappers use sealed traits (∂) and `PhantomData` (∅)
//! to encode state (ς) at the type level.
//!
//! ## Key Pattern
//!
//! ```text
//! TypesafeEntity<StateA> → transition() → TypesafeEntity<StateB>
//!                         (consumes self, returns new state)
//! ```
//!
//! Methods only exist on valid source states, making invalid
//! transitions impossible to express.
//!
//! ## ToV Axiom Correspondence
//!
//! | Axiom | Pattern Element |
//! |-------|-----------------|
//! | A1 (Decomposition) | Each state is a unit type: finite, enumerable |
//! | A2 (Hierarchy) | States form progression: Initial → ... → Terminal |
//! | A3 (Conservation) | `self` consumed: exactly 1 active instance |
//! | A4 (Manifold) | Interior = valid states; Boundary = sealed trait |
//! | A5 (Emergence) | Composition: chain transitions to reach outcome |

pub mod case_lifecycle;
pub mod signal_lifecycle;
pub mod submission_lifecycle;
pub mod workflow_lifecycle;

// Re-export primary types
pub use case_lifecycle::TypesafeCase;
pub use signal_lifecycle::TypesafeSignal;
pub use submission_lifecycle::TypesafeSubmission;
pub use workflow_lifecycle::TypesafeWorkflow;

// Re-export state markers
pub use case_lifecycle::{CaseAssessed, CaseClosed, CaseReceived, CaseTriaged};
pub use signal_lifecycle::{
    SignalActioned, SignalConfirmed, SignalDetected, SignalEvaluated, SignalMonitoring,
    SignalRefuted, SignalValidated,
};
pub use submission_lifecycle::{
    SubmissionAcknowledged, SubmissionDraft, SubmissionSent, SubmissionSigned, SubmissionValidated,
};
pub use workflow_lifecycle::{WorkflowCompleted, WorkflowFailed, WorkflowPending, WorkflowRunning};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

/// Marker trait for lifecycle states (sealed).
///
/// Only states defined in this module can implement this trait,
/// preventing external code from creating invalid states.
///
/// Tier: T2-P (ς + ∂)
pub trait LifecycleState: private::Sealed {
    /// Human-readable state name.
    fn name() -> &'static str;

    /// Whether this state is terminal (no outgoing transitions).
    fn is_terminal() -> bool;

    /// Whether this state is initial (entry point).
    fn is_initial() -> bool;
}

/// Sealed trait module — prevents external implementations.
mod private {
    /// Sealed trait to prevent external implementations.
    pub trait Sealed {}

    // Case states
    impl Sealed for super::CaseReceived {}
    impl Sealed for super::CaseTriaged {}
    impl Sealed for super::CaseAssessed {}
    impl Sealed for super::CaseClosed {}

    // Signal states
    impl Sealed for super::SignalDetected {}
    impl Sealed for super::SignalEvaluated {}
    impl Sealed for super::SignalValidated {}
    impl Sealed for super::SignalActioned {}
    impl Sealed for super::SignalMonitoring {}
    impl Sealed for super::SignalConfirmed {}
    impl Sealed for super::SignalRefuted {}

    // Workflow states
    impl Sealed for super::WorkflowPending {}
    impl Sealed for super::WorkflowRunning {}
    impl Sealed for super::WorkflowCompleted {}
    impl Sealed for super::WorkflowFailed {}

    // Submission states
    impl Sealed for super::SubmissionDraft {}
    impl Sealed for super::SubmissionValidated {}
    impl Sealed for super::SubmissionSigned {}
    impl Sealed for super::SubmissionSent {}
    impl Sealed for super::SubmissionAcknowledged {}
}

/// GroundsTo for the typestate pattern category.
///
/// Tier: T2-P (ς + ∂)
pub struct TypestatePattern;

impl GroundsTo for TypestatePattern {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,    // ς — DOMINANT: compile-time state
            LexPrimitiva::Boundary, // ∂ — sealed trait prevents invalid states
            LexPrimitiva::Void,     // ∅ — PhantomData has no runtime cost
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
    fn test_typestate_pattern_grounding() {
        let comp = TypestatePattern::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 3);
    }
}
