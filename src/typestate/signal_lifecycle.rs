//! # TypesafeSignal — Compile-Time Signal Lifecycle
//!
//! Signal lifecycle with compile-time state enforcement supporting both
//! fast-track assessment and full closed-loop pharmacovigilance.
//!
//! ## Fast Track (3-state)
//! `Detected → Validated → Confirmed | Refuted`
//!
//! ## Full PV Loop (7-state)
//! ```text
//! Detected → Evaluated → Validated → Actioned → Monitoring
//!    ↑                       ↓                      │
//!    │                    Refuted                    ↓
//!    └──────── feedback() ──────────── OR ──→ Confirmed
//! ```
//!
//! Branching terminal states (Confirmed vs Refuted) are type-safe.
//! Monitoring is non-terminal and can feed back to a new Detected cycle.
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
//! - **A1**: 7 states form finite decomposition
//! - **A2**: Detected < Evaluated < Validated < {Confirmed, Refuted, Actioned < Monitoring}
//! - **A3**: Feedback preserves signal identity (entity_id chain)
//! - **A4**: Confirmed/Refuted are terminal (boundary); Monitoring is non-terminal (loop)
//! - **A5**: Full loop emerges from composition of stage transitions

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use super::LifecycleState;
use crate::state::StateContext;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// STATE MARKERS
// ═══════════════════════════════════════════════════════════

/// Signal has been statistically detected.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalDetected;

impl LifecycleState for SignalDetected {
    fn name() -> &'static str {
        "detected"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        true
    }
}

/// Signal has been clinically validated.
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalValidated;

impl LifecycleState for SignalValidated {
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

/// Signal has been confirmed as real (terminal).
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalConfirmed;

impl LifecycleState for SignalConfirmed {
    fn name() -> &'static str {
        "confirmed"
    }
    fn is_terminal() -> bool {
        true
    }
    fn is_initial() -> bool {
        false
    }
}

/// Signal has been refuted as spurious (terminal).
///
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalRefuted;

impl LifecycleState for SignalRefuted {
    fn name() -> &'static str {
        "refuted"
    }
    fn is_terminal() -> bool {
        true
    }
    fn is_initial() -> bool {
        false
    }
}

// ═══════════════════════════════════════════════════════════
// PV LOOP STATE MARKERS (Extensions for closed-loop PV)
// ═══════════════════════════════════════════════════════════

/// Signal evidence has been gathered (literature, labeling, trials).
///
/// PV Loop Stage 2: EVALUATE
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalEvaluated;

impl LifecycleState for SignalEvaluated {
    fn name() -> &'static str {
        "evaluated"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Signal has been actioned (regulatory/clinical response determined).
///
/// PV Loop Stage 4: ACT
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalActioned;

impl LifecycleState for SignalActioned {
    fn name() -> &'static str {
        "actioned"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

/// Signal is under ongoing surveillance. Non-terminal: can feed back
/// to a new Detected cycle or close to Confirmed.
///
/// PV Loop Stage 5: MONITOR
/// Tier: T1 (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalMonitoring;

impl LifecycleState for SignalMonitoring {
    fn name() -> &'static str {
        "monitoring"
    }
    fn is_terminal() -> bool {
        false
    }
    fn is_initial() -> bool {
        false
    }
}

// ═══════════════════════════════════════════════════════════
// TYPESAFE SIGNAL
// ═══════════════════════════════════════════════════════════

/// Signal lifecycle wrapper with compile-time state enforcement.
///
/// Tier: T2-C (ς + ∂ + ∅ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesafeSignal<S: LifecycleState> {
    /// Entity identifier.
    pub entity_id: u64,
    /// Drug associated with this signal.
    pub drug: String,
    /// Event associated with this signal.
    pub event: String,
    /// Context data.
    pub context: StateContext,
    /// Number of transitions applied.
    pub transition_count: u64,
    /// State marker.
    #[serde(skip)]
    _state: PhantomData<S>,
}

impl<S: LifecycleState> TypesafeSignal<S> {
    /// Returns the current state name.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        S::name()
    }

    /// Returns whether the signal is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        S::is_terminal()
    }

    /// Returns the entity ID.
    #[must_use]
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }

    /// Returns the drug name.
    #[must_use]
    pub fn drug(&self) -> &str {
        &self.drug
    }

    /// Returns the event name.
    #[must_use]
    pub fn event(&self) -> &str {
        &self.event
    }

    /// Returns the transition count.
    #[must_use]
    pub fn transition_count(&self) -> u64 {
        self.transition_count
    }
}

impl TypesafeSignal<SignalDetected> {
    /// Creates a new signal in the Detected state.
    #[must_use]
    pub fn new(entity_id: u64, drug: &str, event: &str, timestamp: u64) -> Self {
        Self {
            entity_id,
            drug: drug.to_string(),
            event: event.to_string(),
            context: StateContext::new(entity_id, timestamp),
            transition_count: 0,
            _state: PhantomData,
        }
    }

    /// Fast-track: validate directly → transitions to Validated state.
    /// Use for signals with sufficient evidence to skip evaluation.
    #[must_use]
    pub fn validate(self, timestamp: u64) -> TypesafeSignal<SignalValidated> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }

    /// PV Loop: gather evidence → transitions to Evaluated state.
    /// Stage 1→2: DETECT → EVALUATE
    #[must_use]
    pub fn evaluate(self, timestamp: u64) -> TypesafeSignal<SignalEvaluated> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

impl TypesafeSignal<SignalEvaluated> {
    /// Assess causality → transitions to Validated (assessed) state.
    /// Stage 2→3: EVALUATE → ASSESS
    #[must_use]
    pub fn assess(self, timestamp: u64) -> TypesafeSignal<SignalValidated> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

impl TypesafeSignal<SignalValidated> {
    /// Fast-track: confirm the signal → transitions to Confirmed (terminal).
    #[must_use]
    pub fn confirm(self, timestamp: u64) -> TypesafeSignal<SignalConfirmed> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }

    /// Refute the signal → transitions to Refuted (terminal).
    #[must_use]
    pub fn refute(self, timestamp: u64) -> TypesafeSignal<SignalRefuted> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }

    /// PV Loop: determine regulatory/clinical action → transitions to Actioned.
    /// Stage 3→4: ASSESS → ACT
    #[must_use]
    pub fn action(self, timestamp: u64) -> TypesafeSignal<SignalActioned> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

impl TypesafeSignal<SignalActioned> {
    /// Enter ongoing surveillance → transitions to Monitoring.
    /// Stage 4→5: ACT → MONITOR
    #[must_use]
    pub fn monitor(self, timestamp: u64) -> TypesafeSignal<SignalMonitoring> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

impl TypesafeSignal<SignalMonitoring> {
    /// Feedback: new signal detected during monitoring → spawns new Detected cycle.
    /// Stage 5→1: MONITOR → DETECT (closes the loop)
    ///
    /// Returns a NEW signal in Detected state. The new signal's entity_id
    /// is derived from the parent to maintain provenance chain.
    #[must_use]
    pub fn feedback(self, new_entity_id: u64, timestamp: u64) -> TypesafeSignal<SignalDetected> {
        TypesafeSignal {
            entity_id: new_entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(new_entity_id, timestamp),
            transition_count: 0, // New cycle starts fresh
            _state: PhantomData,
        }
    }

    /// Close monitoring → transitions to Confirmed (terminal).
    /// Signal lifecycle complete, no further monitoring needed.
    #[must_use]
    pub fn close(self, timestamp: u64) -> TypesafeSignal<SignalConfirmed> {
        TypesafeSignal {
            entity_id: self.entity_id,
            drug: self.drug,
            event: self.event,
            context: StateContext::new(self.entity_id, timestamp),
            transition_count: self.transition_count + 1,
            _state: PhantomData,
        }
    }
}

// No transition methods on SignalConfirmed or SignalRefuted — terminal states.

impl<S: LifecycleState> GroundsTo for TypesafeSignal<S> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,
            LexPrimitiva::Boundary,
            LexPrimitiva::Void,
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
    fn test_typesafe_signal_grounding() {
        let comp = TypesafeSignal::<SignalDetected>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.unique().len(), 4);
    }

    // ── Fast-track path (existing, preserved) ──

    #[test]
    fn test_signal_confirm_path() {
        let signal = TypesafeSignal::<SignalDetected>::new(200, "aspirin", "headache", 1000);
        assert_eq!(signal.state_name(), "detected");
        assert!(!signal.is_terminal());

        let signal = signal.validate(2000);
        assert_eq!(signal.state_name(), "validated");

        let signal = signal.confirm(3000);
        assert_eq!(signal.state_name(), "confirmed");
        assert!(signal.is_terminal());
        assert_eq!(signal.drug(), "aspirin");
        assert_eq!(signal.event(), "headache");
    }

    #[test]
    fn test_signal_refute_path() {
        let signal = TypesafeSignal::<SignalDetected>::new(201, "ibuprofen", "nausea", 1000);
        let signal = signal.validate(2000);
        let signal = signal.refute(3000);

        assert_eq!(signal.state_name(), "refuted");
        assert!(signal.is_terminal());
        assert_eq!(signal.transition_count(), 2);
    }

    #[test]
    fn test_state_markers() {
        assert!(SignalDetected::is_initial());
        assert!(!SignalValidated::is_initial());
        assert!(SignalConfirmed::is_terminal());
        assert!(SignalRefuted::is_terminal());
    }

    // ── Full PV Loop path (new) ──

    #[test]
    fn test_pv_loop_state_markers() {
        assert!(!SignalEvaluated::is_terminal());
        assert!(!SignalEvaluated::is_initial());
        assert!(!SignalActioned::is_terminal());
        assert!(!SignalActioned::is_initial());
        assert!(!SignalMonitoring::is_terminal());
        assert!(!SignalMonitoring::is_initial());
        assert_eq!(SignalEvaluated::name(), "evaluated");
        assert_eq!(SignalActioned::name(), "actioned");
        assert_eq!(SignalMonitoring::name(), "monitoring");
    }

    #[test]
    fn test_pv_loop_full_path() {
        // Stage 1: DETECT
        let signal =
            TypesafeSignal::<SignalDetected>::new(300, "metformin", "lactic acidosis", 1000);
        assert_eq!(signal.state_name(), "detected");
        assert_eq!(signal.transition_count(), 0);

        // Stage 2: EVALUATE
        let signal = signal.evaluate(2000);
        assert_eq!(signal.state_name(), "evaluated");
        assert_eq!(signal.transition_count(), 1);

        // Stage 3: ASSESS
        let signal = signal.assess(3000);
        assert_eq!(signal.state_name(), "validated");
        assert_eq!(signal.transition_count(), 2);

        // Stage 4: ACT
        let signal = signal.action(4000);
        assert_eq!(signal.state_name(), "actioned");
        assert_eq!(signal.transition_count(), 3);

        // Stage 5: MONITOR
        let signal = signal.monitor(5000);
        assert_eq!(signal.state_name(), "monitoring");
        assert!(!signal.is_terminal());
        assert_eq!(signal.transition_count(), 4);
        assert_eq!(signal.drug(), "metformin");
        assert_eq!(signal.event(), "lactic acidosis");

        // Close the loop: MONITOR → CONFIRMED
        let signal = signal.close(6000);
        assert_eq!(signal.state_name(), "confirmed");
        assert!(signal.is_terminal());
        assert_eq!(signal.transition_count(), 5);
    }

    #[test]
    fn test_pv_loop_feedback() {
        // Run through to Monitoring
        let signal =
            TypesafeSignal::<SignalDetected>::new(400, "metformin", "lactic acidosis", 1000)
                .evaluate(2000)
                .assess(3000)
                .action(4000)
                .monitor(5000);

        assert_eq!(signal.state_name(), "monitoring");
        assert_eq!(signal.entity_id(), 400);

        // Feedback: new signal detected during monitoring
        let new_signal = signal.feedback(401, 6000);
        assert_eq!(new_signal.state_name(), "detected");
        assert_eq!(new_signal.entity_id(), 401); // New entity
        assert_eq!(new_signal.transition_count(), 0); // Fresh cycle
        assert_eq!(new_signal.drug(), "metformin"); // Drug preserved
        assert_eq!(new_signal.event(), "lactic acidosis"); // Event preserved

        // New signal can run the full loop again
        let confirmed = new_signal
            .evaluate(7000)
            .assess(8000)
            .action(9000)
            .monitor(10000)
            .close(11000);

        assert!(confirmed.is_terminal());
        assert_eq!(confirmed.entity_id(), 401);
    }

    #[test]
    fn test_pv_loop_refute_at_assessment() {
        // Can refute during assessment (validated state)
        let signal = TypesafeSignal::<SignalDetected>::new(500, "aspirin", "tinnitus", 1000)
            .evaluate(2000)
            .assess(3000)
            .refute(4000);

        assert_eq!(signal.state_name(), "refuted");
        assert!(signal.is_terminal());
        assert_eq!(signal.transition_count(), 3);
    }
}
