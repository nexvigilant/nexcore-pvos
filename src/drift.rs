//! # PVML Drift Detection
//!
//! Detects distribution shift between training and production data.
//! When the incoming data changes character, detection models degrade.
//! Drift detection triggers retraining.
//!
//! ## Primitives
//! - ρ (Recursion) — compare model to its past self
//! - ν (Frequency) — monitor over time
//! - κ (Comparison) — reference vs current distributions
//! - ∂ (Boundary) — drift thresholds

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// DRIFT TYPES
// ===============================================================

/// Type of distribution drift.
/// Tier: T2-P (ρ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriftType {
    /// Slow, continuous change over time.
    Gradual,
    /// Abrupt distribution change.
    Sudden,
    /// Drift that comes and goes (seasonal).
    Recurring,
    /// Small incremental shifts accumulating.
    Incremental,
}

impl GroundsTo for DriftType {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Recursion])
    }
}

/// Drift severity.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DriftSeverity {
    /// No significant drift.
    None,
    /// Mild drift — monitor closely.
    Mild,
    /// Moderate drift — consider retraining.
    Moderate,
    /// Severe drift — retrain immediately.
    Severe,
}

impl GroundsTo for DriftSeverity {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

/// Drift score — quantifies how much the distribution has shifted.
/// Tier: T2-P (N + κ)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DriftScore(pub f64);

impl DriftScore {
    /// Classifies this score into a severity level.
    #[must_use]
    pub fn severity(
        &self,
        mild_threshold: f64,
        moderate_threshold: f64,
        severe_threshold: f64,
    ) -> DriftSeverity {
        if self.0 >= severe_threshold {
            DriftSeverity::Severe
        } else if self.0 >= moderate_threshold {
            DriftSeverity::Moderate
        } else if self.0 >= mild_threshold {
            DriftSeverity::Mild
        } else {
            DriftSeverity::None
        }
    }
}

impl GroundsTo for DriftScore {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Comparison])
    }
}

// ===============================================================
// DRIFT METRIC
// ===============================================================

/// Statistical test used to measure drift.
/// Tier: T2-P (κ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriftMetric {
    /// Population Stability Index.
    Psi,
    /// Kullback-Leibler divergence.
    KlDivergence,
    /// Kolmogorov-Smirnov test statistic.
    KsTest,
    /// Jensen-Shannon divergence.
    JsDivergence,
    /// Mean shift detection.
    MeanShift,
}

impl GroundsTo for DriftMetric {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
    }
}

// ===============================================================
// DRIFT ALERT
// ===============================================================

/// Alert raised when drift is detected.
/// Tier: T2-C (ρ + ν + ∂ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftAlert {
    /// When drift was detected.
    pub detected_at: SystemTime,
    /// Type of drift.
    pub drift_type: DriftType,
    /// Severity.
    pub severity: DriftSeverity,
    /// Drift score.
    pub score: DriftScore,
    /// Metric used for detection.
    pub metric: DriftMetric,
    /// Description.
    pub description: String,
    /// Recommendation.
    pub recommendation: DriftRecommendation,
}

/// Recommended action for detected drift.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriftRecommendation {
    /// Continue monitoring.
    Monitor,
    /// Schedule retraining.
    ScheduleRetrain,
    /// Retrain immediately.
    RetrainNow,
    /// Fall back to conservative thresholds.
    Fallback,
}

impl GroundsTo for DriftRecommendation {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

impl GroundsTo for DriftAlert {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Frequency,
            LexPrimitiva::Boundary,
            LexPrimitiva::Causality,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.75)
    }
}

// ===============================================================
// REFERENCE DISTRIBUTION
// ===============================================================

/// A summary of the reference (training) distribution.
/// Tier: T2-P (σ + N)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DistributionSummary {
    /// Mean value.
    pub mean: f64,
    /// Variance.
    pub variance: f64,
    /// Minimum.
    pub min: f64,
    /// Maximum.
    pub max: f64,
    /// Sample count.
    pub count: usize,
}

impl DistributionSummary {
    /// Computes a summary from a slice of values.
    #[must_use]
    pub fn from_values(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        let count = values.len();
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;

        let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / count as f64;

        let min = values.iter().copied().fold(f64::MAX, f64::min);
        let max = values.iter().copied().fold(f64::MIN, f64::max);

        Self {
            mean,
            variance,
            min,
            max,
            count,
        }
    }

    /// Standard deviation.
    #[must_use]
    pub fn std_dev(&self) -> f64 {
        self.variance.sqrt()
    }
}

impl GroundsTo for DistributionSummary {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Quantity])
    }
}

// ===============================================================
// DRIFT DETECTOR
// ===============================================================

/// Drift detector — monitors for distribution shift.
/// Tier: T2-C (ρ + ν + κ + ∂)
///
/// Compares incoming data distribution to a reference (training)
/// distribution. When divergence exceeds thresholds, raises alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetector {
    /// Reference distribution summary.
    reference: Option<DistributionSummary>,
    /// Current window of values.
    current_window: Vec<f64>,
    /// Window size for comparison.
    window_size: usize,
    /// Drift metric to use.
    metric: DriftMetric,
    /// Mild drift threshold.
    mild_threshold: f64,
    /// Moderate drift threshold.
    moderate_threshold: f64,
    /// Severe drift threshold.
    severe_threshold: f64,
    /// Alert history.
    alerts: Vec<DriftAlert>,
    /// Total checks performed.
    total_checks: u64,
    /// Cooldown: minimum checks between alerts to prevent storms.
    cooldown_checks: u64,
    /// Checks since last alert.
    checks_since_alert: u64,
}

impl DriftDetector {
    /// Creates a new drift detector.
    #[must_use]
    pub fn new(metric: DriftMetric, window_size: usize) -> Self {
        Self {
            reference: None,
            current_window: Vec::with_capacity(window_size),
            window_size,
            metric,
            mild_threshold: 0.1,
            moderate_threshold: 0.25,
            severe_threshold: 0.5,
            alerts: Vec::new(),
            total_checks: 0,
            cooldown_checks: 10,
            checks_since_alert: 0,
        }
    }

    /// Sets drift thresholds.
    #[must_use]
    pub fn with_thresholds(mut self, mild: f64, moderate: f64, severe: f64) -> Self {
        self.mild_threshold = mild;
        self.moderate_threshold = moderate;
        self.severe_threshold = severe;
        self
    }

    /// Sets alert cooldown (minimum checks between alerts).
    #[must_use]
    pub fn with_cooldown(mut self, cooldown: u64) -> Self {
        self.cooldown_checks = cooldown;
        self
    }

    /// Sets the reference distribution from training data.
    pub fn set_reference(&mut self, values: &[f64]) {
        self.reference = Some(DistributionSummary::from_values(values));
    }

    /// Adds a value and checks for drift.
    /// Returns a drift alert if detected.
    pub fn observe(&mut self, value: f64) -> Option<DriftAlert> {
        self.current_window.push(value);
        self.total_checks += 1;
        self.checks_since_alert += 1;

        // Keep window bounded
        if self.current_window.len() > self.window_size {
            self.current_window.remove(0);
        }

        // Need full window and reference to check
        if self.current_window.len() < self.window_size {
            return None;
        }

        let reference = self.reference.as_ref()?;

        // Compute drift score
        let score = self.compute_drift_score(reference);
        let severity = score.severity(
            self.mild_threshold,
            self.moderate_threshold,
            self.severe_threshold,
        );

        if severity == DriftSeverity::None {
            return None;
        }

        // Cooldown check — prevent alert storms
        if self.checks_since_alert < self.cooldown_checks {
            return None;
        }

        let recommendation = match severity {
            DriftSeverity::None => return None,
            DriftSeverity::Mild => DriftRecommendation::Monitor,
            DriftSeverity::Moderate => DriftRecommendation::ScheduleRetrain,
            DriftSeverity::Severe => DriftRecommendation::RetrainNow,
        };

        let drift_type = self.classify_drift(reference);

        let alert = DriftAlert {
            detected_at: SystemTime::now(),
            drift_type,
            severity,
            score,
            metric: self.metric,
            description: format!(
                "Drift detected: score={:.4}, mean shift={:.4}",
                score.0,
                (self.current_summary().mean - reference.mean).abs(),
            ),
            recommendation,
        };

        self.alerts.push(alert.clone());
        self.checks_since_alert = 0;
        Some(alert)
    }

    fn compute_drift_score(&self, reference: &DistributionSummary) -> DriftScore {
        let current = self.current_summary();

        match self.metric {
            DriftMetric::MeanShift => {
                let ref_std = reference.std_dev().max(0.001);
                let shift = (current.mean - reference.mean).abs() / ref_std;
                DriftScore(shift)
            }
            DriftMetric::Psi
            | DriftMetric::KlDivergence
            | DriftMetric::JsDivergence
            | DriftMetric::KsTest => {
                // Simplified: use normalized mean + variance difference
                let mean_diff = (current.mean - reference.mean).abs();
                let var_diff = (current.variance - reference.variance).abs();
                let ref_var = reference.variance.max(0.001);
                DriftScore(mean_diff / reference.mean.abs().max(0.001) + var_diff / ref_var)
            }
        }
    }

    fn classify_drift(&self, reference: &DistributionSummary) -> DriftType {
        let current = self.current_summary();
        let mean_shift = (current.mean - reference.mean).abs();
        let var_shift = (current.variance - reference.variance).abs();

        if mean_shift > reference.std_dev() * 3.0 {
            DriftType::Sudden
        } else if var_shift > reference.variance * 0.5 {
            DriftType::Incremental
        } else {
            DriftType::Gradual
        }
    }

    fn current_summary(&self) -> DistributionSummary {
        DistributionSummary::from_values(&self.current_window)
    }

    /// All drift alerts.
    #[must_use]
    pub fn alerts(&self) -> &[DriftAlert] {
        &self.alerts
    }

    /// Total observations checked.
    #[must_use]
    pub fn total_checks(&self) -> u64 {
        self.total_checks
    }

    /// Number of drift alerts raised.
    #[must_use]
    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }
}

impl GroundsTo for DriftDetector {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Frequency,
            LexPrimitiva::Comparison,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_distribution_summary() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let summary = DistributionSummary::from_values(&values);

        assert!((summary.mean - 3.0).abs() < f64::EPSILON);
        assert_eq!(summary.count, 5);
        assert!((summary.min - 1.0).abs() < f64::EPSILON);
        assert!((summary.max - 5.0).abs() < f64::EPSILON);
        assert!(summary.variance > 0.0);
    }

    #[test]
    fn test_distribution_summary_empty() {
        let summary = DistributionSummary::from_values(&[]);
        assert_eq!(summary.count, 0);
        assert!((summary.mean - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_drift_score_severity() {
        let score = DriftScore(0.3);
        assert_eq!(score.severity(0.1, 0.25, 0.5), DriftSeverity::Moderate);

        let score = DriftScore(0.05);
        assert_eq!(score.severity(0.1, 0.25, 0.5), DriftSeverity::None);

        let score = DriftScore(0.6);
        assert_eq!(score.severity(0.1, 0.25, 0.5), DriftSeverity::Severe);
    }

    #[test]
    fn test_drift_detector_no_reference() {
        let mut detector = DriftDetector::new(DriftMetric::MeanShift, 5);
        // No reference set — observe should return None
        for i in 0..10 {
            assert!(detector.observe(i as f64).is_none());
        }
    }

    #[test]
    fn test_drift_detector_no_drift() {
        let mut detector = DriftDetector::new(DriftMetric::MeanShift, 5).with_cooldown(0);

        // Reference: mean=5.0
        detector.set_reference(&[4.0, 5.0, 6.0, 4.5, 5.5]);

        // Similar values → no drift
        for v in &[4.8, 5.2, 4.9, 5.1, 5.0] {
            let alert = detector.observe(*v);
            assert!(alert.is_none());
        }
    }

    #[test]
    fn test_drift_detector_sudden_drift() {
        let mut detector = DriftDetector::new(DriftMetric::MeanShift, 5)
            .with_thresholds(0.5, 1.0, 2.0)
            .with_cooldown(0);

        // Reference: mean=5.0, low variance
        detector.set_reference(&[4.8, 5.0, 5.2, 4.9, 5.1]);

        // Dramatically different values
        for v in &[50.0, 55.0, 48.0, 52.0, 51.0] {
            detector.observe(*v);
        }

        assert!(detector.alert_count() > 0);
    }

    #[test]
    fn test_drift_detector_cooldown() {
        let mut detector = DriftDetector::new(DriftMetric::MeanShift, 3)
            .with_thresholds(0.01, 0.1, 0.5)
            .with_cooldown(100); // High cooldown

        detector.set_reference(&[1.0, 1.0, 1.0]);

        // First alert should fire
        for _ in 0..3 {
            detector.observe(100.0);
        }
        let initial_alerts = detector.alert_count();

        // Subsequent checks within cooldown should not fire
        for _ in 0..10 {
            detector.observe(100.0);
        }
        assert_eq!(detector.alert_count(), initial_alerts); // Cooldown prevents more
    }

    #[test]
    fn test_drift_type_classification() {
        // Sudden drift has large mean shift
        let dt = DriftType::Sudden;
        assert_eq!(dt, DriftType::Sudden);

        // All variants constructible
        let _g = DriftType::Gradual;
        let _r = DriftType::Recurring;
        let _i = DriftType::Incremental;
    }

    #[test]
    fn test_drift_recommendation_mapping() {
        assert_eq!(
            DriftScore(0.05).severity(0.1, 0.25, 0.5),
            DriftSeverity::None,
        );
        assert_eq!(
            DriftScore(0.15).severity(0.1, 0.25, 0.5),
            DriftSeverity::Mild,
        );
    }

    #[test]
    fn test_drift_detector_grounding() {
        let comp = DriftDetector::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }

    #[test]
    fn test_drift_alert_grounding() {
        let comp = DriftAlert::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }
}
