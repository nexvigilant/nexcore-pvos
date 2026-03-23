//! # PV∅ Error Handling Patterns
//!
//! Unified error types with void semantics and recovery strategies.
//! Every error is an absence of expected success — a void where a
//! value should have been.
//!
//! ## Primitives
//! - ∅ (Void) — DOMINANT: errors represent absent success
//! - → (Causality) — error chains trace root cause
//! - ∂ (Boundary) — error classification boundaries
//! - ς (State) — recovery state transitions
//!
//! ## Design
//!
//! Rather than panicking or silently ignoring errors, PV∅ provides
//! structured recovery paths: retry, skip, escalate, or abort.
//! Every error carries enough context for autonomous recovery.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// ERROR KIND
// ===============================================================

/// Classification of PV errors.
/// Tier: T2-P (∅ + ∂)
///
/// Each kind maps to different recovery strategies:
/// - `Missing` → default or request data
/// - `Invalid` → reject and re-request
/// - `Timeout` → retry with backoff
/// - `Rejected` → escalate to human
/// - `System` → alert operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorKind {
    /// Required data is missing (void where value expected).
    Missing,
    /// Data is present but invalid (wrong format, out of range).
    Invalid,
    /// Operation exceeded time limit.
    Timeout,
    /// Operation was rejected by policy or authorization.
    Rejected,
    /// System-level failure (infrastructure, connectivity).
    System,
}

impl ErrorKind {
    /// Returns true if the error is likely transient and retryable.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Timeout | Self::System)
    }

    /// Returns true if the error requires human intervention.
    #[must_use]
    pub fn requires_human(&self) -> bool {
        matches!(self, Self::Rejected)
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(f, "missing"),
            Self::Invalid => write!(f, "invalid"),
            Self::Timeout => write!(f, "timeout"),
            Self::Rejected => write!(f, "rejected"),
            Self::System => write!(f, "system"),
        }
    }
}

impl GroundsTo for ErrorKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,     // ∅ — error is absence of success
            LexPrimitiva::Boundary, // ∂ — classification boundary
        ])
        .with_dominant(LexPrimitiva::Void, 0.85)
    }
}

// ===============================================================
// RECOVERY STRATEGY
// ===============================================================

/// Recommended recovery action for an error.
/// Tier: T2-P (∅ + ς)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Recovery {
    /// Retry the operation (for transient errors).
    Retry,
    /// Skip this item and continue processing.
    Skip,
    /// Use a default value instead.
    Default,
    /// Escalate to human reviewer.
    Escalate,
    /// Abort the entire operation.
    Abort,
}

impl Recovery {
    /// Returns true if this recovery continues processing.
    #[must_use]
    pub fn continues_processing(&self) -> bool {
        matches!(self, Self::Retry | Self::Skip | Self::Default)
    }

    /// Returns true if this recovery stops processing.
    #[must_use]
    pub fn stops_processing(&self) -> bool {
        matches!(self, Self::Escalate | Self::Abort)
    }
}

impl GroundsTo for Recovery {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,  // ∅ — recovering from void
            LexPrimitiva::State, // ς — transition to recovery state
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// PV ERROR
// ===============================================================

/// A pharmacovigilance error with rich context for recovery.
/// Tier: T2-C (∅ + → + ∂ + ς)
///
/// Carries enough information for automated recovery decisions:
/// - What kind of error occurred
/// - Where it happened (context)
/// - What caused it (optional cause chain)
/// - How many times it has been retried
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PvError {
    /// Classification of the error.
    pub kind: ErrorKind,
    /// Human-readable error message.
    pub message: String,
    /// Context where the error occurred (e.g., "signal_detection", "icsr_ingestion").
    pub context: String,
    /// Unique error code for programmatic handling.
    pub code: String,
    /// Number of retry attempts so far.
    pub retry_count: u32,
    /// Maximum retries allowed for this error type.
    pub max_retries: u32,
    /// Timestamp of the error (epoch seconds).
    pub timestamp: u64,
}

impl PvError {
    /// Creates a new PV error.
    #[must_use]
    pub fn new(kind: ErrorKind, message: &str, context: &str, now: u64) -> Self {
        let code = format!("PV-{}", kind);
        Self {
            kind,
            message: message.to_string(),
            context: context.to_string(),
            code,
            retry_count: 0,
            max_retries: 3,
            timestamp: now,
        }
    }

    /// Creates a Missing error.
    #[must_use]
    pub fn missing(message: &str, context: &str, now: u64) -> Self {
        Self::new(ErrorKind::Missing, message, context, now)
    }

    /// Creates an Invalid error.
    #[must_use]
    pub fn invalid(message: &str, context: &str, now: u64) -> Self {
        Self::new(ErrorKind::Invalid, message, context, now)
    }

    /// Creates a Timeout error.
    #[must_use]
    pub fn timeout(message: &str, context: &str, now: u64) -> Self {
        Self::new(ErrorKind::Timeout, message, context, now)
    }

    /// Creates a Rejected error.
    #[must_use]
    pub fn rejected(message: &str, context: &str, now: u64) -> Self {
        Self::new(ErrorKind::Rejected, message, context, now)
    }

    /// Creates a System error.
    #[must_use]
    pub fn system(message: &str, context: &str, now: u64) -> Self {
        Self::new(ErrorKind::System, message, context, now)
    }

    /// Returns true if the error can be retried.
    #[must_use]
    pub fn can_retry(&self) -> bool {
        self.kind.is_transient() && self.retry_count < self.max_retries
    }

    /// Increments the retry counter.
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Sets custom max retries.
    #[must_use]
    pub fn with_max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Recommends a recovery strategy based on error state.
    #[must_use]
    pub fn recommend_recovery(&self) -> Recovery {
        match &self.kind {
            ErrorKind::Missing => Recovery::Default,
            ErrorKind::Invalid => Recovery::Skip,
            ErrorKind::Timeout if self.can_retry() => Recovery::Retry,
            ErrorKind::Timeout => Recovery::Abort,
            ErrorKind::Rejected => Recovery::Escalate,
            ErrorKind::System if self.can_retry() => Recovery::Retry,
            ErrorKind::System => Recovery::Abort,
        }
    }
}

impl std::fmt::Display for PvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} in {}: {}",
            self.code, self.kind, self.context, self.message
        )
    }
}

impl std::error::Error for PvError {}

impl GroundsTo for PvError {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,      // ∅ — error is absent success
            LexPrimitiva::Causality, // → — error chain/cause
            LexPrimitiva::Boundary,  // ∂ — error classification
            LexPrimitiva::State,     // ς — retry state
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// ERROR CHAIN
// ===============================================================

/// A chain of errors tracing back to the root cause.
/// Tier: T2-C (∅ + → + σ + N)
///
/// When an error cascades through layers (detection → triage → report),
/// the chain preserves the full causal history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorChain {
    /// The most recent (surface) error.
    head: PvError,
    /// Previous errors in causal order (oldest first).
    causes: Vec<PvError>,
}

impl ErrorChain {
    /// Creates a new chain with a single error.
    #[must_use]
    pub fn new(error: PvError) -> Self {
        Self {
            head: error,
            causes: Vec::new(),
        }
    }

    /// Wraps the current chain with a new surface error.
    #[must_use]
    pub fn wrap(mut self, new_error: PvError) -> Self {
        let old_head = self.head;
        self.causes.push(old_head);
        self.head = new_error;
        self
    }

    /// Returns the surface (most recent) error.
    #[must_use]
    pub fn head(&self) -> &PvError {
        &self.head
    }

    /// Returns the root cause (oldest error in chain).
    #[must_use]
    pub fn root_cause(&self) -> &PvError {
        self.causes.first().unwrap_or(&self.head)
    }

    /// Returns the chain depth (1 = single error, 2+ = cascaded).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.causes.len() + 1
    }

    /// Returns all errors in causal order (root → surface).
    #[must_use]
    pub fn chain(&self) -> Vec<&PvError> {
        let mut chain: Vec<&PvError> = self.causes.iter().collect();
        chain.push(&self.head);
        chain
    }
}

impl GroundsTo for ErrorChain {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,      // ∅ — cascaded absence
            LexPrimitiva::Causality, // → — causal chain
            LexPrimitiva::Sequence,  // σ — error ordering
            LexPrimitiva::Quantity,  // N — chain depth
        ])
        .with_dominant(LexPrimitiva::Void, 0.75)
    }
}

// ===============================================================
// FALLIBLE — RESULT WITH RECOVERY
// ===============================================================

/// A result type that carries recovery information.
/// Tier: T2-C (∅ + ∃ + → + ς)
///
/// Unlike `Result<T, E>` which is binary (ok/err), `Fallible<T>`
/// has four outcomes: success, recovered (with warning), retry needed,
/// or fatal failure requiring escalation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fallible<T> {
    /// Operation succeeded with no issues.
    Success(T),
    /// Operation succeeded via recovery (default/fallback applied).
    /// Carries the recovered value and the original error.
    Recovered(T, PvError),
    /// Operation needs retry (transient failure).
    Retry(PvError),
    /// Operation failed fatally — escalate to human.
    Escalate(PvError),
    /// Operation failed — abort immediately.
    Abort(PvError),
}

impl<T> Fallible<T> {
    /// Returns true if the operation produced a value (success or recovered).
    #[must_use]
    pub fn has_value(&self) -> bool {
        matches!(self, Self::Success(_) | Self::Recovered(_, _))
    }

    /// Returns true if the operation needs retry.
    #[must_use]
    pub fn needs_retry(&self) -> bool {
        matches!(self, Self::Retry(_))
    }

    /// Returns true if the operation failed fatally.
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Escalate(_) | Self::Abort(_))
    }

    /// Extracts the value if present, regardless of recovery status.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        match self {
            Self::Success(v) | Self::Recovered(v, _) => Some(v),
            Self::Retry(_) | Self::Escalate(_) | Self::Abort(_) => None,
        }
    }

    /// Extracts the error, if any.
    #[must_use]
    pub fn error(&self) -> Option<&PvError> {
        match self {
            Self::Success(_) => None,
            Self::Recovered(_, e) | Self::Retry(e) | Self::Escalate(e) | Self::Abort(e) => Some(e),
        }
    }

    /// Maps the value, preserving the fallible status.
    #[must_use]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Fallible<U> {
        match self {
            Self::Success(v) => Fallible::Success(f(v)),
            Self::Recovered(v, e) => Fallible::Recovered(f(v), e),
            Self::Retry(e) => Fallible::Retry(e),
            Self::Escalate(e) => Fallible::Escalate(e),
            Self::Abort(e) => Fallible::Abort(e),
        }
    }
}

impl<T: Clone> GroundsTo for Fallible<T> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,      // ∅ — failure branches
            LexPrimitiva::Existence, // ∃ — success branch
            LexPrimitiva::Causality, // → — error causation
            LexPrimitiva::State,     // ς — recovery state
        ])
        .with_dominant(LexPrimitiva::Void, 0.75)
    }
}

// ===============================================================
// ERROR HANDLER
// ===============================================================

/// Handles errors with configurable recovery strategies.
/// Tier: T3 (∅ + → + ∂ + ς + κ + σ)
///
/// Retry policy engine maintaining recovery statistics and
/// adaptive error recovery across the PVOS stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEngine {
    /// Default max retries per error kind.
    retry_limits: Vec<(ErrorKind, u32)>,
    /// Total errors handled.
    total_handled: u64,
    /// Errors by kind.
    by_kind: Vec<(ErrorKind, u64)>,
    /// Recoveries applied.
    recoveries_applied: u64,
}

impl RecoveryEngine {
    /// Creates a new error handler with default retry limits.
    #[must_use]
    pub fn new() -> Self {
        Self {
            retry_limits: vec![
                (ErrorKind::Timeout, 3),
                (ErrorKind::System, 2),
                (ErrorKind::Missing, 0),
                (ErrorKind::Invalid, 0),
                (ErrorKind::Rejected, 0),
            ],
            total_handled: 0,
            by_kind: Vec::new(),
            recoveries_applied: 0,
        }
    }

    /// Handles an error and returns the recommended recovery.
    pub fn handle(&mut self, error: &PvError) -> Recovery {
        self.total_handled += 1;

        // Update by-kind counter
        if let Some(entry) = self.by_kind.iter_mut().find(|(k, _)| *k == error.kind) {
            entry.1 += 1;
        } else {
            self.by_kind.push((error.kind.clone(), 1));
        }

        let recovery = error.recommend_recovery();
        if recovery.continues_processing() {
            self.recoveries_applied += 1;
        }

        recovery
    }

    /// Returns the total number of errors handled.
    #[must_use]
    pub fn total_handled(&self) -> u64 {
        self.total_handled
    }

    /// Returns the number of successful recoveries.
    #[must_use]
    pub fn recoveries_applied(&self) -> u64 {
        self.recoveries_applied
    }

    /// Returns the recovery rate (recoveries / total).
    #[must_use]
    pub fn recovery_rate(&self) -> f64 {
        if self.total_handled == 0 {
            return 0.0;
        }
        self.recoveries_applied as f64 / self.total_handled as f64
    }

    /// Returns the max retry limit for an error kind.
    #[must_use]
    pub fn retry_limit(&self, kind: &ErrorKind) -> u32 {
        self.retry_limits
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, limit)| *limit)
            .unwrap_or(0)
    }
}

impl Default for RecoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for RecoveryEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — error handling
            LexPrimitiva::Causality,  // → — error chains
            LexPrimitiva::Boundary,   // ∂ — kind classification
            LexPrimitiva::State,      // ς — recovery state
            LexPrimitiva::Comparison, // κ — threshold checks
            LexPrimitiva::Sequence,   // σ — retry sequence
        ])
        .with_dominant(LexPrimitiva::Void, 0.75)
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    // --- Grounding tests ---

    #[test]
    fn test_error_kind_grounding() {
        let comp = ErrorKind::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_recovery_grounding() {
        let comp = Recovery::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_pv_error_grounding() {
        let comp = PvError::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_error_chain_grounding() {
        let comp = ErrorChain::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_fallible_grounding() {
        let comp = Fallible::<String>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_error_handler_grounding() {
        let comp = RecoveryEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    // --- PvError tests ---

    #[test]
    fn test_error_creation() {
        let err = PvError::missing("drug name is absent", "icsr_ingestion", 1000);
        assert_eq!(err.kind, ErrorKind::Missing);
        assert_eq!(err.context, "icsr_ingestion");
        assert_eq!(err.retry_count, 0);
    }

    #[test]
    fn test_error_can_retry() {
        let timeout = PvError::timeout("connection timed out", "api_call", 1000);
        assert!(timeout.can_retry());

        let missing = PvError::missing("field missing", "icsr", 1000);
        assert!(!missing.can_retry());
    }

    #[test]
    fn test_error_retry_exhaustion() {
        let mut err = PvError::timeout("timeout", "api", 1000).with_max_retries(2);
        assert!(err.can_retry());
        err.record_retry();
        assert!(err.can_retry());
        err.record_retry();
        assert!(!err.can_retry());
    }

    #[test]
    fn test_error_recovery_recommendation() {
        assert_eq!(
            PvError::missing("x", "ctx", 0).recommend_recovery(),
            Recovery::Default
        );
        assert_eq!(
            PvError::invalid("x", "ctx", 0).recommend_recovery(),
            Recovery::Skip
        );
        assert_eq!(
            PvError::timeout("x", "ctx", 0).recommend_recovery(),
            Recovery::Retry
        );
        assert_eq!(
            PvError::rejected("x", "ctx", 0).recommend_recovery(),
            Recovery::Escalate
        );
    }

    #[test]
    fn test_error_display() {
        let err = PvError::missing("drug name", "icsr", 1000);
        let display = format!("{err}");
        assert!(display.contains("missing"));
        assert!(display.contains("icsr"));
        assert!(display.contains("drug name"));
    }

    // --- ErrorChain tests ---

    #[test]
    fn test_error_chain_single() {
        let chain = ErrorChain::new(PvError::missing("root", "ctx", 1000));
        assert_eq!(chain.depth(), 1);
        assert_eq!(chain.head().message, "root");
        assert_eq!(chain.root_cause().message, "root");
    }

    #[test]
    fn test_error_chain_wrapped() {
        let chain = ErrorChain::new(PvError::system("db connection failed", "storage", 1000))
            .wrap(PvError::timeout("query timed out", "detection", 1001))
            .wrap(PvError::missing("signal not computed", "report", 1002));

        assert_eq!(chain.depth(), 3);
        assert_eq!(chain.head().message, "signal not computed");
        assert_eq!(chain.root_cause().message, "db connection failed");

        let full_chain = chain.chain();
        assert_eq!(full_chain.len(), 3);
        assert_eq!(full_chain[0].message, "db connection failed");
        assert_eq!(full_chain[2].message, "signal not computed");
    }

    // --- Fallible tests ---

    #[test]
    fn test_fallible_success() {
        let f: Fallible<u32> = Fallible::Success(42);
        assert!(f.has_value());
        assert!(!f.needs_retry());
        assert!(!f.is_fatal());
        assert!(f.error().is_none());
        assert_eq!(f.into_value(), Some(42));
    }

    #[test]
    fn test_fallible_recovered() {
        let err = PvError::missing("age", "icsr", 1000);
        let f: Fallible<u32> = Fallible::Recovered(0, err);
        assert!(f.has_value());
        assert!(f.error().is_some());
        assert_eq!(f.into_value(), Some(0));
    }

    #[test]
    fn test_fallible_retry() {
        let err = PvError::timeout("slow", "api", 1000);
        let f: Fallible<u32> = Fallible::Retry(err);
        assert!(!f.has_value());
        assert!(f.needs_retry());
        assert!(!f.is_fatal());
        assert_eq!(f.into_value(), None);
    }

    #[test]
    fn test_fallible_abort() {
        let err = PvError::system("crash", "kernel", 1000);
        let f: Fallible<u32> = Fallible::Abort(err);
        assert!(f.is_fatal());
        assert!(!f.has_value());
    }

    #[test]
    fn test_fallible_map() {
        let f: Fallible<u32> = Fallible::Success(10);
        let doubled = f.map(|v| v * 2);
        assert_eq!(doubled.into_value(), Some(20));
    }

    // --- RecoveryEngine tests ---

    #[test]
    fn test_error_handler_handles() {
        let mut handler = RecoveryEngine::new();
        let err = PvError::missing("field", "ctx", 1000);
        let recovery = handler.handle(&err);
        assert_eq!(recovery, Recovery::Default);
        assert_eq!(handler.total_handled(), 1);
        assert_eq!(handler.recoveries_applied(), 1);
    }

    #[test]
    fn test_error_handler_recovery_rate() {
        let mut handler = RecoveryEngine::new();

        handler.handle(&PvError::missing("a", "ctx", 0)); // Default → continues
        handler.handle(&PvError::rejected("b", "ctx", 0)); // Escalate → stops

        assert!((handler.recovery_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_error_handler_retry_limits() {
        let handler = RecoveryEngine::new();
        assert_eq!(handler.retry_limit(&ErrorKind::Timeout), 3);
        assert_eq!(handler.retry_limit(&ErrorKind::Missing), 0);
    }

    // --- ErrorKind tests ---

    #[test]
    fn test_error_kind_properties() {
        assert!(ErrorKind::Timeout.is_transient());
        assert!(ErrorKind::System.is_transient());
        assert!(!ErrorKind::Missing.is_transient());
        assert!(!ErrorKind::Invalid.is_transient());

        assert!(ErrorKind::Rejected.requires_human());
        assert!(!ErrorKind::Timeout.requires_human());
    }

    // --- Recovery tests ---

    #[test]
    fn test_recovery_processing_properties() {
        assert!(Recovery::Retry.continues_processing());
        assert!(Recovery::Skip.continues_processing());
        assert!(Recovery::Default.continues_processing());
        assert!(!Recovery::Escalate.continues_processing());
        assert!(!Recovery::Abort.continues_processing());

        assert!(Recovery::Escalate.stops_processing());
        assert!(Recovery::Abort.stops_processing());
        assert!(!Recovery::Retry.stops_processing());
    }
}
