//! # PVDB Backup — Backup and Restore
//!
//! Point-in-time backups of persistence stores with manifest
//! tracking, incremental support, and restore validation.
//!
//! ## T1 Grounding (dominant: π Persistence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | π | Persistence | 0.35 — durable copies |
//! | ∃ | Existence | 0.25 — preserve existence |
//! | σ | Sequence | 0.20 — backup ordering |
//! | ∂ | Boundary | 0.20 — backup scope |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::store::StoreEntry;

// ═══════════════════════════════════════════════════════════
// BACKUP ID
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a backup.
///
/// Tier: T2-P (π newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackupId(pub u64);

// ═══════════════════════════════════════════════════════════
// BACKUP KIND
// ═══════════════════════════════════════════════════════════

/// Type of backup.
///
/// Tier: T2-P (π + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupKind {
    /// Full backup: complete copy of all entries.
    Full,
    /// Incremental: only entries changed since last backup.
    Incremental,
    /// Differential: entries changed since last full backup.
    Differential,
}

impl BackupKind {
    /// Whether this is a full backup.
    #[must_use]
    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether this captures all data independently.
    #[must_use]
    pub fn is_self_contained(&self) -> bool {
        matches!(self, Self::Full)
    }
}

// ═══════════════════════════════════════════════════════════
// BACKUP ENTRY
// ═══════════════════════════════════════════════════════════

/// A single entry in a backup.
///
/// Tier: T2-C (π + μ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Key of the entry.
    pub key: String,
    /// Value at backup time.
    pub value: String,
    /// Version at backup time.
    pub version: u64,
    /// Whether it was deleted at backup time.
    pub deleted: bool,
    /// Original creation timestamp.
    pub created_at: u64,
    /// Last update timestamp.
    pub updated_at: u64,
}

impl BackupEntry {
    /// Creates a backup entry from a store entry.
    #[must_use]
    pub fn from_store_entry(key: &str, entry: &StoreEntry) -> Self {
        Self {
            key: key.to_string(),
            value: entry.value.0.clone(),
            version: entry.version,
            deleted: entry.deleted,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
        }
    }

    /// Whether this entry is alive (not deleted).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !self.deleted
    }
}

// ═══════════════════════════════════════════════════════════
// BACKUP MANIFEST
// ═══════════════════════════════════════════════════════════

/// Manifest describing a backup's contents and metadata.
///
/// Tier: T2-C (π + σ + ∃ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Backup identifier.
    pub id: BackupId,
    /// Kind of backup.
    pub kind: BackupKind,
    /// Store name that was backed up.
    pub store_name: String,
    /// Timestamp when backup was taken.
    pub timestamp: u64,
    /// Number of entries in the backup.
    pub entry_count: usize,
    /// Number of alive entries.
    pub alive_count: usize,
    /// Parent backup ID (for incremental/differential).
    pub parent_id: Option<BackupId>,
    /// Label for the backup.
    pub label: String,
}

impl BackupManifest {
    /// Returns the deleted entry count.
    #[must_use]
    pub fn deleted_count(&self) -> usize {
        self.entry_count.saturating_sub(self.alive_count)
    }
}

// ═══════════════════════════════════════════════════════════
// RESTORE OUTCOME
// ═══════════════════════════════════════════════════════════

/// Result of a backup restore operation.
///
/// Tier: T2-P (∃ — restoration outcome)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreOutcome {
    /// Fully restored from backup.
    Restored { entries_restored: usize },
    /// Partially restored (some entries failed).
    PartialRestore {
        entries_restored: usize,
        entries_failed: usize,
    },
    /// Backup not found.
    BackupNotFound,
    /// Store name mismatch.
    StoreMismatch { expected: String, actual: String },
}

impl RestoreOutcome {
    /// Whether the restore was fully successful.
    #[must_use]
    pub fn is_restored(&self) -> bool {
        matches!(self, Self::Restored { .. })
    }

    /// Total entries restored.
    #[must_use]
    pub fn entries_restored(&self) -> usize {
        match self {
            Self::Restored { entries_restored }
            | Self::PartialRestore {
                entries_restored, ..
            } => *entries_restored,
            _ => 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// BACKUP STORE
// ═══════════════════════════════════════════════════════════

/// Manages backups for persistence stores.
///
/// Tier: T2-C (π + ∃ + σ + ∂ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStore {
    /// Backup manifests indexed by ID.
    manifests: Vec<BackupManifest>,
    /// Backup data indexed by backup ID.
    data: HashMap<u64, Vec<BackupEntry>>,
    /// Next backup ID.
    next_id: u64,
    /// Total backups created.
    total_backups: u64,
    /// Total restores performed.
    total_restores: u64,
    /// Maximum backups to retain (0 = unlimited).
    max_retained: usize,
}

impl BackupStore {
    /// Creates a new backup store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            manifests: Vec::new(),
            data: HashMap::new(),
            next_id: 1,
            total_backups: 0,
            total_restores: 0,
            max_retained: 0,
        }
    }

    /// Sets the maximum number of backups to retain.
    #[must_use]
    pub fn with_max_retained(mut self, max: usize) -> Self {
        self.max_retained = max;
        self
    }

    /// Creates a full backup of a store's entries.
    pub fn backup_full(
        &mut self,
        store_name: &str,
        entries: &HashMap<String, StoreEntry>,
        timestamp: u64,
        label: &str,
    ) -> BackupId {
        let id = BackupId(self.next_id);
        self.next_id += 1;
        self.total_backups += 1;

        let backup_entries: Vec<BackupEntry> = entries
            .iter()
            .map(|(k, e)| BackupEntry::from_store_entry(k, e))
            .collect();

        let alive_count = backup_entries.iter().filter(|e| e.is_alive()).count();

        let manifest = BackupManifest {
            id,
            kind: BackupKind::Full,
            store_name: store_name.to_string(),
            timestamp,
            entry_count: backup_entries.len(),
            alive_count,
            parent_id: None,
            label: label.to_string(),
        };

        self.manifests.push(manifest);
        self.data.insert(id.0, backup_entries);

        // Enforce retention
        self.enforce_retention();

        id
    }

    /// Creates an incremental backup (entries changed since a parent backup).
    pub fn backup_incremental(
        &mut self,
        store_name: &str,
        entries: &HashMap<String, StoreEntry>,
        parent: BackupId,
        timestamp: u64,
        label: &str,
    ) -> BackupId {
        let id = BackupId(self.next_id);
        self.next_id += 1;
        self.total_backups += 1;

        // Find parent timestamp
        let parent_ts = self
            .manifests
            .iter()
            .find(|m| m.id == parent)
            .map(|m| m.timestamp)
            .unwrap_or(0);

        // Only include entries modified after parent
        let backup_entries: Vec<BackupEntry> = entries
            .iter()
            .filter(|(_, e)| e.updated_at > parent_ts)
            .map(|(k, e)| BackupEntry::from_store_entry(k, e))
            .collect();

        let alive_count = backup_entries.iter().filter(|e| e.is_alive()).count();

        let manifest = BackupManifest {
            id,
            kind: BackupKind::Incremental,
            store_name: store_name.to_string(),
            timestamp,
            entry_count: backup_entries.len(),
            alive_count,
            parent_id: Some(parent),
            label: label.to_string(),
        };

        self.manifests.push(manifest);
        self.data.insert(id.0, backup_entries);

        id
    }

    /// Restores a backup, returning entries to restore.
    pub fn restore(
        &mut self,
        backup_id: BackupId,
        target_store_name: &str,
    ) -> (RestoreOutcome, Vec<BackupEntry>) {
        self.total_restores += 1;

        let manifest = self.manifests.iter().find(|m| m.id == backup_id);
        let manifest = match manifest {
            Some(m) => m.clone(),
            None => return (RestoreOutcome::BackupNotFound, Vec::new()),
        };

        if manifest.store_name != target_store_name {
            return (
                RestoreOutcome::StoreMismatch {
                    expected: manifest.store_name.clone(),
                    actual: target_store_name.to_string(),
                },
                Vec::new(),
            );
        }

        match self.data.get(&backup_id.0) {
            Some(entries) => {
                let restored = entries.clone();
                let count = restored.len();
                (
                    RestoreOutcome::Restored {
                        entries_restored: count,
                    },
                    restored,
                )
            }
            None => (RestoreOutcome::BackupNotFound, Vec::new()),
        }
    }

    /// Enforces retention policy.
    fn enforce_retention(&mut self) {
        if self.max_retained == 0 || self.manifests.len() <= self.max_retained {
            return;
        }

        while self.manifests.len() > self.max_retained {
            if let Some(oldest) = self.manifests.first() {
                let id = oldest.id;
                self.data.remove(&id.0);
            }
            self.manifests.remove(0);
        }
    }

    /// Returns all backup manifests.
    #[must_use]
    pub fn manifests(&self) -> &[BackupManifest] {
        &self.manifests
    }

    /// Gets a specific manifest by ID.
    #[must_use]
    pub fn get_manifest(&self, id: BackupId) -> Option<&BackupManifest> {
        self.manifests.iter().find(|m| m.id == id)
    }

    /// Returns the latest backup manifest.
    #[must_use]
    pub fn latest(&self) -> Option<&BackupManifest> {
        self.manifests.last()
    }

    /// Returns the number of stored backups.
    #[must_use]
    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    /// Whether there are no backups.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }

    /// Total backups ever created.
    #[must_use]
    pub fn total_backups(&self) -> u64 {
        self.total_backups
    }

    /// Total restores performed.
    #[must_use]
    pub fn total_restores(&self) -> u64 {
        self.total_restores
    }
}

impl Default for BackupStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for BackupStore {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — durable copies
            LexPrimitiva::Existence,   // ∃ — preserve existence
            LexPrimitiva::Sequence,    // σ — backup ordering
            LexPrimitiva::Boundary,    // ∂ — scope/retention
            LexPrimitiva::Mapping,     // μ — id→data
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{PersistenceStore, StoreId, StoreKind};

    fn make_store_with_data() -> PersistenceStore {
        let mut store = PersistenceStore::new(StoreId(1), "cases", StoreKind::KeyValue);
        store.put("case:1", "warfarin-bleeding", 1000);
        store.put("case:2", "aspirin-headache", 2000);
        store.put("case:3", "metformin-nausea", 3000);
        store
    }

    #[test]
    fn test_backup_store_grounding() {
        let comp = BackupStore::primitive_composition();
        assert_eq!(
            nexcore_lex_primitiva::GroundingTier::classify(&comp),
            nexcore_lex_primitiva::GroundingTier::T2Composite
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_backup_kind_properties() {
        assert!(BackupKind::Full.is_full());
        assert!(BackupKind::Full.is_self_contained());
        assert!(!BackupKind::Incremental.is_full());
        assert!(!BackupKind::Incremental.is_self_contained());
    }

    #[test]
    fn test_backup_entry_from_store() {
        let store = make_store_with_data();
        let entry = store.get("case:1");
        assert!(entry.is_some());
        if let Some(e) = entry {
            let be = BackupEntry::from_store_entry("case:1", e);
            assert_eq!(be.key, "case:1");
            assert_eq!(be.value, "warfarin-bleeding");
            assert!(be.is_alive());
        }
    }

    #[test]
    fn test_full_backup() {
        let store = make_store_with_data();
        let mut backups = BackupStore::new();

        let id = backups.backup_full("cases", store.raw_entries(), 5000, "daily");
        assert_eq!(id, BackupId(1));
        assert_eq!(backups.len(), 1);

        let manifest = backups.get_manifest(id);
        assert!(manifest.is_some());
        if let Some(m) = manifest {
            assert_eq!(m.entry_count, 3);
            assert_eq!(m.alive_count, 3);
            assert_eq!(m.deleted_count(), 0);
            assert!(m.kind.is_full());
        }
    }

    #[test]
    fn test_incremental_backup() {
        let mut store = make_store_with_data();
        let mut backups = BackupStore::new();

        let full_id = backups.backup_full("cases", store.raw_entries(), 5000, "full");

        // Modify store after full backup
        store.put("case:4", "ibuprofen-rash", 6000);
        store.put("case:1", "warfarin-bleeding-updated", 7000);

        let inc_id =
            backups.backup_incremental("cases", store.raw_entries(), full_id, 8000, "incremental");

        let manifest = backups.get_manifest(inc_id);
        assert!(manifest.is_some());
        if let Some(m) = manifest {
            assert_eq!(m.kind, BackupKind::Incremental);
            assert_eq!(m.parent_id, Some(full_id));
            assert_eq!(m.entry_count, 2); // Only changed entries
        }
    }

    #[test]
    fn test_restore_from_backup() {
        let store = make_store_with_data();
        let mut backups = BackupStore::new();

        let id = backups.backup_full("cases", store.raw_entries(), 5000, "restore-test");

        let (outcome, entries) = backups.restore(id, "cases");
        assert!(outcome.is_restored());
        assert_eq!(outcome.entries_restored(), 3);
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_restore_not_found() {
        let mut backups = BackupStore::new();
        let (outcome, _) = backups.restore(BackupId(99), "cases");
        assert!(matches!(outcome, RestoreOutcome::BackupNotFound));
    }

    #[test]
    fn test_restore_store_mismatch() {
        let store = make_store_with_data();
        let mut backups = BackupStore::new();

        let id = backups.backup_full("cases", store.raw_entries(), 5000, "test");

        let (outcome, _) = backups.restore(id, "signals");
        assert!(matches!(outcome, RestoreOutcome::StoreMismatch { .. }));
    }

    #[test]
    fn test_backup_retention() {
        let store = make_store_with_data();
        let mut backups = BackupStore::new().with_max_retained(2);

        backups.backup_full("cases", store.raw_entries(), 1000, "b1");
        backups.backup_full("cases", store.raw_entries(), 2000, "b2");
        backups.backup_full("cases", store.raw_entries(), 3000, "b3");

        assert_eq!(backups.len(), 2); // Oldest evicted
        assert_eq!(backups.total_backups(), 3); // All counted
    }

    #[test]
    fn test_backup_latest() {
        let store = make_store_with_data();
        let mut backups = BackupStore::new();

        backups.backup_full("cases", store.raw_entries(), 1000, "first");
        backups.backup_full("cases", store.raw_entries(), 2000, "second");

        let latest = backups.latest();
        assert!(latest.is_some());
        if let Some(m) = latest {
            assert_eq!(m.label, "second");
        }
    }

    #[test]
    fn test_backup_counters() {
        let store = make_store_with_data();
        let mut backups = BackupStore::new();

        let id = backups.backup_full("cases", store.raw_entries(), 1000, "test");
        backups.restore(id, "cases");
        backups.restore(id, "cases");

        assert_eq!(backups.total_backups(), 1);
        assert_eq!(backups.total_restores(), 2);
    }

    #[test]
    fn test_backup_with_deleted_entries() {
        let mut store = make_store_with_data();
        store.delete("case:2", 4000);

        let mut backups = BackupStore::new();
        let id = backups.backup_full("cases", store.raw_entries(), 5000, "with-deletes");

        let manifest = backups.get_manifest(id);
        assert!(manifest.is_some());
        if let Some(m) = manifest {
            assert_eq!(m.entry_count, 3); // All entries including deleted
            assert_eq!(m.alive_count, 2);
            assert_eq!(m.deleted_count(), 1);
        }
    }

    #[test]
    fn test_restore_outcome_properties() {
        let restored = RestoreOutcome::Restored {
            entries_restored: 5,
        };
        assert!(restored.is_restored());
        assert_eq!(restored.entries_restored(), 5);

        let partial = RestoreOutcome::PartialRestore {
            entries_restored: 3,
            entries_failed: 2,
        };
        assert!(!partial.is_restored());
        assert_eq!(partial.entries_restored(), 3);

        let not_found = RestoreOutcome::BackupNotFound;
        assert_eq!(not_found.entries_restored(), 0);
    }
}
