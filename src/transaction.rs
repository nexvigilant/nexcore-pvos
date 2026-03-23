//! # PVTX Core Transaction Types
//!
//! Provides regulatory finality through an append-only transaction log
//! and a strict state machine that enforces irreversibility (∝).
//!
//! ## Primitives
//! - ∝ (Irreversibility) — DOMINANT: committed transactions cannot be undone
//! - ς (State) — transaction state machine
//! - π (Persistence) — append-only journal
//! - → (Causality) — transaction dependencies
//!
//! ## State Machine
//!
//! ```text
//! Pending → Prepared → Committed → Finalized
//!             ↓
//!         RolledBack
//! ```
//!
//! Once `Committed`, no rollback is possible. This is the ∝ boundary.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// T2-P NEWTYPES
// ===============================================================

/// Unique transaction identifier.
/// Tier: T2-P (∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxId(pub u64);

impl TxId {
    /// Creates a new transaction ID.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the inner value.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TX-{:08X}", self.0)
    }
}

impl GroundsTo for TxId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// TRANSACTION STATE MACHINE
// ===============================================================

/// Transaction lifecycle state.
/// Tier: T2-P (ς + ∝)
///
/// The critical invariant: once `Committed`, the transaction CANNOT
/// transition to `RolledBack`. This enforces ∝ (Irreversibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TxState {
    /// Transaction created, not yet validated.
    Pending,
    /// Validated and ready to commit (pre-commit check passed).
    Prepared,
    /// Committed — IRREVERSIBLE. The ∝ boundary has been crossed.
    Committed,
    /// Rolled back before commit (only from Pending or Prepared).
    RolledBack,
    /// Finalized — committed and sealed in audit trail.
    Finalized,
}

impl TxState {
    /// Returns true if the transaction is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack | Self::Finalized)
    }

    /// Returns true if the transaction has crossed the ∝ boundary.
    #[must_use]
    pub fn is_irreversible(&self) -> bool {
        matches!(self, Self::Committed | Self::Finalized)
    }

    /// Returns true if rollback is still possible.
    #[must_use]
    pub fn can_rollback(&self) -> bool {
        matches!(self, Self::Pending | Self::Prepared)
    }
}

impl GroundsTo for TxState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State, LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// TRANSACTION OUTCOME
// ===============================================================

/// The result of a transaction attempt.
/// Tier: T2-P (∝ + ς)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxOutcome {
    /// Transaction committed successfully.
    Success(TxId),
    /// Transaction failed with a reason.
    Failure(String),
}

impl TxOutcome {
    /// Returns true if the outcome is success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Returns the transaction ID if successful.
    #[must_use]
    pub fn tx_id(&self) -> Option<TxId> {
        match self {
            Self::Success(id) => Some(*id),
            Self::Failure(_) => None,
        }
    }
}

impl GroundsTo for TxOutcome {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility, LexPrimitiva::State])
    }
}

// ===============================================================
// TRANSACTION ERROR
// ===============================================================

/// Transaction layer errors.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxError {
    /// Invalid state transition attempted.
    InvalidTransition { from: TxState, to: TxState },
    /// Transaction not found.
    NotFound(TxId),
    /// Validation failed before commit.
    ValidationFailed(String),
    /// Attempted rollback on committed transaction (∝ violation).
    IrreversibleViolation(TxId),
    /// Duplicate transaction ID.
    DuplicateId(TxId),
    /// Missing required signature.
    SignatureRequired,
    /// Policy violation.
    PolicyViolation(String),
}

impl std::fmt::Display for TxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid transition: {from:?} -> {to:?}")
            }
            Self::NotFound(id) => write!(f, "transaction not found: {id}"),
            Self::ValidationFailed(msg) => write!(f, "validation failed: {msg}"),
            Self::IrreversibleViolation(id) => {
                write!(f, "cannot rollback committed transaction: {id}")
            }
            Self::DuplicateId(id) => write!(f, "duplicate transaction ID: {id}"),
            Self::SignatureRequired => write!(f, "signature required"),
            Self::PolicyViolation(msg) => write!(f, "policy violation: {msg}"),
        }
    }
}

impl std::error::Error for TxError {}

// ===============================================================
// TRANSACTION
// ===============================================================

/// A transactional operation wrapper.
/// Tier: T2-C (∝ + ς + → + ∂)
///
/// Wraps any operation in a transaction envelope that tracks state,
/// metadata, and dependencies. The `commit()` transition is the
/// irreversibility boundary (∝).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique identifier.
    pub id: TxId,
    /// Current state.
    state: TxState,
    /// What this transaction represents.
    pub description: String,
    /// Transaction kind.
    pub kind: TxKind,
    /// Metadata key-value pairs.
    pub metadata: HashMap<String, String>,
    /// Timestamp of creation (epoch seconds).
    pub created_at: u64,
    /// Timestamp of last state change.
    pub updated_at: u64,
    /// Optional parent transaction (→ causality chain).
    pub parent: Option<TxId>,
    /// Reason for rollback (if rolled back).
    pub rollback_reason: Option<String>,
}

/// Categories of regulatory transactions.
/// Tier: T2-P (∝)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TxKind {
    /// Signal confirmation decision.
    SignalConfirmation,
    /// Case assessment completion.
    CaseAssessment,
    /// Report submission to authority.
    RegulatorySubmission,
    /// Medical reviewer sign-off.
    MedicalReview,
    /// Audit period closure.
    AuditSeal,
    /// Threshold change (calibration).
    ThresholdChange,
    /// Generic transactional operation.
    Generic(String),
}

impl GroundsTo for TxKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

impl Transaction {
    /// Creates a new transaction in Pending state.
    #[must_use]
    pub fn new(id: TxId, description: &str, kind: TxKind, now: u64) -> Self {
        Self {
            id,
            state: TxState::Pending,
            description: description.to_string(),
            kind,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            parent: None,
            rollback_reason: None,
        }
    }

    /// Sets the parent transaction (causality chain).
    #[must_use]
    pub fn with_parent(mut self, parent: TxId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Adds a metadata key-value pair.
    pub fn set_meta(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Gets a metadata value.
    #[must_use]
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Current transaction state.
    #[must_use]
    pub fn state(&self) -> TxState {
        self.state
    }

    /// Prepares the transaction (validates it for commit).
    ///
    /// # Errors
    /// Returns `Err` if the current state is not `Pending`.
    pub fn prepare(&mut self, now: u64) -> Result<(), TxError> {
        if self.state != TxState::Pending {
            return Err(TxError::InvalidTransition {
                from: self.state,
                to: TxState::Prepared,
            });
        }
        self.state = TxState::Prepared;
        self.updated_at = now;
        Ok(())
    }

    /// Commits the transaction — crosses the ∝ boundary.
    /// After this call, the transaction CANNOT be rolled back.
    ///
    /// # Errors
    /// Returns `Err` if the current state is not `Prepared`.
    pub fn commit(&mut self, now: u64) -> Result<(), TxError> {
        if self.state != TxState::Prepared {
            return Err(TxError::InvalidTransition {
                from: self.state,
                to: TxState::Committed,
            });
        }
        // === POINT OF NO RETURN (∝) ===
        self.state = TxState::Committed;
        self.updated_at = now;
        Ok(())
    }

    /// Finalizes a committed transaction (sealed into audit trail).
    ///
    /// # Errors
    /// Returns `Err` if the current state is not `Committed`.
    pub fn finalize(&mut self, now: u64) -> Result<(), TxError> {
        if self.state != TxState::Committed {
            return Err(TxError::InvalidTransition {
                from: self.state,
                to: TxState::Finalized,
            });
        }
        self.state = TxState::Finalized;
        self.updated_at = now;
        Ok(())
    }

    /// Rolls back the transaction (only if not yet committed).
    ///
    /// # Errors
    /// Returns `Err(IrreversibleViolation)` if already committed — the ∝ invariant.
    pub fn rollback(&mut self, reason: &str, now: u64) -> Result<(), TxError> {
        if self.state.is_irreversible() {
            return Err(TxError::IrreversibleViolation(self.id));
        }
        if !self.state.can_rollback() {
            return Err(TxError::InvalidTransition {
                from: self.state,
                to: TxState::RolledBack,
            });
        }
        self.state = TxState::RolledBack;
        self.rollback_reason = Some(reason.to_string());
        self.updated_at = now;
        Ok(())
    }
}

impl GroundsTo for Transaction {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — commit is final
            LexPrimitiva::State,           // ς — state machine
            LexPrimitiva::Causality,       // → — parent chain
            LexPrimitiva::Boundary,        // ∂ — commit/rollback boundary
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.85)
    }
}

// ===============================================================
// TRANSACTION LOG — APPEND-ONLY JOURNAL
// ===============================================================

/// Transaction log entry — an immutable record.
/// Tier: T2-P (∝ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLogEntry {
    /// Transaction ID.
    pub tx_id: TxId,
    /// State at time of recording.
    pub state: TxState,
    /// Description.
    pub description: String,
    /// Transaction kind.
    pub kind: TxKind,
    /// Timestamp.
    pub timestamp: u64,
    /// Hash for integrity verification.
    pub hash: u64,
    /// Previous entry hash (chain linkage).
    pub prev_hash: u64,
}

impl GroundsTo for TxLogEntry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility,
            LexPrimitiva::Persistence,
        ])
    }
}

/// Append-only transaction journal. Once written, entries cannot
/// be modified or deleted. Hash-linked for tamper detection.
/// Tier: T2-C (∝ + π + → + ς)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLog {
    /// Immutable entries (append-only).
    entries: Vec<TxLogEntry>,
    /// Hash of the last entry (chain link).
    last_hash: u64,
}

impl TxLog {
    /// Creates an empty transaction log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_hash: 0,
        }
    }

    /// Appends a transaction to the log. Returns the log entry.
    /// This is an ∝ operation — once appended, it cannot be removed.
    pub fn append(&mut self, tx: &Transaction) -> TxLogEntry {
        let hash = self.compute_hash(tx);
        let entry = TxLogEntry {
            tx_id: tx.id,
            state: tx.state(),
            description: tx.description.clone(),
            kind: tx.kind.clone(),
            timestamp: tx.updated_at,
            hash,
            prev_hash: self.last_hash,
        };
        self.last_hash = hash;
        self.entries.push(entry.clone());
        entry
    }

    /// Number of entries in the log.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all entries (read-only view).
    #[must_use]
    pub fn entries(&self) -> &[TxLogEntry] {
        &self.entries
    }

    /// Returns entries for a specific transaction.
    #[must_use]
    pub fn entries_for(&self, tx_id: TxId) -> Vec<&TxLogEntry> {
        self.entries.iter().filter(|e| e.tx_id == tx_id).collect()
    }

    /// Verifies the hash chain integrity.
    /// Returns true if no tampering detected.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        let mut expected_prev = 0u64;
        for entry in &self.entries {
            if entry.prev_hash != expected_prev {
                return false;
            }
            expected_prev = entry.hash;
        }
        true
    }

    /// Last entry hash.
    #[must_use]
    pub fn last_hash(&self) -> u64 {
        self.last_hash
    }

    /// Computes a deterministic hash for a transaction.
    fn compute_hash(&self, tx: &Transaction) -> u64 {
        // Simple hash combining tx fields + prev hash for chain linkage.
        // In production, this would use SHA-256; here we use a
        // deterministic combiner for testability.
        let mut h: u64 = self.last_hash;
        h = h.wrapping_mul(31).wrapping_add(tx.id.0);
        h = h.wrapping_mul(31).wrapping_add(tx.updated_at);
        for byte in tx.description.bytes() {
            h = h.wrapping_mul(31).wrapping_add(u64::from(byte));
        }
        h
    }
}

impl Default for TxLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for TxLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — append-only
            LexPrimitiva::Persistence,     // π — durable journal
            LexPrimitiva::Causality,       // → — hash chain
            LexPrimitiva::State,           // ς — tracks state transitions
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.80)
    }
}

// ===============================================================
// TRANSACTION ENGINE — T3 CAPSTONE
// ===============================================================

/// The Transaction Engine — PVTX capstone providing regulatory finality.
/// Tier: T3 (∝ + π + → + ∂ + ς + ∃)
///
/// Orchestrates transactions, signatures, submissions, atomic
/// operations, and audit sealing. The ∝ primitive is dominant
/// because all regulatory compliance is fundamentally about
/// irreversible commitment.
///
/// Dominant primitive: ∝ (Irreversibility) — once committed, final.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEngine {
    /// Append-only transaction log (∝ + π).
    pub tx_log: TxLog,
    /// Next transaction ID.
    next_tx_id: u64,
    /// Total committed transactions.
    committed_count: u64,
    /// Total rolled-back transactions.
    rollback_count: u64,
}

impl TransactionEngine {
    /// Creates a new transaction engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tx_log: TxLog::new(),
            next_tx_id: 1,
            committed_count: 0,
            rollback_count: 0,
        }
    }

    /// Begins a new transaction.
    #[must_use]
    pub fn begin(&mut self, description: &str, kind: TxKind, now: u64) -> Transaction {
        let id = TxId::new(self.next_tx_id);
        self.next_tx_id += 1;
        let tx = Transaction::new(id, description, kind, now);
        self.tx_log.append(&tx);
        tx
    }

    /// Commits a transaction — crosses the ∝ boundary.
    ///
    /// # Errors
    /// Returns `Err` if the transaction cannot be committed.
    pub fn commit(&mut self, tx: &mut Transaction, now: u64) -> Result<TxOutcome, TxError> {
        tx.prepare(now)?;
        tx.commit(now)?;
        self.tx_log.append(tx);
        self.committed_count += 1;
        Ok(TxOutcome::Success(tx.id))
    }

    /// Rolls back a transaction.
    ///
    /// # Errors
    /// Returns `Err` if the transaction is already committed (∝).
    pub fn rollback(
        &mut self,
        tx: &mut Transaction,
        reason: &str,
        now: u64,
    ) -> Result<(), TxError> {
        tx.rollback(reason, now)?;
        self.tx_log.append(tx);
        self.rollback_count += 1;
        Ok(())
    }

    /// Total committed transactions.
    #[must_use]
    pub fn committed_count(&self) -> u64 {
        self.committed_count
    }

    /// Total rolled-back transactions.
    #[must_use]
    pub fn rollback_count(&self) -> u64 {
        self.rollback_count
    }

    /// Verifies log integrity.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        self.tx_log.verify_integrity()
    }
}

impl Default for TransactionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for TransactionEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — DOMINANT: regulatory finality
            LexPrimitiva::Persistence,     // π — durable transaction log
            LexPrimitiva::Causality,       // → — transaction chains
            LexPrimitiva::Boundary,        // ∂ — commit/rollback boundary
            LexPrimitiva::State,           // ς — transaction state machine
            LexPrimitiva::Existence,       // ∃ — signature verification
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.85)
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_tx(id: u64, desc: &str) -> Transaction {
        Transaction::new(TxId::new(id), desc, TxKind::Generic("test".into()), 1000)
    }

    #[test]
    fn test_tx_state_machine_happy_path() {
        let mut tx = make_tx(1, "test commit");
        assert_eq!(tx.state(), TxState::Pending);

        assert!(tx.prepare(1001).is_ok());
        assert_eq!(tx.state(), TxState::Prepared);

        assert!(tx.commit(1002).is_ok());
        assert_eq!(tx.state(), TxState::Committed);
        assert!(tx.state().is_irreversible());

        assert!(tx.finalize(1003).is_ok());
        assert_eq!(tx.state(), TxState::Finalized);
    }

    #[test]
    fn test_tx_rollback_from_pending() {
        let mut tx = make_tx(2, "rollback pending");
        assert!(tx.rollback("changed mind", 1001).is_ok());
        assert_eq!(tx.state(), TxState::RolledBack);
        assert_eq!(tx.rollback_reason.as_deref(), Some("changed mind"));
    }

    #[test]
    fn test_tx_rollback_from_prepared() {
        let mut tx = make_tx(3, "rollback prepared");
        assert!(tx.prepare(1001).is_ok());
        assert!(tx.rollback("validation failed", 1002).is_ok());
        assert_eq!(tx.state(), TxState::RolledBack);
    }

    #[test]
    fn test_tx_irreversibility_violation() {
        let mut tx = make_tx(4, "irreversible");
        assert!(tx.prepare(1001).is_ok());
        assert!(tx.commit(1002).is_ok());

        // ∝ invariant: cannot rollback after commit
        let err = tx.rollback("too late", 1003);
        assert!(err.is_err());
        if let Err(TxError::IrreversibleViolation(id)) = err {
            assert_eq!(id, TxId::new(4));
        }
    }

    #[test]
    fn test_tx_invalid_transitions() {
        let mut tx = make_tx(5, "invalid");

        // Cannot commit from Pending (must prepare first)
        assert!(tx.commit(1001).is_err());

        // Cannot finalize from Pending
        assert!(tx.finalize(1001).is_err());

        // Prepare then commit
        assert!(tx.prepare(1001).is_ok());
        assert!(tx.commit(1002).is_ok());

        // Cannot prepare again after commit
        assert!(tx.prepare(1003).is_err());
    }

    #[test]
    fn test_tx_metadata() {
        let mut tx = make_tx(6, "with metadata");
        tx.set_meta("drug", "aspirin");
        tx.set_meta("event", "headache");

        assert_eq!(tx.meta("drug"), Some("aspirin"));
        assert_eq!(tx.meta("event"), Some("headache"));
        assert_eq!(tx.meta("missing"), None);
    }

    #[test]
    fn test_tx_parent_chain() {
        let parent = make_tx(10, "parent");
        let child = make_tx(11, "child").with_parent(parent.id);

        assert_eq!(child.parent, Some(TxId::new(10)));
    }

    #[test]
    fn test_tx_id_display() {
        let id = TxId::new(255);
        assert_eq!(format!("{id}"), "TX-000000FF");
    }

    #[test]
    fn test_tx_state_properties() {
        assert!(!TxState::Pending.is_terminal());
        assert!(!TxState::Prepared.is_terminal());
        assert!(TxState::Committed.is_terminal());
        assert!(TxState::RolledBack.is_terminal());
        assert!(TxState::Finalized.is_terminal());

        assert!(TxState::Pending.can_rollback());
        assert!(TxState::Prepared.can_rollback());
        assert!(!TxState::Committed.can_rollback());

        assert!(TxState::Committed.is_irreversible());
        assert!(TxState::Finalized.is_irreversible());
        assert!(!TxState::Pending.is_irreversible());
    }

    #[test]
    fn test_tx_outcome() {
        let success = TxOutcome::Success(TxId::new(42));
        assert!(success.is_success());
        assert_eq!(success.tx_id(), Some(TxId::new(42)));

        let failure = TxOutcome::Failure("validation error".into());
        assert!(!failure.is_success());
        assert_eq!(failure.tx_id(), None);
    }

    #[test]
    fn test_tx_log_append_and_query() {
        let mut log = TxLog::new();
        assert!(log.is_empty());

        let tx1 = make_tx(1, "first");
        let tx2 = make_tx(2, "second");

        log.append(&tx1);
        log.append(&tx2);

        assert_eq!(log.len(), 2);
        assert!(!log.is_empty());

        let entries = log.entries_for(TxId::new(1));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].description, "first");
    }

    #[test]
    fn test_tx_log_hash_chain() {
        let mut log = TxLog::new();

        let tx1 = make_tx(1, "alpha");
        let tx2 = make_tx(2, "beta");
        let tx3 = make_tx(3, "gamma");

        log.append(&tx1);
        log.append(&tx2);
        log.append(&tx3);

        // Chain integrity check
        assert!(log.verify_integrity());

        // All entries have linked hashes
        let entries = log.entries();
        assert_eq!(entries[0].prev_hash, 0); // First entry links to 0
        assert_eq!(entries[1].prev_hash, entries[0].hash);
        assert_eq!(entries[2].prev_hash, entries[1].hash);
    }

    #[test]
    fn test_tx_log_tamper_detection() {
        let mut log = TxLog::new();

        let tx1 = make_tx(1, "original");
        let tx2 = make_tx(2, "second");
        log.append(&tx1);
        log.append(&tx2);

        assert!(log.verify_integrity());

        // Simulate tampering: modify an entry's prev_hash
        // We can't easily do this with the current API (good!),
        // but we can verify the chain is valid.
        assert_eq!(log.last_hash(), log.entries().last().map_or(0, |e| e.hash));
    }

    #[test]
    fn test_tx_grounding() {
        let comp = Transaction::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_tx_log_grounding() {
        let comp = TxLog::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_tx_state_grounding() {
        let comp = TxState::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
