//! # PVDB WAL — Write-Ahead Log
//!
//! Write-ahead logging for crash recovery. Every mutation is first
//! written to the WAL before being applied to the store, enabling
//! replay-based recovery after failures.
//!
//! ## T1 Grounding (dominant: π Persistence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | π | Persistence | 0.35 — durable before apply |
//! | σ | Sequence | 0.25 — ordered log entries |
//! | → | Causality | 0.20 — log → apply sequence |
//! | ∝ | Irreversibility | 0.20 — committed finality |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// WAL ENTRY ID
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a WAL entry.
///
/// Tier: T2-P (π newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalEntryId(pub u64);

// ═══════════════════════════════════════════════════════════
// WAL STATE
// ═══════════════════════════════════════════════════════════

/// State of a WAL transaction.
///
/// Tier: T2-P (ς + ∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalState {
    /// Transaction is being recorded.
    Active,
    /// Transaction has been committed (applied to store).
    Committed,
    /// Transaction was rolled back.
    RolledBack,
    /// Transaction failed during apply.
    Failed,
}

impl WalState {
    /// Whether the WAL is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack | Self::Failed)
    }

    /// Whether the WAL committed successfully.
    #[must_use]
    pub fn is_committed(&self) -> bool {
        matches!(self, Self::Committed)
    }
}

// ═══════════════════════════════════════════════════════════
// WAL ENTRY
// ═══════════════════════════════════════════════════════════

/// A single WAL entry recording a mutation.
///
/// Tier: T2-C (π + σ + → + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Entry identifier.
    pub id: WalEntryId,
    /// Operation name (CREATE, UPDATE, DELETE).
    pub op: String,
    /// Key being mutated.
    pub key: String,
    /// Value before mutation (None for creates).
    pub before: Option<String>,
    /// Value after mutation (None for deletes).
    pub after: Option<String>,
    /// Timestamp of the entry.
    pub timestamp: u64,
}

impl WalEntry {
    /// Whether this entry is a create.
    #[must_use]
    pub fn is_create(&self) -> bool {
        self.before.is_none() && self.after.is_some()
    }

    /// Whether this entry is a delete.
    #[must_use]
    pub fn is_delete(&self) -> bool {
        self.before.is_some() && self.after.is_none()
    }

    /// Whether this entry is an update.
    #[must_use]
    pub fn is_update(&self) -> bool {
        self.before.is_some() && self.after.is_some()
    }
}

// ═══════════════════════════════════════════════════════════
// WAL CHECKPOINT
// ═══════════════════════════════════════════════════════════

/// WAL checkpoint: a known-good state for truncation.
///
/// Tier: T2-C (π + σ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalCheckpoint {
    /// Last committed entry ID at checkpoint time.
    pub last_committed: WalEntryId,
    /// Number of entries at checkpoint.
    pub entry_count: usize,
    /// Timestamp of checkpoint.
    pub timestamp: u64,
    /// Label for this checkpoint.
    pub label: String,
}

// ═══════════════════════════════════════════════════════════
// WRITE-AHEAD LOG
// ═══════════════════════════════════════════════════════════

/// Write-ahead log: durably records mutations before they are applied.
///
/// Tier: T2-C (π + σ + → + ∝ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteAheadLog {
    /// Ordered WAL entries.
    entries: Vec<WalEntry>,
    /// Current WAL state.
    state: WalState,
    /// Checkpoints taken.
    checkpoints: Vec<WalCheckpoint>,
    /// Next entry ID.
    next_id: u64,
    /// Total entries ever written (including truncated).
    total_written: u64,
    /// Total commits.
    total_commits: u64,
    /// Total rollbacks.
    total_rollbacks: u64,
}

impl WriteAheadLog {
    /// Creates a new empty WAL.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            state: WalState::Active,
            checkpoints: Vec::new(),
            next_id: 1,
            total_written: 0,
            total_commits: 0,
            total_rollbacks: 0,
        }
    }

    /// Appends a mutation entry to the WAL.
    /// Returns the entry ID, or None if the WAL is not active.
    pub fn append(
        &mut self,
        op: &str,
        key: &str,
        before: Option<&str>,
        after: Option<&str>,
        timestamp: u64,
    ) -> Option<WalEntryId> {
        if self.state.is_terminal() {
            return None;
        }

        let id = WalEntryId(self.next_id);
        self.next_id += 1;
        self.total_written += 1;

        self.entries.push(WalEntry {
            id,
            op: op.to_string(),
            key: key.to_string(),
            before: before.map(String::from),
            after: after.map(String::from),
            timestamp,
        });

        Some(id)
    }

    /// Commits the WAL: marks all pending entries as committed.
    pub fn commit(&mut self, timestamp: u64) -> bool {
        if self.state.is_terminal() {
            return false;
        }
        self.state = WalState::Committed;
        self.total_commits += 1;

        // Auto-checkpoint on commit
        if !self.entries.is_empty() {
            let last_id = self.entries.last().map(|e| e.id).unwrap_or(WalEntryId(0));
            self.checkpoints.push(WalCheckpoint {
                last_committed: last_id,
                entry_count: self.entries.len(),
                timestamp,
                label: format!("commit-{}", self.total_commits),
            });
        }

        true
    }

    /// Rolls back: discards all pending entries.
    pub fn rollback(&mut self) -> bool {
        if self.state.is_terminal() {
            return false;
        }
        self.entries.clear();
        self.state = WalState::RolledBack;
        self.total_rollbacks += 1;
        true
    }

    /// Resets the WAL for reuse after commit/rollback.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.state = WalState::Active;
    }

    /// Returns the current WAL state.
    #[must_use]
    pub fn state(&self) -> WalState {
        self.state
    }

    /// Returns all entries in the WAL.
    #[must_use]
    pub fn entries(&self) -> &[WalEntry] {
        &self.entries
    }

    /// Returns the number of pending entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the WAL is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns total entries ever written.
    #[must_use]
    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    /// Returns total commits.
    #[must_use]
    pub fn total_commits(&self) -> u64 {
        self.total_commits
    }

    /// Returns total rollbacks.
    #[must_use]
    pub fn total_rollbacks(&self) -> u64 {
        self.total_rollbacks
    }

    /// Returns all checkpoints.
    #[must_use]
    pub fn checkpoints(&self) -> &[WalCheckpoint] {
        &self.checkpoints
    }

    /// Returns entries for a specific key (for undo operations).
    #[must_use]
    pub fn entries_for_key(&self, key: &str) -> Vec<&WalEntry> {
        self.entries.iter().filter(|e| e.key == key).collect()
    }

    /// Returns the last entry for a key (latest mutation).
    #[must_use]
    pub fn last_entry_for_key(&self, key: &str) -> Option<&WalEntry> {
        self.entries.iter().rev().find(|e| e.key == key)
    }
}

impl Default for WriteAheadLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for WriteAheadLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence,     // π — durable before apply
            LexPrimitiva::Sequence,        // σ — ordered entries
            LexPrimitiva::Causality,       // → — log precedes apply
            LexPrimitiva::Irreversibility, // ∝ — committed finality
            LexPrimitiva::Boundary,        // ∂ — commit/rollback boundary
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// WAL RECOVERY
// ═══════════════════════════════════════════════════════════

/// WAL recovery: replays a committed WAL to rebuild store state.
///
/// Tier: T2-C (π + σ + → + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalRecovery {
    /// Total recoveries performed.
    total_recoveries: u64,
    /// Total entries replayed across all recoveries.
    total_replayed: u64,
}

impl WalRecovery {
    /// Creates a new WAL recovery manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_recoveries: 0,
            total_replayed: 0,
        }
    }

    /// Replays a committed WAL, returning key→value pairs to restore.
    /// Only committed WALs can be replayed.
    pub fn replay(&mut self, wal: &WriteAheadLog) -> Option<HashMap<String, Option<String>>> {
        if !wal.state().is_committed() {
            return None;
        }

        self.total_recoveries += 1;

        let mut state: HashMap<String, Option<String>> = HashMap::new();
        for entry in wal.entries() {
            self.total_replayed += 1;
            if entry.after.is_some() {
                state.insert(entry.key.clone(), entry.after.clone());
            } else {
                state.insert(entry.key.clone(), None); // Deletion
            }
        }

        Some(state)
    }

    /// Returns total recoveries performed.
    #[must_use]
    pub fn total_recoveries(&self) -> u64 {
        self.total_recoveries
    }

    /// Returns total entries replayed.
    #[must_use]
    pub fn total_replayed(&self) -> u64 {
        self.total_replayed
    }
}

impl Default for WalRecovery {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for WalRecovery {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — restore from durable log
            LexPrimitiva::Sequence,    // σ — replay in order
            LexPrimitiva::Causality,   // → — log causes state rebuild
            LexPrimitiva::Existence,   // ∃ — restore existence
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_write_ahead_log_grounding() {
        let comp = WriteAheadLog::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_wal_recovery_grounding() {
        let comp = WalRecovery::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }

    #[test]
    fn test_wal_state_properties() {
        assert!(!WalState::Active.is_terminal());
        assert!(WalState::Committed.is_terminal());
        assert!(WalState::RolledBack.is_terminal());
        assert!(WalState::Failed.is_terminal());
        assert!(WalState::Committed.is_committed());
        assert!(!WalState::Active.is_committed());
    }

    #[test]
    fn test_wal_append() {
        let mut wal = WriteAheadLog::new();

        let id = wal.append("CREATE", "k1", None, Some("v1"), 1000);
        assert!(id.is_some());
        assert_eq!(wal.len(), 1);

        let id2 = wal.append("UPDATE", "k1", Some("v1"), Some("v2"), 2000);
        assert!(id2.is_some());
        assert_eq!(wal.len(), 2);
    }

    #[test]
    fn test_wal_commit() {
        let mut wal = WriteAheadLog::new();
        wal.append("CREATE", "k1", None, Some("v1"), 1000);
        wal.append("CREATE", "k2", None, Some("v2"), 2000);

        assert!(wal.commit(3000));
        assert_eq!(wal.state(), WalState::Committed);
        assert_eq!(wal.total_commits(), 1);
        assert_eq!(wal.checkpoints().len(), 1);

        // Cannot append after commit
        let id = wal.append("CREATE", "k3", None, Some("v3"), 4000);
        assert!(id.is_none());
    }

    #[test]
    fn test_wal_rollback() {
        let mut wal = WriteAheadLog::new();
        wal.append("CREATE", "k1", None, Some("v1"), 1000);
        wal.append("CREATE", "k2", None, Some("v2"), 2000);

        assert!(wal.rollback());
        assert_eq!(wal.state(), WalState::RolledBack);
        assert!(wal.is_empty());
        assert_eq!(wal.total_rollbacks(), 1);
        assert_eq!(wal.total_written(), 2); // Still counted
    }

    #[test]
    fn test_wal_reset() {
        let mut wal = WriteAheadLog::new();
        wal.append("CREATE", "k1", None, Some("v1"), 1000);
        wal.commit(2000);

        wal.reset();
        assert_eq!(wal.state(), WalState::Active);
        assert!(wal.is_empty());

        // Can append again after reset
        let id = wal.append("CREATE", "k2", None, Some("v2"), 3000);
        assert!(id.is_some());
    }

    #[test]
    fn test_wal_entry_types() {
        let create = WalEntry {
            id: WalEntryId(1),
            op: "CREATE".into(),
            key: "k1".into(),
            before: None,
            after: Some("v1".into()),
            timestamp: 1000,
        };
        assert!(create.is_create());
        assert!(!create.is_delete());
        assert!(!create.is_update());

        let update = WalEntry {
            id: WalEntryId(2),
            op: "UPDATE".into(),
            key: "k1".into(),
            before: Some("v1".into()),
            after: Some("v2".into()),
            timestamp: 2000,
        };
        assert!(update.is_update());

        let delete = WalEntry {
            id: WalEntryId(3),
            op: "DELETE".into(),
            key: "k1".into(),
            before: Some("v1".into()),
            after: None,
            timestamp: 3000,
        };
        assert!(delete.is_delete());
    }

    #[test]
    fn test_wal_key_lookup() {
        let mut wal = WriteAheadLog::new();
        wal.append("CREATE", "k1", None, Some("v1"), 1000);
        wal.append("CREATE", "k2", None, Some("v2"), 2000);
        wal.append("UPDATE", "k1", Some("v1"), Some("v1-updated"), 3000);

        assert_eq!(wal.entries_for_key("k1").len(), 2);
        assert_eq!(wal.entries_for_key("k2").len(), 1);

        let last = wal.last_entry_for_key("k1");
        assert!(last.is_some());
        if let Some(e) = last {
            assert_eq!(e.op, "UPDATE");
        }
    }

    #[test]
    fn test_wal_recovery_replay() {
        let mut wal = WriteAheadLog::new();
        wal.append("CREATE", "k1", None, Some("v1"), 1000);
        wal.append("CREATE", "k2", None, Some("v2"), 2000);
        wal.append("DELETE", "k1", Some("v1"), None, 3000);
        wal.commit(4000);

        let mut recovery = WalRecovery::new();
        let state = recovery.replay(&wal);
        assert!(state.is_some());

        if let Some(s) = state {
            assert_eq!(s.get("k1"), Some(&None)); // Deleted
            assert_eq!(s.get("k2"), Some(&Some("v2".into())));
        }

        assert_eq!(recovery.total_recoveries(), 1);
        assert_eq!(recovery.total_replayed(), 3);
    }

    #[test]
    fn test_wal_recovery_rejects_uncommitted() {
        let mut wal = WriteAheadLog::new();
        wal.append("CREATE", "k1", None, Some("v1"), 1000);

        let mut recovery = WalRecovery::new();
        let state = recovery.replay(&wal);
        assert!(state.is_none()); // Not committed
    }

    #[test]
    fn test_wal_multiple_cycles() {
        let mut wal = WriteAheadLog::new();

        // First cycle
        wal.append("CREATE", "k1", None, Some("v1"), 1000);
        wal.commit(2000);
        wal.reset();

        // Second cycle
        wal.append("UPDATE", "k1", Some("v1"), Some("v2"), 3000);
        wal.commit(4000);

        assert_eq!(wal.total_written(), 2);
        assert_eq!(wal.total_commits(), 2);
        assert_eq!(wal.checkpoints().len(), 2);
    }
}
