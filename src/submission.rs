//! # PVTX Regulatory Submissions
//!
//! Manages regulatory filing packages with lifecycle tracking.
//! Once transmitted to a regulatory authority, a submission
//! cannot be recalled — this is the core ∝ guarantee.
//!
//! ## Primitives
//! - ∝ (Irreversibility) — DOMINANT: transmission is final
//! - σ (Sequence) — submission workflow
//! - ∂ (Boundary) — validation gates
//! - π (Persistence) — durable submission records
//! - N (Quantity) — deadline tracking
//!
//! ## Regulatory Context
//!
//! | Report Type | Deadline       | Authority |
//! |-------------|----------------|-----------|
//! | ICSR        | 15 calendar    | FDA/EMA   |
//! | PSUR        | 90 calendar    | EMA       |
//! | DSUR        | 365 calendar   | ICH       |
//! | RMP         | Per lifecycle  | EMA       |
//! | Signal      | 15 or 90 days  | Various   |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::signature::SignatureId;
use super::transaction::TxId;

// ===============================================================
// T2-P NEWTYPES
// ===============================================================

/// Unique submission identifier.
/// Tier: T2-P (∝)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubmissionId(pub u64);

impl SubmissionId {
    /// Creates a new submission ID.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SubmissionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SUB-{:08X}", self.0)
    }
}

impl GroundsTo for SubmissionId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// SUBMISSION TYPE
// ===============================================================

/// Type of regulatory report being submitted.
/// Tier: T2-P (∝)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubmissionType {
    /// Individual Case Safety Report (E2B).
    Icsr,
    /// Periodic Safety Update Report.
    Psur,
    /// Development Safety Update Report.
    Dsur,
    /// Risk Management Plan.
    Rmp,
    /// Signal assessment report.
    SignalReport,
    /// Expedited safety report (15-day).
    Expedited,
    /// Other regulatory filing.
    Other(String),
}

impl SubmissionType {
    /// Default deadline in calendar days for this submission type.
    #[must_use]
    pub fn default_deadline_days(&self) -> u64 {
        match self {
            Self::Icsr | Self::Expedited => 15,
            Self::SignalReport => 90,
            Self::Psur => 90,
            Self::Dsur => 365,
            Self::Rmp => 180,
            Self::Other(_) => 30,
        }
    }

    /// Whether this type requires human signature before submission.
    #[must_use]
    pub fn requires_human_signature(&self) -> bool {
        match self {
            Self::Icsr | Self::Expedited | Self::SignalReport => true,
            Self::Psur | Self::Dsur | Self::Rmp => true,
            Self::Other(_) => false,
        }
    }
}

impl GroundsTo for SubmissionType {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// DESTINATION
// ===============================================================

/// Regulatory authority destination.
/// Tier: T2-P (∝ + λ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubmissionDest {
    /// U.S. Food and Drug Administration.
    Fda,
    /// European Medicines Agency.
    Ema,
    /// Pharmaceuticals and Medical Devices Agency (Japan).
    Pmda,
    /// Health Canada.
    HealthCanada,
    /// Therapeutic Goods Administration (Australia).
    Tga,
    /// World Health Organization.
    Who,
    /// Multiple authorities simultaneously.
    Multi(Vec<SubmissionDest>),
    /// Custom destination.
    Other(String),
}

impl SubmissionDest {
    /// Returns the official name of the authority.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::Fda => "FDA".to_string(),
            Self::Ema => "EMA".to_string(),
            Self::Pmda => "PMDA".to_string(),
            Self::HealthCanada => "Health Canada".to_string(),
            Self::Tga => "TGA".to_string(),
            Self::Who => "WHO".to_string(),
            Self::Multi(dests) => {
                let names: Vec<String> = dests.iter().map(|d| d.name()).collect();
                names.join(", ")
            }
            Self::Other(name) => name.clone(),
        }
    }
}

impl GroundsTo for SubmissionDest {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility, LexPrimitiva::Location])
    }
}

// ===============================================================
// SUBMISSION STATE
// ===============================================================

/// Submission lifecycle state.
/// Tier: T2-P (ς + ∝)
///
/// ```text
/// Draft → Validated → Signed → Transmitted → Acknowledged
///   ↓         ↓
/// Rejected  Rejected
/// ```
///
/// Once `Transmitted`, the submission cannot be recalled (∝).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubmissionState {
    /// Initial draft, not yet validated.
    Draft,
    /// Passed validation checks.
    Validated,
    /// Required signatures applied.
    Signed,
    /// Transmitted to authority — IRREVERSIBLE (∝).
    Transmitted,
    /// Acknowledged by authority.
    Acknowledged,
    /// Rejected during validation or signing.
    Rejected,
}

impl SubmissionState {
    /// Whether the submission has been sent (∝ boundary crossed).
    #[must_use]
    pub fn is_transmitted(&self) -> bool {
        matches!(self, Self::Transmitted | Self::Acknowledged)
    }

    /// Whether the submission is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Acknowledged | Self::Rejected)
    }

    /// Whether the submission can still be modified.
    #[must_use]
    pub fn is_mutable(&self) -> bool {
        matches!(self, Self::Draft)
    }
}

impl GroundsTo for SubmissionState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State, LexPrimitiva::Irreversibility])
    }
}

// ===============================================================
// DEADLINE
// ===============================================================

/// Regulatory deadline with temporal finality.
/// Tier: T2-P (∝ + N)
///
/// Deadlines are irreversible — once passed, they cannot be extended.
/// This is a manifestation of ∝ in the temporal domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deadline {
    /// Deadline in calendar days from event.
    pub days: u64,
    /// Start timestamp (when the clock started).
    pub start_epoch: u64,
    /// Whether the deadline has been met.
    pub met: bool,
}

impl Deadline {
    /// Creates a deadline starting now.
    #[must_use]
    pub fn new(days: u64, start_epoch: u64) -> Self {
        Self {
            days,
            start_epoch,
            met: false,
        }
    }

    /// Creates a 15-day expedited deadline.
    #[must_use]
    pub fn expedited(start_epoch: u64) -> Self {
        Self::new(15, start_epoch)
    }

    /// Creates a 90-day periodic deadline.
    #[must_use]
    pub fn periodic(start_epoch: u64) -> Self {
        Self::new(90, start_epoch)
    }

    /// Epoch timestamp when the deadline expires.
    #[must_use]
    pub fn expires_at(&self) -> u64 {
        self.start_epoch + (self.days * 86400)
    }

    /// Days remaining from a given timestamp. Returns 0 if past due.
    #[must_use]
    pub fn days_remaining(&self, now_epoch: u64) -> u64 {
        let expires = self.expires_at();
        if now_epoch >= expires {
            0
        } else {
            (expires - now_epoch) / 86400
        }
    }

    /// Whether the deadline has been exceeded.
    #[must_use]
    pub fn is_overdue(&self, now_epoch: u64) -> bool {
        !self.met && now_epoch > self.expires_at()
    }

    /// Marks the deadline as met.
    pub fn mark_met(&mut self) {
        self.met = true;
    }
}

impl GroundsTo for Deadline {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Irreversibility, LexPrimitiva::Quantity])
    }
}

// ===============================================================
// SUBMISSION
// ===============================================================

/// A regulatory submission package.
/// Tier: T2-C (∝ + σ + ∂ + π + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    /// Unique submission identifier.
    pub id: SubmissionId,
    /// Associated transaction.
    pub tx_id: TxId,
    /// Submission type.
    pub submission_type: SubmissionType,
    /// Where to submit.
    pub destination: SubmissionDest,
    /// Current state.
    state: SubmissionState,
    /// Regulatory deadline.
    pub deadline: Deadline,
    /// Applied signatures.
    pub signatures: Vec<SignatureId>,
    /// Submission content hash.
    pub content_hash: u64,
    /// Created timestamp.
    pub created_at: u64,
    /// Last updated timestamp.
    pub updated_at: u64,
    /// Rejection reason (if rejected).
    pub rejection_reason: Option<String>,
    /// Acknowledgement reference (from authority).
    pub ack_reference: Option<String>,
}

impl Submission {
    /// Creates a new submission in Draft state.
    #[must_use]
    pub fn new(
        id: SubmissionId,
        tx_id: TxId,
        submission_type: SubmissionType,
        destination: SubmissionDest,
        content_hash: u64,
        now: u64,
    ) -> Self {
        let deadline_days = submission_type.default_deadline_days();
        Self {
            id,
            tx_id,
            submission_type,
            destination,
            state: SubmissionState::Draft,
            deadline: Deadline::new(deadline_days, now),
            signatures: Vec::new(),
            content_hash,
            created_at: now,
            updated_at: now,
            rejection_reason: None,
            ack_reference: None,
        }
    }

    /// Current submission state.
    #[must_use]
    pub fn state(&self) -> SubmissionState {
        self.state
    }

    /// Whether the submission has been signed.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        !self.signatures.is_empty()
    }

    /// Validates the submission (Draft → Validated).
    ///
    /// # Errors
    /// Returns `Err` if not in Draft state or content invalid.
    pub fn validate(&mut self, now: u64) -> Result<(), SubmissionError> {
        if self.state != SubmissionState::Draft {
            return Err(SubmissionError::InvalidTransition {
                from: self.state,
                to: SubmissionState::Validated,
            });
        }
        if self.content_hash == 0 {
            return Err(SubmissionError::EmptyContent);
        }
        self.state = SubmissionState::Validated;
        self.updated_at = now;
        Ok(())
    }

    /// Attaches a signature (Validated → Signed).
    ///
    /// # Errors
    /// Returns `Err` if not validated.
    pub fn attach_signature(
        &mut self,
        sig_id: SignatureId,
        now: u64,
    ) -> Result<(), SubmissionError> {
        if self.state != SubmissionState::Validated && self.state != SubmissionState::Signed {
            return Err(SubmissionError::InvalidTransition {
                from: self.state,
                to: SubmissionState::Signed,
            });
        }
        self.signatures.push(sig_id);
        self.state = SubmissionState::Signed;
        self.updated_at = now;
        Ok(())
    }

    /// Transmits to the authority — IRREVERSIBLE (∝).
    ///
    /// # Errors
    /// Returns `Err` if not signed or if human signature required but missing.
    pub fn transmit(&mut self, now: u64) -> Result<(), SubmissionError> {
        if self.state != SubmissionState::Signed {
            return Err(SubmissionError::InvalidTransition {
                from: self.state,
                to: SubmissionState::Transmitted,
            });
        }
        if self.submission_type.requires_human_signature() && self.signatures.is_empty() {
            return Err(SubmissionError::SignatureRequired);
        }
        // === POINT OF NO RETURN (∝) ===
        self.state = SubmissionState::Transmitted;
        self.deadline.mark_met();
        self.updated_at = now;
        Ok(())
    }

    /// Records acknowledgement from the authority.
    ///
    /// # Errors
    /// Returns `Err` if not transmitted.
    pub fn acknowledge(&mut self, reference: &str, now: u64) -> Result<(), SubmissionError> {
        if self.state != SubmissionState::Transmitted {
            return Err(SubmissionError::InvalidTransition {
                from: self.state,
                to: SubmissionState::Acknowledged,
            });
        }
        self.state = SubmissionState::Acknowledged;
        self.ack_reference = Some(reference.to_string());
        self.updated_at = now;
        Ok(())
    }

    /// Rejects the submission with a reason.
    ///
    /// # Errors
    /// Returns `Err` if already transmitted (cannot reject ∝).
    pub fn reject(&mut self, reason: &str, now: u64) -> Result<(), SubmissionError> {
        if self.state.is_transmitted() {
            return Err(SubmissionError::AlreadyTransmitted(self.id));
        }
        self.state = SubmissionState::Rejected;
        self.rejection_reason = Some(reason.to_string());
        self.updated_at = now;
        Ok(())
    }
}

impl GroundsTo for Submission {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — transmission is final
            LexPrimitiva::Sequence,        // σ — submission workflow
            LexPrimitiva::Boundary,        // ∂ — validation gates
            LexPrimitiva::Persistence,     // π — permanent record
            LexPrimitiva::Quantity,        // N — deadlines
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.85)
    }
}

// ===============================================================
// SUBMISSION QUEUE
// ===============================================================

/// Queue of pending and completed submissions.
/// Tier: T2-C (∝ + σ + π)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubmissionQueue {
    /// All submissions (append-only for completed).
    submissions: Vec<Submission>,
    /// Next submission ID.
    next_id: u64,
}

impl SubmissionQueue {
    /// Creates a new empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            submissions: Vec::new(),
            next_id: 1,
        }
    }

    /// Creates a new submission and adds it to the queue.
    pub fn create(
        &mut self,
        tx_id: TxId,
        submission_type: SubmissionType,
        destination: SubmissionDest,
        content_hash: u64,
        now: u64,
    ) -> SubmissionId {
        let id = SubmissionId::new(self.next_id);
        self.next_id += 1;
        let sub = Submission::new(id, tx_id, submission_type, destination, content_hash, now);
        self.submissions.push(sub);
        id
    }

    /// Gets a submission by ID (mutable).
    #[must_use]
    pub fn get(&self, id: SubmissionId) -> Option<&Submission> {
        self.submissions.iter().find(|s| s.id == id)
    }

    /// Gets a mutable submission by ID.
    pub fn get_mut(&mut self, id: SubmissionId) -> Option<&mut Submission> {
        self.submissions.iter_mut().find(|s| s.id == id)
    }

    /// Returns all submissions for a transaction.
    #[must_use]
    pub fn for_tx(&self, tx_id: TxId) -> Vec<&Submission> {
        self.submissions
            .iter()
            .filter(|s| s.tx_id == tx_id)
            .collect()
    }

    /// Returns submissions that are overdue.
    #[must_use]
    pub fn overdue(&self, now: u64) -> Vec<&Submission> {
        self.submissions
            .iter()
            .filter(|s| s.deadline.is_overdue(now) && !s.state().is_terminal())
            .collect()
    }

    /// Total submissions in queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.submissions.len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.submissions.is_empty()
    }
}

impl GroundsTo for SubmissionQueue {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Irreversibility, // ∝ — tracks transmitted
            LexPrimitiva::Sequence,        // σ — queue ordering
            LexPrimitiva::Persistence,     // π — durable records
        ])
        .with_dominant(LexPrimitiva::Irreversibility, 0.80)
    }
}

// ===============================================================
// SUBMISSION ERROR
// ===============================================================

/// Submission operation errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmissionError {
    /// Invalid state transition.
    InvalidTransition {
        from: SubmissionState,
        to: SubmissionState,
    },
    /// Submission has no content.
    EmptyContent,
    /// Required signature not attached.
    SignatureRequired,
    /// Cannot modify after transmission (∝).
    AlreadyTransmitted(SubmissionId),
    /// Submission not found.
    NotFound(SubmissionId),
}

impl std::fmt::Display for SubmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid transition: {from:?} -> {to:?}")
            }
            Self::EmptyContent => write!(f, "submission has no content"),
            Self::SignatureRequired => write!(f, "human signature required"),
            Self::AlreadyTransmitted(id) => write!(f, "cannot modify transmitted submission: {id}"),
            Self::NotFound(id) => write!(f, "submission not found: {id}"),
        }
    }
}

impl std::error::Error for SubmissionError {}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_submission() -> Submission {
        Submission::new(
            SubmissionId::new(1),
            TxId::new(100),
            SubmissionType::Icsr,
            SubmissionDest::Fda,
            0xBEEF,
            1000,
        )
    }

    #[test]
    fn test_submission_lifecycle_happy_path() {
        let mut sub = make_submission();
        assert_eq!(sub.state(), SubmissionState::Draft);

        assert!(sub.validate(1001).is_ok());
        assert_eq!(sub.state(), SubmissionState::Validated);

        assert!(sub.attach_signature(SignatureId::new(1), 1002).is_ok());
        assert_eq!(sub.state(), SubmissionState::Signed);
        assert!(sub.is_signed());

        assert!(sub.transmit(1003).is_ok());
        assert_eq!(sub.state(), SubmissionState::Transmitted);
        assert!(sub.state().is_transmitted());
        assert!(sub.deadline.met);

        assert!(sub.acknowledge("ACK-FDA-001", 1004).is_ok());
        assert_eq!(sub.state(), SubmissionState::Acknowledged);
        assert_eq!(sub.ack_reference.as_deref(), Some("ACK-FDA-001"));
    }

    #[test]
    fn test_submission_rejection() {
        let mut sub = make_submission();
        assert!(sub.validate(1001).is_ok());
        assert!(sub.reject("incomplete data", 1002).is_ok());
        assert_eq!(sub.state(), SubmissionState::Rejected);
        assert_eq!(sub.rejection_reason.as_deref(), Some("incomplete data"));
    }

    #[test]
    fn test_cannot_reject_transmitted() {
        let mut sub = make_submission();
        assert!(sub.validate(1001).is_ok());
        assert!(sub.attach_signature(SignatureId::new(1), 1002).is_ok());
        assert!(sub.transmit(1003).is_ok());

        // ∝: cannot reject after transmission
        let err = sub.reject("too late", 1004);
        assert!(err.is_err());
        if let Err(SubmissionError::AlreadyTransmitted(id)) = err {
            assert_eq!(id, SubmissionId::new(1));
        }
    }

    #[test]
    fn test_submission_empty_content() {
        let mut sub = Submission::new(
            SubmissionId::new(2),
            TxId::new(200),
            SubmissionType::Psur,
            SubmissionDest::Ema,
            0, // Empty!
            1000,
        );

        let err = sub.validate(1001);
        assert!(err.is_err());
        assert!(matches!(err, Err(SubmissionError::EmptyContent)));
    }

    #[test]
    fn test_deadline_expedited() {
        let d = Deadline::expedited(1_000_000);
        assert_eq!(d.days, 15);
        assert_eq!(d.expires_at(), 1_000_000 + (15 * 86400));
        assert_eq!(d.days_remaining(1_000_000), 15);
        assert!(!d.is_overdue(1_000_000));

        // After 16 days
        let sixteen_days = 1_000_000 + (16 * 86400);
        assert_eq!(d.days_remaining(sixteen_days), 0);
        assert!(d.is_overdue(sixteen_days));
    }

    #[test]
    fn test_deadline_met() {
        let mut d = Deadline::expedited(1_000_000);
        d.mark_met();
        // Once met, not overdue even if past deadline
        assert!(!d.is_overdue(1_000_000 + (20 * 86400)));
    }

    #[test]
    fn test_submission_type_deadlines() {
        assert_eq!(SubmissionType::Icsr.default_deadline_days(), 15);
        assert_eq!(SubmissionType::Expedited.default_deadline_days(), 15);
        assert_eq!(SubmissionType::Psur.default_deadline_days(), 90);
        assert_eq!(SubmissionType::Dsur.default_deadline_days(), 365);
    }

    #[test]
    fn test_destination_names() {
        assert_eq!(SubmissionDest::Fda.name(), "FDA");
        assert_eq!(SubmissionDest::Ema.name(), "EMA");
        assert_eq!(
            SubmissionDest::Multi(vec![SubmissionDest::Fda, SubmissionDest::Ema]).name(),
            "FDA, EMA"
        );
    }

    #[test]
    fn test_submission_queue() {
        let mut queue = SubmissionQueue::new();
        assert!(queue.is_empty());

        let id = queue.create(
            TxId::new(1),
            SubmissionType::Icsr,
            SubmissionDest::Fda,
            0xCAFE,
            1000,
        );

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let sub = queue.get(id);
        assert!(sub.is_some());
    }

    #[test]
    fn test_submission_queue_overdue() {
        let mut queue = SubmissionQueue::new();
        queue.create(
            TxId::new(1),
            SubmissionType::Expedited,
            SubmissionDest::Fda,
            0xBEEF,
            1000,
        );

        // Not overdue initially
        assert!(queue.overdue(1000).is_empty());

        // Overdue after 16 days
        let overdue_time = 1000 + (16 * 86400);
        assert_eq!(queue.overdue(overdue_time).len(), 1);
    }

    #[test]
    fn test_submission_state_properties() {
        assert!(!SubmissionState::Draft.is_transmitted());
        assert!(SubmissionState::Transmitted.is_transmitted());
        assert!(SubmissionState::Acknowledged.is_transmitted());

        assert!(SubmissionState::Draft.is_mutable());
        assert!(!SubmissionState::Validated.is_mutable());
    }

    #[test]
    fn test_submission_grounding() {
        let comp = Submission::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
    }

    #[test]
    fn test_deadline_grounding() {
        let comp = Deadline::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
