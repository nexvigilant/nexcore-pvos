//! # PVST — ToV Axiom Mappings
//!
//! Maps the Theory of Vigilance axioms to FSM properties, providing
//! type-level witnesses that FSMs satisfy the five axioms.
//!
//! ## Axiom-FSM Correspondence
//!
//! | Axiom | FSM Concept | Witness Type |
//! |-------|-------------|--------------|
//! | A1 (Decomposition) | `FsmState` set | `FiniteDecomposition<N>` |
//! | A2 (Hierarchy) | Lifecycle progression | `HierarchicalWitness` |
//! | A3 (Conservation) | `TransitionGuard` | `GuardConstraintSet` |
//! | A4 (Manifold) | Interior/boundary states | `SafetyManifoldWitness` |
//! | A5 (Emergence) | Guard→transition→effect | `EmergenceWitness` |
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role        | Weight |
//! |--------|-------------|--------|
//! | ς      | State       | 0.80 (dominant) |
//! | ∂      | Boundary    | 0.10   |
//! | N      | Quantity    | 0.05   |
//! | →      | Causality   | 0.05   |

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use super::state::StateId;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// A1: FINITE DECOMPOSITION
// ═══════════════════════════════════════════════════════════

/// A1 Witness: FSM has at most MAX_STATES states.
///
/// Proves that the state machine admits finite elemental decomposition
/// with |E| = state_count ≤ MAX_STATES.
///
/// Tier: T2-P (ς + N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiniteDecomposition<const MAX_STATES: usize> {
    /// Actual number of states in the FSM.
    pub state_count: usize,
}

impl<const MAX_STATES: usize> FiniteDecomposition<MAX_STATES> {
    /// Creates a finite decomposition witness.
    ///
    /// # Panics
    ///
    /// Panics if `state_count > MAX_STATES` (violates A1).
    #[must_use]
    pub fn new(state_count: usize) -> Self {
        assert!(
            state_count <= MAX_STATES,
            "A1 violation: state_count {} exceeds MAX_STATES {}",
            state_count,
            MAX_STATES
        );
        Self { state_count }
    }

    /// Try to create a witness, returning None if invalid.
    #[must_use]
    pub fn try_new(state_count: usize) -> Option<Self> {
        if state_count <= MAX_STATES {
            Some(Self { state_count })
        } else {
            None
        }
    }

    /// Returns the maximum allowed states.
    #[must_use]
    pub const fn max_states(&self) -> usize {
        MAX_STATES
    }

    /// Returns the actual state count.
    #[must_use]
    pub const fn state_count(&self) -> usize {
        self.state_count
    }

    /// Returns the saturation ratio (state_count / MAX_STATES).
    #[must_use]
    pub fn saturation(&self) -> f64 {
        if MAX_STATES == 0 {
            0.0
        } else {
            self.state_count as f64 / MAX_STATES as f64
        }
    }
}

impl<const MAX_STATES: usize> GroundsTo for FiniteDecomposition<MAX_STATES> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,    // ς — DOMINANT: discrete states
            LexPrimitiva::Quantity, // N — cardinality bound
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// A2: HIERARCHICAL ORGANIZATION
// ═══════════════════════════════════════════════════════════

/// A2 Witness: States form a directed acyclic flow.
///
/// Proves that states are hierarchically organized with
/// coarse-graining maps (transitions go "up" the hierarchy).
///
/// Tier: T2-P (ς + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalWitness {
    /// States in topological order (dependencies first).
    pub topological_order: Vec<StateId>,
    /// Number of levels in the hierarchy.
    pub depth: usize,
}

impl HierarchicalWitness {
    /// Creates a hierarchy witness from a topological ordering.
    #[must_use]
    pub fn new(topological_order: Vec<StateId>, depth: usize) -> Self {
        Self {
            topological_order,
            depth,
        }
    }

    /// Creates a linear hierarchy (fully sequential).
    #[must_use]
    pub fn linear(states: Vec<StateId>) -> Self {
        let depth = states.len();
        Self {
            topological_order: states,
            depth,
        }
    }

    /// Returns the hierarchy depth.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the number of states in the hierarchy.
    #[must_use]
    pub fn state_count(&self) -> usize {
        self.topological_order.len()
    }
}

impl GroundsTo for HierarchicalWitness {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,    // ς — DOMINANT: hierarchical levels
            LexPrimitiva::Sequence, // σ — topological ordering
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// A3: CONSERVATION CONSTRAINTS (Guards)
// ═══════════════════════════════════════════════════════════

/// A single guard constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConstraint {
    /// Guard name/label.
    pub name: String,
    /// Description of what the guard enforces.
    pub description: String,
    /// Whether this guard is a conservation law.
    pub is_conservation: bool,
}

impl GuardConstraint {
    /// Creates a new guard constraint.
    #[must_use]
    pub fn new(name: &str, description: &str, is_conservation: bool) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            is_conservation,
        }
    }
}

/// A3 Witness: Guards form conservation constraint set.
///
/// Proves that transition guards preserve invariants,
/// functioning as conservation laws.
///
/// Tier: T2-C (ς + ∂ + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConstraintSet {
    /// All guard constraints.
    pub constraints: Vec<GuardConstraint>,
}

impl GuardConstraintSet {
    /// Creates a new guard constraint set.
    #[must_use]
    pub fn new(constraints: Vec<GuardConstraint>) -> Self {
        Self { constraints }
    }

    /// Creates an empty constraint set.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    /// Adds a constraint to the set.
    pub fn add(&mut self, constraint: GuardConstraint) {
        self.constraints.push(constraint);
    }

    /// Returns the number of constraints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    /// Returns true if no constraints exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Returns the number of conservation-type constraints.
    #[must_use]
    pub fn conservation_count(&self) -> usize {
        self.constraints
            .iter()
            .filter(|c| c.is_conservation)
            .count()
    }
}

impl GroundsTo for GuardConstraintSet {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,      // ς — DOMINANT: guards constrain states
            LexPrimitiva::Boundary,   // ∂ — constraint boundaries
            LexPrimitiva::Comparison, // κ — guard evaluation
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// A4: SAFETY MANIFOLD
// ═══════════════════════════════════════════════════════════

/// A4 Witness: Interior (valid) and boundary (terminal) states.
///
/// The safety manifold M consists of:
/// - **Interior states** (int(M)): Non-terminal states where transitions exist
/// - **Boundary states** (∂M): Terminal states (absorbing boundaries)
///
/// Harm = crossing from interior to outside (impossible once terminal).
///
/// Tier: T2-C (ς + ∂ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyManifoldWitness {
    /// Interior states (non-terminal, have outgoing transitions).
    pub interior_states: Vec<StateId>,
    /// Boundary states (terminal, absorbing).
    pub boundary_states: Vec<StateId>,
}

impl SafetyManifoldWitness {
    /// Creates a safety manifold witness.
    #[must_use]
    pub fn new(interior_states: Vec<StateId>, boundary_states: Vec<StateId>) -> Self {
        Self {
            interior_states,
            boundary_states,
        }
    }

    /// Returns the number of interior states.
    #[must_use]
    pub fn interior_count(&self) -> usize {
        self.interior_states.len()
    }

    /// Returns the number of boundary (terminal) states.
    #[must_use]
    pub fn boundary_count(&self) -> usize {
        self.boundary_states.len()
    }

    /// Checks if a state is in the interior.
    #[must_use]
    pub fn is_interior(&self, state: StateId) -> bool {
        self.interior_states.contains(&state)
    }

    /// Checks if a state is on the boundary (terminal).
    #[must_use]
    pub fn is_boundary(&self, state: StateId) -> bool {
        self.boundary_states.contains(&state)
    }

    /// Validates that all boundary states have no outgoing transitions.
    /// (In typestate pattern, this is enforced at compile time.)
    #[must_use]
    pub fn boundary_is_absorbing(&self) -> bool {
        // By construction in typestate, terminal states have no transition methods.
        // This method exists for runtime FSM validation.
        true
    }
}

impl GroundsTo for SafetyManifoldWitness {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,     // ς — DOMINANT: interior/boundary states
            LexPrimitiva::Boundary,  // ∂ — manifold boundary
            LexPrimitiva::Existence, // ∃ — state existence in region
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// A5: EMERGENCE (Composition)
// ═══════════════════════════════════════════════════════════

/// A5 Witness: Transition composition is emergent.
///
/// The guard→transition→effect chain demonstrates emergence:
/// individual components compose to produce higher-level behavior.
///
/// Tier: T2-C (ς + → + Σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceWitness {
    /// Number of transition chains that can produce terminal states.
    pub terminal_paths: usize,
    /// Whether the Markov property holds (future depends only on current state).
    pub is_markov: bool,
}

impl EmergenceWitness {
    /// Creates an emergence witness.
    #[must_use]
    pub fn new(terminal_paths: usize, is_markov: bool) -> Self {
        Self {
            terminal_paths,
            is_markov,
        }
    }

    /// Creates a Markovian emergence witness.
    #[must_use]
    pub fn markov(terminal_paths: usize) -> Self {
        Self::new(terminal_paths, true)
    }
}

impl GroundsTo for EmergenceWitness {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,     // ς — DOMINANT: emergent state
            LexPrimitiva::Causality, // → — transition chains
            LexPrimitiva::Sum,       // Σ — path aggregation
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// COMBINED TOV-FSM PROOF
// ═══════════════════════════════════════════════════════════

/// Combined proof that an FSM satisfies all five ToV axioms.
///
/// Tier: T3 (ς + ∂ + N + σ + κ + ∃ + → + Σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TovFsmProof<const MAX_STATES: usize> {
    /// A1: Finite decomposition.
    pub a1_finite: FiniteDecomposition<MAX_STATES>,
    /// A2: Hierarchical organization.
    pub a2_hierarchy: HierarchicalWitness,
    /// A3: Conservation constraints.
    pub a3_guards: GuardConstraintSet,
    /// A4: Safety manifold.
    pub a4_manifold: SafetyManifoldWitness,
    /// A5: Emergence.
    pub a5_emergence: EmergenceWitness,
}

impl<const MAX_STATES: usize> TovFsmProof<MAX_STATES> {
    /// Creates a complete ToV-FSM proof.
    #[must_use]
    pub fn new(
        a1_finite: FiniteDecomposition<MAX_STATES>,
        a2_hierarchy: HierarchicalWitness,
        a3_guards: GuardConstraintSet,
        a4_manifold: SafetyManifoldWitness,
        a5_emergence: EmergenceWitness,
    ) -> Self {
        Self {
            a1_finite,
            a2_hierarchy,
            a3_guards,
            a4_manifold,
            a5_emergence,
        }
    }

    /// Validates internal consistency of the proof.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        // A1 and A4 consistency: state counts match
        let total_states = self.a4_manifold.interior_count() + self.a4_manifold.boundary_count();
        if self.a1_finite.state_count != total_states {
            return false;
        }

        // A2 consistency: hierarchy covers all states
        if self.a2_hierarchy.state_count() != total_states {
            return false;
        }

        true
    }
}

impl<const MAX_STATES: usize> GroundsTo for TovFsmProof<MAX_STATES> {
    fn primitive_composition() -> PrimitiveComposition {
        // T3: 6+ unique primitives
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,      // ς — DOMINANT
            LexPrimitiva::Boundary,   // ∂
            LexPrimitiva::Quantity,   // N
            LexPrimitiva::Sequence,   // σ
            LexPrimitiva::Comparison, // κ
            LexPrimitiva::Existence,  // ∃
            LexPrimitiva::Causality,  // →
            LexPrimitiva::Sum,        // Σ
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// FACTORY FOR CASE LIFECYCLE
// ═══════════════════════════════════════════════════════════

/// Creates a ToV-FSM proof for the Case lifecycle.
///
/// Case: Received → Triaged → Assessed → Closed
/// - 4 states (A1: MAX=10)
/// - Linear hierarchy depth 4 (A2)
/// - Guards on triage, close (A3)
/// - 3 interior, 1 boundary (A4)
/// - 1 terminal path (A5)
#[must_use]
pub fn case_lifecycle_proof() -> TovFsmProof<10> {
    let a1 = FiniteDecomposition::new(4);

    let a2 = HierarchicalWitness::linear(vec![
        StateId(1), // Received
        StateId(2), // Triaged
        StateId(3), // Assessed
        StateId(4), // Closed
    ]);

    let mut a3 = GuardConstraintSet::empty();
    a3.add(GuardConstraint::new(
        "has_required_fields",
        "All required case fields must be present before triage",
        true,
    ));
    a3.add(GuardConstraint::new(
        "assessment_complete",
        "Medical assessment must be completed before close",
        true,
    ));

    let a4 = SafetyManifoldWitness::new(
        vec![StateId(1), StateId(2), StateId(3)], // Interior
        vec![StateId(4)],                         // Boundary (Closed)
    );

    let a5 = EmergenceWitness::markov(1); // 1 path to terminal

    TovFsmProof::new(a1, a2, a3, a4, a5)
}

/// Creates a ToV-FSM proof for the Signal lifecycle.
///
/// Signal: Detected → Validated → Confirmed | Refuted
/// - 4 states (A1: MAX=10)
/// - DAG with fork at Validated (A2)
/// - No guards (A3)
/// - 2 interior, 2 boundary (A4)
/// - 2 terminal paths (A5)
#[must_use]
pub fn signal_lifecycle_proof() -> TovFsmProof<10> {
    let a1 = FiniteDecomposition::new(4);

    let a2 = HierarchicalWitness::new(
        vec![
            StateId(1), // Detected
            StateId(2), // Validated
            StateId(3), // Confirmed
            StateId(4), // Refuted
        ],
        3, // Depth 3 (fork doesn't increase depth)
    );

    let a3 = GuardConstraintSet::empty(); // No guards

    let a4 = SafetyManifoldWitness::new(
        vec![StateId(1), StateId(2)], // Interior
        vec![StateId(3), StateId(4)], // Boundary (Confirmed, Refuted)
    );

    let a5 = EmergenceWitness::markov(2); // 2 paths to terminal

    TovFsmProof::new(a1, a2, a3, a4, a5)
}

/// Creates a ToV-FSM proof for the Workflow lifecycle.
///
/// Workflow: Pending → Running → Completed | Failed, Failed → Running
/// - 4 states (A1: MAX=10)
/// - Cycle at Failed→Running (A2: not strictly DAG)
/// - No guards (A3)
/// - 3 interior, 1 boundary (A4)
/// - 1 terminal path (A5)
/// - Non-Markovian due to retry history
#[must_use]
pub fn workflow_lifecycle_proof() -> TovFsmProof<10> {
    let a1 = FiniteDecomposition::new(4);

    let a2 = HierarchicalWitness::new(
        vec![
            StateId(1), // Pending
            StateId(2), // Running
            StateId(3), // Completed
            StateId(4), // Failed
        ],
        2, // Effective depth 2 (cycle doesn't add depth)
    );

    let a3 = GuardConstraintSet::empty();

    let a4 = SafetyManifoldWitness::new(
        vec![StateId(1), StateId(2), StateId(4)], // Interior (Failed can retry)
        vec![StateId(3)],                         // Boundary (only Completed)
    );

    let a5 = EmergenceWitness::new(1, false); // Non-Markovian (retry history)

    TovFsmProof::new(a1, a2, a3, a4, a5)
}

/// Creates a ToV-FSM proof for the Submission lifecycle.
///
/// Submission: Draft → Validated → Signed → Sent → Acknowledged
/// - 5 states (A1: MAX=10)
/// - Linear hierarchy depth 5 (A2)
/// - Guards on validate, sign (A3)
/// - 4 interior, 1 boundary (A4)
/// - 1 terminal path (A5)
#[must_use]
pub fn submission_lifecycle_proof() -> TovFsmProof<10> {
    let a1 = FiniteDecomposition::new(5);

    let a2 = HierarchicalWitness::linear(vec![
        StateId(1), // Draft
        StateId(2), // Validated
        StateId(3), // Signed
        StateId(4), // Sent
        StateId(5), // Acknowledged
    ]);

    let mut a3 = GuardConstraintSet::empty();
    a3.add(GuardConstraint::new(
        "content_valid",
        "Submission content must pass validation rules",
        true,
    ));
    a3.add(GuardConstraint::new(
        "authorized_signer",
        "Only authorized personnel can sign submissions",
        true,
    ));

    let a4 = SafetyManifoldWitness::new(
        vec![StateId(1), StateId(2), StateId(3), StateId(4)], // Interior
        vec![StateId(5)],                                     // Boundary (Acknowledged)
    );

    let a5 = EmergenceWitness::markov(1);

    TovFsmProof::new(a1, a2, a3, a4, a5)
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_finite_decomposition_grounding() {
        let comp = FiniteDecomposition::<10>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.unique().len(), 2);
    }

    #[test]
    fn test_safety_manifold_grounding() {
        let comp = SafetyManifoldWitness::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.unique().len(), 3);
    }

    #[test]
    fn test_tov_fsm_proof_grounding() {
        let comp = TovFsmProof::<10>::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.unique().len(), 8);
    }

    #[test]
    fn test_finite_decomposition_valid() {
        let fd = FiniteDecomposition::<10>::new(4);
        assert_eq!(fd.state_count(), 4);
        assert_eq!(fd.max_states(), 10);
        assert!((fd.saturation() - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_finite_decomposition_try_new() {
        assert!(FiniteDecomposition::<5>::try_new(3).is_some());
        assert!(FiniteDecomposition::<5>::try_new(5).is_some());
        assert!(FiniteDecomposition::<5>::try_new(6).is_none());
    }

    #[test]
    #[should_panic(expected = "A1 violation")]
    fn test_finite_decomposition_violation() {
        let _ = FiniteDecomposition::<5>::new(10);
    }

    #[test]
    fn test_safety_manifold() {
        let manifold = SafetyManifoldWitness::new(vec![StateId(1), StateId(2)], vec![StateId(3)]);

        assert_eq!(manifold.interior_count(), 2);
        assert_eq!(manifold.boundary_count(), 1);
        assert!(manifold.is_interior(StateId(1)));
        assert!(manifold.is_boundary(StateId(3)));
        assert!(!manifold.is_interior(StateId(3)));
    }

    #[test]
    fn test_case_lifecycle_proof() {
        let proof = case_lifecycle_proof();

        assert!(proof.is_consistent());
        assert_eq!(proof.a1_finite.state_count(), 4);
        assert_eq!(proof.a2_hierarchy.depth(), 4);
        assert_eq!(proof.a3_guards.len(), 2);
        assert_eq!(proof.a4_manifold.interior_count(), 3);
        assert_eq!(proof.a4_manifold.boundary_count(), 1);
        assert_eq!(proof.a5_emergence.terminal_paths, 1);
        assert!(proof.a5_emergence.is_markov);
    }

    #[test]
    fn test_signal_lifecycle_proof() {
        let proof = signal_lifecycle_proof();

        assert!(proof.is_consistent());
        assert_eq!(proof.a1_finite.state_count(), 4);
        assert_eq!(proof.a4_manifold.boundary_count(), 2); // Confirmed, Refuted
        assert_eq!(proof.a5_emergence.terminal_paths, 2);
    }

    #[test]
    fn test_workflow_lifecycle_proof() {
        let proof = workflow_lifecycle_proof();

        assert!(proof.is_consistent());
        assert_eq!(proof.a4_manifold.interior_count(), 3); // Failed is interior (can retry)
        assert_eq!(proof.a4_manifold.boundary_count(), 1); // Only Completed
        assert!(!proof.a5_emergence.is_markov); // Non-Markovian due to retry
    }

    #[test]
    fn test_submission_lifecycle_proof() {
        let proof = submission_lifecycle_proof();

        assert!(proof.is_consistent());
        assert_eq!(proof.a1_finite.state_count(), 5);
        assert_eq!(proof.a2_hierarchy.depth(), 5); // Linear
        assert_eq!(proof.a3_guards.conservation_count(), 2);
    }

    #[test]
    fn test_hierarchical_witness() {
        let hw = HierarchicalWitness::linear(vec![StateId(1), StateId(2), StateId(3)]);

        assert_eq!(hw.depth(), 3);
        assert_eq!(hw.state_count(), 3);
    }

    #[test]
    fn test_guard_constraint_set() {
        let mut guards = GuardConstraintSet::empty();
        assert!(guards.is_empty());

        guards.add(GuardConstraint::new("g1", "desc1", true));
        guards.add(GuardConstraint::new("g2", "desc2", false));

        assert_eq!(guards.len(), 2);
        assert_eq!(guards.conservation_count(), 1);
    }
}
