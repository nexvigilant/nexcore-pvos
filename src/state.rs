//! # PVST — Core State Types
//!
//! Typed state wrappers, finite state machine definitions, and contexts
//! for the PVOS state layer. All lifecycle management is fundamentally
//! about **state and transitions** — this module provides the bedrock.
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role        | Weight |
//! |--------|-------------|--------|
//! | ς      | State       | 0.80 (dominant) |
//!
//! Core state types are pure ς — they represent discrete modes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P IDENTIFIERS
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a state within a machine.
///
/// Tier: T2-P (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub u64);

impl GroundsTo for StateId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State]).with_dominant(LexPrimitiva::State, 1.0)
    }
}

/// Unique identifier for a state machine.
///
/// Tier: T2-P (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateMachineId(pub u64);

impl GroundsTo for StateMachineId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State]).with_dominant(LexPrimitiva::State, 1.0)
    }
}

// ═══════════════════════════════════════════════════════════
// FSM STATE DEFINITION
// ═══════════════════════════════════════════════════════════

/// A named state in a finite state machine.
///
/// Each state has a unique ID, a human-readable name, and flags
/// indicating whether it's initial (entry point) or terminal (end).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsmState {
    /// Unique state identifier.
    pub id: StateId,
    /// Human-readable state name.
    pub name: String,
    /// Whether this is the initial (entry) state.
    pub is_initial: bool,
    /// Whether this is a terminal (end) state.
    pub is_terminal: bool,
}

impl FsmState {
    /// Creates a new state with the given ID and name.
    #[must_use]
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id: StateId(id),
            name: name.to_string(),
            is_initial: false,
            is_terminal: false,
        }
    }

    /// Marks this state as the initial state.
    #[must_use]
    pub fn initial(mut self) -> Self {
        self.is_initial = true;
        self
    }

    /// Marks this state as a terminal state.
    #[must_use]
    pub fn terminal(mut self) -> Self {
        self.is_terminal = true;
        self
    }
}

// ═══════════════════════════════════════════════════════════
// STATE CONTEXT
// ═══════════════════════════════════════════════════════════

/// Context data associated with a state machine instance.
///
/// Carries the entity ID, entry timestamp, and arbitrary key-value
/// data that persists across state transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateContext {
    /// The entity this machine tracks.
    pub entity_id: u64,
    /// Timestamp when current state was entered.
    pub entered_at: u64,
    /// Arbitrary context data.
    pub data: HashMap<String, String>,
}

impl StateContext {
    /// Creates a new context for the given entity.
    #[must_use]
    pub fn new(entity_id: u64, timestamp: u64) -> Self {
        Self {
            entity_id,
            entered_at: timestamp,
            data: HashMap::new(),
        }
    }

    /// Adds a key-value pair to the context.
    #[must_use]
    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }

    /// Gets a value from the context.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Sets a value in the context.
    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }
}

// ═══════════════════════════════════════════════════════════
// CURRENT STATE INFO
// ═══════════════════════════════════════════════════════════

/// Information about the current state of a machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentState {
    /// The current state ID.
    pub state_id: StateId,
    /// The current state name.
    pub state_name: String,
    /// When the current state was entered.
    pub entered_at: u64,
    /// How many transitions have occurred.
    pub transition_count: u64,
}

// ═══════════════════════════════════════════════════════════
// TRANSITION DEFINITION
// ═══════════════════════════════════════════════════════════

/// Definition of a state transition: from state + event → to state.
///
/// Optionally includes guard and effect labels for documentation
/// and audit purposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDef {
    /// Source state.
    pub from: StateId,
    /// Event that triggers the transition.
    pub event: String,
    /// Target state.
    pub to: StateId,
    /// Optional guard label (for documentation/audit).
    pub guard_label: Option<String>,
    /// Optional effect label (for documentation/audit).
    pub effect_label: Option<String>,
}

impl TransitionDef {
    /// Creates a new transition definition.
    #[must_use]
    pub fn new(from: StateId, event: &str, to: StateId) -> Self {
        Self {
            from,
            event: event.to_string(),
            to,
            guard_label: None,
            effect_label: None,
        }
    }

    /// Adds a guard label for documentation.
    #[must_use]
    pub fn with_guard(mut self, label: &str) -> Self {
        self.guard_label = Some(label.to_string());
        self
    }

    /// Adds an effect label for documentation.
    #[must_use]
    pub fn with_effect(mut self, label: &str) -> Self {
        self.effect_label = Some(label.to_string());
        self
    }
}

// ═══════════════════════════════════════════════════════════
// FINITE STATE MACHINE
// ═══════════════════════════════════════════════════════════

/// A finite state machine with states and transitions.
///
/// Manages discrete modes (ς), applies transitions (→), and
/// enforces boundary conditions (∂) through guards.
///
/// Tier: T2-C (ς + → + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    /// Machine identifier.
    pub id: StateMachineId,
    /// Human-readable machine name.
    pub name: String,
    /// All defined states.
    states: Vec<FsmState>,
    /// All defined transitions.
    transitions: Vec<TransitionDef>,
    /// Current state ID.
    current: StateId,
    /// Associated context data.
    context: StateContext,
    /// Total transitions applied.
    transition_count: u64,
}

impl StateMachine {
    /// Creates a new state machine.
    #[must_use]
    pub fn new(
        id: u64,
        name: &str,
        initial_state: StateId,
        entity_id: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            id: StateMachineId(id),
            name: name.to_string(),
            states: Vec::new(),
            transitions: Vec::new(),
            current: initial_state,
            context: StateContext::new(entity_id, timestamp),
            transition_count: 0,
        }
    }

    /// Adds a state to the machine.
    pub fn add_state(&mut self, state: FsmState) {
        self.states.push(state);
    }

    /// Adds a transition definition to the machine.
    pub fn add_transition(&mut self, transition: TransitionDef) {
        self.transitions.push(transition);
    }

    /// Returns the current state ID.
    #[must_use]
    pub fn current_state(&self) -> StateId {
        self.current
    }

    /// Returns the current state name, if the state exists.
    #[must_use]
    pub fn current_state_name(&self) -> Option<&str> {
        self.states
            .iter()
            .find(|s| s.id == self.current)
            .map(|s| s.name.as_str())
    }

    /// Returns information about the current state.
    #[must_use]
    pub fn current_info(&self) -> CurrentState {
        CurrentState {
            state_id: self.current,
            state_name: self.current_state_name().unwrap_or("unknown").to_string(),
            entered_at: self.context.entered_at,
            transition_count: self.transition_count,
        }
    }

    /// Checks whether the current state is terminal.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.states
            .iter()
            .find(|s| s.id == self.current)
            .map_or(false, |s| s.is_terminal)
    }

    /// Returns all events available from the current state.
    #[must_use]
    pub fn available_events(&self) -> Vec<String> {
        self.transitions
            .iter()
            .filter(|t| t.from == self.current)
            .map(|t| t.event.clone())
            .collect()
    }

    /// Checks whether a transition for the given event exists.
    #[must_use]
    pub fn can_transition(&self, event: &str) -> bool {
        self.transitions
            .iter()
            .any(|t| t.from == self.current && t.event == event)
    }

    /// Finds the transition definition for the given event.
    #[must_use]
    pub fn find_transition(&self, event: &str) -> Option<&TransitionDef> {
        self.transitions
            .iter()
            .find(|t| t.from == self.current && t.event == event)
    }

    /// Applies a transition for the given event.
    /// Returns the new state ID if successful, None if no valid transition.
    pub fn apply_transition(&mut self, event: &str, timestamp: u64) -> Option<StateId> {
        let target = self
            .transitions
            .iter()
            .find(|t| t.from == self.current && t.event == event)
            .map(|t| t.to);

        if let Some(to) = target {
            self.current = to;
            self.context.entered_at = timestamp;
            self.transition_count += 1;
            Some(to)
        } else {
            None
        }
    }

    /// Forces the machine to a specific state (for recovery).
    pub fn force_state(&mut self, state: StateId, timestamp: u64) {
        self.current = state;
        self.context.entered_at = timestamp;
    }

    /// Returns the context.
    #[must_use]
    pub fn context(&self) -> &StateContext {
        &self.context
    }

    /// Returns mutable context.
    pub fn context_mut(&mut self) -> &mut StateContext {
        &mut self.context
    }

    /// Returns the number of defined states.
    #[must_use]
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Returns the number of defined transitions.
    #[must_use]
    pub fn transition_def_count(&self) -> usize {
        self.transitions.len()
    }

    /// Returns the total number of transitions applied.
    #[must_use]
    pub fn transition_count(&self) -> u64 {
        self.transition_count
    }

    /// Returns all defined states.
    #[must_use]
    pub fn states(&self) -> &[FsmState] {
        &self.states
    }

    /// Checks whether a state with the given ID exists.
    #[must_use]
    pub fn has_state(&self, id: StateId) -> bool {
        self.states.iter().any(|s| s.id == id)
    }

    /// Returns all transition definitions.
    #[must_use]
    pub fn transitions(&self) -> &[TransitionDef] {
        &self.transitions
    }
}

impl GroundsTo for StateMachine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,     // ς — DOMINANT: discrete modes
            LexPrimitiva::Causality, // → — transitions cause state change
            LexPrimitiva::Boundary,  // ∂ — guards constrain transitions
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
    fn test_state_id_t1_grounding() {
        let comp = StateId::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
    }

    #[test]
    fn test_state_machine_id_t1_grounding() {
        let comp = StateMachineId::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
    }

    #[test]
    fn test_state_machine_t2c_grounding() {
        let comp = StateMachine::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 3);
    }

    #[test]
    fn test_fsm_state_creation() {
        let state = FsmState::new(1, "received").initial();
        assert_eq!(state.id, StateId(1));
        assert_eq!(state.name, "received");
        assert!(state.is_initial);
        assert!(!state.is_terminal);

        let terminal = FsmState::new(4, "closed").terminal();
        assert!(!terminal.is_initial);
        assert!(terminal.is_terminal);
    }

    #[test]
    fn test_state_context() {
        let ctx = StateContext::new(42, 1000)
            .with_data("drug", "aspirin")
            .with_data("event", "headache");

        assert_eq!(ctx.entity_id, 42);
        assert_eq!(ctx.entered_at, 1000);
        assert_eq!(ctx.get("drug"), Some("aspirin"));
        assert_eq!(ctx.get("event"), Some("headache"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_state_context_mutation() {
        let mut ctx = StateContext::new(1, 100);
        ctx.set("status", "active");
        assert_eq!(ctx.get("status"), Some("active"));
        ctx.set("status", "closed");
        assert_eq!(ctx.get("status"), Some("closed"));
    }

    #[test]
    fn test_transition_def() {
        let td = TransitionDef::new(StateId(1), "triage", StateId(2))
            .with_guard("has_required_fields")
            .with_effect("notify_reviewer");

        assert_eq!(td.from, StateId(1));
        assert_eq!(td.event, "triage");
        assert_eq!(td.to, StateId(2));
        assert_eq!(td.guard_label, Some("has_required_fields".into()));
        assert_eq!(td.effect_label, Some("notify_reviewer".into()));
    }

    #[test]
    fn test_state_machine_basic() {
        let mut fsm = StateMachine::new(1, "case_lifecycle", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "received").initial());
        fsm.add_state(FsmState::new(2, "triaged"));
        fsm.add_state(FsmState::new(3, "closed").terminal());
        fsm.add_transition(TransitionDef::new(StateId(1), "triage", StateId(2)));
        fsm.add_transition(TransitionDef::new(StateId(2), "close", StateId(3)));

        assert_eq!(fsm.current_state(), StateId(1));
        assert_eq!(fsm.current_state_name(), Some("received"));
        assert_eq!(fsm.state_count(), 3);
        assert_eq!(fsm.transition_def_count(), 2);
        assert!(!fsm.is_terminal());
    }

    #[test]
    fn test_state_machine_transitions() {
        let mut fsm = StateMachine::new(1, "test", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "a").initial());
        fsm.add_state(FsmState::new(2, "b"));
        fsm.add_state(FsmState::new(3, "c").terminal());
        fsm.add_transition(TransitionDef::new(StateId(1), "go_b", StateId(2)));
        fsm.add_transition(TransitionDef::new(StateId(2), "go_c", StateId(3)));

        assert!(fsm.can_transition("go_b"));
        assert!(!fsm.can_transition("go_c"));

        let result = fsm.apply_transition("go_b", 2000);
        assert_eq!(result, Some(StateId(2)));
        assert_eq!(fsm.current_state(), StateId(2));
        assert_eq!(fsm.transition_count(), 1);

        let result = fsm.apply_transition("go_c", 3000);
        assert_eq!(result, Some(StateId(3)));
        assert!(fsm.is_terminal());
        assert_eq!(fsm.transition_count(), 2);
    }

    #[test]
    fn test_invalid_transition() {
        let mut fsm = StateMachine::new(1, "test", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "a").initial());
        fsm.add_transition(TransitionDef::new(StateId(1), "go_b", StateId(2)));

        let result = fsm.apply_transition("invalid_event", 2000);
        assert_eq!(result, None);
        assert_eq!(fsm.current_state(), StateId(1));
        assert_eq!(fsm.transition_count(), 0);
    }

    #[test]
    fn test_available_events() {
        let mut fsm = StateMachine::new(1, "test", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "hub"));
        fsm.add_transition(TransitionDef::new(StateId(1), "alpha", StateId(2)));
        fsm.add_transition(TransitionDef::new(StateId(1), "beta", StateId(3)));
        fsm.add_transition(TransitionDef::new(StateId(2), "gamma", StateId(3)));

        let events = fsm.available_events();
        assert_eq!(events.len(), 2);
        assert!(events.contains(&"alpha".to_string()));
        assert!(events.contains(&"beta".to_string()));
    }

    #[test]
    fn test_force_state() {
        let mut fsm = StateMachine::new(1, "test", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "a"));
        fsm.add_state(FsmState::new(5, "e"));

        fsm.force_state(StateId(5), 9999);
        assert_eq!(fsm.current_state(), StateId(5));
        assert_eq!(fsm.context().entered_at, 9999);
    }

    #[test]
    fn test_current_info() {
        let mut fsm = StateMachine::new(1, "test", StateId(1), 42, 1000);
        fsm.add_state(FsmState::new(1, "initial_state").initial());

        let info = fsm.current_info();
        assert_eq!(info.state_id, StateId(1));
        assert_eq!(info.state_name, "initial_state");
        assert_eq!(info.entered_at, 1000);
        assert_eq!(info.transition_count, 0);
    }

    #[test]
    fn test_has_state() {
        let mut fsm = StateMachine::new(1, "test", StateId(1), 100, 1000);
        fsm.add_state(FsmState::new(1, "a"));
        fsm.add_state(FsmState::new(2, "b"));

        assert!(fsm.has_state(StateId(1)));
        assert!(fsm.has_state(StateId(2)));
        assert!(!fsm.has_state(StateId(99)));
    }
}
