//! # PVML Threshold Calibration
//!
//! Adjusts signal detection thresholds based on observed FP/FN rates.
//! The calibrator is a ρ-operation on ∂-boundaries: it recursively
//! refines thresholds to minimize error.
//!
//! ## Primitives
//! - ρ (Recursion) — iterative refinement
//! - ∂ (Boundary) — the thresholds being calibrated
//! - N (Quantity) — error rates, metrics
//! - κ (Comparison) — current vs target performance

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::feedback::{Feedback, Outcome};

// ===============================================================
// CALIBRATION TARGETS
// ===============================================================

/// What metric to optimize during calibration.
/// Tier: T2-P (N + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CalibrationTarget {
    /// Minimize false positive rate.
    MinimizeFPR,
    /// Minimize false negative rate.
    MinimizeFNR,
    /// Maximize F1 score (balance precision/recall).
    MaximizeF1,
    /// Maximize accuracy.
    MaximizeAccuracy,
    /// Target a specific FPR.
    TargetFPR(u32), // Stored as FPR * 1000 for Eq/Hash
    /// Target a specific FNR.
    TargetFNR(u32), // Stored as FNR * 1000 for Eq/Hash
}

impl GroundsTo for CalibrationTarget {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Boundary])
    }
}

/// Strategy for threshold search.
/// Tier: T2-P (ρ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CalibrationStrategy {
    /// Grid search over threshold candidates.
    Grid,
    /// Binary search (bisection) toward target.
    Bisection,
    /// Gradient-based adjustment.
    Gradient,
}

impl GroundsTo for CalibrationStrategy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Recursion])
    }
}

// ===============================================================
// THRESHOLD HISTORY
// ===============================================================

/// A single threshold change record.
/// Tier: T2-P (∂ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdChange {
    /// Algorithm this threshold applies to.
    pub algorithm: String,
    /// Previous threshold value.
    pub previous: f64,
    /// New threshold value.
    pub new: f64,
    /// Reason for the change.
    pub reason: String,
    /// When the change occurred.
    pub changed_at: SystemTime,
    /// Error rate that triggered the change.
    pub trigger_error_rate: f64,
}

impl GroundsTo for ThresholdChange {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Persistence])
    }
}

/// History of all threshold changes.
/// Tier: T2-P (σ + π)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThresholdHistory {
    /// All recorded changes.
    changes: Vec<ThresholdChange>,
}

impl ThresholdHistory {
    /// Records a threshold change.
    pub fn record(&mut self, change: ThresholdChange) {
        self.changes.push(change);
    }

    /// All changes.
    #[must_use]
    pub fn changes(&self) -> &[ThresholdChange] {
        &self.changes
    }

    /// Number of changes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Whether any changes have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Changes for a specific algorithm.
    #[must_use]
    pub fn changes_for(&self, algorithm: &str) -> Vec<&ThresholdChange> {
        self.changes
            .iter()
            .filter(|c| c.algorithm == algorithm)
            .collect()
    }
}

impl GroundsTo for ThresholdHistory {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Persistence])
    }
}

// ===============================================================
// CALIBRATION RESULT
// ===============================================================

/// Result of a calibration run.
/// Tier: T2-P (N + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    /// Algorithm calibrated.
    pub algorithm: String,
    /// Previous threshold.
    pub previous_threshold: f64,
    /// New recommended threshold.
    pub new_threshold: f64,
    /// Improvement in target metric.
    pub improvement: f64,
    /// Number of iterations to converge.
    pub iterations: usize,
    /// Whether the calibration converged.
    pub converged: bool,
}

impl GroundsTo for CalibrationResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Boundary])
    }
}

// ===============================================================
// CALIBRATOR
// ===============================================================

/// Threshold calibrator — adjusts detection thresholds from feedback.
/// Tier: T2-C (ρ + ∂ + N + κ)
///
/// The ρ-operation on ∂: recursively refines thresholds by evaluating
/// error rates against target metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calibrator {
    /// Calibration strategy.
    strategy: CalibrationStrategy,
    /// Optimization target.
    target: CalibrationTarget,
    /// Current thresholds by algorithm name.
    thresholds: Vec<(String, f64)>,
    /// Threshold change history.
    history: ThresholdHistory,
    /// Maximum iterations per calibration run.
    max_iterations: usize,
    /// Convergence tolerance.
    tolerance: f64,
    /// Minimum feedback samples before calibrating.
    min_samples: usize,
    /// Total calibrations performed.
    total_calibrations: u64,
}

impl Calibrator {
    /// Creates a new calibrator.
    #[must_use]
    pub fn new(
        strategy: CalibrationStrategy,
        target: CalibrationTarget,
        max_iterations: usize,
    ) -> Self {
        Self {
            strategy,
            target,
            thresholds: Vec::new(),
            history: ThresholdHistory::default(),
            max_iterations: max_iterations.max(1),
            tolerance: 0.001,
            min_samples: 10,
            total_calibrations: 0,
        }
    }

    /// Sets convergence tolerance.
    #[must_use]
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Sets minimum sample count before calibration.
    #[must_use]
    pub fn with_min_samples(mut self, min: usize) -> Self {
        self.min_samples = min;
        self
    }

    /// Sets or updates a threshold for an algorithm.
    pub fn set_threshold(&mut self, algorithm: &str, threshold: f64) {
        if let Some(entry) = self.thresholds.iter_mut().find(|(a, _)| a == algorithm) {
            entry.1 = threshold;
        } else {
            self.thresholds.push((algorithm.to_string(), threshold));
        }
    }

    /// Gets the current threshold for an algorithm.
    #[must_use]
    pub fn threshold(&self, algorithm: &str) -> Option<f64> {
        self.thresholds
            .iter()
            .find(|(a, _)| a == algorithm)
            .map(|(_, t)| *t)
    }

    /// Calibrates a threshold based on feedback.
    /// Returns `None` if insufficient samples.
    pub fn calibrate(
        &mut self,
        algorithm: &str,
        feedback: &[Feedback],
    ) -> Option<CalibrationResult> {
        // Filter feedback for this algorithm
        let relevant: Vec<&Feedback> = feedback
            .iter()
            .filter(|f| f.attribution.algorithm == algorithm && f.outcome.is_resolved())
            .collect();

        if relevant.len() < self.min_samples {
            return None;
        }

        let current_threshold = self.threshold(algorithm).unwrap_or(2.0);

        let result = match self.strategy {
            CalibrationStrategy::Grid => {
                self.calibrate_grid(algorithm, &relevant, current_threshold)
            }
            CalibrationStrategy::Bisection => {
                self.calibrate_bisection(algorithm, &relevant, current_threshold)
            }
            CalibrationStrategy::Gradient => {
                self.calibrate_gradient(algorithm, &relevant, current_threshold)
            }
        };

        // Apply the new threshold
        self.set_threshold(algorithm, result.new_threshold);
        self.total_calibrations += 1;

        // Record history
        self.history.record(ThresholdChange {
            algorithm: algorithm.to_string(),
            previous: result.previous_threshold,
            new: result.new_threshold,
            reason: format!("{:?} optimization", self.target),
            changed_at: SystemTime::now(),
            trigger_error_rate: result.improvement,
        });

        Some(result)
    }

    fn calibrate_grid(
        &self,
        algorithm: &str,
        feedback: &[&Feedback],
        current: f64,
    ) -> CalibrationResult {
        let mut best_threshold = current;
        let mut best_score = self.evaluate_threshold(feedback, current);

        // Search grid around current threshold
        let step = current * 0.1; // 10% steps
        let mut iterations = 0;

        for i in 0..self.max_iterations {
            iterations = i + 1;
            let candidate = current + step * (i as f64 - self.max_iterations as f64 / 2.0);
            if candidate <= 0.0 {
                continue;
            }

            let score = self.evaluate_threshold(feedback, candidate);
            if score > best_score {
                best_score = score;
                best_threshold = candidate;
            }
        }

        CalibrationResult {
            algorithm: algorithm.to_string(),
            previous_threshold: current,
            new_threshold: best_threshold,
            improvement: best_score - self.evaluate_threshold(feedback, current),
            iterations,
            converged: true,
        }
    }

    fn calibrate_bisection(
        &self,
        algorithm: &str,
        feedback: &[&Feedback],
        current: f64,
    ) -> CalibrationResult {
        let mut lo = current * 0.5;
        let mut hi = current * 2.0;
        let mut iterations = 0;
        let mut converged = false;

        for i in 0..self.max_iterations {
            iterations = i + 1;
            let mid = (lo + hi) / 2.0;

            let score_lo = self.evaluate_threshold(feedback, lo);
            let score_hi = self.evaluate_threshold(feedback, hi);

            if score_lo > score_hi {
                hi = mid;
            } else {
                lo = mid;
            }

            if (hi - lo).abs() < self.tolerance {
                converged = true;
                break;
            }
        }

        let best = (lo + hi) / 2.0;
        CalibrationResult {
            algorithm: algorithm.to_string(),
            previous_threshold: current,
            new_threshold: best,
            improvement: self.evaluate_threshold(feedback, best)
                - self.evaluate_threshold(feedback, current),
            iterations,
            converged,
        }
    }

    fn calibrate_gradient(
        &self,
        algorithm: &str,
        feedback: &[&Feedback],
        current: f64,
    ) -> CalibrationResult {
        let mut threshold = current;
        let learning_rate = 0.01;
        let mut iterations = 0;
        let mut converged = false;

        for i in 0..self.max_iterations {
            iterations = i + 1;

            let score = self.evaluate_threshold(feedback, threshold);
            let score_plus = self.evaluate_threshold(feedback, threshold + self.tolerance);

            let gradient = (score_plus - score) / self.tolerance;

            let new_threshold = threshold + learning_rate * gradient;
            if new_threshold <= 0.0 {
                break;
            }

            if (new_threshold - threshold).abs() < self.tolerance {
                converged = true;
                threshold = new_threshold;
                break;
            }

            threshold = new_threshold;
        }

        CalibrationResult {
            algorithm: algorithm.to_string(),
            previous_threshold: current,
            new_threshold: threshold,
            improvement: self.evaluate_threshold(feedback, threshold)
                - self.evaluate_threshold(feedback, current),
            iterations,
            converged,
        }
    }

    /// Evaluates a threshold against feedback, returning a score to maximize.
    fn evaluate_threshold(&self, feedback: &[&Feedback], threshold: f64) -> f64 {
        let mut tp: u64 = 0;
        let mut fp: u64 = 0;
        let mut tn: u64 = 0;
        let mut r#fn: u64 = 0;

        for f in feedback {
            let would_signal = f.attribution.predicted_statistic >= threshold;
            match f.outcome {
                Outcome::Confirmed if would_signal => tp += 1,
                Outcome::Confirmed if !would_signal => tn += 1,
                Outcome::FalsePositive if would_signal => fp += 1,
                Outcome::FalseNegative if !would_signal => r#fn += 1,
                _ => {}
            }
        }

        match self.target {
            CalibrationTarget::MinimizeFPR => {
                let fpr = if fp + tn > 0 {
                    fp as f64 / (fp + tn) as f64
                } else {
                    0.0
                };
                1.0 - fpr
            }
            CalibrationTarget::MinimizeFNR => {
                let fnr = if r#fn + tp > 0 {
                    r#fn as f64 / (r#fn + tp) as f64
                } else {
                    0.0
                };
                1.0 - fnr
            }
            CalibrationTarget::MaximizeAccuracy => {
                let total = tp + fp + tn + r#fn;
                if total == 0 {
                    return 0.0;
                }
                (tp + tn) as f64 / total as f64
            }
            CalibrationTarget::MaximizeF1 => {
                let precision = if tp + fp > 0 {
                    tp as f64 / (tp + fp) as f64
                } else {
                    0.0
                };
                let recall = if tp + r#fn > 0 {
                    tp as f64 / (tp + r#fn) as f64
                } else {
                    0.0
                };
                if precision + recall == 0.0 {
                    0.0
                } else {
                    2.0 * precision * recall / (precision + recall)
                }
            }
            CalibrationTarget::TargetFPR(target_fpr) => {
                let fpr = if fp + tn > 0 {
                    fp as f64 / (fp + tn) as f64
                } else {
                    0.0
                };
                let target = target_fpr as f64 / 1000.0;
                1.0 - (fpr - target).abs()
            }
            CalibrationTarget::TargetFNR(target_fnr) => {
                let fnr = if r#fn + tp > 0 {
                    r#fn as f64 / (r#fn + tp) as f64
                } else {
                    0.0
                };
                let target = target_fnr as f64 / 1000.0;
                1.0 - (fnr - target).abs()
            }
        }
    }

    /// Threshold change history.
    #[must_use]
    pub fn history(&self) -> &ThresholdHistory {
        &self.history
    }

    /// Total calibrations performed.
    #[must_use]
    pub fn total_calibrations(&self) -> u64 {
        self.total_calibrations
    }
}

impl GroundsTo for Calibrator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Boundary,
            LexPrimitiva::Quantity,
            LexPrimitiva::Comparison,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::{Attribution, OutcomeSource};
    use nexcore_lex_primitiva::GroundingTier;

    fn make_feedback(statistic: f64, outcome: Outcome) -> Feedback {
        Feedback::new(
            super::super::feedback::FeedbackId(1),
            Attribution::new("drug", "event", "PRR", statistic, statistic >= 2.0),
            outcome,
            OutcomeSource::Temporal,
        )
    }

    #[test]
    fn test_calibrator_set_get_threshold() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Grid,
            CalibrationTarget::MaximizeAccuracy,
            10,
        );

        cal.set_threshold("PRR", 2.0);
        assert_eq!(cal.threshold("PRR"), Some(2.0));
        assert_eq!(cal.threshold("ROR"), None);
    }

    #[test]
    fn test_calibrator_update_threshold() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Grid,
            CalibrationTarget::MaximizeAccuracy,
            10,
        );

        cal.set_threshold("PRR", 2.0);
        cal.set_threshold("PRR", 3.0);
        assert_eq!(cal.threshold("PRR"), Some(3.0));
    }

    #[test]
    fn test_calibrator_insufficient_samples() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Grid,
            CalibrationTarget::MaximizeAccuracy,
            10,
        )
        .with_min_samples(100);

        let feedback = vec![make_feedback(3.0, Outcome::Confirmed)];
        let result = cal.calibrate("PRR", &feedback);
        assert!(result.is_none());
    }

    #[test]
    fn test_calibrator_grid() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Grid,
            CalibrationTarget::MaximizeAccuracy,
            20,
        )
        .with_min_samples(5);

        cal.set_threshold("PRR", 2.0);

        let feedback: Vec<Feedback> = (0..20)
            .map(|i| {
                let stat = 1.0 + (i as f64) * 0.3;
                let outcome = if stat >= 2.5 {
                    Outcome::Confirmed
                } else {
                    Outcome::FalsePositive
                };
                make_feedback(stat, outcome)
            })
            .collect();

        let result = cal.calibrate("PRR", &feedback);
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(r.iterations > 0);
        }
    }

    #[test]
    fn test_calibrator_bisection() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Bisection,
            CalibrationTarget::MaximizeAccuracy,
            50,
        )
        .with_min_samples(5);

        cal.set_threshold("PRR", 2.0);

        let feedback: Vec<Feedback> = (0..20)
            .map(|i| {
                let stat = 1.0 + (i as f64) * 0.2;
                let outcome = if stat >= 2.0 {
                    Outcome::Confirmed
                } else {
                    Outcome::FalsePositive
                };
                make_feedback(stat, outcome)
            })
            .collect();

        let result = cal.calibrate("PRR", &feedback);
        assert!(result.is_some());
    }

    #[test]
    fn test_calibrator_gradient() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Gradient,
            CalibrationTarget::MaximizeAccuracy,
            100,
        )
        .with_min_samples(5);

        cal.set_threshold("PRR", 2.0);

        let feedback: Vec<Feedback> = (0..20)
            .map(|i| {
                let stat = 1.0 + (i as f64) * 0.2;
                let outcome = if stat >= 2.0 {
                    Outcome::Confirmed
                } else {
                    Outcome::FalseNegative
                };
                make_feedback(stat, outcome)
            })
            .collect();

        let result = cal.calibrate("PRR", &feedback);
        assert!(result.is_some());
    }

    #[test]
    fn test_calibrator_history() {
        let mut cal = Calibrator::new(
            CalibrationStrategy::Grid,
            CalibrationTarget::MaximizeAccuracy,
            10,
        )
        .with_min_samples(5);

        cal.set_threshold("PRR", 2.0);

        let feedback: Vec<Feedback> = (0..10)
            .map(|i| make_feedback(i as f64, Outcome::Confirmed))
            .collect();

        cal.calibrate("PRR", &feedback);
        assert_eq!(cal.history().len(), 1);
        assert_eq!(cal.total_calibrations(), 1);
    }

    #[test]
    fn test_threshold_history_filter() {
        let mut history = ThresholdHistory::default();
        history.record(ThresholdChange {
            algorithm: "PRR".into(),
            previous: 2.0,
            new: 2.5,
            reason: "test".into(),
            changed_at: SystemTime::now(),
            trigger_error_rate: 0.1,
        });
        history.record(ThresholdChange {
            algorithm: "ROR".into(),
            previous: 1.0,
            new: 1.5,
            reason: "test".into(),
            changed_at: SystemTime::now(),
            trigger_error_rate: 0.2,
        });

        assert_eq!(history.changes_for("PRR").len(), 1);
        assert_eq!(history.changes_for("ROR").len(), 1);
        assert_eq!(history.changes_for("IC").len(), 0);
    }

    #[test]
    fn test_calibration_target_variants() {
        let t1 = CalibrationTarget::MinimizeFPR;
        let t2 = CalibrationTarget::MaximizeF1;
        let t3 = CalibrationTarget::TargetFPR(50); // 5.0% FPR
        assert_ne!(t1, t2);
        assert_ne!(t2, t3);
    }

    #[test]
    fn test_calibrator_grounding() {
        let comp = Calibrator::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }

    #[test]
    fn test_calibration_result_grounding() {
        let comp = CalibrationResult::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
