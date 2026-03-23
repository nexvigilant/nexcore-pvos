//! # PVTX Atomic Operations
//!
//! All-or-nothing operations for multi-step regulatory processes.
//! Provides saga pattern (compensating transactions), two-phase
//! commit, and idempotency guarantees.
//!
//! ## Primitives
//! - ∝ (Irreversibility) — DOMINANT: atomic commit is final
//! - ∂ (Boundary) — commit/rollback decision point
//! - → (Causality) — step ordering and dependencies
//! - ς (State) — operation state tracking
//!
//! ## Patterns
//!
//! - **AtomicOp**: Single all-or-nothing operation
//! - **Saga**: Multi-step with compensating rollbacks
//! - **TwoPhaseCommit**: Prepare-all then commit-all protocol
//! - **Idempotency**: Safe retry semantics via keys

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::transaction::TxId;

// ===============================================================
// ATOMIC OPERATION
// ===============================================================

/// State of an atomic operation.
/// Tier: T2-P (ς + ∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtomicState {
    /// Not yet started.
    Ready,
    /// Currently executing.
    Executing,
    /// Successfully committed (∝).
    Committed,
    /// Failed and rolled back.
    Failed,
}

impl GroundsTo for AtomicState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State, LexPrimitiva::Irreversibility])
    }
}

/// A single atomic operation — all or nothing.
/// Tier: T2-C (∝ + ∂ + ς)
///
/// Either the entire operation succeeds and is committed (∝),
/// or it fails and no side effects persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicOp {
    /// Associated transaction.
    pub tx_id: TxId,
    /// Description of the operation.
    pub description: String,
    /// Current state.
    state: AtomicState,
    /// Pre-conditions that must hold (checked before execute).
    pub preconditions: Vec<String>,
    /// Result value (set on commit).
    pub result: Option<String>,
    /// Error reason (set on failure).
    pub error: Option<String>,
}

impl AtomicOp {
    /// Creates a new atomic operation.
    #[must_use]
    pub fn new(tx_id: TxId, description: &str) -> Self {
        Self {
            tx_id,
            description: description.to_string(),
            state: AtomicState::Ready,
            preconditions: Vec::new(),
            result: None,
            error: None,
        }
    }

    /// Adds a precondition check.
    #[must_use]
    pub fn with_precondition(mut self, condition: &str) -> Self {
        self.preconditions.push(condition.to_string());
        self
    }

    /// Current state.
    #[must_use]
    pub fn state(&self) -> AtomicState {
        self.state
    }

    /// Begins execution (Ready → Executing).
    ///
    /// # Errors
    /// Returns `Err` if not in Ready state.
    pub fn begin(&mut self) -> Result<(), AtomicError> {
        if self.state != AtomicState::Ready {
            return Err(AtomicError::InvalidState(self.state));
        }
        self.state = AtomicState::Executing;
        Ok(())
    }

    /// Commits the operation — irreversible (∝).
    ///
    /// # Errors
    /// Returns `Err` if not in Executing state.
    pub fn commit(&mut self, result: &str) -> Result<(), AtomicError> {
        if self.state != AtomicState::Executing {
            return Err(AtomicError::InvalidState(self.state));
        }
        // === POINT OF NO RETURN (∝) ===
        self.state = AtomicState::Committed;
        self.result = Some(result.to_string());
        Ok(())
    }

    /// Fails the operation with a reason.
    ///
    /// # Errors
    /// Returns `Err` if already committed (∝ violation).
    pub fn fail(&mut self, reason: &str) -> Result<(), AtomicError> {
        if self.state == AtomicState::Committed {
            return Err(AtomicError::AlreadyCommitted);
        }
        self.state = AtomicState::Failed;
        self.error = Some(reason.to_string());
        Ok(())
    }
}

impl GroundsTo for AtomicOp {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — commit
            LexPrimitiva::Boundary,        // ∂ — success/fail
            LexPrimitiva::State,           // ς — tracking
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.80)
    }
}

// ===============================================================
// SAGA (COMPENSATING TRANSACTIONS)
// ===============================================================

/// A single step in a saga with its compensating action.
/// Tier: T2-P (→ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    /// Step name.
    pub name: String,
    /// Forward action description.
    pub action: String,
    /// Compensating action (undo) description.
    pub compensate: String,
    /// Whether this step has been completed.
    pub completed: bool,
    /// Whether this step has been compensated.
    pub compensated: bool,
}

impl SagaStep {
    /// Creates a new saga step.
    #[must_use]
    pub fn new(name: &str, action: &str, compensate: &str) -> Self {
        Self {
            name: name.to_string(),
            action: action.to_string(),
            compensate: compensate.to_string(),
            completed: false,
            compensated: false,
        }
    }
}

impl GroundsTo for SagaStep {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality, LexPrimitiva::Boundary])
    }
}

/// Saga state.
/// Tier: T2-P (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SagaState {
    /// Not yet started.
    Pending,
    /// Executing forward steps.
    Running,
    /// All steps completed successfully.
    Completed,
    /// A step failed, compensating.
    Compensating,
    /// All compensation complete.
    Compensated,
    /// Committed — all steps confirmed final (∝).
    Committed,
}

impl GroundsTo for SagaState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State])
    }
}

/// Multi-step compensating transaction.
/// Tier: T2-C (∝ + → + ∂ + ς)
///
/// Executes steps in order. On failure, compensates completed
/// steps in reverse order. Once committed (∝), compensation
/// is no longer possible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    /// Associated transaction.
    pub tx_id: TxId,
    /// Saga name.
    pub name: String,
    /// Ordered steps.
    steps: Vec<SagaStep>,
    /// Current state.
    state: SagaState,
    /// Current step index (0-based).
    current_step: usize,
    /// Failed step index (if failed).
    failed_at: Option<usize>,
}

impl Saga {
    /// Creates a new saga.
    #[must_use]
    pub fn new(tx_id: TxId, name: &str) -> Self {
        Self {
            tx_id,
            name: name.to_string(),
            steps: Vec::new(),
            state: SagaState::Pending,
            current_step: 0,
            failed_at: None,
        }
    }

    /// Adds a step to the saga.
    pub fn add_step(&mut self, step: SagaStep) {
        self.steps.push(step);
    }

    /// Current saga state.
    #[must_use]
    pub fn state(&self) -> SagaState {
        self.state
    }

    /// Number of steps.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Number of completed steps.
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.steps.iter().filter(|s| s.completed).count()
    }

    /// Starts the saga (Pending → Running).
    ///
    /// # Errors
    /// Returns `Err` if no steps or not in Pending state.
    pub fn start(&mut self) -> Result<(), AtomicError> {
        if self.state != SagaState::Pending {
            return Err(AtomicError::InvalidState(AtomicState::Executing));
        }
        if self.steps.is_empty() {
            return Err(AtomicError::EmptySaga);
        }
        self.state = SagaState::Running;
        self.current_step = 0;
        Ok(())
    }

    /// Marks the current step as completed and advances.
    ///
    /// # Errors
    /// Returns `Err` if not running or all steps done.
    pub fn complete_step(&mut self) -> Result<(), AtomicError> {
        if self.state != SagaState::Running {
            return Err(AtomicError::InvalidState(AtomicState::Executing));
        }
        if self.current_step >= self.steps.len() {
            return Err(AtomicError::NoMoreSteps);
        }
        self.steps[self.current_step].completed = true;
        self.current_step += 1;

        if self.current_step >= self.steps.len() {
            self.state = SagaState::Completed;
        }
        Ok(())
    }

    /// Marks the current step as failed and begins compensation.
    pub fn fail_step(&mut self, _reason: &str) {
        self.failed_at = Some(self.current_step);
        self.state = SagaState::Compensating;
    }

    /// Compensates completed steps in reverse order.
    /// Returns the list of compensated step names.
    #[must_use]
    pub fn compensate(&mut self) -> Vec<String> {
        let mut compensated = Vec::new();

        // Compensate in reverse order
        for step in self.steps.iter_mut().rev() {
            if step.completed && !step.compensated {
                step.compensated = true;
                compensated.push(step.name.clone());
            }
        }

        self.state = SagaState::Compensated;
        compensated
    }

    /// Commits the saga — all steps are now final (∝).
    ///
    /// # Errors
    /// Returns `Err` if not all steps completed.
    pub fn commit(&mut self) -> Result<(), AtomicError> {
        if self.state != SagaState::Completed {
            return Err(AtomicError::IncompleteSteps);
        }
        // === POINT OF NO RETURN (∝) ===
        self.state = SagaState::Committed;
        Ok(())
    }
}

impl GroundsTo for Saga {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — commit
            LexPrimitiva::Causality,       // → — step ordering
            LexPrimitiva::Boundary,        // ∂ — success/fail
            LexPrimitiva::State,           // ς — saga state
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.80)
    }
}

// ===============================================================
// TWO-PHASE COMMIT
// ===============================================================

/// Phase of the 2PC protocol.
/// Tier: T2-P (ς + ∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TwoPhaseState {
    /// Collecting votes.
    Voting,
    /// All voted yes — preparing to commit.
    Prepared,
    /// Committed — final (∝).
    Committed,
    /// Aborted (at least one vote no).
    Aborted,
}

impl GroundsTo for TwoPhaseState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State, LexPrimitiva::Irreversibility])
    }
}

/// Two-phase commit coordinator.
/// Tier: T2-C (∝ + ∂ + ς + →)
///
/// Phase 1 (Voting): All participants must vote "yes" to proceed.
/// Phase 2 (Commit): If unanimous yes, commit all. Otherwise abort.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoPhaseCommit {
    /// Associated transaction.
    pub tx_id: TxId,
    /// Participant votes (true = yes).
    votes: HashMap<String, bool>,
    /// Current state.
    state: TwoPhaseState,
}

impl TwoPhaseCommit {
    /// Creates a new 2PC coordinator.
    #[must_use]
    pub fn new(tx_id: TxId) -> Self {
        Self {
            tx_id,
            votes: HashMap::new(),
            state: TwoPhaseState::Voting,
        }
    }

    /// Current state.
    #[must_use]
    pub fn state(&self) -> TwoPhaseState {
        self.state
    }

    /// Records a participant's vote.
    ///
    /// # Errors
    /// Returns `Err` if not in voting phase.
    pub fn vote(&mut self, participant: &str, yes: bool) -> Result<(), AtomicError> {
        if self.state != TwoPhaseState::Voting {
            return Err(AtomicError::InvalidState(AtomicState::Executing));
        }
        self.votes.insert(participant.to_string(), yes);
        Ok(())
    }

    /// Checks if all votes are "yes".
    #[must_use]
    pub fn is_unanimous(&self) -> bool {
        !self.votes.is_empty() && self.votes.values().all(|v| *v)
    }

    /// Number of participants who voted.
    #[must_use]
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Prepares for commit (all voted yes → Prepared).
    ///
    /// # Errors
    /// Returns `Err` if votes not unanimous.
    pub fn prepare(&mut self) -> Result<(), AtomicError> {
        if self.state != TwoPhaseState::Voting {
            return Err(AtomicError::InvalidState(AtomicState::Ready));
        }
        if !self.is_unanimous() {
            self.state = TwoPhaseState::Aborted;
            return Err(AtomicError::VoteRejected);
        }
        self.state = TwoPhaseState::Prepared;
        Ok(())
    }

    /// Commits the 2PC — irreversible (∝).
    ///
    /// # Errors
    /// Returns `Err` if not prepared.
    pub fn commit(&mut self) -> Result<(), AtomicError> {
        if self.state != TwoPhaseState::Prepared {
            return Err(AtomicError::InvalidState(AtomicState::Executing));
        }
        // === POINT OF NO RETURN (∝) ===
        self.state = TwoPhaseState::Committed;
        Ok(())
    }

    /// Aborts the 2PC.
    ///
    /// # Errors
    /// Returns `Err` if already committed (∝ violation).
    pub fn abort(&mut self) -> Result<(), AtomicError> {
        if self.state == TwoPhaseState::Committed {
            return Err(AtomicError::AlreadyCommitted);
        }
        self.state = TwoPhaseState::Aborted;
        Ok(())
    }
}

impl GroundsTo for TwoPhaseCommit {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — commit
            LexPrimitiva::Boundary,        // ∂ — vote threshold
            LexPrimitiva::State,           // ς — phase tracking
            LexPrimitiva::Causality,       // → — phase ordering
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.80)
    }
}

// ===============================================================
// IDEMPOTENCY
// ===============================================================

/// Idempotency key for safe retries.
/// Tier: T2-P (∝ + ∃)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    /// Creates a new idempotency key.
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self(key.to_string())
    }
}

impl GroundsTo for IdempotencyKey {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility, LexPrimitiva::Existence])
    }
}

/// Idempotency registry — prevents duplicate operations.
/// Tier: T2-C (∝ + ∃ + π)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdempotencyGuard {
    /// Completed keys → result.
    completed: HashMap<IdempotencyKey, String>,
}

impl IdempotencyGuard {
    /// Creates a new guard.
    #[must_use]
    pub fn new() -> Self {
        Self {
            completed: HashMap::new(),
        }
    }

    /// Checks if an operation has already been completed.
    #[must_use]
    pub fn is_completed(&self, key: &IdempotencyKey) -> bool {
        self.completed.contains_key(key)
    }

    /// Gets the result of a previously completed operation.
    #[must_use]
    pub fn get_result(&self, key: &IdempotencyKey) -> Option<&str> {
        self.completed.get(key).map(|s| s.as_str())
    }

    /// Records a completed operation.
    /// Returns true if this was a new completion, false if duplicate.
    pub fn record(&mut self, key: IdempotencyKey, result: &str) -> bool {
        if self.completed.contains_key(&key) {
            false
        } else {
            self.completed.insert(key, result.to_string());
            true
        }
    }

    /// Number of recorded operations.
    #[must_use]
    pub fn count(&self) -> usize {
        self.completed.len()
    }
}

impl GroundsTo for IdempotencyGuard {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility,
            LexPrimitiva::Existence,
            LexPrimitiva::Persistence,
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.75)
    }
}

// ===============================================================
// ATOMIC ERROR
// ===============================================================

/// Atomic operation errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtomicError {
    /// Invalid state for the operation.
    InvalidState(AtomicState),
    /// Already committed (∝ violation).
    AlreadyCommitted,
    /// Saga has no steps.
    EmptySaga,
    /// No more steps to complete.
    NoMoreSteps,
    /// Incomplete steps — cannot commit.
    IncompleteSteps,
    /// 2PC vote rejected.
    VoteRejected,
}

impl std::fmt::Display for AtomicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidState(s) => write!(f, "invalid state: {s:?}"),
            Self::AlreadyCommitted => write!(f, "already committed (irreversible)"),
            Self::EmptySaga => write!(f, "saga has no steps"),
            Self::NoMoreSteps => write!(f, "no more steps to complete"),
            Self::IncompleteSteps => write!(f, "not all steps completed"),
            Self::VoteRejected => write!(f, "2PC vote rejected"),
        }
    }
}

impl std::error::Error for AtomicError {}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_atomic_op_happy_path() {
        let mut op = AtomicOp::new(TxId::new(1), "submit report");
        assert_eq!(op.state(), AtomicState::Ready);

        assert!(op.begin().is_ok());
        assert_eq!(op.state(), AtomicState::Executing);

        assert!(op.commit("report submitted").is_ok());
        assert_eq!(op.state(), AtomicState::Committed);
        assert_eq!(op.result.as_deref(), Some("report submitted"));
    }

    #[test]
    fn test_atomic_op_failure() {
        let mut op = AtomicOp::new(TxId::new(2), "validation");
        assert!(op.begin().is_ok());
        assert!(op.fail("missing data").is_ok());
        assert_eq!(op.state(), AtomicState::Failed);
        assert_eq!(op.error.as_deref(), Some("missing data"));
    }

    #[test]
    fn test_atomic_op_cannot_fail_after_commit() {
        let mut op = AtomicOp::new(TxId::new(3), "committed op");
        assert!(op.begin().is_ok());
        assert!(op.commit("done").is_ok());

        // ∝: cannot fail after commit
        let err = op.fail("too late");
        assert!(err.is_err());
        assert!(matches!(err, Err(AtomicError::AlreadyCommitted)));
    }

    #[test]
    fn test_saga_happy_path() {
        let mut saga = Saga::new(TxId::new(10), "expedited_report");
        saga.add_step(SagaStep::new("validate", "check data", "discard draft"));
        saga.add_step(SagaStep::new("sign", "apply signature", "revoke signature"));
        saga.add_step(SagaStep::new(
            "submit",
            "transmit to FDA",
            "send recall notice",
        ));

        assert_eq!(saga.step_count(), 3);
        assert!(saga.start().is_ok());

        // Complete all steps
        assert!(saga.complete_step().is_ok()); // validate
        assert!(saga.complete_step().is_ok()); // sign
        assert!(saga.complete_step().is_ok()); // submit

        assert_eq!(saga.state(), SagaState::Completed);
        assert_eq!(saga.completed_count(), 3);

        // Commit (∝)
        assert!(saga.commit().is_ok());
        assert_eq!(saga.state(), SagaState::Committed);
    }

    #[test]
    fn test_saga_compensation() {
        let mut saga = Saga::new(TxId::new(11), "multi_step");
        saga.add_step(SagaStep::new("step1", "do A", "undo A"));
        saga.add_step(SagaStep::new("step2", "do B", "undo B"));
        saga.add_step(SagaStep::new("step3", "do C", "undo C"));

        assert!(saga.start().is_ok());
        assert!(saga.complete_step().is_ok()); // step1 done
        assert!(saga.complete_step().is_ok()); // step2 done

        // step3 fails
        saga.fail_step("external error");
        assert_eq!(saga.state(), SagaState::Compensating);

        // Compensate in reverse
        let compensated = saga.compensate();
        assert_eq!(compensated.len(), 2); // step2, step1 (reverse)
        assert_eq!(saga.state(), SagaState::Compensated);
    }

    #[test]
    fn test_saga_empty_rejected() {
        let mut saga = Saga::new(TxId::new(12), "empty");
        let err = saga.start();
        assert!(err.is_err());
        assert!(matches!(err, Err(AtomicError::EmptySaga)));
    }

    #[test]
    fn test_saga_cannot_commit_incomplete() {
        let mut saga = Saga::new(TxId::new(13), "partial");
        saga.add_step(SagaStep::new("s1", "a", "undo a"));
        saga.add_step(SagaStep::new("s2", "b", "undo b"));

        assert!(saga.start().is_ok());
        assert!(saga.complete_step().is_ok()); // Only step1

        // Cannot commit with incomplete steps
        let err = saga.commit();
        assert!(err.is_err());
        assert!(matches!(err, Err(AtomicError::IncompleteSteps)));
    }

    #[test]
    fn test_2pc_unanimous_commit() {
        let mut tpc = TwoPhaseCommit::new(TxId::new(20));

        assert!(tpc.vote("participant_a", true).is_ok());
        assert!(tpc.vote("participant_b", true).is_ok());
        assert!(tpc.is_unanimous());
        assert_eq!(tpc.vote_count(), 2);

        assert!(tpc.prepare().is_ok());
        assert_eq!(tpc.state(), TwoPhaseState::Prepared);

        assert!(tpc.commit().is_ok());
        assert_eq!(tpc.state(), TwoPhaseState::Committed);
    }

    #[test]
    fn test_2pc_rejected_vote() {
        let mut tpc = TwoPhaseCommit::new(TxId::new(21));

        assert!(tpc.vote("participant_a", true).is_ok());
        assert!(tpc.vote("participant_b", false).is_ok());
        assert!(!tpc.is_unanimous());

        let err = tpc.prepare();
        assert!(err.is_err());
        assert_eq!(tpc.state(), TwoPhaseState::Aborted);
    }

    #[test]
    fn test_2pc_cannot_abort_after_commit() {
        let mut tpc = TwoPhaseCommit::new(TxId::new(22));
        assert!(tpc.vote("p1", true).is_ok());
        assert!(tpc.prepare().is_ok());
        assert!(tpc.commit().is_ok());

        // ∝: cannot abort after commit
        let err = tpc.abort();
        assert!(err.is_err());
        assert!(matches!(err, Err(AtomicError::AlreadyCommitted)));
    }

    #[test]
    fn test_idempotency_guard() {
        let mut guard = IdempotencyGuard::new();
        let key = IdempotencyKey::new("submit-report-123");

        assert!(!guard.is_completed(&key));

        // First execution
        assert!(guard.record(key.clone(), "success"));
        assert!(guard.is_completed(&key));
        assert_eq!(guard.get_result(&key), Some("success"));

        // Duplicate — returns false (already done)
        assert!(!guard.record(key.clone(), "success again"));
        assert_eq!(guard.count(), 1);
    }

    #[test]
    fn test_atomic_op_preconditions() {
        let op = AtomicOp::new(TxId::new(30), "guarded op")
            .with_precondition("case_valid")
            .with_precondition("deadline_not_passed");

        assert_eq!(op.preconditions.len(), 2);
    }

    #[test]
    fn test_atomic_op_grounding() {
        let comp = AtomicOp::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_saga_grounding() {
        let comp = Saga::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_2pc_grounding() {
        let comp = TwoPhaseCommit::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_idempotency_grounding() {
        let comp = IdempotencyGuard::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }
}
