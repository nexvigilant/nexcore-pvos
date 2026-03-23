//! # PVST — Conservation Laws
//!
//! FSM invariants expressed as conservation laws, mapping to the
//! Theory of Vigilance Axiom 3 (Conservation Constraints).
//!
//! ## Laws Implemented
//!
//! - **L3 (State)**: Machine is in exactly one state at any moment
//! - **L4 (Flux)**: Non-terminal states have ≥1 outgoing transition
//! - **L11 (Structure)**: State/transition counts are immutable post-construction
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role        | Weight |
//! |--------|-------------|--------|
//! | ς      | State       | 0.80 (dominant) |
//! | →      | Causality   | 0.10   |
//! | κ      | Comparison  | 0.10   |
//!
//! ## ToV A3 Mapping
//!
//! Conservation laws are constraint functions g: S × U × Θ → ℝ.
//! Harm occurs iff gᵢ(s, u, θ) > 0 (constraint violation).

use serde::{Deserialize, Serialize};

use crate::state::{StateId, StateMachine};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// CONSERVATION LAW TRAIT
// ═══════════════════════════════════════════════════════════

/// A conservation law that can be verified against an FSM.
pub trait ConservationLaw {
    /// Law name/identifier.
    fn name(&self) -> &'static str;

    /// Description of what the law ensures.
    fn description(&self) -> &'static str;

    /// Verify the law against the given state machine.
    fn verify(&self, machine: &StateMachine) -> LawVerification;
}

/// Result of verifying a single conservation law.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LawVerification {
    /// Law is satisfied.
    Satisfied,
    /// Law is violated with explanation.
    Violated(String),
}

impl LawVerification {
    /// Returns true if the law is satisfied.
    #[must_use]
    pub fn is_satisfied(&self) -> bool {
        matches!(self, LawVerification::Satisfied)
    }
}

// ═══════════════════════════════════════════════════════════
// L3: SINGLE STATE LAW
// ═══════════════════════════════════════════════════════════

/// L3: Machine is in exactly one state at any moment.
///
/// This law is trivially satisfied by the `StateMachine` structure
/// (single `current: StateId` field), but we encode it for completeness.
///
/// Tier: T2-P (ς + κ)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct L3SingleState;

impl L3SingleState {
    /// Creates a new L3 law instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ConservationLaw for L3SingleState {
    fn name(&self) -> &'static str {
        "L3: Single State"
    }

    fn description(&self) -> &'static str {
        "Machine is in exactly one state at any moment"
    }

    fn verify(&self, machine: &StateMachine) -> LawVerification {
        // By construction, StateMachine has a single `current: StateId` field.
        // We verify that this state exists in the machine's state set.
        let current = machine.current_state();

        if machine.has_state(current) {
            LawVerification::Satisfied
        } else {
            LawVerification::Violated(format!(
                "Current state {:?} not in machine's state set",
                current
            ))
        }
    }
}

impl GroundsTo for L3SingleState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,      // ς — DOMINANT: single state
            LexPrimitiva::Comparison, // κ — state existence check
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// L4: NON-TERMINAL FLUX LAW
// ═══════════════════════════════════════════════════════════

/// L4: Non-terminal states have at least one outgoing transition.
///
/// Terminal states are absorbing (no outgoing transitions).
/// Non-terminal states must have flux (can transition out).
///
/// Tier: T2-P (ς + →)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct L4NonTerminalFlux;

impl L4NonTerminalFlux {
    /// Creates a new L4 law instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ConservationLaw for L4NonTerminalFlux {
    fn name(&self) -> &'static str {
        "L4: Non-Terminal Flux"
    }

    fn description(&self) -> &'static str {
        "Non-terminal states have at least one outgoing transition"
    }

    fn verify(&self, machine: &StateMachine) -> LawVerification {
        for state in machine.states() {
            if state.is_terminal {
                // Terminal states should have NO outgoing transitions
                let outgoing = machine
                    .transitions()
                    .iter()
                    .filter(|t| t.from == state.id)
                    .count();

                if outgoing > 0 {
                    return LawVerification::Violated(format!(
                        "Terminal state '{}' has {} outgoing transitions (should be 0)",
                        state.name, outgoing
                    ));
                }
            } else {
                // Non-terminal states should have ≥1 outgoing transition
                let outgoing = machine
                    .transitions()
                    .iter()
                    .filter(|t| t.from == state.id)
                    .count();

                if outgoing == 0 {
                    return LawVerification::Violated(format!(
                        "Non-terminal state '{}' has no outgoing transitions",
                        state.name
                    ));
                }
            }
        }

        LawVerification::Satisfied
    }
}

impl GroundsTo for L4NonTerminalFlux {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,     // ς — DOMINANT: state classification
            LexPrimitiva::Causality, // → — transition existence
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// L11: STRUCTURE IMMUTABILITY LAW
// ═══════════════════════════════════════════════════════════

/// L11: State and transition counts are immutable after construction.
///
/// This law verifies that a snapshot of structure counts matches
/// the expected values. In practice, this is enforced by not
/// exposing mutation methods after initial construction.
///
/// Tier: T2-P (ς + N)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct L11StructureImmutability {
    /// Expected state count.
    pub expected_states: usize,
    /// Expected transition count.
    pub expected_transitions: usize,
}

impl L11StructureImmutability {
    /// Creates a new L11 law with expected counts.
    #[must_use]
    pub fn new(expected_states: usize, expected_transitions: usize) -> Self {
        Self {
            expected_states,
            expected_transitions,
        }
    }

    /// Creates L11 from a machine's current structure.
    #[must_use]
    pub fn from_machine(machine: &StateMachine) -> Self {
        Self {
            expected_states: machine.state_count(),
            expected_transitions: machine.transition_def_count(),
        }
    }
}

impl ConservationLaw for L11StructureImmutability {
    fn name(&self) -> &'static str {
        "L11: Structure Immutability"
    }

    fn description(&self) -> &'static str {
        "State and transition counts are immutable after construction"
    }

    fn verify(&self, machine: &StateMachine) -> LawVerification {
        let actual_states = machine.state_count();
        let actual_transitions = machine.transition_def_count();

        if actual_states != self.expected_states {
            return LawVerification::Violated(format!(
                "State count changed: expected {}, actual {}",
                self.expected_states, actual_states
            ));
        }

        if actual_transitions != self.expected_transitions {
            return LawVerification::Violated(format!(
                "Transition count changed: expected {}, actual {}",
                self.expected_transitions, actual_transitions
            ));
        }

        LawVerification::Satisfied
    }
}

impl GroundsTo for L11StructureImmutability {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,    // ς — DOMINANT: structure integrity
            LexPrimitiva::Quantity, // N — count invariants
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// CONSERVATION VERIFIER
// ═══════════════════════════════════════════════════════════

/// Verifies multiple conservation laws against a state machine.
///
/// Tier: T2-C (ς + → + κ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConservationVerifier {
    /// Results of verification.
    results: Vec<(String, LawVerification)>,
}

impl ConservationVerifier {
    /// Creates a new verifier.
    #[must_use]
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Verifies a single law and records the result.
    pub fn verify(&mut self, law: &impl ConservationLaw, machine: &StateMachine) {
        let result = law.verify(machine);
        self.results.push((law.name().to_string(), result));
    }

    /// Verifies all standard PVST conservation laws.
    pub fn verify_all(&mut self, machine: &StateMachine) {
        self.verify(&L3SingleState::new(), machine);
        self.verify(&L4NonTerminalFlux::new(), machine);
        self.verify(&L11StructureImmutability::from_machine(machine), machine);
    }

    /// Returns all verification results.
    #[must_use]
    pub fn results(&self) -> &[(String, LawVerification)] {
        &self.results
    }

    /// Returns true if all verified laws are satisfied.
    #[must_use]
    pub fn all_satisfied(&self) -> bool {
        self.results.iter().all(|(_, v)| v.is_satisfied())
    }

    /// Returns the number of satisfied laws.
    #[must_use]
    pub fn satisfied_count(&self) -> usize {
        self.results
            .iter()
            .filter(|(_, v)| v.is_satisfied())
            .count()
    }

    /// Returns the number of violated laws.
    #[must_use]
    pub fn violated_count(&self) -> usize {
        self.results.len() - self.satisfied_count()
    }

    /// Returns violations only.
    #[must_use]
    pub fn violations(&self) -> Vec<&(String, LawVerification)> {
        self.results
            .iter()
            .filter(|(_, v)| !v.is_satisfied())
            .collect()
    }
}

impl GroundsTo for ConservationVerifier {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,      // ς — DOMINANT: law verification
            LexPrimitiva::Causality,  // → — flux laws
            LexPrimitiva::Comparison, // κ — satisfaction checks
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// VERIFICATION RESULT
// ═══════════════════════════════════════════════════════════

/// Complete verification result for an FSM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Machine name.
    pub machine_name: String,
    /// Whether all laws are satisfied.
    pub all_satisfied: bool,
    /// Number of laws verified.
    pub laws_verified: usize,
    /// Number of laws satisfied.
    pub laws_satisfied: usize,
    /// Detailed results.
    pub details: Vec<(String, LawVerification)>,
}

impl VerificationResult {
    /// Creates a verification result from a verifier.
    #[must_use]
    pub fn from_verifier(verifier: &ConservationVerifier, machine_name: &str) -> Self {
        Self {
            machine_name: machine_name.to_string(),
            all_satisfied: verifier.all_satisfied(),
            laws_verified: verifier.results().len(),
            laws_satisfied: verifier.satisfied_count(),
            details: verifier.results().to_vec(),
        }
    }

    /// Returns the compliance ratio.
    #[must_use]
    pub fn compliance_ratio(&self) -> f64 {
        if self.laws_verified == 0 {
            1.0
        } else {
            self.laws_satisfied as f64 / self.laws_verified as f64
        }
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::case_lifecycle;
    use crate::state::{FsmState, TransitionDef};
    use nexcore_lex_primitiva::GroundingTier;

    fn build_valid_fsm() -> StateMachine {
        case_lifecycle(1, 100, 1000)
    }

    fn build_invalid_fsm() -> StateMachine {
        // FSM with non-terminal state having no outgoing transitions
        let mut fsm = StateMachine::new(1, "invalid", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "start").initial());
        fsm.add_state(FsmState::new(2, "middle")); // Non-terminal, no outgoing!
        fsm.add_state(FsmState::new(3, "end").terminal());
        fsm.add_transition(TransitionDef::new(StateId(1), "go", StateId(2)));
        // Missing: transition from middle to end
        fsm
    }

    #[test]
    fn test_l3_grounding() {
        let comp = L3SingleState::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.unique().len(), 2);
    }

    #[test]
    fn test_l4_grounding() {
        let comp = L4NonTerminalFlux::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.unique().len(), 2);
    }

    #[test]
    fn test_l11_grounding() {
        let comp = L11StructureImmutability::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.unique().len(), 2);
    }

    #[test]
    fn test_verifier_grounding() {
        let comp = ConservationVerifier::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.unique().len(), 3);
    }

    #[test]
    fn test_l3_single_state_valid() {
        let fsm = build_valid_fsm();
        let law = L3SingleState::new();
        let result = law.verify(&fsm);
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_l4_flux_valid() {
        let fsm = build_valid_fsm();
        let law = L4NonTerminalFlux::new();
        let result = law.verify(&fsm);
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_l4_flux_invalid() {
        let fsm = build_invalid_fsm();
        let law = L4NonTerminalFlux::new();
        let result = law.verify(&fsm);
        assert!(!result.is_satisfied());
        if let LawVerification::Violated(msg) = result {
            assert!(msg.contains("middle"));
            assert!(msg.contains("no outgoing"));
        }
    }

    #[test]
    fn test_l11_structure_valid() {
        let fsm = build_valid_fsm();
        let law = L11StructureImmutability::from_machine(&fsm);

        assert_eq!(law.expected_states, 4);
        assert_eq!(law.expected_transitions, 3);

        let result = law.verify(&fsm);
        assert!(result.is_satisfied());
    }

    #[test]
    fn test_verifier_all_valid() {
        let fsm = build_valid_fsm();
        let mut verifier = ConservationVerifier::new();
        verifier.verify_all(&fsm);

        assert!(verifier.all_satisfied());
        assert_eq!(verifier.satisfied_count(), 3);
        assert_eq!(verifier.violated_count(), 0);
    }

    #[test]
    fn test_verifier_with_violations() {
        let fsm = build_invalid_fsm();
        let mut verifier = ConservationVerifier::new();
        verifier.verify_all(&fsm);

        assert!(!verifier.all_satisfied());
        assert!(verifier.violated_count() >= 1);

        let violations = verifier.violations();
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_verification_result() {
        let fsm = build_valid_fsm();
        let mut verifier = ConservationVerifier::new();
        verifier.verify_all(&fsm);

        let result = VerificationResult::from_verifier(&verifier, "case_lifecycle");

        assert_eq!(result.machine_name, "case_lifecycle");
        assert!(result.all_satisfied);
        assert_eq!(result.laws_verified, 3);
        assert_eq!(result.laws_satisfied, 3);
        assert!((result.compliance_ratio() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_law_names() {
        assert_eq!(L3SingleState::new().name(), "L3: Single State");
        assert_eq!(L4NonTerminalFlux::new().name(), "L4: Non-Terminal Flux");

        let l11 = L11StructureImmutability::new(4, 3);
        assert_eq!(l11.name(), "L11: Structure Immutability");
    }
}
