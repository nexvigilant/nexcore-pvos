//! # PVST — State History
//!
//! Ordered sequences of past states, rewind capability, diffs,
//! and auditable history with retention policies.
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role       | Weight |
//! |--------|------------|--------|
//! | ς      | State      | 0.80 (dominant) |
//! | σ      | Sequence   | 0.10   |
//! | π      | Persistence| 0.05   |
//! | κ      | Comparison | 0.05   |
//!
//! State history is ς + σ — discrete modes ordered over time.

use serde::{Deserialize, Serialize};

use super::state::StateId;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// HISTORY ENTRY
// ═══════════════════════════════════════════════════════════

/// A single entry in the state history.
///
/// Records what state was active, when it was entered,
/// and what event caused the transition into it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryEntry {
    /// The state that was entered.
    pub state_id: StateId,
    /// Human-readable state name.
    pub state_name: String,
    /// Timestamp when this state was entered.
    pub entered_at: u64,
    /// Event that caused transition into this state (None for initial).
    pub caused_by: Option<String>,
}

impl StateHistoryEntry {
    /// Creates a new history entry.
    #[must_use]
    pub fn new(state_id: StateId, state_name: &str, entered_at: u64) -> Self {
        Self {
            state_id,
            state_name: state_name.to_string(),
            entered_at,
            caused_by: None,
        }
    }

    /// Sets the causing event.
    #[must_use]
    pub fn with_cause(mut self, event: &str) -> Self {
        self.caused_by = Some(event.to_string());
        self
    }
}

// ═══════════════════════════════════════════════════════════
// HISTORY POLICY
// ═══════════════════════════════════════════════════════════

/// Policy for how much history to retain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoryPolicy {
    /// Keep all history entries (unlimited).
    Unlimited,
    /// Keep at most N entries, dropping oldest.
    MaxEntries(usize),
    /// Keep entries within a time window.
    TimeWindow {
        /// Maximum age in time units.
        max_age: u64,
    },
}

impl Default for HistoryPolicy {
    fn default() -> Self {
        Self::Unlimited
    }
}

// ═══════════════════════════════════════════════════════════
// STATE DIFF
// ═══════════════════════════════════════════════════════════

/// What changed between two consecutive states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Previous state.
    pub from: StateId,
    /// Previous state name.
    pub from_name: String,
    /// New state.
    pub to: StateId,
    /// New state name.
    pub to_name: String,
    /// Event that caused the change.
    pub event: Option<String>,
    /// Time elapsed between states.
    pub duration: u64,
}

// ═══════════════════════════════════════════════════════════
// STATE HISTORY
// ═══════════════════════════════════════════════════════════

/// Ordered sequence of past states for an entity.
///
/// Provides rewind capability, diff generation, and time-in-state
/// analysis for regulatory audit requirements.
///
/// Tier: T2-C (ς + σ + π + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistory {
    /// Entity this history belongs to.
    entity_id: u64,
    /// Machine name.
    machine_name: String,
    /// Ordered entries (oldest first).
    entries: Vec<StateHistoryEntry>,
    /// Retention policy.
    policy: HistoryPolicy,
}

impl StateHistory {
    /// Creates a new state history.
    #[must_use]
    pub fn new(entity_id: u64, machine_name: &str) -> Self {
        Self {
            entity_id,
            machine_name: machine_name.to_string(),
            entries: Vec::new(),
            policy: HistoryPolicy::default(),
        }
    }

    /// Sets the retention policy.
    #[must_use]
    pub fn with_policy(mut self, policy: HistoryPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Records a state entry.
    pub fn record(
        &mut self,
        state_id: StateId,
        state_name: &str,
        timestamp: u64,
        event: Option<&str>,
    ) {
        let mut entry = StateHistoryEntry::new(state_id, state_name, timestamp);
        if let Some(evt) = event {
            entry = entry.with_cause(evt);
        }
        self.entries.push(entry);
        self.enforce_policy(timestamp);
    }

    /// Enforces the retention policy.
    fn enforce_policy(&mut self, now: u64) {
        match &self.policy {
            HistoryPolicy::Unlimited => {}
            HistoryPolicy::MaxEntries(max) => {
                while self.entries.len() > *max {
                    self.entries.remove(0);
                }
            }
            HistoryPolicy::TimeWindow { max_age } => {
                let cutoff = now.saturating_sub(*max_age);
                self.entries.retain(|e| e.entered_at >= cutoff);
            }
        }
    }

    /// Returns all entries (oldest first).
    #[must_use]
    pub fn entries(&self) -> &[StateHistoryEntry] {
        &self.entries
    }

    /// Returns the most recent entry.
    #[must_use]
    pub fn latest(&self) -> Option<&StateHistoryEntry> {
        self.entries.last()
    }

    /// Returns the N most recent entries.
    #[must_use]
    pub fn recent(&self, n: usize) -> Vec<&StateHistoryEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].iter().collect()
    }

    /// Returns the total number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if no entries exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all entries for a specific state.
    #[must_use]
    pub fn entries_for_state(&self, state_id: StateId) -> Vec<&StateHistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.state_id == state_id)
            .collect()
    }

    /// Counts how many times a specific state was visited.
    #[must_use]
    pub fn visit_count(&self, state_id: StateId) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state_id == state_id)
            .count()
    }

    /// Computes the diff between two consecutive entries.
    #[must_use]
    pub fn diff_at(&self, index: usize) -> Option<StateDiff> {
        if index == 0 || index >= self.entries.len() {
            return None;
        }

        let prev = &self.entries[index - 1];
        let curr = &self.entries[index];

        Some(StateDiff {
            from: prev.state_id,
            from_name: prev.state_name.clone(),
            to: curr.state_id,
            to_name: curr.state_name.clone(),
            event: curr.caused_by.clone(),
            duration: curr.entered_at.saturating_sub(prev.entered_at),
        })
    }

    /// Returns all diffs in the history.
    #[must_use]
    pub fn all_diffs(&self) -> Vec<StateDiff> {
        (1..self.entries.len())
            .filter_map(|i| self.diff_at(i))
            .collect()
    }

    /// Computes time spent in each state.
    #[must_use]
    pub fn time_in_states(&self, now: u64) -> Vec<(StateId, String, u64)> {
        let mut result = Vec::new();

        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let end_time = if i + 1 < self.entries.len() {
                self.entries[i + 1].entered_at
            } else {
                now
            };
            let duration = end_time.saturating_sub(entry.entered_at);
            result.push((entry.state_id, entry.state_name.clone(), duration));
        }

        result
    }

    /// Returns the entity ID.
    #[must_use]
    pub fn entity_id(&self) -> u64 {
        self.entity_id
    }

    /// Returns the machine name.
    #[must_use]
    pub fn machine_name(&self) -> &str {
        &self.machine_name
    }
}

impl GroundsTo for StateHistory {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,       // ς — DOMINANT: state sequences
            LexPrimitiva::Sequence,    // σ — ordered entries
            LexPrimitiva::Persistence, // π — durable history
            LexPrimitiva::Comparison,  // κ — diffs between states
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// STATE REWIND
// ═══════════════════════════════════════════════════════════

/// Capability to rewind a state machine to a previous state.
///
/// Uses history entries to determine valid rewind targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRewind {
    /// Maximum number of steps to rewind.
    pub max_steps: usize,
    /// Whether rewinding is allowed.
    pub enabled: bool,
}

impl StateRewind {
    /// Creates a new rewind capability.
    #[must_use]
    pub fn new(max_steps: usize) -> Self {
        Self {
            max_steps,
            enabled: true,
        }
    }

    /// Disables rewinding.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            max_steps: 0,
            enabled: false,
        }
    }

    /// Computes the rewind target from the history.
    ///
    /// Returns the state ID to rewind to and the index in history,
    /// or None if rewinding is not possible.
    #[must_use]
    pub fn rewind_target(&self, history: &StateHistory, steps: usize) -> Option<(StateId, usize)> {
        if !self.enabled || steps == 0 || steps > self.max_steps {
            return None;
        }

        if history.len() <= steps {
            return None;
        }

        let target_index = history.len() - 1 - steps;
        let entry = &history.entries()[target_index];
        Some((entry.state_id, target_index))
    }
}

// ═══════════════════════════════════════════════════════════
// AUDITABLE HISTORY
// ═══════════════════════════════════════════════════════════

/// Regulatory-grade history store across multiple entities.
///
/// Tier: T2-C (ς + σ + π + κ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditableHistory {
    /// Histories keyed by entity ID.
    histories: std::collections::HashMap<u64, StateHistory>,
}

impl AuditableHistory {
    /// Creates a new auditable history store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            histories: std::collections::HashMap::new(),
        }
    }

    /// Gets or creates a history for an entity.
    pub fn get_or_create(&mut self, entity_id: u64, machine_name: &str) -> &mut StateHistory {
        self.histories
            .entry(entity_id)
            .or_insert_with(|| StateHistory::new(entity_id, machine_name))
    }

    /// Gets the history for an entity.
    #[must_use]
    pub fn get(&self, entity_id: u64) -> Option<&StateHistory> {
        self.histories.get(&entity_id)
    }

    /// Returns all tracked entity IDs.
    #[must_use]
    pub fn entity_ids(&self) -> Vec<u64> {
        self.histories.keys().copied().collect()
    }

    /// Returns the total number of tracked entities.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.histories.len()
    }

    /// Returns the total number of history entries across all entities.
    #[must_use]
    pub fn total_entries(&self) -> usize {
        self.histories.values().map(|h| h.len()).sum()
    }
}

impl GroundsTo for AuditableHistory {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,       // ς — DOMINANT: state tracking
            LexPrimitiva::Sequence,    // σ — ordered entries
            LexPrimitiva::Persistence, // π — regulatory-grade durability
            LexPrimitiva::Comparison,  // κ — state diffs
            LexPrimitiva::Mapping,     // μ — entity → history mapping
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
    fn test_state_history_grounding() {
        let comp = StateHistory::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_auditable_history_grounding() {
        let comp = AuditableHistory::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_history_record_and_retrieve() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "received", 1000, None);
        history.record(StateId(2), "triaged", 2000, Some("triage"));
        history.record(StateId(3), "assessed", 3000, Some("assess"));

        assert_eq!(history.len(), 3);
        assert!(!history.is_empty());
        assert_eq!(history.entity_id(), 100);
        assert_eq!(history.machine_name(), "case");
    }

    #[test]
    fn test_history_latest() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "received", 1000, None);
        history.record(StateId(2), "triaged", 2000, Some("triage"));

        let latest = history.latest();
        assert!(latest.is_some());
        if let Some(entry) = latest {
            assert_eq!(entry.state_id, StateId(2));
            assert_eq!(entry.state_name, "triaged");
            assert_eq!(entry.caused_by, Some("triage".into()));
        }
    }

    #[test]
    fn test_history_recent() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "a", 1000, None);
        history.record(StateId(2), "b", 2000, None);
        history.record(StateId(3), "c", 3000, None);
        history.record(StateId(4), "d", 4000, None);

        let recent = history.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].state_id, StateId(3));
        assert_eq!(recent[1].state_id, StateId(4));
    }

    #[test]
    fn test_history_visit_count() {
        let mut history = StateHistory::new(100, "workflow");
        history.record(StateId(2), "running", 1000, Some("start"));
        history.record(StateId(4), "failed", 2000, Some("fail"));
        history.record(StateId(2), "running", 3000, Some("retry"));
        history.record(StateId(3), "completed", 4000, Some("complete"));

        assert_eq!(history.visit_count(StateId(2)), 2);
        assert_eq!(history.visit_count(StateId(4)), 1);
        assert_eq!(history.visit_count(StateId(3)), 1);
        assert_eq!(history.visit_count(StateId(99)), 0);
    }

    #[test]
    fn test_history_diff() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "received", 1000, None);
        history.record(StateId(2), "triaged", 2000, Some("triage"));
        history.record(StateId(3), "assessed", 5000, Some("assess"));

        let diff = history.diff_at(1);
        assert!(diff.is_some());
        if let Some(d) = diff {
            assert_eq!(d.from, StateId(1));
            assert_eq!(d.to, StateId(2));
            assert_eq!(d.event, Some("triage".into()));
            assert_eq!(d.duration, 1000);
        }

        let diff2 = history.diff_at(2);
        assert!(diff2.is_some());
        if let Some(d) = diff2 {
            assert_eq!(d.duration, 3000);
        }

        assert!(history.diff_at(0).is_none());
        assert!(history.diff_at(99).is_none());
    }

    #[test]
    fn test_history_all_diffs() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "a", 1000, None);
        history.record(StateId(2), "b", 2000, Some("ab"));
        history.record(StateId(3), "c", 4000, Some("bc"));

        let diffs = history.all_diffs();
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].duration, 1000);
        assert_eq!(diffs[1].duration, 2000);
    }

    #[test]
    fn test_history_time_in_states() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "received", 1000, None);
        history.record(StateId(2), "triaged", 3000, Some("triage"));
        history.record(StateId(3), "assessed", 5000, Some("assess"));

        let times = history.time_in_states(7000);
        assert_eq!(times.len(), 3);
        assert_eq!(times[0].2, 2000); // received: 3000 - 1000
        assert_eq!(times[1].2, 2000); // triaged: 5000 - 3000
        assert_eq!(times[2].2, 2000); // assessed: 7000 - 5000
    }

    #[test]
    fn test_history_policy_max_entries() {
        let mut history = StateHistory::new(100, "case").with_policy(HistoryPolicy::MaxEntries(2));

        history.record(StateId(1), "a", 1000, None);
        history.record(StateId(2), "b", 2000, None);
        history.record(StateId(3), "c", 3000, None);

        assert_eq!(history.len(), 2);
        assert_eq!(history.entries()[0].state_id, StateId(2));
        assert_eq!(history.entries()[1].state_id, StateId(3));
    }

    #[test]
    fn test_history_policy_time_window() {
        let mut history =
            StateHistory::new(100, "case").with_policy(HistoryPolicy::TimeWindow { max_age: 2000 });

        history.record(StateId(1), "a", 1000, None);
        history.record(StateId(2), "b", 2000, None);
        history.record(StateId(3), "c", 3500, None);

        // At timestamp 3500, max_age 2000 → cutoff = 1500
        // Entry at 1000 is before cutoff, so dropped
        assert_eq!(history.len(), 2);
        assert_eq!(history.entries()[0].state_id, StateId(2));
    }

    #[test]
    fn test_state_rewind() {
        let mut history = StateHistory::new(100, "case");
        history.record(StateId(1), "a", 1000, None);
        history.record(StateId(2), "b", 2000, None);
        history.record(StateId(3), "c", 3000, None);

        let rewind = StateRewind::new(5);
        let target = rewind.rewind_target(&history, 1);
        assert!(target.is_some());
        if let Some((state_id, _idx)) = target {
            assert_eq!(state_id, StateId(2)); // one step back from c → b
        }

        let target2 = rewind.rewind_target(&history, 2);
        assert!(target2.is_some());
        if let Some((state_id, _idx)) = target2 {
            assert_eq!(state_id, StateId(1)); // two steps back → a
        }

        // Can't rewind more steps than history
        assert!(rewind.rewind_target(&history, 3).is_none());
    }

    #[test]
    fn test_state_rewind_disabled() {
        let history = StateHistory::new(100, "case");
        let rewind = StateRewind::disabled();
        assert!(rewind.rewind_target(&history, 1).is_none());
    }

    #[test]
    fn test_auditable_history() {
        let mut audit = AuditableHistory::new();

        let h1 = audit.get_or_create(100, "case");
        h1.record(StateId(1), "received", 1000, None);
        h1.record(StateId(2), "triaged", 2000, Some("triage"));

        let h2 = audit.get_or_create(200, "signal");
        h2.record(StateId(1), "detected", 1500, None);

        assert_eq!(audit.entity_count(), 2);
        assert_eq!(audit.total_entries(), 3);

        let ids = audit.entity_ids();
        assert!(ids.contains(&100));
        assert!(ids.contains(&200));

        let case_history = audit.get(100);
        assert!(case_history.is_some());
        if let Some(h) = case_history {
            assert_eq!(h.len(), 2);
        }
    }
}
