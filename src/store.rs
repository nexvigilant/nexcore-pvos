//! # PVDB Store — Core Persistence Abstractions
//!
//! Foundational types for the persistence layer: stores, entries,
//! keys, values, and configuration. Everything in PVOS that needs
//! durable storage uses these primitives.
//!
//! ## T1 Grounding (dominant: π Persistence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | π | Persistence | 0.40 — durable storage |
//! | μ | Mapping | 0.25 — key→value |
//! | ∃ | Existence | 0.20 — entry presence |
//! | σ | Sequence | 0.15 — version ordering |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a persistence store.
///
/// Tier: T2-P (π newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoreId(pub u64);

/// Human-readable store name.
///
/// Tier: T2-P (π + λ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoreName(pub String);

impl StoreName {
    /// Creates a new store name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Returns the name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ═══════════════════════════════════════════════════════════
// STORE KIND
// ═══════════════════════════════════════════════════════════

/// Kind of persistence store.
///
/// Tier: T2-P (π + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreKind {
    /// In-memory store (no disk persistence).
    InMemory,
    /// Append-only log store.
    AppendOnly,
    /// Key-value store with upsert.
    KeyValue,
    /// Relational store with indexed columns.
    Relational,
}

impl StoreKind {
    /// Whether this store kind supports updates.
    #[must_use]
    pub fn supports_update(&self) -> bool {
        matches!(self, Self::KeyValue | Self::Relational)
    }

    /// Whether this store kind supports deletion.
    #[must_use]
    pub fn supports_delete(&self) -> bool {
        matches!(self, Self::KeyValue | Self::Relational)
    }

    /// Whether this store kind is durable.
    #[must_use]
    pub fn is_durable(&self) -> bool {
        !matches!(self, Self::InMemory)
    }
}

// ═══════════════════════════════════════════════════════════
// STORAGE KEY AND VALUE
// ═══════════════════════════════════════════════════════════

/// Key used to index stored entries.
///
/// Tier: T2-P (μ — maps to value)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageKey(pub String);

impl StorageKey {
    /// Creates a new storage key.
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self(key.to_string())
    }

    /// Returns key as str.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Value stored in persistence.
///
/// Tier: T2-P (π — persisted content)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageValue(pub String);

impl StorageValue {
    /// Creates a new storage value.
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_string())
    }

    /// Returns the value length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the value is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ═══════════════════════════════════════════════════════════
// STORE ENTRY
// ═══════════════════════════════════════════════════════════

/// A single persisted entry with metadata.
///
/// Tier: T2-C (π + μ + σ + ∃)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreEntry {
    /// The key for this entry.
    pub key: StorageKey,
    /// The stored value.
    pub value: StorageValue,
    /// Timestamp when first created.
    pub created_at: u64,
    /// Timestamp of last update.
    pub updated_at: u64,
    /// Version counter (incremented on each update).
    pub version: u64,
    /// Whether this entry has been soft-deleted.
    pub deleted: bool,
}

impl StoreEntry {
    /// Creates a new store entry.
    #[must_use]
    pub fn new(key: StorageKey, value: StorageValue, timestamp: u64) -> Self {
        Self {
            key,
            value,
            created_at: timestamp,
            updated_at: timestamp,
            version: 1,
            deleted: false,
        }
    }

    /// Updates the value and increments version.
    pub fn update(&mut self, value: StorageValue, timestamp: u64) {
        self.value = value;
        self.updated_at = timestamp;
        self.version += 1;
    }

    /// Marks as soft-deleted.
    pub fn soft_delete(&mut self, timestamp: u64) {
        self.deleted = true;
        self.updated_at = timestamp;
        self.version += 1;
    }

    /// Returns whether this entry is alive (not deleted).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !self.deleted
    }

    /// Returns the age of this entry (now - created_at).
    #[must_use]
    pub fn age(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }
}

impl GroundsTo for StoreEntry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — durable storage
            LexPrimitiva::Mapping,     // μ — key→value
            LexPrimitiva::Sequence,    // σ — version ordering
            LexPrimitiva::Existence,   // ∃ — alive/deleted
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// STORE CONFIGURATION
// ═══════════════════════════════════════════════════════════

/// Configuration for a persistence store.
///
/// Tier: T2-C (π + ∂ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Maximum number of entries (0 = unlimited).
    pub max_entries: usize,
    /// Retention period in seconds (0 = forever).
    pub retention_secs: u64,
    /// Whether to allow soft-deletes vs hard-deletes.
    pub soft_delete: bool,
    /// Whether to keep version history.
    pub versioned: bool,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_entries: 0,
            retention_secs: 0,
            soft_delete: true,
            versioned: true,
        }
    }
}

impl StoreConfig {
    /// Creates a config with max entries limit.
    #[must_use]
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Creates a config with retention period.
    #[must_use]
    pub fn with_retention(mut self, secs: u64) -> Self {
        self.retention_secs = secs;
        self
    }

    /// Whether capacity is bounded.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        self.max_entries > 0
    }

    /// Whether entries expire.
    #[must_use]
    pub fn has_retention(&self) -> bool {
        self.retention_secs > 0
    }
}

// ═══════════════════════════════════════════════════════════
// PERSISTENCE STORE
// ═══════════════════════════════════════════════════════════

/// Core persistence store — durable key-value storage with versioning.
///
/// Tier: T2-C (π + μ + ∃ + σ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceStore {
    /// Store identifier.
    pub id: StoreId,
    /// Store name.
    pub name: StoreName,
    /// Store kind.
    pub kind: StoreKind,
    /// Configuration.
    pub config: StoreConfig,
    /// Stored entries indexed by key.
    entries: HashMap<String, StoreEntry>,
    /// Total write operations.
    total_writes: u64,
    /// Total read operations.
    total_reads: u64,
    /// Next store sequence number.
    next_seq: u64,
}

impl PersistenceStore {
    /// Creates a new persistence store.
    #[must_use]
    pub fn new(id: StoreId, name: &str, kind: StoreKind) -> Self {
        Self {
            id,
            name: StoreName::new(name),
            kind,
            config: StoreConfig::default(),
            entries: HashMap::new(),
            total_writes: 0,
            total_reads: 0,
            next_seq: 1,
        }
    }

    /// Sets the store configuration.
    #[must_use]
    pub fn with_config(mut self, config: StoreConfig) -> Self {
        self.config = config;
        self
    }

    /// Puts an entry into the store. Returns the version.
    pub fn put(&mut self, key: &str, value: &str, timestamp: u64) -> u64 {
        self.total_writes += 1;

        if let Some(entry) = self.entries.get_mut(key) {
            entry.update(StorageValue::new(value), timestamp);
            entry.version
        } else {
            // Check capacity
            if self.config.is_bounded() && self.entries.len() >= self.config.max_entries {
                return 0; // At capacity
            }
            let entry = StoreEntry::new(StorageKey::new(key), StorageValue::new(value), timestamp);
            let version = entry.version;
            self.entries.insert(key.to_string(), entry);
            version
        }
    }

    /// Gets an entry by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&StoreEntry> {
        self.entries.get(key).filter(|e| e.is_alive())
    }

    /// Gets a mutable entry by key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut StoreEntry> {
        self.entries.get_mut(key).filter(|e| e.is_alive())
    }

    /// Checks whether a key exists (and is alive).
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.entries.get(key).is_some_and(|e| e.is_alive())
    }

    /// Deletes an entry. Returns true if it existed.
    pub fn delete(&mut self, key: &str, timestamp: u64) -> bool {
        self.total_writes += 1;
        if self.config.soft_delete {
            if let Some(entry) = self.entries.get_mut(key) {
                if entry.is_alive() {
                    entry.soft_delete(timestamp);
                    return true;
                }
            }
            false
        } else {
            self.entries.remove(key).is_some()
        }
    }

    /// Returns all alive entries.
    #[must_use]
    pub fn entries(&self) -> Vec<&StoreEntry> {
        self.entries.values().filter(|e| e.is_alive()).collect()
    }

    /// Returns the number of alive entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.values().filter(|e| e.is_alive()).count()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns all keys (alive only).
    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, e)| e.is_alive())
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Returns total write operations.
    #[must_use]
    pub fn total_writes(&self) -> u64 {
        self.total_writes
    }

    /// Returns total read operations.
    #[must_use]
    pub fn total_reads(&self) -> u64 {
        self.total_reads
    }

    /// Increments read counter (called by CrudEngine).
    pub fn record_read(&mut self) {
        self.total_reads += 1;
    }

    /// Evicts expired entries based on retention policy.
    pub fn evict_expired(&mut self, now: u64) -> usize {
        if !self.config.has_retention() {
            return 0;
        }

        let threshold = now.saturating_sub(self.config.retention_secs);
        let before = self.entries.len();

        self.entries.retain(|_, e| e.updated_at >= threshold);

        before - self.entries.len()
    }

    /// Returns a snapshot of all entries (including deleted) for backup.
    #[must_use]
    pub fn raw_entries(&self) -> &HashMap<String, StoreEntry> {
        &self.entries
    }

    /// Restores entries from a backup.
    pub fn restore_entries(&mut self, entries: HashMap<String, StoreEntry>) {
        self.entries = entries;
    }
}

impl GroundsTo for PersistenceStore {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — durable storage
            LexPrimitiva::Mapping,     // μ — key→value index
            LexPrimitiva::Existence,   // ∃ — alive/deleted
            LexPrimitiva::Sequence,    // σ — version ordering
            LexPrimitiva::Boundary,    // ∂ — capacity limits
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_store_entry_grounding() {
        let comp = StoreEntry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }

    #[test]
    fn test_persistence_store_grounding() {
        let comp = PersistenceStore::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_store_kind_properties() {
        assert!(!StoreKind::InMemory.is_durable());
        assert!(StoreKind::AppendOnly.is_durable());
        assert!(StoreKind::KeyValue.supports_update());
        assert!(!StoreKind::AppendOnly.supports_update());
        assert!(StoreKind::Relational.supports_delete());
        assert!(!StoreKind::AppendOnly.supports_delete());
    }

    #[test]
    fn test_storage_key_value() {
        let key = StorageKey::new("drug:123");
        assert_eq!(key.as_str(), "drug:123");

        let val = StorageValue::new("aspirin");
        assert_eq!(val.len(), 7);
        assert!(!val.is_empty());

        let empty = StorageValue::new("");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_store_entry_lifecycle() {
        let mut entry = StoreEntry::new(StorageKey::new("k1"), StorageValue::new("v1"), 1000);
        assert_eq!(entry.version, 1);
        assert!(entry.is_alive());
        assert_eq!(entry.age(2000), 1000);

        entry.update(StorageValue::new("v2"), 1500);
        assert_eq!(entry.version, 2);
        assert_eq!(entry.updated_at, 1500);

        entry.soft_delete(2000);
        assert!(!entry.is_alive());
        assert_eq!(entry.version, 3);
    }

    #[test]
    fn test_store_config_defaults() {
        let config = StoreConfig::default();
        assert!(!config.is_bounded());
        assert!(!config.has_retention());
        assert!(config.soft_delete);
        assert!(config.versioned);
    }

    #[test]
    fn test_store_config_bounded() {
        let config = StoreConfig::default()
            .with_max_entries(100)
            .with_retention(3600);
        assert!(config.is_bounded());
        assert!(config.has_retention());
    }

    #[test]
    fn test_persistence_store_put_get() {
        let mut store = PersistenceStore::new(StoreId(1), "cases", StoreKind::KeyValue);

        let v = store.put("case:1", "warfarin-bleeding", 1000);
        assert_eq!(v, 1);
        assert!(store.contains("case:1"));
        assert!(!store.contains("case:2"));

        let entry = store.get("case:1");
        assert!(entry.is_some());
        if let Some(e) = entry {
            assert_eq!(e.value.0, "warfarin-bleeding");
        }
    }

    #[test]
    fn test_persistence_store_update() {
        let mut store = PersistenceStore::new(StoreId(1), "signals", StoreKind::KeyValue);

        store.put("sig:1", "prr=2.5", 1000);
        let v = store.put("sig:1", "prr=3.5", 2000);
        assert_eq!(v, 2);

        let entry = store.get("sig:1");
        assert!(entry.is_some());
        if let Some(e) = entry {
            assert_eq!(e.value.0, "prr=3.5");
            assert_eq!(e.version, 2);
        }
    }

    #[test]
    fn test_persistence_store_delete() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);

        store.put("k1", "v1", 1000);
        assert_eq!(store.len(), 1);

        let deleted = store.delete("k1", 2000);
        assert!(deleted);
        assert_eq!(store.len(), 0);
        assert!(!store.contains("k1"));

        // Double delete returns false
        let deleted2 = store.delete("k1", 3000);
        assert!(!deleted2);
    }

    #[test]
    fn test_persistence_store_capacity() {
        let config = StoreConfig::default().with_max_entries(2);
        let mut store =
            PersistenceStore::new(StoreId(1), "bounded", StoreKind::KeyValue).with_config(config);

        store.put("k1", "v1", 1000);
        store.put("k2", "v2", 2000);
        let v = store.put("k3", "v3", 3000);
        assert_eq!(v, 0); // At capacity, returns 0
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_persistence_store_eviction() {
        let config = StoreConfig::default().with_retention(1000);
        let mut store =
            PersistenceStore::new(StoreId(1), "expiring", StoreKind::KeyValue).with_config(config);

        store.put("old", "data", 100);
        store.put("new", "data", 2000);

        let evicted = store.evict_expired(2500);
        assert_eq!(evicted, 1);
        assert!(!store.contains("old"));
        assert!(store.contains("new"));
    }

    #[test]
    fn test_persistence_store_keys() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        store.put("a", "1", 1000);
        store.put("b", "2", 1000);
        store.put("c", "3", 1000);

        let mut keys = store.keys();
        keys.sort();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_persistence_store_counters() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        store.put("k1", "v1", 1000);
        store.put("k2", "v2", 2000);
        store.delete("k1", 3000);

        assert_eq!(store.total_writes(), 3);
        store.record_read();
        assert_eq!(store.total_reads(), 1);
    }
}
