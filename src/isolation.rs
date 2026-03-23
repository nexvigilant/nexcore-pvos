//! # PVDB Isolation — Transaction Isolation and Locking
//!
//! Concurrency control for persistence stores. Manages locks,
//! isolation levels, and conflict detection to ensure data
//! consistency under concurrent access.
//!
//! ## T1 Grounding (dominant: π Persistence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | π | Persistence | 0.30 — data consistency |
//! | ∂ | Boundary | 0.25 — isolation boundaries |
//! | ς | State | 0.20 — lock states |
//! | κ | Comparison | 0.15 — conflict detection |
//! | ∃ | Existence | 0.10 — lock existence |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// ISOLATION LEVEL
// ═══════════════════════════════════════════════════════════

/// Database-style isolation level.
///
/// Tier: T2-P (∂ — boundary strictness)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// Dirty reads allowed — minimal isolation.
    ReadUncommitted,
    /// Only committed data visible.
    ReadCommitted,
    /// Snapshot isolation — repeatable reads.
    RepeatableRead,
    /// Full serialization — strictest.
    Serializable,
}

impl IsolationLevel {
    /// Whether dirty reads are possible.
    #[must_use]
    pub fn allows_dirty_reads(&self) -> bool {
        matches!(self, Self::ReadUncommitted)
    }

    /// Whether phantom reads are possible.
    #[must_use]
    pub fn allows_phantom_reads(&self) -> bool {
        matches!(self, Self::ReadUncommitted | Self::ReadCommitted)
    }

    /// Whether non-repeatable reads are possible.
    #[must_use]
    pub fn allows_non_repeatable_reads(&self) -> bool {
        matches!(self, Self::ReadUncommitted | Self::ReadCommitted)
    }

    /// Returns the strictness level (0-3).
    #[must_use]
    pub fn strictness(&self) -> u8 {
        match self {
            Self::ReadUncommitted => 0,
            Self::ReadCommitted => 1,
            Self::RepeatableRead => 2,
            Self::Serializable => 3,
        }
    }
}

impl Default for IsolationLevel {
    fn default() -> Self {
        Self::ReadCommitted
    }
}

// ═══════════════════════════════════════════════════════════
// LOCK TYPES
// ═══════════════════════════════════════════════════════════

/// Kind of lock.
///
/// Tier: T2-P (∂ + ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockKind {
    /// Shared lock — multiple readers allowed.
    Shared,
    /// Exclusive lock — single writer.
    Exclusive,
}

impl LockKind {
    /// Whether this lock is compatible with another.
    #[must_use]
    pub fn compatible_with(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Shared, Self::Shared))
    }

    /// Whether this is an exclusive lock.
    #[must_use]
    pub fn is_exclusive(&self) -> bool {
        matches!(self, Self::Exclusive)
    }
}

/// A lock on a specific key held by an owner.
///
/// Tier: T2-C (∂ + ς + π + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbLock {
    /// Key being locked.
    pub key: String,
    /// Lock kind.
    pub kind: LockKind,
    /// Owner of the lock (transaction/session ID).
    pub owner: u64,
    /// Timestamp when lock was acquired.
    pub acquired_at: u64,
    /// Optional timeout in seconds (0 = no timeout).
    pub timeout_secs: u64,
}

impl DbLock {
    /// Creates a new lock.
    #[must_use]
    pub fn new(key: &str, kind: LockKind, owner: u64, timestamp: u64) -> Self {
        Self {
            key: key.to_string(),
            kind,
            owner,
            acquired_at: timestamp,
            timeout_secs: 0,
        }
    }

    /// Sets the lock timeout.
    #[must_use]
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Whether this lock has expired.
    #[must_use]
    pub fn is_expired(&self, now: u64) -> bool {
        if self.timeout_secs == 0 {
            return false;
        }
        now.saturating_sub(self.acquired_at) >= self.timeout_secs
    }

    /// Whether this lock is held by a specific owner.
    #[must_use]
    pub fn held_by(&self, owner: u64) -> bool {
        self.owner == owner
    }
}

// ═══════════════════════════════════════════════════════════
// LOCK MANAGER
// ═══════════════════════════════════════════════════════════

/// Manages locks across all keys.
///
/// Tier: T2-C (π + ∂ + ς + κ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockManager {
    /// Active locks: key → list of locks.
    locks: HashMap<String, Vec<DbLock>>,
    /// Total lock acquisitions.
    total_acquired: u64,
    /// Total lock releases.
    total_released: u64,
    /// Total lock conflicts (acquisition denied).
    total_conflicts: u64,
    /// Default isolation level.
    default_isolation: IsolationLevel,
}

impl LockManager {
    /// Creates a new lock manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            locks: HashMap::new(),
            total_acquired: 0,
            total_released: 0,
            total_conflicts: 0,
            default_isolation: IsolationLevel::default(),
        }
    }

    /// Sets the default isolation level.
    #[must_use]
    pub fn with_isolation(mut self, level: IsolationLevel) -> Self {
        self.default_isolation = level;
        self
    }

    /// Attempts to acquire a lock on a key.
    /// Returns true if the lock was acquired.
    pub fn acquire(&mut self, key: &str, kind: LockKind, owner: u64, timestamp: u64) -> bool {
        // Clean expired locks first
        self.clean_expired(key, timestamp);

        let existing = self.locks.entry(key.to_string()).or_default();

        // Check compatibility
        for lock in existing.iter() {
            if lock.owner == owner {
                // Same owner can always re-acquire
                continue;
            }
            if !kind.compatible_with(&lock.kind) {
                self.total_conflicts += 1;
                return false;
            }
        }

        // Acquire the lock
        existing.push(DbLock::new(key, kind, owner, timestamp));
        self.total_acquired += 1;
        true
    }

    /// Releases a lock on a key held by an owner.
    /// Returns true if a lock was released.
    pub fn release(&mut self, key: &str, owner: u64) -> bool {
        if let Some(locks) = self.locks.get_mut(key) {
            let before = locks.len();
            locks.retain(|l| l.owner != owner);
            let released = before - locks.len();

            if locks.is_empty() {
                self.locks.remove(key);
            }

            if released > 0 {
                self.total_released += released as u64;
                return true;
            }
        }
        false
    }

    /// Releases all locks held by an owner.
    pub fn release_all(&mut self, owner: u64) -> usize {
        let mut total = 0;
        let keys: Vec<String> = self.locks.keys().cloned().collect();

        for key in keys {
            if let Some(locks) = self.locks.get_mut(&key) {
                let before = locks.len();
                locks.retain(|l| l.owner != owner);
                total += before - locks.len();
                if locks.is_empty() {
                    self.locks.remove(&key);
                }
            }
        }

        self.total_released += total as u64;
        total
    }

    /// Checks whether a key is locked.
    #[must_use]
    pub fn is_locked(&self, key: &str) -> bool {
        self.locks.get(key).is_some_and(|l| !l.is_empty())
    }

    /// Checks whether a key is exclusively locked.
    #[must_use]
    pub fn is_exclusively_locked(&self, key: &str) -> bool {
        self.locks
            .get(key)
            .is_some_and(|l| l.iter().any(|lock| lock.kind.is_exclusive()))
    }

    /// Returns locks on a specific key.
    #[must_use]
    pub fn locks_on(&self, key: &str) -> Vec<&DbLock> {
        self.locks
            .get(key)
            .map(|l| l.iter().collect())
            .unwrap_or_default()
    }

    /// Returns all locks held by an owner.
    #[must_use]
    pub fn locks_by_owner(&self, owner: u64) -> Vec<&DbLock> {
        self.locks
            .values()
            .flat_map(|locks| locks.iter())
            .filter(|l| l.owner == owner)
            .collect()
    }

    /// Cleans expired locks for a key.
    fn clean_expired(&mut self, key: &str, now: u64) {
        if let Some(locks) = self.locks.get_mut(key) {
            locks.retain(|l| !l.is_expired(now));
            if locks.is_empty() {
                self.locks.remove(key);
            }
        }
    }

    /// Returns the total number of active locks.
    #[must_use]
    pub fn active_lock_count(&self) -> usize {
        self.locks.values().map(|l| l.len()).sum()
    }

    /// Returns total lock acquisitions.
    #[must_use]
    pub fn total_acquired(&self) -> u64 {
        self.total_acquired
    }

    /// Returns total lock releases.
    #[must_use]
    pub fn total_released(&self) -> u64 {
        self.total_released
    }

    /// Returns total conflicts.
    #[must_use]
    pub fn total_conflicts(&self) -> u64 {
        self.total_conflicts
    }

    /// Returns the default isolation level.
    #[must_use]
    pub fn default_isolation(&self) -> IsolationLevel {
        self.default_isolation
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for LockManager {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — data consistency
            LexPrimitiva::Boundary,    // ∂ — isolation boundaries
            LexPrimitiva::State,       // ς — lock states
            LexPrimitiva::Comparison,  // κ — conflict detection
            LexPrimitiva::Existence,   // ∃ — lock existence
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// CONFLICT DETECTOR
// ═══════════════════════════════════════════════════════════

/// Detects write-write conflicts between concurrent operations.
///
/// Tier: T2-C (κ + π + ∂ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetector {
    /// Write set: keys written by each owner.
    write_sets: HashMap<u64, Vec<String>>,
    /// Total conflicts detected.
    total_detected: u64,
}

impl ConflictDetector {
    /// Creates a new conflict detector.
    #[must_use]
    pub fn new() -> Self {
        Self {
            write_sets: HashMap::new(),
            total_detected: 0,
        }
    }

    /// Records a write by an owner.
    pub fn record_write(&mut self, owner: u64, key: &str) {
        self.write_sets
            .entry(owner)
            .or_default()
            .push(key.to_string());
    }

    /// Checks for conflicts between two owners.
    #[must_use]
    pub fn has_conflict(&self, owner_a: u64, owner_b: u64) -> bool {
        let set_a = self.write_sets.get(&owner_a);
        let set_b = self.write_sets.get(&owner_b);

        match (set_a, set_b) {
            (Some(a), Some(b)) => a.iter().any(|k| b.contains(k)),
            _ => false,
        }
    }

    /// Returns conflicting keys between two owners.
    #[must_use]
    pub fn conflicting_keys(&self, owner_a: u64, owner_b: u64) -> Vec<String> {
        let set_a = self.write_sets.get(&owner_a);
        let set_b = self.write_sets.get(&owner_b);

        match (set_a, set_b) {
            (Some(a), Some(b)) => a.iter().filter(|k| b.contains(k)).cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// Clears the write set for an owner (after commit/rollback).
    pub fn clear_owner(&mut self, owner: u64) {
        self.write_sets.remove(&owner);
    }

    /// Clears all write sets.
    pub fn clear_all(&mut self) {
        self.write_sets.clear();
    }

    /// Returns total conflicts detected.
    #[must_use]
    pub fn total_detected(&self) -> u64 {
        self.total_detected
    }

    /// Checks and records a conflict if present.
    pub fn check_and_record(&mut self, owner_a: u64, owner_b: u64) -> bool {
        if self.has_conflict(owner_a, owner_b) {
            self.total_detected += 1;
            true
        } else {
            false
        }
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for ConflictDetector {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — data integrity
            LexPrimitiva::Comparison,  // κ — conflict check
            LexPrimitiva::Boundary,    // ∂ — owner boundaries
            LexPrimitiva::Existence,   // ∃ — write presence
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_lock_manager_grounding() {
        let comp = LockManager::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_conflict_detector_grounding() {
        let comp = ConflictDetector::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }

    #[test]
    fn test_isolation_level_ordering() {
        assert!(IsolationLevel::ReadUncommitted < IsolationLevel::ReadCommitted);
        assert!(IsolationLevel::ReadCommitted < IsolationLevel::RepeatableRead);
        assert!(IsolationLevel::RepeatableRead < IsolationLevel::Serializable);
    }

    #[test]
    fn test_isolation_level_properties() {
        let ru = IsolationLevel::ReadUncommitted;
        assert!(ru.allows_dirty_reads());
        assert!(ru.allows_phantom_reads());
        assert_eq!(ru.strictness(), 0);

        let rc = IsolationLevel::ReadCommitted;
        assert!(!rc.allows_dirty_reads());
        assert!(rc.allows_phantom_reads());
        assert_eq!(rc.strictness(), 1);

        let rr = IsolationLevel::RepeatableRead;
        assert!(!rr.allows_non_repeatable_reads());
        assert_eq!(rr.strictness(), 2);

        let s = IsolationLevel::Serializable;
        assert!(!s.allows_phantom_reads());
        assert_eq!(s.strictness(), 3);
    }

    #[test]
    fn test_lock_kind_compatibility() {
        assert!(LockKind::Shared.compatible_with(&LockKind::Shared));
        assert!(!LockKind::Shared.compatible_with(&LockKind::Exclusive));
        assert!(!LockKind::Exclusive.compatible_with(&LockKind::Shared));
        assert!(!LockKind::Exclusive.compatible_with(&LockKind::Exclusive));
    }

    #[test]
    fn test_db_lock_creation() {
        let lock = DbLock::new("k1", LockKind::Shared, 100, 1000);
        assert_eq!(lock.key, "k1");
        assert!(lock.held_by(100));
        assert!(!lock.held_by(200));
        assert!(!lock.is_expired(2000));
    }

    #[test]
    fn test_db_lock_timeout() {
        let lock = DbLock::new("k1", LockKind::Exclusive, 100, 1000).with_timeout(500);
        assert!(!lock.is_expired(1400));
        assert!(lock.is_expired(1500));
        assert!(lock.is_expired(2000));
    }

    #[test]
    fn test_lock_manager_acquire_release() {
        let mut mgr = LockManager::new();

        assert!(mgr.acquire("k1", LockKind::Shared, 100, 1000));
        assert!(mgr.is_locked("k1"));
        assert!(!mgr.is_exclusively_locked("k1"));

        assert!(mgr.release("k1", 100));
        assert!(!mgr.is_locked("k1"));
    }

    #[test]
    fn test_lock_manager_shared_compatibility() {
        let mut mgr = LockManager::new();

        // Two shared locks on same key — should work
        assert!(mgr.acquire("k1", LockKind::Shared, 100, 1000));
        assert!(mgr.acquire("k1", LockKind::Shared, 200, 1000));
        assert_eq!(mgr.active_lock_count(), 2);
    }

    #[test]
    fn test_lock_manager_exclusive_conflict() {
        let mut mgr = LockManager::new();

        assert!(mgr.acquire("k1", LockKind::Exclusive, 100, 1000));
        assert!(!mgr.acquire("k1", LockKind::Shared, 200, 1000));
        assert!(!mgr.acquire("k1", LockKind::Exclusive, 200, 1000));
        assert_eq!(mgr.total_conflicts(), 2);
    }

    #[test]
    fn test_lock_manager_release_all() {
        let mut mgr = LockManager::new();

        mgr.acquire("k1", LockKind::Shared, 100, 1000);
        mgr.acquire("k2", LockKind::Exclusive, 100, 1000);
        mgr.acquire("k3", LockKind::Shared, 200, 1000);

        let released = mgr.release_all(100);
        assert_eq!(released, 2);
        assert!(!mgr.is_locked("k1"));
        assert!(!mgr.is_locked("k2"));
        assert!(mgr.is_locked("k3")); // Still held by 200
    }

    #[test]
    fn test_lock_manager_owner_query() {
        let mut mgr = LockManager::new();

        mgr.acquire("k1", LockKind::Shared, 100, 1000);
        mgr.acquire("k2", LockKind::Exclusive, 100, 1000);

        let owner_locks = mgr.locks_by_owner(100);
        assert_eq!(owner_locks.len(), 2);
    }

    #[test]
    fn test_conflict_detector_basic() {
        let mut detector = ConflictDetector::new();

        detector.record_write(100, "k1");
        detector.record_write(100, "k2");
        detector.record_write(200, "k2");
        detector.record_write(200, "k3");

        assert!(detector.has_conflict(100, 200));
        assert_eq!(detector.conflicting_keys(100, 200), vec!["k2".to_string()]);
    }

    #[test]
    fn test_conflict_detector_no_conflict() {
        let mut detector = ConflictDetector::new();

        detector.record_write(100, "k1");
        detector.record_write(200, "k2");

        assert!(!detector.has_conflict(100, 200));
        assert!(detector.conflicting_keys(100, 200).is_empty());
    }

    #[test]
    fn test_conflict_detector_clear() {
        let mut detector = ConflictDetector::new();

        detector.record_write(100, "k1");
        detector.record_write(200, "k1");

        assert!(detector.has_conflict(100, 200));

        detector.clear_owner(100);
        assert!(!detector.has_conflict(100, 200));
    }

    #[test]
    fn test_conflict_detector_check_and_record() {
        let mut detector = ConflictDetector::new();

        detector.record_write(100, "k1");
        detector.record_write(200, "k1");

        assert!(detector.check_and_record(100, 200));
        assert_eq!(detector.total_detected(), 1);

        assert!(!detector.check_and_record(100, 300));
        assert_eq!(detector.total_detected(), 1);
    }
}
