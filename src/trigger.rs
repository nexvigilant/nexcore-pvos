//! # PVOC Trigger Definitions
//!
//! Condition → action mappings that fire when events match.
//! Triggers are the causal glue — when X happens, do Y.
//!
//! ## Primitives
//! - → (Causality) — trigger IS causality (condition causes action)
//! - ∂ (Boundary) — guard conditions, constraints
//! - ν (Frequency) — debounce, rate limiting

use serde::{Deserialize, Serialize};

use super::event::{EventKind, EventSource, OrcEventId};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique trigger identifier.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(pub u64);

impl GroundsTo for TriggerId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

// ═══════════════════════════════════════════════════════════
// TRIGGER PRIORITY
// ═══════════════════════════════════════════════════════════

/// Priority ordering when multiple triggers match the same event.
/// Higher priority triggers fire first.
///
/// Tier: T2-P (κ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TriggerPriority {
    /// Lowest priority — fire last
    Low = 0,
    /// Default priority
    Normal = 1,
    /// Elevated priority
    High = 2,
    /// Highest priority — fire first
    Critical = 3,
}

impl Default for TriggerPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl GroundsTo for TriggerPriority {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
    }
}

// ═══════════════════════════════════════════════════════════
// TRIGGER CONDITION
// ═══════════════════════════════════════════════════════════

/// When a trigger should fire.
/// Tier: T2-C (→ + ∂ + κ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Fire when a specific event kind occurs
    OnEvent(EventKind),
    /// Fire when event from a specific source
    FromSource(EventSource),
    /// Fire when event kind AND source match
    OnEventFrom {
        kind: EventKind,
        source: EventSource,
    },
    /// Fire when a metric crosses a threshold
    MetricCrosses { metric_name: String, threshold: f64 },
    /// Fire when a numeric field in payload exceeds value
    PayloadExceeds { field: String, threshold: f64 },
    /// Fire on any event (wildcard)
    Always,
}

impl TriggerCondition {
    /// Creates a condition matching a specific event kind.
    #[must_use]
    pub fn on_event(kind: EventKind) -> Self {
        Self::OnEvent(kind)
    }

    /// Creates a condition matching events from a source.
    #[must_use]
    pub fn from_source(source: EventSource) -> Self {
        Self::FromSource(source)
    }

    /// Creates a condition matching event kind + source.
    #[must_use]
    pub fn on_event_from(kind: EventKind, source: EventSource) -> Self {
        Self::OnEventFrom { kind, source }
    }

    /// Creates a metric threshold crossing condition.
    #[must_use]
    pub fn metric_crosses(name: &str, threshold: f64) -> Self {
        Self::MetricCrosses {
            metric_name: name.into(),
            threshold,
        }
    }

    /// Checks if a given event kind and source match this condition.
    #[must_use]
    pub fn matches(&self, kind: &EventKind, source: &EventSource) -> bool {
        match self {
            Self::OnEvent(expected_kind) => kind == expected_kind,
            Self::FromSource(expected_source) => source == expected_source,
            Self::OnEventFrom {
                kind: expected_kind,
                source: expected_source,
            } => kind == expected_kind && source == expected_source,
            Self::MetricCrosses { .. } => {
                // Metric conditions are checked against metric values, not event kinds
                matches!(kind, EventKind::MetricUpdated | EventKind::ThresholdCrossed)
            }
            Self::PayloadExceeds { .. } => true, // Checked at payload level
            Self::Always => true,
        }
    }
}

impl GroundsTo for TriggerCondition {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,  // → — defines when causation occurs
            LexPrimitiva::Boundary,   // ∂ — conditional constraints
            LexPrimitiva::Comparison, // κ — matching logic
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// TRIGGER ACTION
// ═══════════════════════════════════════════════════════════

/// What to do when a trigger fires.
/// Tier: T2-C (→ + σ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerAction {
    /// Emit a new orchestration event (→ chain)
    EmitEvent {
        kind: EventKind,
        source: EventSource,
    },
    /// Start a named workflow
    StartWorkflow(String),
    /// Increment a metric counter
    IncrementMetric(String),
    /// Send an alert with severity and message
    SendAlert { severity: String, message: String },
    /// Log a message to the audit trail
    AuditLog(String),
    /// Execute multiple actions in sequence
    Sequence(Vec<TriggerAction>),
    /// No action (useful for testing/dry-run)
    Noop,
}

impl TriggerAction {
    /// Creates an emit-event action.
    #[must_use]
    pub fn emit(kind: EventKind, source: EventSource) -> Self {
        Self::EmitEvent { kind, source }
    }

    /// Creates a workflow start action.
    #[must_use]
    pub fn workflow(name: &str) -> Self {
        Self::StartWorkflow(name.into())
    }

    /// Creates a metric increment action.
    #[must_use]
    pub fn metric_inc(name: &str) -> Self {
        Self::IncrementMetric(name.into())
    }

    /// Creates an alert action.
    #[must_use]
    pub fn alert(severity: &str, message: &str) -> Self {
        Self::SendAlert {
            severity: severity.into(),
            message: message.into(),
        }
    }

    /// Returns the number of actions (1 for simple, N for Sequence).
    #[must_use]
    pub fn action_count(&self) -> usize {
        match self {
            Self::Sequence(actions) => actions.len(),
            Self::Noop => 0,
            _ => 1,
        }
    }
}

impl GroundsTo for TriggerAction {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — the effect in cause→effect
            LexPrimitiva::Sequence,  // σ — action sequencing
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// TRIGGER GUARD
// ═══════════════════════════════════════════════════════════

/// Additional constraints that must be satisfied for a trigger to fire.
/// Guards act as ∂ boundaries — even if the condition matches,
/// the guard can prevent firing.
///
/// Tier: T2-C (∂ + ν + κ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerGuard {
    /// Minimum time between firings (debounce, in millis)
    pub debounce_ms: Option<u64>,
    /// Maximum total firings before auto-disable
    pub max_firings: Option<u64>,
    /// Required source for the event
    pub required_source: Option<EventSource>,
    /// Whether the trigger is currently enabled
    pub enabled: bool,
}

impl TriggerGuard {
    /// Creates a guard with no constraints.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            debounce_ms: None,
            max_firings: None,
            required_source: None,
            enabled: true,
        }
    }

    /// Adds debounce interval.
    #[must_use]
    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = Some(ms);
        self
    }

    /// Adds max firing limit.
    #[must_use]
    pub fn with_max_firings(mut self, max: u64) -> Self {
        self.max_firings = Some(max);
        self
    }

    /// Requires events from a specific source.
    #[must_use]
    pub fn with_required_source(mut self, source: EventSource) -> Self {
        self.required_source = Some(source);
        self
    }

    /// Checks if the guard allows firing given current state.
    #[must_use]
    pub fn allows(&self, firing_count: u64, last_fired_ms: Option<u64>, now_ms: u64) -> bool {
        if !self.enabled {
            return false;
        }

        // Check max firings
        if let Some(max) = self.max_firings {
            if firing_count >= max {
                return false;
            }
        }

        // Check debounce
        if let Some(debounce) = self.debounce_ms {
            if let Some(last) = last_fired_ms {
                if now_ms.saturating_sub(last) < debounce {
                    return false;
                }
            }
        }

        true
    }
}

impl Default for TriggerGuard {
    fn default() -> Self {
        Self::permissive()
    }
}

impl GroundsTo for TriggerGuard {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary,   // ∂ — constraints
            LexPrimitiva::Frequency,  // ν — debounce/rate
            LexPrimitiva::Comparison, // κ — threshold checks
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// DEBOUNCE
// ═══════════════════════════════════════════════════════════

/// Rate limiter for trigger firing.
/// Prevents the same trigger from flooding the system.
///
/// Tier: T2-P (ν + ∂)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Debounce {
    /// Minimum interval between firings (millis)
    interval_ms: u64,
    /// Last firing timestamp
    last_fired: Option<u64>,
    /// Total suppressed firings
    suppressed: u64,
}

impl Debounce {
    /// Creates a debounce with given interval.
    #[must_use]
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            last_fired: None,
            suppressed: 0,
        }
    }

    /// Checks if enough time has passed since last firing.
    #[must_use]
    pub fn can_fire(&self, now_ms: u64) -> bool {
        match self.last_fired {
            None => true,
            Some(last) => now_ms.saturating_sub(last) >= self.interval_ms,
        }
    }

    /// Records a firing. Returns true if allowed, false if suppressed.
    pub fn try_fire(&mut self, now_ms: u64) -> bool {
        if self.can_fire(now_ms) {
            self.last_fired = Some(now_ms);
            true
        } else {
            self.suppressed += 1;
            false
        }
    }

    /// Returns total suppressed firings.
    #[must_use]
    pub fn suppressed(&self) -> u64 {
        self.suppressed
    }

    /// Resets the debounce state.
    pub fn reset(&mut self) {
        self.last_fired = None;
        self.suppressed = 0;
    }
}

impl GroundsTo for Debounce {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency, // ν — rate limiting
            LexPrimitiva::Boundary,  // ∂ — time-based constraint
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// TRIGGER — THE COMPOSED TYPE
// ═══════════════════════════════════════════════════════════

/// A trigger: when condition is met and guard allows, execute action.
/// This IS causality — the fundamental cause→effect binding.
///
/// Tier: T2-C (→ + ∂ + ν + κ + σ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    /// Unique trigger ID
    pub id: TriggerId,
    /// Human-readable name
    pub name: String,
    /// When to fire
    pub condition: TriggerCondition,
    /// What to do
    pub action: TriggerAction,
    /// Additional constraints
    pub guard: TriggerGuard,
    /// Firing priority
    pub priority: TriggerPriority,
    /// Total times this trigger has fired
    firing_count: u64,
    /// Last firing timestamp
    last_fired_ms: Option<u64>,
    /// Debounce controller
    debounce: Option<Debounce>,
}

impl Trigger {
    /// Creates a new trigger.
    #[must_use]
    pub fn new(
        id: TriggerId,
        name: &str,
        condition: TriggerCondition,
        action: TriggerAction,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            condition,
            action,
            guard: TriggerGuard::default(),
            priority: TriggerPriority::default(),
            firing_count: 0,
            last_fired_ms: None,
            debounce: None,
        }
    }

    /// Sets the guard.
    #[must_use]
    pub fn with_guard(mut self, guard: TriggerGuard) -> Self {
        self.guard = guard;
        self
    }

    /// Sets the priority.
    #[must_use]
    pub fn with_priority(mut self, priority: TriggerPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Adds debounce control.
    #[must_use]
    pub fn with_debounce(mut self, interval_ms: u64) -> Self {
        self.debounce = Some(Debounce::new(interval_ms));
        self
    }

    /// Checks if this trigger matches the given event kind and source.
    #[must_use]
    pub fn matches(&self, kind: &EventKind, source: &EventSource) -> bool {
        self.condition.matches(kind, source)
    }

    /// Attempts to fire the trigger. Returns the action if allowed,
    /// None if guard or debounce prevents firing.
    pub fn try_fire(&mut self, now_ms: u64) -> Option<&TriggerAction> {
        // Check guard
        if !self
            .guard
            .allows(self.firing_count, self.last_fired_ms, now_ms)
        {
            return None;
        }

        // Check debounce
        if let Some(ref mut debounce) = self.debounce {
            if !debounce.try_fire(now_ms) {
                return None;
            }
        }

        self.firing_count += 1;
        self.last_fired_ms = Some(now_ms);
        Some(&self.action)
    }

    /// Returns total firing count.
    #[must_use]
    pub fn firing_count(&self) -> u64 {
        self.firing_count
    }

    /// Returns true if the trigger is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.guard.enabled
    }

    /// Disables the trigger.
    pub fn disable(&mut self) {
        self.guard.enabled = false;
    }

    /// Enables the trigger.
    pub fn enable(&mut self) {
        self.guard.enabled = true;
    }

    /// Returns the last event ID that caused this trigger to fire.
    #[must_use]
    pub fn last_fired_at(&self) -> Option<u64> {
        self.last_fired_ms
    }
}

impl GroundsTo for Trigger {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,  // → — trigger IS causality
            LexPrimitiva::Boundary,   // ∂ — guard conditions
            LexPrimitiva::Frequency,  // ν — debounce
            LexPrimitiva::Comparison, // κ — condition matching
            LexPrimitiva::Sequence,   // σ — action sequencing
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
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
    fn test_trigger_id_grounding() {
        let comp = TriggerId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
    }

    #[test]
    fn test_trigger_priority_ordering() {
        assert!(TriggerPriority::Critical > TriggerPriority::High);
        assert!(TriggerPriority::High > TriggerPriority::Normal);
        assert!(TriggerPriority::Normal > TriggerPriority::Low);
    }

    #[test]
    fn test_trigger_condition_on_event() {
        let cond = TriggerCondition::on_event(EventKind::SignalDetected);
        assert!(cond.matches(&EventKind::SignalDetected, &EventSource::Avc));
        assert!(!cond.matches(&EventKind::WorkflowStarted, &EventSource::Avc));
    }

    #[test]
    fn test_trigger_condition_from_source() {
        let cond = TriggerCondition::from_source(EventSource::Pvml);
        assert!(cond.matches(&EventKind::ModelRetrained, &EventSource::Pvml));
        assert!(!cond.matches(&EventKind::ModelRetrained, &EventSource::Avc));
    }

    #[test]
    fn test_trigger_condition_on_event_from() {
        let cond = TriggerCondition::on_event_from(EventKind::SignalDetected, EventSource::Avc);
        assert!(cond.matches(&EventKind::SignalDetected, &EventSource::Avc));
        assert!(!cond.matches(&EventKind::SignalDetected, &EventSource::Pvos));
        assert!(!cond.matches(&EventKind::WorkflowStarted, &EventSource::Avc));
    }

    #[test]
    fn test_trigger_condition_always() {
        let cond = TriggerCondition::Always;
        assert!(cond.matches(&EventKind::SystemBooted, &EventSource::Pvos));
        assert!(cond.matches(&EventKind::SignalDetected, &EventSource::Avc));
    }

    #[test]
    fn test_trigger_guard_permissive() {
        let guard = TriggerGuard::permissive();
        assert!(guard.allows(0, None, 1000));
        assert!(guard.allows(999, None, 1000));
    }

    #[test]
    fn test_trigger_guard_max_firings() {
        let guard = TriggerGuard::permissive().with_max_firings(3);
        assert!(guard.allows(0, None, 1000));
        assert!(guard.allows(2, None, 1000));
        assert!(!guard.allows(3, None, 1000));
    }

    #[test]
    fn test_trigger_guard_debounce() {
        let guard = TriggerGuard::permissive().with_debounce(100);
        // First firing: no last fired, should allow
        assert!(guard.allows(0, None, 1000));
        // Too soon after last firing
        assert!(!guard.allows(1, Some(950), 1000));
        // Enough time passed
        assert!(guard.allows(1, Some(850), 1000));
    }

    #[test]
    fn test_trigger_guard_disabled() {
        let mut guard = TriggerGuard::permissive();
        guard.enabled = false;
        assert!(!guard.allows(0, None, 1000));
    }

    #[test]
    fn test_debounce_basic() {
        let mut debounce = Debounce::new(100);
        assert!(debounce.can_fire(0));

        assert!(debounce.try_fire(0));
        assert!(!debounce.try_fire(50)); // Too soon
        assert_eq!(debounce.suppressed(), 1);

        assert!(debounce.try_fire(100)); // Exactly at interval
        assert_eq!(debounce.suppressed(), 1);
    }

    #[test]
    fn test_debounce_reset() {
        let mut debounce = Debounce::new(100);
        assert!(debounce.try_fire(0));
        assert!(!debounce.try_fire(50));
        assert_eq!(debounce.suppressed(), 1);

        debounce.reset();
        assert!(debounce.try_fire(50));
        assert_eq!(debounce.suppressed(), 0);
    }

    #[test]
    fn test_trigger_creation_and_fire() {
        let mut trigger = Trigger::new(
            TriggerId(1),
            "signal_to_workflow",
            TriggerCondition::on_event(EventKind::SignalDetected),
            TriggerAction::workflow("signal_triage"),
        );

        assert!(trigger.matches(&EventKind::SignalDetected, &EventSource::Avc));
        assert!(!trigger.matches(&EventKind::WorkflowStarted, &EventSource::Pvwf));

        let action = trigger.try_fire(1000);
        assert!(action.is_some());
        assert_eq!(trigger.firing_count(), 1);
    }

    #[test]
    fn test_trigger_with_guard_prevents_fire() {
        let mut trigger = Trigger::new(
            TriggerId(2),
            "limited_trigger",
            TriggerCondition::Always,
            TriggerAction::Noop,
        )
        .with_guard(TriggerGuard::permissive().with_max_firings(2));

        assert!(trigger.try_fire(100).is_some());
        assert!(trigger.try_fire(200).is_some());
        assert!(trigger.try_fire(300).is_none()); // Max reached
        assert_eq!(trigger.firing_count(), 2);
    }

    #[test]
    fn test_trigger_with_debounce() {
        let mut trigger = Trigger::new(
            TriggerId(3),
            "debounced_trigger",
            TriggerCondition::Always,
            TriggerAction::metric_inc("count"),
        )
        .with_debounce(100);

        assert!(trigger.try_fire(0).is_some());
        assert!(trigger.try_fire(50).is_none()); // Debounced
        assert!(trigger.try_fire(100).is_some()); // OK
        assert_eq!(trigger.firing_count(), 2);
    }

    #[test]
    fn test_trigger_disable_enable() {
        let mut trigger = Trigger::new(
            TriggerId(4),
            "toggle_trigger",
            TriggerCondition::Always,
            TriggerAction::Noop,
        );

        assert!(trigger.is_enabled());
        trigger.disable();
        assert!(!trigger.is_enabled());
        assert!(trigger.try_fire(1000).is_none());

        trigger.enable();
        assert!(trigger.try_fire(1000).is_some());
    }

    #[test]
    fn test_trigger_grounding() {
        let comp = Trigger::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_trigger_action_count() {
        assert_eq!(TriggerAction::Noop.action_count(), 0);
        assert_eq!(TriggerAction::workflow("test").action_count(), 1);
        assert_eq!(
            TriggerAction::Sequence(vec![
                TriggerAction::metric_inc("a"),
                TriggerAction::metric_inc("b"),
            ])
            .action_count(),
            2
        );
    }

    #[test]
    fn test_trigger_condition_metric_crosses() {
        let cond = TriggerCondition::metric_crosses("signal_count", 100.0);
        // Metric conditions match metric-related events
        assert!(cond.matches(&EventKind::MetricUpdated, &EventSource::Pvmx));
        assert!(cond.matches(&EventKind::ThresholdCrossed, &EventSource::Pvmx));
        // But not random events
        assert!(!cond.matches(&EventKind::SystemBooted, &EventSource::Pvos));
    }
}
