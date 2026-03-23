//! # TypesafeSubmission — Compile-Time Submission Lifecycle
//!
//! Submission lifecycle with compile-time state enforcement:
//! `Draft → Validated → Signed → Sent → Acknowledged`
//!
//! Linear progression with 5 states, demonstrating the longest
//! typestate chain in the PVST layer.
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role      | Weight |
//! |--------|-----------|--------|
//! | ς      | State     | 0.80 (dominant) |
//! | σ      | Sequence  | 0.10   |
//! | ∂      | Boundary  | 0.05   |
//! | →      | Causality | 0.05   |
//!
//! ## ToV Axiom Mapping
//!
//! - **A1**: 5 states form finite decomposition (MAX_STATES = 5)
//! - **A2**: Strictly linear: Draft < Validated < Signed < Sent < Acknowledged
//! - **A4**: Acknowledged is terminal (absorbing boundary)

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use super::LifecycleState;
use crate::state::StateContext;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// STATE MARKERS
// ═══════════════════════════════════════════════════════════

/// Submission is being drafted.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubmissionDraft;

impl LifecycleState for SubmissionDraft {
    fn name() -> &'static str {
        "draft"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        true
    }
}

/// Submission has been validated.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubmissionValidated;

impl LifecycleState for SubmissionValidated {
    fn name() -> &'static str {
        "validated"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Submission has been digitally signed.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubmissionSigned;

impl LifecycleState for SubmissionSigned {
    fn name() -> &'static str {
        "signed"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Submission has been sent to authority.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubmissionSent;

impl LifecycleState for SubmissionSent {
    fn name() -> &'static str {
        "sent"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Submission has been acknowledged by authority (terminal).
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubmissionAcknowledged;

impl LifecycleState for SubmissionAcknowledged {
    fn name() -> &'static str {
        "acknowledged"
    }
    fn is_terminal() -> bool {
        true
    }
    fn is_initial() -> bool {
        false
    }
}

// ═══════════════════════════════════════════════════════════
// TYPESAFE SUBMISSION
// ═══════════════════════════════════════════════════════════

/// Submission lifecycle wrapper with compile-time state enforcement.
///
/// Tier: T2-C (ς + σ + ∂ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesafeSubmission<S: LifecycleState> {
    /// Entity identifier.
    pub entity_id: u64,
    /// Submission type (e.g., "ICSR", "PSUR").
    pub submission_type: String,
    /// Authority destination (e.g., "FDA", "EMA").
    pub authority: String,
    /// Context data.
    pub context: StateContext,
    /// Number of transitions applied.
    pub transition_count: u64,
    /// Signer identity (set when signed).
    pub signer: Option<String>,
    /// Acknowledgment reference (set when acknowledged).
    pub ack_reference: Option<String>,
    /// State marker.
    #[serde(skip)]
    _state: PhantomData<S>,
}

impl<S: LifecycleState> TypesafeSubmission<S> {
    /// Returns the current state name.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        S::name()
    }

    /// Returns whether the submission is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        S::is_terminal()
    }

    /// Returns the entity ID.
    #[must_use]
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }

    /// Returns the submission type.
    #[must_use]
    pub fn submission_type(&self) -> &str {
        &self.submission_type
    }

    /// Returns the authority.
    #[must_use]
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// Returns the signer if available.
    #[must_use]
    pub fn signer(&self) -> Option<&str> {
        self.signer.as_deref()
    }

    /// Returns the acknowledgment reference if available.
    #[must_use]
    pub fn ack_reference(&self) -> Option<&str> {
        self.ack_reference.as_deref()
    }
}

impl TypesafeSubmission<SubmissionDraft> {
    /// Creates a new submission in the Draft state.
    #[must_use]
    pub fn new(entity_id: u64, submission_type: &str, authority: &str, timestamp: u64) -> Self {
        Self {
            entity_id,
            submission_type: submission_type.to_string(),
            authority: authority.to_string(),
            context: StateContext::new(entity_id, timestamp),
            transition_count: 0,
            signer: None,
            ack_reference: None,
            _state: PhantomData,
        }
    }

    /// Validate the submission → transitions to Validated.
    #[must_use]
    pub fn validate(self, timestamp: u64) -> TypesafeSubmission<SubmissionValidated> {
        TypesafeSubmission {
            entity_id: self.entity_id,
            submission_type: self.submission_type,
            authority: self.authority,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            signer: self.signer,
            ack_reference: self.ack_reference,
            _state: PhantomData,
        }
    }
}

impl TypesafeSubmission<SubmissionValidated> {
    /// Sign the submission → transitions to Signed.
    #[must_use]
    pub fn sign(mut self, signer: &str, timestamp: u64) -> TypesafeSubmission<SubmissionSigned> {
        self.signer = Some(signer.to_string());
        TypesafeSubmission {
            entity_id: self.entity_id,
            submission_type: self.submission_type,
            authority: self.authority,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            signer: self.signer,
            ack_reference: self.ack_reference,
            _state: PhantomData,
        }
    }
}

impl TypesafeSubmission<SubmissionSigned> {
    /// Send the submission → transitions to Sent.
    #[must_use]
    pub fn send(self, timestamp: u64) -> TypesafeSubmission<SubmissionSent> {
        TypesafeSubmission {
            entity_id: self.entity_id,
            submission_type: self.submission_type,
            authority: self.authority,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            signer: self.signer,
            ack_reference: self.ack_reference,
            _state: PhantomData,
        }
    }
}

impl TypesafeSubmission<SubmissionSent> {
    /// Acknowledge the submission → transitions to Acknowledged (terminal).
    #[must_use]
    pub fn acknowledge(
        mut self,
        ack_ref: &str,
        timestamp: u64,
    ) -> TypesafeSubmission<SubmissionAcknowledged> {
        self.ack_reference = Some(ack_ref.to_string());
        TypesafeSubmission {
            entity_id: self.entity_id,
            submission_type: self.submission_type,
            authority: self.authority,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            signer: self.signer,
            ack_reference: self.ack_reference,
            _state: PhantomData,
        }
    }
}

// No transition methods on SubmissionAcknowledged — terminal state.

impl<S: LifecycleState> GroundsTo for TypesafeSubmission<S> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,
            LexPrimitiva::Sequence, // σ — linear progression
            LexPrimitiva::Boundary,
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
    fn test_typesafe_submission_grounding() {
        let comp = TypesafeSubmission::<SubmissionDraft>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_submission_full_lifecycle() {
        let sub = TypesafeSubmission::<SubmissionDraft>::new(400, "ICSR", "FDA", 1000);
        assert_eq!(sub.state_name(), "draft");
        assert!(sub.signer().is_none());

        let sub = sub.validate(2000);
        assert_eq!(sub.state_name(), "validated");

        let sub = sub.sign("Dr. Smith", 3000);
        assert_eq!(sub.state_name(), "signed");
        assert_eq!(sub.signer(), Some("Dr. Smith"));

        let sub = sub.send(4000);
        assert_eq!(sub.state_name(), "sent");

        let sub = sub.acknowledge("ACK-2026-001", 5000);
        assert_eq!(sub.state_name(), "acknowledged");
        assert!(sub.is_terminal());
        assert_eq!(sub.ack_reference(), Some("ACK-2026-001"));
        assert_eq!(sub.transition_count, 4);
    }

    #[test]
    fn test_submission_metadata() {
        let sub = TypesafeSubmission::<SubmissionDraft>::new(401, "PSUR", "EMA", 1000);
        assert_eq!(sub.submission_type(), "PSUR");
        assert_eq!(sub.authority(), "EMA");
        assert_eq!(sub.entity_id(), 401);
    }

    #[test]
    fn test_state_progression() {
        assert!(SubmissionDraft::is_initial());
        assert!(!SubmissionDraft::is_terminal());

        assert!(!SubmissionValidated::is_initial());
        assert!(!SubmissionValidated::is_terminal());

        assert!(!SubmissionSigned::is_initial());
        assert!(!SubmissionSigned::is_terminal());

        assert!(!SubmissionSent::is_initial());
        assert!(!SubmissionSent::is_terminal());

        assert!(!SubmissionAcknowledged::is_initial());
        assert!(SubmissionAcknowledged::is_terminal());
    }
}
