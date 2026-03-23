//! # TypesafeCase — Compile-Time Case Lifecycle
//!
//! Case lifecycle with compile-time state enforcement:
//! `Received → Triaged → Assessed → Closed`
//!
//! Invalid transitions (e.g., `Closed → Triage`) are compile errors.
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role      | Weight |
//! |--------|-----------|--------|
//! | ς      | State     | 0.80 (dominant) |
//! | ∂      | Boundary  | 0.10   |
//! | ∅      | Void      | 0.05   |
//! | →      | Causality | 0.05   |
//!
//! ## ToV Axiom Mapping
//!
//! - **A1**: 4 states form finite decomposition
//! - **A2**: Received < Triaged < Assessed < Closed (directed acyclic)
//! - **A4**: Closed is terminal (boundary); Received-Assessed are interior

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use super::LifecycleState;
use crate::state::StateContext;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// STATE MARKERS (Zero-Sized Types)
// ═══════════════════════════════════════════════════════════

/// Case has been received but not yet reviewed.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaseReceived;

impl LifecycleState for CaseReceived {
    fn name() -> &'static str {
        "received"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        true
    }
}

/// Case has been triaged for seriousness.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaseTriaged;

impl LifecycleState for CaseTriaged {
    fn name() -> &'static str {
        "triaged"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Case has been medically assessed.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaseAssessed;

impl LifecycleState for CaseAssessed {
    fn name() -> &'static str {
        "assessed"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Case is closed (terminal state).
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaseClosed;

impl LifecycleState for CaseClosed {
    fn name() -> &'static str {
        "closed"
    }
    fn is_terminal() -> bool {
        true
    }
    fn is_initial() -> bool {
        false
    }
}

// ═══════════════════════════════════════════════════════════
// TYPESAFE CASE
// ═══════════════════════════════════════════════════════════

/// Case lifecycle wrapper with compile-time state enforcement.
///
/// The state type parameter `S` encodes the current lifecycle state.
/// Transition methods consume `self` and return a new state type,
/// making invalid transitions impossible to compile.
///
/// Tier: T2-C (ς + ∂ + ∅ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesafeCase<S: LifecycleState> {
    /// Entity identifier.
    pub entity_id: u64,
    /// Context data.
    pub context: StateContext,
    /// Number of transitions applied.
    pub transition_count: u64,
    /// State marker (zero-sized, compile-time only).
    #[serde(skip)]
    _state: PhantomData<S>,
}

impl<S: LifecycleState> TypesafeCase<S> {
    /// Returns the current state name.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        S::name()
    }

    /// Returns whether the case is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        S::is_terminal()
    }

    /// Returns the entity ID.
    #[must_use]
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }

    /// Returns the transition count.
    #[must_use]
    pub fn transition_count(&self) -> u64 {
        self.transition_count
    }

    /// Returns a reference to the context.
    #[must_use]
    pub fn context(&self) -> &StateContext {
        &self.context
    }

    /// Returns a mutable reference to the context.
    pub fn context_mut(&mut self) -> &mut StateContext {
        &mut self.context
    }
}

impl TypesafeCase<CaseReceived> {
    /// Creates a new case in the Received state.
    #[must_use]
    pub fn new(entity_id: u64, timestamp: u64) -> Self {
        Self {
            entity_id,
            context: StateContext::new(entity_id, timestamp),
            transition_count: 0,
            _state: PhantomData,
        }
    }

    /// Triage the case → transitions to Triaged state.
    ///
    /// Consumes self (A3 conservation: exactly 1 instance).
    #[must_use]
    pub fn triage(self, timestamp: u64) -> TypesafeCase<CaseTriaged> {
        TypesafeCase {
            entity_id: self.entity_id,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

impl TypesafeCase<CaseTriaged> {
    /// Assess the case → transitions to Assessed state.
    #[must_use]
    pub fn assess(self, timestamp: u64) -> TypesafeCase<CaseAssessed> {
        TypesafeCase {
            entity_id: self.entity_id,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

impl TypesafeCase<CaseAssessed> {
    /// Close the case → transitions to Closed (terminal) state.
    #[must_use]
    pub fn close(self, timestamp: u64) -> TypesafeCase<CaseClosed> {
        TypesafeCase {
            entity_id: self.entity_id,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

// TypesafeCase<CaseClosed> has NO transition methods — terminal state.
// Attempting to call .triage(), .assess(), or .close() is a compile error.

impl<S: LifecycleState> GroundsTo for TypesafeCase<S> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,     // ς — DOMINANT: lifecycle state
            LexPrimitiva::Boundary,  // ∂ — sealed trait constrains states
            LexPrimitiva::Void,      // ∅ — PhantomData has no runtime cost
            LexPrimitiva::Causality, // → — transitions cause state change
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
    fn test_typesafe_case_grounding() {
        let comp = TypesafeCase::<CaseReceived>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_case_lifecycle_happy_path() {
        let case = TypesafeCase::<CaseReceived>::new(100, 1000);
        assert_eq!(case.state_name(), "received");
        assert!(!case.is_terminal());
        assert_eq!(case.transition_count(), 0);

        let case = case.triage(2000);
        assert_eq!(case.state_name(), "triaged");
        assert_eq!(case.transition_count(), 1);

        let case = case.assess(3000);
        assert_eq!(case.state_name(), "assessed");
        assert_eq!(case.transition_count(), 2);

        let case = case.close(4000);
        assert_eq!(case.state_name(), "closed");
        assert!(case.is_terminal());
        assert_eq!(case.transition_count(), 3);

        // At this point, case is TypesafeCase<CaseClosed>.
        // Calling case.triage() would be a COMPILE ERROR because
        // TypesafeCase<CaseClosed> has no triage() method.
    }

    #[test]
    fn test_state_markers() {
        assert!(CaseReceived::is_initial());
        assert!(!CaseReceived::is_terminal());

        assert!(!CaseTriaged::is_initial());
        assert!(!CaseTriaged::is_terminal());

        assert!(!CaseAssessed::is_initial());
        assert!(!CaseAssessed::is_terminal());

        assert!(!CaseClosed::is_initial());
        assert!(CaseClosed::is_terminal());
    }

    #[test]
    fn test_state_names() {
        assert_eq!(CaseReceived::name(), "received");
        assert_eq!(CaseTriaged::name(), "triaged");
        assert_eq!(CaseAssessed::name(), "assessed");
        assert_eq!(CaseClosed::name(), "closed");
    }

    #[test]
    fn test_context_access() {
        let mut case = TypesafeCase::<CaseReceived>::new(42, 1000);
        case.context_mut().set("drug", "aspirin");
        assert_eq!(case.context().get("drug"), Some("aspirin"));
    }

    // The following test demonstrates compile-time enforcement.
    // Uncommenting would cause a compile error:
    //
    // #[test]
    // fn test_invalid_transition_compile_error() {
    //     let case = TypesafeCase::<CaseReceived>::new(100, 1000);
    //     let closed = case.triage(2000).assess(3000).close(4000);
    //     // This line would NOT compile:
    //     // closed.triage(5000);  // ERROR: no method `triage` on TypesafeCase<CaseClosed>
    // }
}
