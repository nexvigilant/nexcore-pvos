//! # PVST — State Persistence (Snapshots)
//!
//! Point-in-time state captures, persistent storage, recovery,
//! checkpoint policies, and multi-entity consistent snapshots.
//!
//! ## Primitive Grounding
//!
//! | Symbol | Role        | Weight |
//! |--------|-------------|--------|
//! | ς      | State       | 0.80 (dominant) |
//! | π      | Persistence | 0.10   |
//! | σ      | Sequence    | 0.05   |
//! | ∃      | Existence   | 0.05   |
//!
//! State snapshots are ς + π — capturing discrete modes for durability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::state::StateId;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P IDENTIFIERS
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a state snapshot.
///
/// Tier: T2-P (ς + π)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub u64);

// ═══════════════════════════════════════════════════════════
// STATE SNAPSHOT
// ═══════════════════════════════════════════════════════════

/// A point-in-time capture of an entity's state.
///
/// Includes the state ID, context data, and metadata for
/// regulatory-grade state recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Snapshot identifier.
    pub id: SnapshotId,
    /// Entity this snapshot belongs to.
    pub entity_id: u64,
    /// Machine name at time of snapshot.
    pub machine_name: String,
    /// State at time of snapshot.
    pub state_id: StateId,
    /// State name at time of snapshot.
    pub state_name: String,
    /// Context data at time of snapshot.
    pub context_data: HashMap<String, String>,
    /// Timestamp of the snapshot.
    pub timestamp: u64,
    /// Number of transitions at time of snapshot.
    pub transition_count: u64,
    /// Optional description/reason for snapshot.
    pub reason: Option<String>,
}

impl StateSnapshot {
    /// Creates a new snapshot with the given parameters.
    #[must_use]
    pub fn new(
        id: SnapshotId,
        entity_id: u64,
        machine_name: &str,
        state_id: StateId,
        state_name: &str,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            entity_id,
            machine_name: machine_name.to_string(),
            state_id,
            state_name: state_name.to_string(),
            context_data: HashMap::new(),
            timestamp,
            transition_count: 0,
            reason: None,
        }
    }

    /// Sets the context data from a map.
    #[must_use]
    pub fn with_context(mut self, data: HashMap<String, String>) -> Self {
        self.context_data = data;
        self
    }

    /// Sets the transition count.
    #[must_use]
    pub fn with_transition_count(mut self, count: u64) -> Self {
        self.transition_count = count;
        self
    }

    /// Sets the reason for the snapshot.
    #[must_use]
    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = Some(reason.to_string());
        self
    }
}

// ═══════════════════════════════════════════════════════════
// CHECKPOINT POLICY
// ═══════════════════════════════════════════════════════════

/// Policy determining when to take automatic snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointPolicy {
    /// Never take automatic snapshots.
    Manual,
    /// Snapshot every N transitions.
    EveryNTransitions(u64),
    /// Snapshot when entering specific states.
    OnStates(Vec<StateId>),
    /// Snapshot on every transition.
    Always,
}

impl Default for CheckpointPolicy {
    fn default() -> Self {
        Self::Manual
    }
}

impl CheckpointPolicy {
    /// Determines whether a snapshot should be taken.
    #[must_use]
    pub fn should_snapshot(&self, transition_count: u64, current_state: StateId) -> bool {
        match self {
            Self::Manual => false,
            Self::EveryNTransitions(n) => *n > 0 && transition_count % n == 0,
            Self::OnStates(states) => states.contains(&current_state),
            Self::Always => true,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// SNAPSHOT STORE
// ═══════════════════════════════════════════════════════════

/// Persistent storage for state snapshots.
///
/// Provides creation, retrieval, and query capabilities for
/// regulatory-grade state persistence.
///
/// Tier: T2-C (ς + π + σ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStore {
    /// All stored snapshots.
    snapshots: Vec<StateSnapshot>,
    /// Next snapshot ID.
    next_id: u64,
    /// Maximum snapshots to retain per entity (0 = unlimited).
    max_per_entity: usize,
}

impl SnapshotStore {
    /// Creates a new snapshot store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            next_id: 1,
            max_per_entity: 0,
        }
    }

    /// Sets the maximum snapshots per entity.
    #[must_use]
    pub fn with_max_per_entity(mut self, max: usize) -> Self {
        self.max_per_entity = max;
        self
    }

    /// Takes a snapshot and stores it.
    pub fn take_snapshot(
        &mut self,
        entity_id: u64,
        machine_name: &str,
        state_id: StateId,
        state_name: &str,
        context: &HashMap<String, String>,
        transition_count: u64,
        timestamp: u64,
        reason: Option<&str>,
    ) -> SnapshotId {
        let id = SnapshotId(self.next_id);
        self.next_id += 1;

        let mut snapshot =
            StateSnapshot::new(id, entity_id, machine_name, state_id, state_name, timestamp)
                .with_context(context.clone())
                .with_transition_count(transition_count);

        if let Some(r) = reason {
            snapshot = snapshot.with_reason(r);
        }

        self.snapshots.push(snapshot);
        self.enforce_limits(entity_id);

        id
    }

    /// Enforces per-entity limits.
    fn enforce_limits(&mut self, entity_id: u64) {
        if self.max_per_entity == 0 {
            return;
        }

        let entity_count = self
            .snapshots
            .iter()
            .filter(|s| s.entity_id == entity_id)
            .count();

        if entity_count > self.max_per_entity {
            // Find and remove the oldest snapshot for this entity
            if let Some(idx) = self.snapshots.iter().position(|s| s.entity_id == entity_id) {
                self.snapshots.remove(idx);
            }
        }
    }

    /// Gets a snapshot by ID.
    #[must_use]
    pub fn get(&self, id: SnapshotId) -> Option<&StateSnapshot> {
        self.snapshots.iter().find(|s| s.id == id)
    }

    /// Gets the latest snapshot for an entity.
    #[must_use]
    pub fn latest_for_entity(&self, entity_id: u64) -> Option<&StateSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.entity_id == entity_id)
            .last()
    }

    /// Gets all snapshots for an entity.
    #[must_use]
    pub fn for_entity(&self, entity_id: u64) -> Vec<&StateSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.entity_id == entity_id)
            .collect()
    }

    /// Returns the total number of stored snapshots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Returns true if no snapshots are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Returns snapshots taken after a given timestamp.
    #[must_use]
    pub fn after(&self, timestamp: u64) -> Vec<&StateSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.timestamp > timestamp)
            .collect()
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for SnapshotStore {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,       // ς — DOMINANT: state captures
            LexPrimitiva::Persistence, // π — durable storage
            LexPrimitiva::Sequence,    // σ — ordered snapshots
            LexPrimitiva::Existence,   // ∃ — entity existence verification
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// STATE RECOVERY
// ═══════════════════════════════════════════════════════════

/// Recovery outcome from restoring a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryOutcome {
    /// Successfully restored to snapshot state.
    Restored {
        snapshot_id: SnapshotId,
        restored_state: StateId,
    },
    /// Snapshot not found.
    SnapshotNotFound(SnapshotId),
    /// Entity mismatch — snapshot belongs to different entity.
    EntityMismatch { expected: u64, actual: u64 },
}

impl RecoveryOutcome {
    /// Returns true if recovery succeeded.
    #[must_use]
    pub fn is_restored(&self) -> bool {
        matches!(self, Self::Restored { .. })
    }
}

/// State recovery manager.
///
/// Restores state machines from snapshots with validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRecovery {
    /// Total recovery attempts.
    pub total_attempts: u64,
    /// Total successful recoveries.
    pub total_restored: u64,
    /// Total failed recoveries.
    pub total_failed: u64,
}

impl StateRecovery {
    /// Creates a new state recovery manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_attempts: 0,
            total_restored: 0,
            total_failed: 0,
        }
    }

    /// Attempts to recover an entity to a snapshot state.
    pub fn recover(
        &mut self,
        store: &SnapshotStore,
        snapshot_id: SnapshotId,
        entity_id: u64,
    ) -> RecoveryOutcome {
        self.total_attempts += 1;

        let snapshot = match store.get(snapshot_id) {
            Some(s) => s,
            None => {
                self.total_failed += 1;
                return RecoveryOutcome::SnapshotNotFound(snapshot_id);
            }
        };

        if snapshot.entity_id != entity_id {
            self.total_failed += 1;
            return RecoveryOutcome::EntityMismatch {
                expected: entity_id,
                actual: snapshot.entity_id,
            };
        }

        self.total_restored += 1;
        RecoveryOutcome::Restored {
            snapshot_id,
            restored_state: snapshot.state_id,
        }
    }
}

impl Default for StateRecovery {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════
// CONSISTENT SNAPSHOT
// ═══════════════════════════════════════════════════════════

/// A consistent snapshot across multiple entities.
///
/// Captures multiple entity states at the same logical point
/// in time for coherent recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistentSnapshot {
    /// Unique identifier.
    pub id: SnapshotId,
    /// Individual entity snapshots.
    pub entity_snapshots: Vec<SnapshotId>,
    /// Timestamp of the consistent snapshot.
    pub timestamp: u64,
    /// Description/reason.
    pub reason: String,
    /// Number of entities captured.
    pub entity_count: usize,
}

impl ConsistentSnapshot {
    /// Creates a new consistent snapshot.
    #[must_use]
    pub fn new(id: SnapshotId, timestamp: u64, reason: &str) -> Self {
        Self {
            id,
            entity_snapshots: Vec::new(),
            timestamp,
            reason: reason.to_string(),
            entity_count: 0,
        }
    }

    /// Adds an entity snapshot to this consistent snapshot.
    pub fn add_entity_snapshot(&mut self, snapshot_id: SnapshotId) {
        self.entity_snapshots.push(snapshot_id);
        self.entity_count = self.entity_snapshots.len();
    }

    /// Returns all entity snapshot IDs.
    #[must_use]
    pub fn entity_snapshot_ids(&self) -> &[SnapshotId] {
        &self.entity_snapshots
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
    fn test_snapshot_store_grounding() {
        let comp = SnapshotStore::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_snapshot_creation() {
        let snapshot = StateSnapshot::new(
            SnapshotId(1),
            100,
            "case_lifecycle",
            StateId(2),
            "triaged",
            2000,
        )
        .with_transition_count(1)
        .with_reason("pre-assessment checkpoint");

        assert_eq!(snapshot.id, SnapshotId(1));
        assert_eq!(snapshot.entity_id, 100);
        assert_eq!(snapshot.state_id, StateId(2));
        assert_eq!(snapshot.state_name, "triaged");
        assert_eq!(snapshot.transition_count, 1);
        assert_eq!(snapshot.reason, Some("pre-assessment checkpoint".into()));
    }

    #[test]
    fn test_snapshot_store_basic() {
        let mut store = SnapshotStore::new();
        let ctx = HashMap::new();

        let id1 = store.take_snapshot(100, "case", StateId(1), "received", &ctx, 0, 1000, None);
        let id2 = store.take_snapshot(
            100,
            "case",
            StateId(2),
            "triaged",
            &ctx,
            1,
            2000,
            Some("triage"),
        );

        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());

        let s1 = store.get(id1);
        assert!(s1.is_some());
        if let Some(s) = s1 {
            assert_eq!(s.state_name, "received");
        }

        let s2 = store.get(id2);
        assert!(s2.is_some());
        if let Some(s) = s2 {
            assert_eq!(s.reason, Some("triage".into()));
        }
    }

    #[test]
    fn test_snapshot_store_latest_for_entity() {
        let mut store = SnapshotStore::new();
        let ctx = HashMap::new();

        store.take_snapshot(100, "case", StateId(1), "received", &ctx, 0, 1000, None);
        store.take_snapshot(100, "case", StateId(2), "triaged", &ctx, 1, 2000, None);
        store.take_snapshot(200, "signal", StateId(1), "detected", &ctx, 0, 1500, None);

        let latest = store.latest_for_entity(100);
        assert!(latest.is_some());
        if let Some(s) = latest {
            assert_eq!(s.state_name, "triaged");
        }

        let entity_snaps = store.for_entity(100);
        assert_eq!(entity_snaps.len(), 2);
    }

    #[test]
    fn test_snapshot_store_max_per_entity() {
        let mut store = SnapshotStore::new().with_max_per_entity(2);
        let ctx = HashMap::new();

        store.take_snapshot(100, "case", StateId(1), "a", &ctx, 0, 1000, None);
        store.take_snapshot(100, "case", StateId(2), "b", &ctx, 1, 2000, None);
        store.take_snapshot(100, "case", StateId(3), "c", &ctx, 2, 3000, None);

        // Only 2 should remain — oldest dropped
        let snaps = store.for_entity(100);
        assert_eq!(snaps.len(), 2);
        assert_eq!(snaps[0].state_name, "b");
        assert_eq!(snaps[1].state_name, "c");
    }

    #[test]
    fn test_snapshot_store_after() {
        let mut store = SnapshotStore::new();
        let ctx = HashMap::new();

        store.take_snapshot(100, "case", StateId(1), "a", &ctx, 0, 1000, None);
        store.take_snapshot(100, "case", StateId(2), "b", &ctx, 1, 2000, None);
        store.take_snapshot(100, "case", StateId(3), "c", &ctx, 2, 3000, None);

        let after = store.after(1500);
        assert_eq!(after.len(), 2);
    }

    #[test]
    fn test_checkpoint_policy_manual() {
        let policy = CheckpointPolicy::Manual;
        assert!(!policy.should_snapshot(1, StateId(1)));
        assert!(!policy.should_snapshot(100, StateId(5)));
    }

    #[test]
    fn test_checkpoint_policy_every_n() {
        let policy = CheckpointPolicy::EveryNTransitions(3);
        assert!(policy.should_snapshot(3, StateId(1)));
        assert!(policy.should_snapshot(6, StateId(1)));
        assert!(!policy.should_snapshot(4, StateId(1)));
    }

    #[test]
    fn test_checkpoint_policy_on_states() {
        let policy = CheckpointPolicy::OnStates(vec![StateId(2), StateId(4)]);
        assert!(policy.should_snapshot(1, StateId(2)));
        assert!(policy.should_snapshot(1, StateId(4)));
        assert!(!policy.should_snapshot(1, StateId(1)));
        assert!(!policy.should_snapshot(1, StateId(3)));
    }

    #[test]
    fn test_checkpoint_policy_always() {
        let policy = CheckpointPolicy::Always;
        assert!(policy.should_snapshot(0, StateId(0)));
        assert!(policy.should_snapshot(999, StateId(99)));
    }

    #[test]
    fn test_state_recovery_success() {
        let mut store = SnapshotStore::new();
        let ctx = HashMap::new();
        let snap_id = store.take_snapshot(100, "case", StateId(2), "triaged", &ctx, 1, 2000, None);

        let mut recovery = StateRecovery::new();
        let outcome = recovery.recover(&store, snap_id, 100);

        assert!(outcome.is_restored());
        assert_eq!(recovery.total_attempts, 1);
        assert_eq!(recovery.total_restored, 1);
        assert_eq!(recovery.total_failed, 0);
    }

    #[test]
    fn test_state_recovery_not_found() {
        let store = SnapshotStore::new();
        let mut recovery = StateRecovery::new();

        let outcome = recovery.recover(&store, SnapshotId(999), 100);
        assert!(matches!(outcome, RecoveryOutcome::SnapshotNotFound(_)));
        assert_eq!(recovery.total_failed, 1);
    }

    #[test]
    fn test_state_recovery_entity_mismatch() {
        let mut store = SnapshotStore::new();
        let ctx = HashMap::new();
        let snap_id = store.take_snapshot(100, "case", StateId(2), "triaged", &ctx, 1, 2000, None);

        let mut recovery = StateRecovery::new();
        let outcome = recovery.recover(&store, snap_id, 200); // wrong entity

        assert!(matches!(
            outcome,
            RecoveryOutcome::EntityMismatch {
                expected: 200,
                actual: 100
            }
        ));
        assert_eq!(recovery.total_failed, 1);
    }

    #[test]
    fn test_consistent_snapshot() {
        let mut consistent =
            ConsistentSnapshot::new(SnapshotId(100), 5000, "pre-submission checkpoint");

        consistent.add_entity_snapshot(SnapshotId(1));
        consistent.add_entity_snapshot(SnapshotId(2));
        consistent.add_entity_snapshot(SnapshotId(3));

        assert_eq!(consistent.entity_count, 3);
        assert_eq!(consistent.entity_snapshot_ids().len(), 3);
        assert_eq!(consistent.reason, "pre-submission checkpoint");
    }
}
