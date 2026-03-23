//! # PVDB CRUD — Create/Read/Update/Delete Operations
//!
//! Systematic CRUD operations for the persistence layer.
//! Maps domain operations to storage actions with audit logging.
//!
//! ## T1 Grounding (dominant: π Persistence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | π | Persistence | 0.35 — durable operations |
//! | μ | Mapping | 0.25 — operation→storage |
//! | → | Causality | 0.20 — op causes state change |
//! | σ | Sequence | 0.20 — ordered log |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::store::{PersistenceStore, StorageValue, StoreEntry};

// ═══════════════════════════════════════════════════════════
// CRUD OPERATIONS
// ═══════════════════════════════════════════════════════════

/// CRUD operation type.
///
/// Tier: T2-P (μ — maps to storage action)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrudOp {
    /// Create a new entry.
    Create { key: String, value: String },
    /// Read an entry by key.
    Read { key: String },
    /// Update an existing entry.
    Update { key: String, value: String },
    /// Delete an entry by key.
    Delete { key: String },
}

impl CrudOp {
    /// Returns the key targeted by this operation.
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Create { key, .. }
            | Self::Read { key }
            | Self::Update { key, .. }
            | Self::Delete { key } => key,
        }
    }

    /// Returns the operation name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Create { .. } => "CREATE",
            Self::Read { .. } => "READ",
            Self::Update { .. } => "UPDATE",
            Self::Delete { .. } => "DELETE",
        }
    }

    /// Whether this is a write operation.
    #[must_use]
    pub fn is_write(&self) -> bool {
        !matches!(self, Self::Read { .. })
    }
}

// ═══════════════════════════════════════════════════════════
// CRUD RESULT
// ═══════════════════════════════════════════════════════════

/// Result of a CRUD operation.
///
/// Tier: T2-P (∃ — existence outcome)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrudResult {
    /// Entry was created (returns version).
    Created { version: u64 },
    /// Entry was found (returns value and version).
    Found { value: String, version: u64 },
    /// Entry was updated (returns new version).
    Updated { version: u64 },
    /// Entry was deleted.
    Deleted,
    /// Entry not found.
    NotFound,
    /// Conflict: entry already exists (for create) or version mismatch.
    Conflict { reason: String },
    /// Store at capacity.
    AtCapacity,
}

impl CrudResult {
    /// Whether the operation succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Created { .. } | Self::Found { .. } | Self::Updated { .. } | Self::Deleted
        )
    }

    /// Whether the result is a not-found.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }
}

// ═══════════════════════════════════════════════════════════
// CRUD FILTER
// ═══════════════════════════════════════════════════════════

/// Filter for listing/querying stored entries.
///
/// Tier: T2-C (π + κ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudFilter {
    /// Key prefix to match (empty = all).
    pub key_prefix: String,
    /// Minimum version to include.
    pub min_version: Option<u64>,
    /// Maximum results.
    pub limit: usize,
    /// Only include alive entries.
    pub alive_only: bool,
}

impl Default for CrudFilter {
    fn default() -> Self {
        Self {
            key_prefix: String::new(),
            min_version: None,
            limit: 100,
            alive_only: true,
        }
    }
}

impl CrudFilter {
    /// Creates a filter matching all entries.
    #[must_use]
    pub fn all() -> Self {
        Self {
            limit: usize::MAX,
            ..Self::default()
        }
    }

    /// Creates a filter with key prefix.
    #[must_use]
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            key_prefix: prefix.to_string(),
            ..Self::default()
        }
    }

    /// Sets the maximum results.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Checks whether an entry matches this filter.
    #[must_use]
    pub fn matches(&self, entry: &StoreEntry) -> bool {
        if self.alive_only && !entry.is_alive() {
            return false;
        }
        if !self.key_prefix.is_empty() && !entry.key.as_str().starts_with(&self.key_prefix) {
            return false;
        }
        if let Some(min_v) = self.min_version {
            if entry.version < min_v {
                return false;
            }
        }
        true
    }
}

// ═══════════════════════════════════════════════════════════
// CRUD BATCH
// ═══════════════════════════════════════════════════════════

/// A batch of CRUD operations executed atomically.
///
/// Tier: T2-C (π + σ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudBatch {
    /// Operations in execution order.
    pub ops: Vec<CrudOp>,
    /// Whether all ops must succeed (atomic batch).
    pub atomic: bool,
}

impl CrudBatch {
    /// Creates a new batch.
    #[must_use]
    pub fn new(atomic: bool) -> Self {
        Self {
            ops: Vec::new(),
            atomic,
        }
    }

    /// Adds an operation to the batch.
    pub fn add(&mut self, op: CrudOp) {
        self.ops.push(op);
    }

    /// Returns the number of operations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Returns number of write operations.
    #[must_use]
    pub fn write_count(&self) -> usize {
        self.ops.iter().filter(|op| op.is_write()).count()
    }
}

// ═══════════════════════════════════════════════════════════
// CRUD LOG
// ═══════════════════════════════════════════════════════════

/// Record of a single CRUD operation for audit.
///
/// Tier: T2-C (π + σ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudLogEntry {
    /// Sequence number.
    pub seq: u64,
    /// Operation that was performed.
    pub op_name: String,
    /// Key targeted.
    pub key: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Timestamp of the operation.
    pub timestamp: u64,
}

/// Audit log of CRUD operations.
///
/// Tier: T2-C (π + σ + → + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudLog {
    /// Ordered log entries.
    entries: Vec<CrudLogEntry>,
    /// Next sequence number.
    next_seq: u64,
}

impl CrudLog {
    /// Creates a new empty CRUD log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_seq: 1,
        }
    }

    /// Records an operation.
    pub fn record(&mut self, op_name: &str, key: &str, success: bool, timestamp: u64) {
        self.entries.push(CrudLogEntry {
            seq: self.next_seq,
            op_name: op_name.to_string(),
            key: key.to_string(),
            success,
            timestamp,
        });
        self.next_seq += 1;
    }

    /// Returns all log entries.
    #[must_use]
    pub fn entries(&self) -> &[CrudLogEntry] {
        &self.entries
    }

    /// Returns the total number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the count of successful operations.
    #[must_use]
    pub fn success_count(&self) -> usize {
        self.entries.iter().filter(|e| e.success).count()
    }

    /// Returns the count of failed operations.
    #[must_use]
    pub fn failure_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.success).count()
    }

    /// Returns entries for a specific key.
    #[must_use]
    pub fn entries_for_key(&self, key: &str) -> Vec<&CrudLogEntry> {
        self.entries.iter().filter(|e| e.key == key).collect()
    }
}

impl Default for CrudLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for CrudLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — durable log
            LexPrimitiva::Sequence,    // σ — ordered entries
            LexPrimitiva::Causality,   // → — op causes record
            LexPrimitiva::Comparison,  // κ — success/failure
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// CRUD ENGINE
// ═══════════════════════════════════════════════════════════

/// CRUD engine: executes operations against a persistence store.
///
/// Tier: T2-C (π + μ + → + σ + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudEngine {
    /// Audit log of all operations.
    log: CrudLog,
    /// Total operations executed.
    total_ops: u64,
}

impl CrudEngine {
    /// Creates a new CRUD engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            log: CrudLog::new(),
            total_ops: 0,
        }
    }

    /// Executes a single CRUD operation against a store.
    pub fn execute(
        &mut self,
        store: &mut PersistenceStore,
        op: &CrudOp,
        timestamp: u64,
    ) -> CrudResult {
        self.total_ops += 1;
        let result = match op {
            CrudOp::Create { key, value } => {
                if store.contains(key) {
                    CrudResult::Conflict {
                        reason: "key already exists".to_string(),
                    }
                } else {
                    let version = store.put(key, value, timestamp);
                    if version == 0 {
                        CrudResult::AtCapacity
                    } else {
                        CrudResult::Created { version }
                    }
                }
            }
            CrudOp::Read { key } => {
                store.record_read();
                match store.get(key) {
                    Some(entry) => CrudResult::Found {
                        value: entry.value.0.clone(),
                        version: entry.version,
                    },
                    None => CrudResult::NotFound,
                }
            }
            CrudOp::Update { key, value } => {
                if store.contains(key) {
                    let version = store.put(key, value, timestamp);
                    CrudResult::Updated { version }
                } else {
                    CrudResult::NotFound
                }
            }
            CrudOp::Delete { key } => {
                if store.delete(key, timestamp) {
                    CrudResult::Deleted
                } else {
                    CrudResult::NotFound
                }
            }
        };

        self.log
            .record(op.name(), op.key(), result.is_success(), timestamp);

        result
    }

    /// Executes a batch of operations.
    pub fn execute_batch(
        &mut self,
        store: &mut PersistenceStore,
        batch: &CrudBatch,
        timestamp: u64,
    ) -> Vec<CrudResult> {
        let mut results = Vec::with_capacity(batch.ops.len());

        for op in &batch.ops {
            let result = self.execute(store, op, timestamp);
            if batch.atomic && !result.is_success() {
                // In atomic mode, stop on first failure
                results.push(result);
                return results;
            }
            results.push(result);
        }

        results
    }

    /// Queries entries matching a filter.
    #[must_use]
    pub fn query<'a>(
        &self,
        store: &'a PersistenceStore,
        filter: &CrudFilter,
    ) -> Vec<&'a StoreEntry> {
        store
            .entries()
            .into_iter()
            .filter(|e| filter.matches(e))
            .take(filter.limit)
            .collect()
    }

    /// Returns the audit log.
    #[must_use]
    pub fn log(&self) -> &CrudLog {
        &self.log
    }

    /// Returns total operations executed.
    #[must_use]
    pub fn total_ops(&self) -> u64 {
        self.total_ops
    }
}

impl Default for CrudEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for CrudEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — durable operations
            LexPrimitiva::Mapping,     // μ — op→storage
            LexPrimitiva::Causality,   // → — op causes change
            LexPrimitiva::Sequence,    // σ — ordered log
            LexPrimitiva::Comparison,  // κ — exists/not-found
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{StoreId, StoreKind};
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_crud_log_grounding() {
        let comp = CrudLog::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }

    #[test]
    fn test_crud_engine_grounding() {
        let comp = CrudEngine::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_crud_op_properties() {
        let create = CrudOp::Create {
            key: "k1".into(),
            value: "v1".into(),
        };
        assert_eq!(create.key(), "k1");
        assert_eq!(create.name(), "CREATE");
        assert!(create.is_write());

        let read = CrudOp::Read { key: "k1".into() };
        assert!(!read.is_write());
    }

    #[test]
    fn test_crud_result_states() {
        assert!(CrudResult::Created { version: 1 }.is_success());
        assert!(
            CrudResult::Found {
                value: "v".into(),
                version: 1
            }
            .is_success()
        );
        assert!(CrudResult::Updated { version: 2 }.is_success());
        assert!(CrudResult::Deleted.is_success());
        assert!(!CrudResult::NotFound.is_success());
        assert!(CrudResult::NotFound.is_not_found());
    }

    #[test]
    fn test_crud_filter_matching() {
        let filter = CrudFilter::with_prefix("case:");

        let entry = super::super::store::StoreEntry::new(
            super::super::store::StorageKey::new("case:1"),
            StorageValue::new("data"),
            1000,
        );
        assert!(filter.matches(&entry));

        let other = super::super::store::StoreEntry::new(
            super::super::store::StorageKey::new("signal:1"),
            StorageValue::new("data"),
            1000,
        );
        assert!(!filter.matches(&other));
    }

    #[test]
    fn test_crud_engine_create_read() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        let result = engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "k1".into(),
                value: "v1".into(),
            },
            1000,
        );
        assert!(result.is_success());

        let result = engine.execute(&mut store, &CrudOp::Read { key: "k1".into() }, 2000);
        assert!(matches!(result, CrudResult::Found { value, .. } if value == "v1"));
    }

    #[test]
    fn test_crud_engine_update() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "k1".into(),
                value: "v1".into(),
            },
            1000,
        );

        let result = engine.execute(
            &mut store,
            &CrudOp::Update {
                key: "k1".into(),
                value: "v2".into(),
            },
            2000,
        );
        assert!(matches!(result, CrudResult::Updated { version: 2 }));
    }

    #[test]
    fn test_crud_engine_delete() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "k1".into(),
                value: "v1".into(),
            },
            1000,
        );

        let result = engine.execute(&mut store, &CrudOp::Delete { key: "k1".into() }, 2000);
        assert!(matches!(result, CrudResult::Deleted));

        let result = engine.execute(&mut store, &CrudOp::Read { key: "k1".into() }, 3000);
        assert!(result.is_not_found());
    }

    #[test]
    fn test_crud_engine_conflict() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "k1".into(),
                value: "v1".into(),
            },
            1000,
        );

        let result = engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "k1".into(),
                value: "v2".into(),
            },
            2000,
        );
        assert!(matches!(result, CrudResult::Conflict { .. }));
    }

    #[test]
    fn test_crud_batch_execution() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        let mut batch = CrudBatch::new(false);
        batch.add(CrudOp::Create {
            key: "a".into(),
            value: "1".into(),
        });
        batch.add(CrudOp::Create {
            key: "b".into(),
            value: "2".into(),
        });
        batch.add(CrudOp::Create {
            key: "c".into(),
            value: "3".into(),
        });

        assert_eq!(batch.len(), 3);
        assert_eq!(batch.write_count(), 3);

        let results = engine.execute_batch(&mut store, &batch, 1000);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_success()));
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_crud_atomic_batch_stops_on_failure() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        let mut batch = CrudBatch::new(true);
        batch.add(CrudOp::Create {
            key: "a".into(),
            value: "1".into(),
        });
        batch.add(CrudOp::Update {
            key: "missing".into(),
            value: "x".into(),
        }); // Will fail
        batch.add(CrudOp::Create {
            key: "c".into(),
            value: "3".into(),
        });

        let results = engine.execute_batch(&mut store, &batch, 1000);
        assert_eq!(results.len(), 2); // Stops at first failure
    }

    #[test]
    fn test_crud_engine_query() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "case:1".into(),
                value: "c1".into(),
            },
            1000,
        );
        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "case:2".into(),
                value: "c2".into(),
            },
            2000,
        );
        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "signal:1".into(),
                value: "s1".into(),
            },
            3000,
        );

        let cases = engine.query(&store, &CrudFilter::with_prefix("case:"));
        assert_eq!(cases.len(), 2);
    }

    #[test]
    fn test_crud_log_audit() {
        let mut store = PersistenceStore::new(StoreId(1), "test", StoreKind::KeyValue);
        let mut engine = CrudEngine::new();

        engine.execute(
            &mut store,
            &CrudOp::Create {
                key: "k1".into(),
                value: "v1".into(),
            },
            1000,
        );
        engine.execute(&mut store, &CrudOp::Read { key: "k1".into() }, 2000);
        engine.execute(
            &mut store,
            &CrudOp::Delete {
                key: "missing".into(),
            },
            3000,
        );

        let log = engine.log();
        assert_eq!(log.len(), 3);
        assert_eq!(log.success_count(), 2);
        assert_eq!(log.failure_count(), 1);
        assert_eq!(log.entries_for_key("k1").len(), 2);
    }
}
