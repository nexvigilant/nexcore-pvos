//! # PVML Feedback Loop
//!
//! Captures prediction-outcome pairs, buffers them, and triggers
//! learning when sufficient feedback accumulates.
//!
//! The feedback loop is the fundamental ρ-primitive: outcomes from
//! detection (κ) flow back to improve future detection.
//!
//! ## Primitives
//! - ρ (Recursion) — the loop itself: outcome → model → better outcome
//! - κ (Comparison) — predicted vs actual
//! - σ (Sequence) — ordered feedback history
//! - → (Causality) — attribution: which prediction caused which outcome

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// FEEDBACK TYPES
// ===============================================================

/// Unique feedback entry identifier.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeedbackId(pub u64);

impl GroundsTo for FeedbackId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

/// Source of the outcome observation.
/// Tier: T2-P (→)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutcomeSource {
    /// Human expert review.
    Human(String),
    /// Downstream system observation.
    Downstream(String),
    /// Temporal: outcome became clear over time.
    Temporal,
    /// Automated validation.
    Automated,
}

impl GroundsTo for OutcomeSource {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

/// The actual outcome of a prediction.
/// Tier: T2-P (κ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    /// Prediction was correct (true positive or true negative).
    Confirmed,
    /// Prediction was wrong — false positive.
    FalsePositive,
    /// Prediction was wrong — false negative.
    FalseNegative,
    /// Outcome is still unknown.
    Pending,
    /// Outcome cannot be determined.
    Indeterminate,
}

impl Outcome {
    /// Whether this outcome indicates an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::FalsePositive | Self::FalseNegative)
    }

    /// Whether this outcome is resolved (not pending/indeterminate).
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        matches!(
            self,
            Self::Confirmed | Self::FalsePositive | Self::FalseNegative
        )
    }
}

impl GroundsTo for Outcome {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
    }
}

/// Attribution linking an outcome to its originating prediction.
/// Tier: T2-P (→ + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    /// What drug-event pair was predicted.
    pub drug: String,
    /// Event name.
    pub event: String,
    /// Algorithm used for prediction.
    pub algorithm: String,
    /// Original prediction statistic.
    pub predicted_statistic: f64,
    /// Whether the prediction signaled positive.
    pub predicted_positive: bool,
}

impl Attribution {
    /// Creates a direct attribution from prediction fields.
    #[must_use]
    pub fn new(drug: &str, event: &str, algorithm: &str, statistic: f64, positive: bool) -> Self {
        Self {
            drug: drug.to_string(),
            event: event.to_string(),
            algorithm: algorithm.to_string(),
            predicted_statistic: statistic,
            predicted_positive: positive,
        }
    }
}

impl GroundsTo for Attribution {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality, LexPrimitiva::Comparison])
    }
}

/// A single feedback entry pairing prediction with outcome.
/// Tier: T2-C (ρ + κ + → + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    /// Unique identifier.
    pub id: FeedbackId,
    /// Attribution to the original prediction.
    pub attribution: Attribution,
    /// Observed outcome.
    pub outcome: Outcome,
    /// Source of the outcome.
    pub source: OutcomeSource,
    /// When the feedback was recorded.
    pub recorded_at: SystemTime,
}

impl Feedback {
    /// Creates new feedback.
    #[must_use]
    pub fn new(
        id: FeedbackId,
        attribution: Attribution,
        outcome: Outcome,
        source: OutcomeSource,
    ) -> Self {
        Self {
            id,
            attribution,
            outcome,
            source,
            recorded_at: SystemTime::now(),
        }
    }
}

impl GroundsTo for Feedback {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Comparison,
            LexPrimitiva::Causality,
            LexPrimitiva::Sequence,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.75)
    }
}

// ===============================================================
// FEEDBACK METRICS
// ===============================================================

/// Running metrics computed from feedback.
/// Tier: T2-P (N + κ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackMetrics {
    /// Total feedback entries.
    pub total: u64,
    /// True positives + true negatives.
    pub confirmed: u64,
    /// False positives.
    pub false_positives: u64,
    /// False negatives.
    pub false_negatives: u64,
    /// Pending outcomes.
    pub pending: u64,
}

impl FeedbackMetrics {
    /// Accuracy: confirmed / (confirmed + errors).
    #[must_use]
    pub fn accuracy(&self) -> f64 {
        let resolved = self.confirmed + self.false_positives + self.false_negatives;
        if resolved == 0 {
            return 0.0;
        }
        self.confirmed as f64 / resolved as f64
    }

    /// False positive rate: FP / (FP + confirmed).
    #[must_use]
    pub fn false_positive_rate(&self) -> f64 {
        let denom = self.false_positives + self.confirmed;
        if denom == 0 {
            return 0.0;
        }
        self.false_positives as f64 / denom as f64
    }

    /// False negative rate: FN / (FN + confirmed).
    #[must_use]
    pub fn false_negative_rate(&self) -> f64 {
        let denom = self.false_negatives + self.confirmed;
        if denom == 0 {
            return 0.0;
        }
        self.false_negatives as f64 / denom as f64
    }

    /// Error rate: (FP + FN) / total resolved.
    #[must_use]
    pub fn error_rate(&self) -> f64 {
        let resolved = self.confirmed + self.false_positives + self.false_negatives;
        if resolved == 0 {
            return 0.0;
        }
        (self.false_positives + self.false_negatives) as f64 / resolved as f64
    }

    fn record(&mut self, outcome: &Outcome) {
        self.total += 1;
        match outcome {
            Outcome::Confirmed => self.confirmed += 1,
            Outcome::FalsePositive => self.false_positives += 1,
            Outcome::FalseNegative => self.false_negatives += 1,
            Outcome::Pending => self.pending += 1,
            Outcome::Indeterminate => {}
        }
    }
}

impl GroundsTo for FeedbackMetrics {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Comparison])
    }
}

// ===============================================================
// FEEDBACK LOOP
// ===============================================================

/// The feedback loop — buffers feedback and triggers learning.
/// Tier: T2-C (ρ + κ + σ + →)
///
/// This is the core ρ-structure: outcomes flow back to improve
/// future predictions. When enough feedback accumulates, learning
/// is triggered.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackLoop {
    /// Buffered feedback entries.
    buffer: Vec<Feedback>,
    /// All-time feedback history (post-learning).
    history: Vec<Feedback>,
    /// Running metrics.
    metrics: FeedbackMetrics,
    /// Next feedback ID.
    next_id: u64,
    /// Batch size before triggering learning.
    batch_size: usize,
    /// Total learning triggers.
    learning_triggers: u64,
}

impl FeedbackLoop {
    /// Creates a new feedback loop with given batch size.
    #[must_use]
    pub fn new(batch_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            history: Vec::new(),
            metrics: FeedbackMetrics::default(),
            next_id: 1,
            batch_size: batch_size.max(1),
            learning_triggers: 0,
        }
    }

    /// Records a feedback entry.
    /// Returns `true` if learning should be triggered (batch full).
    pub fn record(
        &mut self,
        attribution: Attribution,
        outcome: Outcome,
        source: OutcomeSource,
    ) -> bool {
        let id = FeedbackId(self.next_id);
        self.next_id += 1;

        let feedback = Feedback::new(id, attribution, outcome, source);
        self.metrics.record(&feedback.outcome);
        self.buffer.push(feedback);

        self.buffer.len() >= self.batch_size
    }

    /// Drains the buffer for learning consumption.
    /// Returns the batch and increments trigger count.
    pub fn drain_batch(&mut self) -> Vec<Feedback> {
        self.learning_triggers += 1;
        let batch: Vec<Feedback> = self.buffer.drain(..).collect();
        self.history.extend(batch.clone());
        batch
    }

    /// Peeks at the current buffer without consuming.
    #[must_use]
    pub fn buffer(&self) -> &[Feedback] {
        &self.buffer
    }

    /// Current buffer size.
    #[must_use]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether the buffer has reached batch size.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= self.batch_size
    }

    /// Running metrics.
    #[must_use]
    pub fn metrics(&self) -> &FeedbackMetrics {
        &self.metrics
    }

    /// Total learning triggers.
    #[must_use]
    pub fn learning_triggers(&self) -> u64 {
        self.learning_triggers
    }

    /// Total feedback ever recorded.
    #[must_use]
    pub fn total_feedback(&self) -> u64 {
        self.metrics.total
    }

    /// History of all consumed feedback.
    #[must_use]
    pub fn history(&self) -> &[Feedback] {
        &self.history
    }
}

impl GroundsTo for FeedbackLoop {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Comparison,
            LexPrimitiva::Sequence,
            LexPrimitiva::Causality,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn sample_attribution() -> Attribution {
        Attribution::new("aspirin", "headache", "PRR", 3.5, true)
    }

    #[test]
    fn test_outcome_is_error() {
        assert!(!Outcome::Confirmed.is_error());
        assert!(Outcome::FalsePositive.is_error());
        assert!(Outcome::FalseNegative.is_error());
        assert!(!Outcome::Pending.is_error());
    }

    #[test]
    fn test_outcome_is_resolved() {
        assert!(Outcome::Confirmed.is_resolved());
        assert!(Outcome::FalsePositive.is_resolved());
        assert!(Outcome::FalseNegative.is_resolved());
        assert!(!Outcome::Pending.is_resolved());
        assert!(!Outcome::Indeterminate.is_resolved());
    }

    #[test]
    fn test_feedback_loop_record() {
        let mut fl = FeedbackLoop::new(3);

        let ready = fl.record(
            sample_attribution(),
            Outcome::Confirmed,
            OutcomeSource::Temporal,
        );
        assert!(!ready);
        assert_eq!(fl.buffer_len(), 1);

        fl.record(
            sample_attribution(),
            Outcome::FalsePositive,
            OutcomeSource::Temporal,
        );
        let ready = fl.record(
            sample_attribution(),
            Outcome::Confirmed,
            OutcomeSource::Temporal,
        );
        assert!(ready);
        assert!(fl.is_ready());
    }

    #[test]
    fn test_feedback_loop_drain() {
        let mut fl = FeedbackLoop::new(2);

        fl.record(
            sample_attribution(),
            Outcome::Confirmed,
            OutcomeSource::Temporal,
        );
        fl.record(
            sample_attribution(),
            Outcome::FalsePositive,
            OutcomeSource::Temporal,
        );

        let batch = fl.drain_batch();
        assert_eq!(batch.len(), 2);
        assert_eq!(fl.buffer_len(), 0);
        assert_eq!(fl.learning_triggers(), 1);
        assert_eq!(fl.history().len(), 2);
    }

    #[test]
    fn test_feedback_metrics_accuracy() {
        let mut fl = FeedbackLoop::new(100);

        for _ in 0..8 {
            fl.record(
                sample_attribution(),
                Outcome::Confirmed,
                OutcomeSource::Temporal,
            );
        }
        fl.record(
            sample_attribution(),
            Outcome::FalsePositive,
            OutcomeSource::Temporal,
        );
        fl.record(
            sample_attribution(),
            Outcome::FalseNegative,
            OutcomeSource::Temporal,
        );

        let m = fl.metrics();
        assert!((m.accuracy() - 0.8).abs() < f64::EPSILON);
        assert_eq!(m.total, 10);
    }

    #[test]
    fn test_feedback_metrics_rates() {
        let mut m = FeedbackMetrics::default();
        m.confirmed = 90;
        m.false_positives = 5;
        m.false_negatives = 5;

        assert!((m.error_rate() - 0.1).abs() < f64::EPSILON);
        assert!(m.false_positive_rate() > 0.0);
        assert!(m.false_negative_rate() > 0.0);
    }

    #[test]
    fn test_feedback_metrics_empty() {
        let m = FeedbackMetrics::default();
        assert!((m.accuracy() - 0.0).abs() < f64::EPSILON);
        assert!((m.false_positive_rate() - 0.0).abs() < f64::EPSILON);
        assert!((m.error_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_attribution_creation() {
        let a = Attribution::new("drug", "event", "algo", 2.5, true);
        assert_eq!(a.drug, "drug");
        assert!(a.predicted_positive);
    }

    #[test]
    fn test_outcome_source_variants() {
        let s1 = OutcomeSource::Human("reviewer".into());
        let s2 = OutcomeSource::Downstream("workflow".into());
        let s3 = OutcomeSource::Temporal;
        let s4 = OutcomeSource::Automated;

        // All constructible and distinct
        assert_ne!(s1, s2);
        assert_ne!(s3, s4);
    }

    #[test]
    fn test_feedback_loop_total() {
        let mut fl = FeedbackLoop::new(100);

        for _ in 0..5 {
            fl.record(
                sample_attribution(),
                Outcome::Confirmed,
                OutcomeSource::Temporal,
            );
        }

        assert_eq!(fl.total_feedback(), 5);
    }

    #[test]
    fn test_feedback_loop_grounding() {
        let comp = FeedbackLoop::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }

    #[test]
    fn test_feedback_grounding() {
        let comp = Feedback::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }
}
