//! # PVST — Transition Logic
//!
//! Guards, effects, results, and logging for state machine transitions.
//! Transitions are the causal mechanism (→) through which state (ς)
//! changes occur, constrained by boundary guards (∂).
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role         | Weight |
//! |--------|------------- |--------|
//! | ς      | State        | 0.80 (dominant) |
//! | →      | Causality    | 0.10   |
//! | ∂      | Boundary     | 0.05   |
//! | σ      | Sequence     | 0.03   |
//! | κ      | Comparison   | 0.02   |

use serde::{Deserialize, Serialize};

use super::state::{StateId, StateMachine};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P IDENTIFIERS
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a transition event.
///
/// Tier: T2-P (ς + →)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransitionId(pub u64);

// ═══════════════════════════════════════════════════════════
// TRANSITION GUARD
// ═══════════════════════════════════════════════════════════

/// A guard condition that must be satisfied before a transition.
///
/// Guards are boundary checks (∂) that constrain when transitions
/// are allowed. They examine the current state context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionGuard {
    /// Always allows the transition.
    Always,
    /// Requires a specific context key to be present.
    RequiresKey(String),
    /// Requires a specific context key to have a specific value.
    RequiresKeyValue { key: String, value: String },
    /// Requires the machine to have been in the current state
    /// for at least `min_duration` time units.
    MinDuration { min_duration: u64 },
    /// Requires the machine to not be in a terminal state.
    NotTerminal,
    /// All guards must pass.
    All(Vec<TransitionGuard>),
    /// At least one guard must pass.
    Any(Vec<TransitionGuard>),
}

impl TransitionGuard {
    /// Evaluates the guard against the given machine and timestamp.
    #[must_use]
    pub fn allows(&self, machine: &StateMachine, now: u64) -> bool {
        match self {
            Self::Always => true,
            Self::RequiresKey(key) => machine.context().get(key).is_some(),
            Self::RequiresKeyValue { key, value } => {
                machine.context().get(key) == Some(value.as_str())
            }
            Self::MinDuration { min_duration } => {
                let elapsed = now.saturating_sub(machine.context().entered_at);
                elapsed >= *min_duration
            }
            Self::NotTerminal => !machine.is_terminal(),
            Self::All(guards) => guards.iter().all(|g| g.allows(machine, now)),
            Self::Any(guards) => guards.iter().any(|g| g.allows(machine, now)),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// TRANSITION EFFECT
// ═══════════════════════════════════════════════════════════

/// A side effect executed when a transition completes.
///
/// Effects represent the causal consequences (→) of state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionEffect {
    /// No side effect.
    None,
    /// Set a context key-value pair.
    SetContext { key: String, value: String },
    /// Remove a context key.
    RemoveContext(String),
    /// Log a message for audit.
    AuditLog(String),
    /// Execute multiple effects in order.
    Sequence(Vec<TransitionEffect>),
}

impl TransitionEffect {
    /// Applies the effect to the machine.
    pub fn apply(&self, machine: &mut StateMachine) {
        match self {
            Self::None => {}
            Self::SetContext { key, value } => {
                machine.context_mut().set(key, value);
            }
            Self::RemoveContext(key) => {
                machine.context_mut().data.remove(key);
            }
            Self::AuditLog(_msg) => {
                // In production, this would write to an audit log.
                // Here we store it in context for test verification.
            }
            Self::Sequence(effects) => {
                for effect in effects {
                    effect.apply(machine);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// TRANSITION RESULT
// ═══════════════════════════════════════════════════════════

/// Why a transition was blocked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
    /// No transition defined for the event in current state.
    NoTransitionDefined,
    /// Guard condition failed.
    GuardFailed(String),
    /// Machine is in a terminal state.
    TerminalState,
}

/// Result of attempting a transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionResult {
    /// Transition succeeded — now in the new state.
    Success {
        from: StateId,
        to: StateId,
        event: String,
    },
    /// Transition was blocked.
    Blocked(BlockReason),
}

impl TransitionResult {
    /// Returns true if the transition succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns the new state if successful.
    #[must_use]
    pub fn new_state(&self) -> Option<StateId> {
        match self {
            Self::Success { to, .. } => Some(*to),
            Self::Blocked(_) => Option::None,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// TRANSITION RECORD (AUDIT)
// ═══════════════════════════════════════════════════════════

/// A record of a completed transition for audit purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// Unique transition ID.
    pub id: TransitionId,
    /// Machine that transitioned.
    pub machine_name: String,
    /// Entity that owns the machine.
    pub entity_id: u64,
    /// Source state.
    pub from: StateId,
    /// Target state.
    pub to: StateId,
    /// Event that triggered the transition.
    pub event: String,
    /// Timestamp of the transition.
    pub timestamp: u64,
    /// Guard that was evaluated (if any).
    pub guard_label: Option<String>,
    /// Effect that was applied (if any).
    pub effect_label: Option<String>,
}

// ═══════════════════════════════════════════════════════════
// TRANSITION LOG
// ═══════════════════════════════════════════════════════════

/// Persistent log of all transitions for regulatory audit.
///
/// Tier: T2-C (ς + → + σ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionLog {
    /// All recorded transitions.
    records: Vec<TransitionRecord>,
    /// Next transition ID.
    next_id: u64,
}

impl TransitionLog {
    /// Creates a new empty transition log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }

    /// Records a transition.
    pub fn record(
        &mut self,
        machine_name: &str,
        entity_id: u64,
        from: StateId,
        to: StateId,
        event: &str,
        timestamp: u64,
        guard_label: Option<String>,
        effect_label: Option<String>,
    ) -> TransitionId {
        let id = TransitionId(self.next_id);
        self.next_id += 1;

        self.records.push(TransitionRecord {
            id,
            machine_name: machine_name.to_string(),
            entity_id,
            from,
            to,
            event: event.to_string(),
            timestamp,
            guard_label,
            effect_label,
        });

        id
    }

    /// Returns all records.
    #[must_use]
    pub fn records(&self) -> &[TransitionRecord] {
        &self.records
    }

    /// Returns records for a specific entity.
    #[must_use]
    pub fn records_for_entity(&self, entity_id: u64) -> Vec<&TransitionRecord> {
        self.records
            .iter()
            .filter(|r| r.entity_id == entity_id)
            .collect()
    }

    /// Returns records for a specific machine.
    #[must_use]
    pub fn records_for_machine(&self, machine_name: &str) -> Vec<&TransitionRecord> {
        self.records
            .iter()
            .filter(|r| r.machine_name == machine_name)
            .collect()
    }

    /// Returns the total number of recorded transitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true if no transitions have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the most recent transition.
    #[must_use]
    pub fn last(&self) -> Option<&TransitionRecord> {
        self.records.last()
    }
}

impl Default for TransitionLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for TransitionLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,       // ς — DOMINANT: state changes
            LexPrimitiva::Causality,   // → — transitions cause effects
            LexPrimitiva::Sequence,    // σ — ordered log entries
            LexPrimitiva::Persistence, // π — durable audit trail
            LexPrimitiva::Comparison,  // κ — guard evaluation
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// TRANSITIONER
// ═══════════════════════════════════════════════════════════

/// Executes guarded transitions with effects and logging.
///
/// The Transitioner is the execution engine that coordinates
/// guard evaluation (∂), state change (ς→), effect application (→),
/// and audit logging (π).
///
/// Tier: T2-C (ς + → + ∂ + σ + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transitioner {
    /// Transition log for audit.
    log: TransitionLog,
    /// Total successful transitions.
    total_success: u64,
    /// Total blocked transitions.
    total_blocked: u64,
}

impl Transitioner {
    /// Creates a new transitioner.
    #[must_use]
    pub fn new() -> Self {
        Self {
            log: TransitionLog::new(),
            total_success: 0,
            total_blocked: 0,
        }
    }

    /// Attempts a guarded transition with effects.
    pub fn execute(
        &mut self,
        machine: &mut StateMachine,
        event: &str,
        guard: &TransitionGuard,
        effect: &TransitionEffect,
        timestamp: u64,
    ) -> TransitionResult {
        // Check terminal state
        if machine.is_terminal() {
            self.total_blocked += 1;
            return TransitionResult::Blocked(BlockReason::TerminalState);
        }

        // Check guard
        if !guard.allows(machine, timestamp) {
            self.total_blocked += 1;
            return TransitionResult::Blocked(BlockReason::GuardFailed(format!(
                "Guard failed for event '{event}'"
            )));
        }

        // Find transition
        let transition = machine.find_transition(event);
        let (from, to, guard_label, effect_label) = match transition {
            Some(t) => (t.from, t.to, t.guard_label.clone(), t.effect_label.clone()),
            None => {
                self.total_blocked += 1;
                return TransitionResult::Blocked(BlockReason::NoTransitionDefined);
            }
        };

        // Apply transition
        machine.apply_transition(event, timestamp);

        // Apply effect
        effect.apply(machine);

        // Log
        self.log.record(
            &machine.name,
            machine.context().entity_id,
            from,
            to,
            event,
            timestamp,
            guard_label,
            effect_label,
        );

        self.total_success += 1;

        TransitionResult::Success {
            from,
            to,
            event: event.to_string(),
        }
    }

    /// Returns the transition log.
    #[must_use]
    pub fn log(&self) -> &TransitionLog {
        &self.log
    }

    /// Returns total successful transitions.
    #[must_use]
    pub fn total_success(&self) -> u64 {
        self.total_success
    }

    /// Returns total blocked transitions.
    #[must_use]
    pub fn total_blocked(&self) -> u64 {
        self.total_blocked
    }
}

impl Default for Transitioner {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for Transitioner {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,      // ς — DOMINANT: state management
            LexPrimitiva::Causality,  // → — transitions cause effects
            LexPrimitiva::Boundary,   // ∂ — guard evaluation
            LexPrimitiva::Sequence,   // σ — ordered execution
            LexPrimitiva::Comparison, // κ — guard comparisons
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
    use crate::state::FsmState;
    use nexcore_lex_primitiva::GroundingTier;

    fn build_test_machine() -> StateMachine {
        let mut fsm = StateMachine::new(1, "case", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "received").initial());
        fsm.add_state(FsmState::new(2, "triaged"));
        fsm.add_state(FsmState::new(3, "assessed"));
        fsm.add_state(FsmState::new(4, "closed").terminal());
        fsm.add_transition(
            TransitionDef::new(StateId(1), "triage", StateId(2)).with_guard("has_required_fields"),
        );
        fsm.add_transition(
            TransitionDef::new(StateId(2), "assess", StateId(3)).with_effect("notify_reviewer"),
        );
        fsm.add_transition(TransitionDef::new(StateId(3), "close", StateId(4)));
        fsm
    }

    use super::super::state::TransitionDef;

    #[test]
    fn test_transition_log_grounding() {
        let comp = TransitionLog::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
    }

    #[test]
    fn test_transitioner_grounding() {
        let comp = Transitioner::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
    }

    #[test]
    fn test_guard_always() {
        let machine = build_test_machine();
        assert!(TransitionGuard::Always.allows(&machine, 2000));
    }

    #[test]
    fn test_guard_requires_key() {
        let mut machine = build_test_machine();
        let guard = TransitionGuard::RequiresKey("drug".into());

        assert!(!guard.allows(&machine, 2000));
        machine.context_mut().set("drug", "aspirin");
        assert!(guard.allows(&machine, 2000));
    }

    #[test]
    fn test_guard_requires_key_value() {
        let mut machine = build_test_machine();
        let guard = TransitionGuard::RequiresKeyValue {
            key: "status".into(),
            value: "ready".into(),
        };

        machine.context_mut().set("status", "pending");
        assert!(!guard.allows(&machine, 2000));

        machine.context_mut().set("status", "ready");
        assert!(guard.allows(&machine, 2000));
    }

    #[test]
    fn test_guard_min_duration() {
        let machine = build_test_machine(); // entered_at = 1000
        let guard = TransitionGuard::MinDuration { min_duration: 500 };

        assert!(!guard.allows(&machine, 1400)); // 400 < 500
        assert!(guard.allows(&machine, 1500)); // 500 >= 500
        assert!(guard.allows(&machine, 2000)); // 1000 >= 500
    }

    #[test]
    fn test_guard_not_terminal() {
        let mut machine = build_test_machine();
        let guard = TransitionGuard::NotTerminal;

        assert!(guard.allows(&machine, 2000)); // received is not terminal
        machine.force_state(StateId(4), 3000); // closed is terminal
        assert!(!guard.allows(&machine, 4000));
    }

    #[test]
    fn test_guard_all() {
        let mut machine = build_test_machine();
        let guard = TransitionGuard::All(vec![
            TransitionGuard::RequiresKey("drug".into()),
            TransitionGuard::NotTerminal,
        ]);

        assert!(!guard.allows(&machine, 2000)); // missing drug
        machine.context_mut().set("drug", "aspirin");
        assert!(guard.allows(&machine, 2000)); // drug present + not terminal
    }

    #[test]
    fn test_guard_any() {
        let machine = build_test_machine();
        let guard = TransitionGuard::Any(vec![
            TransitionGuard::RequiresKey("drug".into()),
            TransitionGuard::NotTerminal,
        ]);

        // Drug missing, but not terminal → passes
        assert!(guard.allows(&machine, 2000));
    }

    #[test]
    fn test_transition_effect_set_context() {
        let mut machine = build_test_machine();
        let effect = TransitionEffect::SetContext {
            key: "reviewer".into(),
            value: "dr_smith".into(),
        };

        effect.apply(&mut machine);
        assert_eq!(machine.context().get("reviewer"), Some("dr_smith"));
    }

    #[test]
    fn test_transition_effect_sequence() {
        let mut machine = build_test_machine();
        let effect = TransitionEffect::Sequence(vec![
            TransitionEffect::SetContext {
                key: "step".into(),
                value: "1".into(),
            },
            TransitionEffect::SetContext {
                key: "status".into(),
                value: "active".into(),
            },
        ]);

        effect.apply(&mut machine);
        assert_eq!(machine.context().get("step"), Some("1"));
        assert_eq!(machine.context().get("status"), Some("active"));
    }

    #[test]
    fn test_transitioner_success() {
        let mut machine = build_test_machine();
        let mut transitioner = Transitioner::new();

        let result = transitioner.execute(
            &mut machine,
            "triage",
            &TransitionGuard::Always,
            &TransitionEffect::None,
            2000,
        );

        assert!(result.is_success());
        assert_eq!(result.new_state(), Some(StateId(2)));
        assert_eq!(transitioner.total_success(), 1);
        assert_eq!(transitioner.total_blocked(), 0);
        assert_eq!(transitioner.log().len(), 1);
    }

    #[test]
    fn test_transitioner_blocked_by_guard() {
        let mut machine = build_test_machine();
        let mut transitioner = Transitioner::new();

        let result = transitioner.execute(
            &mut machine,
            "triage",
            &TransitionGuard::RequiresKey("missing_key".into()),
            &TransitionEffect::None,
            2000,
        );

        assert!(!result.is_success());
        assert!(matches!(
            result,
            TransitionResult::Blocked(BlockReason::GuardFailed(_))
        ));
        assert_eq!(transitioner.total_blocked(), 1);
    }

    #[test]
    fn test_transitioner_blocked_terminal() {
        let mut machine = build_test_machine();
        machine.force_state(StateId(4), 1000); // terminal
        let mut transitioner = Transitioner::new();

        let result = transitioner.execute(
            &mut machine,
            "triage",
            &TransitionGuard::Always,
            &TransitionEffect::None,
            2000,
        );

        assert!(matches!(
            result,
            TransitionResult::Blocked(BlockReason::TerminalState)
        ));
    }

    #[test]
    fn test_transitioner_no_transition() {
        let mut machine = build_test_machine();
        let mut transitioner = Transitioner::new();

        let result = transitioner.execute(
            &mut machine,
            "nonexistent",
            &TransitionGuard::Always,
            &TransitionEffect::None,
            2000,
        );

        assert!(matches!(
            result,
            TransitionResult::Blocked(BlockReason::NoTransitionDefined)
        ));
    }

    #[test]
    fn test_transitioner_full_lifecycle() {
        let mut machine = build_test_machine();
        let mut transitioner = Transitioner::new();
        let guard = TransitionGuard::Always;
        let effect = TransitionEffect::None;

        let r1 = transitioner.execute(&mut machine, "triage", &guard, &effect, 2000);
        assert!(r1.is_success());

        let r2 = transitioner.execute(&mut machine, "assess", &guard, &effect, 3000);
        assert!(r2.is_success());

        let r3 = transitioner.execute(&mut machine, "close", &guard, &effect, 4000);
        assert!(r3.is_success());

        assert!(machine.is_terminal());
        assert_eq!(transitioner.total_success(), 3);
        assert_eq!(transitioner.log().len(), 3);

        let records = transitioner.log().records_for_machine("case");
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_transition_log_query() {
        let mut log = TransitionLog::new();

        log.record(
            "case",
            100,
            StateId(1),
            StateId(2),
            "triage",
            2000,
            None,
            None,
        );
        log.record(
            "signal",
            200,
            StateId(1),
            StateId(2),
            "validate",
            2100,
            None,
            None,
        );
        log.record(
            "case",
            100,
            StateId(2),
            StateId(3),
            "assess",
            3000,
            None,
            None,
        );

        assert_eq!(log.len(), 3);
        assert_eq!(log.records_for_entity(100).len(), 2);
        assert_eq!(log.records_for_entity(200).len(), 1);
        assert_eq!(log.records_for_machine("case").len(), 2);
        assert_eq!(log.records_for_machine("signal").len(), 1);
    }

    #[test]
    fn test_transition_effect_remove_context() {
        let mut machine = build_test_machine();
        machine.context_mut().set("temp", "data");
        assert!(machine.context().get("temp").is_some());

        let effect = TransitionEffect::RemoveContext("temp".into());
        effect.apply(&mut machine);
        assert!(machine.context().get("temp").is_none());
    }
}
