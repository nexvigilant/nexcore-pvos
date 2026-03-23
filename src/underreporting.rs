//! # PV∅ Underreporting Detection
//!
//! Detects suspicious absences in pharmacovigilance reporting: drugs with
//! fewer reports than expected, silent periods with no reports, and
//! stimulated reporting spikes (Weber effect).
//!
//! ## Primitives
//! - ∅ (Void) — DOMINANT: absent reports are the core signal
//! - κ (Comparison) — expected vs actual report counts
//! - ν (Frequency) — reporting rates over time
//! - N (Quantity) — counts and thresholds
//! - σ (Sequence) — temporal patterns
//!
//! ## PV Context
//!
//! A new drug on market with zero adverse event reports is suspicious,
//! not reassuring. Expected reporting rates come from:
//! - Class-level baselines (similar drugs)
//! - Market authorization conditions
//! - Historical reporting patterns

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// T2-P NEWTYPES
// ===============================================================

/// Unique identifier for a drug in the underreporting context.
/// Tier: T2-P (∅)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DrugKey(pub String);

impl DrugKey {
    /// Creates a new drug key.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for DrugKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GroundsTo for DrugKey {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Void])
    }
}

/// Unique identifier for an adverse event type.
/// Tier: T2-P (∅)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventKey(pub String);

impl EventKey {
    /// Creates a new event key.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for EventKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl GroundsTo for EventKey {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Void])
    }
}

// ===============================================================
// EXPECTED RATE
// ===============================================================

/// Baseline expected reporting rate for a drug-event pair.
/// Tier: T2-P (∅ + ν)
///
/// Expressed as expected reports per time period (e.g., per quarter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedRate {
    /// Drug identifier.
    pub drug: DrugKey,
    /// Event identifier.
    pub event: EventKey,
    /// Expected reports per period.
    pub rate_per_period: f64,
    /// Source of the baseline (e.g., "class_average", "rmp_commitment").
    pub source: String,
}

impl ExpectedRate {
    /// Creates a new expected rate.
    #[must_use]
    pub fn new(drug: DrugKey, event: EventKey, rate: f64, source: &str) -> Self {
        Self {
            drug,
            event,
            rate_per_period: rate,
            source: source.to_string(),
        }
    }
}

impl GroundsTo for ExpectedRate {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,      // ∅ — defines expected absence level
            LexPrimitiva::Frequency, // ν — rate over time
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// GAP SEVERITY
// ===============================================================

/// Severity classification for a reporting gap.
/// Tier: T2-P (∅ + κ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GapSeverity {
    /// Minor gap (ratio 0.5-0.8 of expected).
    Low,
    /// Moderate gap (ratio 0.2-0.5 of expected).
    Medium,
    /// Major gap (ratio < 0.2 of expected).
    High,
    /// Complete silence (zero reports when some expected).
    Critical,
}

impl GapSeverity {
    /// Classifies a gap based on the actual/expected ratio.
    #[must_use]
    pub fn from_ratio(ratio: f64) -> Self {
        if ratio <= 0.0 {
            Self::Critical
        } else if ratio < 0.2 {
            Self::High
        } else if ratio < 0.5 {
            Self::Medium
        } else {
            Self::Low
        }
    }
}

impl GroundsTo for GapSeverity {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — absence severity
            LexPrimitiva::Comparison, // κ — threshold classification
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// REPORTING GAP
// ===============================================================

/// A detected gap between expected and actual report counts.
/// Tier: T2-C (∅ + κ + ν + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingGap {
    /// Drug with the gap.
    pub drug: DrugKey,
    /// Event with the gap.
    pub event: EventKey,
    /// Expected report count for the period.
    pub expected: f64,
    /// Actual report count.
    pub actual: u64,
    /// Ratio of actual to expected (< 1.0 = underreporting).
    pub ratio: f64,
    /// Severity classification.
    pub severity: GapSeverity,
    /// Period identifier (e.g., "2024-Q1").
    pub period: String,
}

impl GroundsTo for ReportingGap {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — absent reports
            LexPrimitiva::Comparison, // κ — expected vs actual
            LexPrimitiva::Frequency,  // ν — rate comparison
            LexPrimitiva::Quantity,   // N — report counts
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// SILENT PERIOD
// ===============================================================

/// A time window with zero reports when some were expected.
/// Tier: T2-C (∅ + σ + ν + κ)
///
/// Silent periods are the strongest underreporting signal:
/// complete absence over a non-trivial time span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilentPeriod {
    /// Drug with no reports.
    pub drug: DrugKey,
    /// Start of the silent window (epoch seconds).
    pub start: u64,
    /// End of the silent window (epoch seconds).
    pub end: u64,
    /// Expected reports during this window.
    pub expected_count: f64,
    /// Duration in seconds.
    pub duration_secs: u64,
}

impl SilentPeriod {
    /// Creates a new silent period.
    #[must_use]
    pub fn new(drug: DrugKey, start: u64, end: u64, expected: f64) -> Self {
        Self {
            drug,
            start,
            end,
            expected_count: expected,
            duration_secs: end.saturating_sub(start),
        }
    }

    /// Returns duration in days.
    #[must_use]
    pub fn duration_days(&self) -> f64 {
        self.duration_secs as f64 / 86400.0
    }
}

impl GroundsTo for SilentPeriod {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — complete absence
            LexPrimitiva::Sequence,   // σ — temporal window
            LexPrimitiva::Frequency,  // ν — expected rate
            LexPrimitiva::Comparison, // κ — zero vs expected
        ])
        .with_dominant(LexPrimitiva::Void, 0.85)
    }
}

// ===============================================================
// STIMULATED REPORTING
// ===============================================================

/// Detects reporting spikes that may indicate stimulated reporting
/// (Weber effect: media attention, Dear Doctor letters, etc.).
/// Tier: T2-C (ν + κ + ∅ + σ)
///
/// Not exactly "underreporting" — the opposite. But understanding
/// stimulated periods is essential to correctly identifying true
/// underreporting (a spike followed by a trough may be normal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StimulatedReporting {
    /// Drug exhibiting the spike.
    pub drug: DrugKey,
    /// Period of the spike.
    pub period: String,
    /// Actual reports in the period.
    pub actual: u64,
    /// Expected reports (baseline).
    pub baseline: f64,
    /// Spike ratio (actual / baseline).
    pub spike_ratio: f64,
    /// Suspected cause of stimulation.
    pub suspected_cause: String,
}

impl StimulatedReporting {
    /// Returns true if this is a significant spike (> 2x baseline).
    #[must_use]
    pub fn is_significant(&self) -> bool {
        self.spike_ratio > 2.0
    }
}

impl GroundsTo for StimulatedReporting {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,  // ν — rate spike
            LexPrimitiva::Comparison, // κ — baseline comparison
            LexPrimitiva::Void,       // ∅ — inverted void (excess vs absence)
            LexPrimitiva::Sequence,   // σ — temporal context
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.70)
    }
}

// ===============================================================
// UNDERREPORTING DETECTOR
// ===============================================================

/// Detects underreporting by comparing actual vs expected report counts.
/// Tier: T3 (∅ + κ + ν + N + σ + ∂)
///
/// The detector maintains expected baselines and incoming report counts,
/// then identifies gaps, silent periods, and stimulated reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderreportingDetector {
    /// Expected rates by drug-event pair.
    baselines: HashMap<(String, String), ExpectedRate>,
    /// Actual report counts by drug-event-period.
    report_counts: HashMap<(String, String, String), u64>,
    /// Last report timestamp per drug (epoch seconds).
    last_report_time: HashMap<String, u64>,
    /// Threshold below which a gap is flagged.
    gap_threshold: f64,
    /// Minimum silent period duration to flag (seconds).
    min_silent_duration: u64,
}

impl UnderreportingDetector {
    /// Creates a new detector with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self {
            baselines: HashMap::new(),
            report_counts: HashMap::new(),
            last_report_time: HashMap::new(),
            gap_threshold: 0.5,             // Flag when actual < 50% of expected
            min_silent_duration: 7_776_000, // 90 days in seconds
        }
    }

    /// Creates a detector with custom thresholds.
    #[must_use]
    pub fn with_thresholds(gap_threshold: f64, min_silent_days: u64) -> Self {
        Self {
            baselines: HashMap::new(),
            report_counts: HashMap::new(),
            last_report_time: HashMap::new(),
            gap_threshold,
            min_silent_duration: min_silent_days * 86400,
        }
    }

    /// Registers an expected reporting rate baseline.
    pub fn register_baseline(&mut self, rate: ExpectedRate) {
        let key = (rate.drug.0.clone(), rate.event.0.clone());
        self.baselines.insert(key, rate);
    }

    /// Records an incoming report.
    pub fn record_report(&mut self, drug: &str, event: &str, period: &str, now: u64) {
        let key = (drug.to_string(), event.to_string(), period.to_string());
        *self.report_counts.entry(key).or_insert(0) += 1;
        self.last_report_time.insert(drug.to_string(), now);
    }

    /// Returns the actual report count for a drug-event-period.
    #[must_use]
    pub fn actual_count(&self, drug: &str, event: &str, period: &str) -> u64 {
        let key = (drug.to_string(), event.to_string(), period.to_string());
        self.report_counts.get(&key).copied().unwrap_or(0)
    }

    /// Detects a reporting gap for a specific drug-event pair.
    #[must_use]
    pub fn detect_gap(&self, drug: &str, event: &str, period: &str) -> Option<ReportingGap> {
        let baseline_key = (drug.to_string(), event.to_string());
        let baseline = self.baselines.get(&baseline_key)?;

        let actual = self.actual_count(drug, event, period);
        let expected = baseline.rate_per_period;

        if expected <= 0.0 {
            return None;
        }

        let ratio = actual as f64 / expected;

        if ratio < self.gap_threshold {
            Some(ReportingGap {
                drug: DrugKey::new(drug),
                event: EventKey::new(event),
                expected,
                actual,
                ratio,
                severity: GapSeverity::from_ratio(ratio),
                period: period.to_string(),
            })
        } else {
            None
        }
    }

    /// Detects all reporting gaps across all registered baselines.
    #[must_use]
    pub fn detect_all_gaps(&self, period: &str) -> Vec<ReportingGap> {
        self.baselines
            .iter()
            .filter_map(|((drug, event), _)| self.detect_gap(drug, event, period))
            .collect()
    }

    /// Detects silent periods (drugs with no reports beyond threshold).
    #[must_use]
    pub fn detect_silent_periods(&self, now: u64) -> Vec<SilentPeriod> {
        let mut silent = Vec::new();

        for ((drug, _event), baseline) in &self.baselines {
            let last_time = self.last_report_time.get(drug).copied().unwrap_or(0);
            let silence_duration = now.saturating_sub(last_time);

            if silence_duration >= self.min_silent_duration {
                let expected_per_second = baseline.rate_per_period / 7_776_000.0; // per 90 days
                let expected_during_silence = expected_per_second * silence_duration as f64;

                silent.push(SilentPeriod::new(
                    DrugKey::new(drug),
                    last_time,
                    now,
                    expected_during_silence,
                ));
            }
        }

        silent
    }

    /// Detects stimulated reporting for a drug in a period.
    #[must_use]
    pub fn detect_stimulated(
        &self,
        drug: &str,
        event: &str,
        period: &str,
        suspected_cause: &str,
    ) -> Option<StimulatedReporting> {
        let baseline_key = (drug.to_string(), event.to_string());
        let baseline = self.baselines.get(&baseline_key)?;

        let actual = self.actual_count(drug, event, period);
        let expected = baseline.rate_per_period;

        if expected <= 0.0 {
            return None;
        }

        let ratio = actual as f64 / expected;

        if ratio > 2.0 {
            Some(StimulatedReporting {
                drug: DrugKey::new(drug),
                period: period.to_string(),
                actual,
                baseline: expected,
                spike_ratio: ratio,
                suspected_cause: suspected_cause.to_string(),
            })
        } else {
            None
        }
    }

    /// Returns the number of registered baselines.
    #[must_use]
    pub fn baseline_count(&self) -> usize {
        self.baselines.len()
    }
}

impl Default for UnderreportingDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for UnderreportingDetector {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — absent reports
            LexPrimitiva::Comparison, // κ — expected vs actual
            LexPrimitiva::Frequency,  // ν — reporting rates
            LexPrimitiva::Quantity,   // N — counts
            LexPrimitiva::Sequence,   // σ — temporal patterns
            LexPrimitiva::Boundary,   // ∂ — threshold boundaries
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

    fn setup_detector() -> UnderreportingDetector {
        let mut detector = UnderreportingDetector::new();
        detector.register_baseline(ExpectedRate::new(
            DrugKey::new("warfarin"),
            EventKey::new("bleeding"),
            10.0,
            "class_average",
        ));
        detector.register_baseline(ExpectedRate::new(
            DrugKey::new("statin_x"),
            EventKey::new("myopathy"),
            5.0,
            "rmp_commitment",
        ));
        detector
    }

    // --- Grounding tests ---

    #[test]
    fn test_expected_rate_grounding() {
        let comp = ExpectedRate::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_gap_severity_grounding() {
        let comp = GapSeverity::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_reporting_gap_grounding() {
        let comp = ReportingGap::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_silent_period_grounding() {
        let comp = SilentPeriod::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_stimulated_reporting_grounding() {
        let comp = StimulatedReporting::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }

    #[test]
    fn test_underreporting_detector_grounding() {
        let comp = UnderreportingDetector::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    // --- Detection tests ---

    #[test]
    fn test_no_gap_when_reports_sufficient() {
        let mut detector = setup_detector();
        // 8 reports out of 10 expected = ratio 0.8 > threshold 0.5
        for _ in 0..8 {
            detector.record_report("warfarin", "bleeding", "2024-Q1", 1000);
        }
        let gap = detector.detect_gap("warfarin", "bleeding", "2024-Q1");
        assert!(gap.is_none());
    }

    #[test]
    fn test_gap_detected_when_underreporting() {
        let mut detector = setup_detector();
        // 2 reports out of 10 expected = ratio 0.2 < threshold 0.5
        detector.record_report("warfarin", "bleeding", "2024-Q1", 1000);
        detector.record_report("warfarin", "bleeding", "2024-Q1", 1001);

        let gap = detector.detect_gap("warfarin", "bleeding", "2024-Q1");
        assert!(gap.is_some());
        if let Some(g) = gap {
            assert_eq!(g.actual, 2);
            assert!((g.expected - 10.0).abs() < f64::EPSILON);
            assert!((g.ratio - 0.2).abs() < f64::EPSILON);
            assert_eq!(g.severity, GapSeverity::Medium);
        }
    }

    #[test]
    fn test_critical_gap_zero_reports() {
        let detector = setup_detector();
        let gap = detector.detect_gap("warfarin", "bleeding", "2024-Q1");
        assert!(gap.is_some());
        if let Some(g) = gap {
            assert_eq!(g.actual, 0);
            assert_eq!(g.severity, GapSeverity::Critical);
        }
    }

    #[test]
    fn test_detect_all_gaps() {
        let detector = setup_detector();
        let gaps = detector.detect_all_gaps("2024-Q1");
        // Both baselines have zero reports → both flagged
        assert_eq!(gaps.len(), 2);
    }

    #[test]
    fn test_gap_severity_classification() {
        assert_eq!(GapSeverity::from_ratio(0.0), GapSeverity::Critical);
        assert_eq!(GapSeverity::from_ratio(-0.1), GapSeverity::Critical);
        assert_eq!(GapSeverity::from_ratio(0.1), GapSeverity::High);
        assert_eq!(GapSeverity::from_ratio(0.3), GapSeverity::Medium);
        assert_eq!(GapSeverity::from_ratio(0.6), GapSeverity::Low);
    }

    #[test]
    fn test_silent_period_detection() {
        let detector = setup_detector();
        // No reports ever → silent since time 0
        // now = 10_000_000 (> 90 days = 7_776_000 seconds)
        let silent = detector.detect_silent_periods(10_000_000);
        assert!(!silent.is_empty());
        assert!(silent[0].duration_days() > 90.0);
    }

    #[test]
    fn test_no_silent_period_when_recent() {
        let mut detector = setup_detector();
        // Report just filed at now-1000
        let now = 10_000_000u64;
        detector.record_report("warfarin", "bleeding", "2024-Q1", now - 1000);
        detector.record_report("statin_x", "myopathy", "2024-Q1", now - 1000);

        let silent = detector.detect_silent_periods(now);
        assert!(silent.is_empty());
    }

    #[test]
    fn test_stimulated_reporting_detected() {
        let mut detector = setup_detector();
        // 25 reports when 10 expected = 2.5x spike
        for i in 0..25 {
            detector.record_report("warfarin", "bleeding", "2024-Q2", 2000 + i);
        }

        let stim = detector.detect_stimulated("warfarin", "bleeding", "2024-Q2", "media_coverage");
        assert!(stim.is_some());
        if let Some(s) = stim {
            assert!(s.is_significant());
            assert!((s.spike_ratio - 2.5).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_no_stimulated_when_normal() {
        let mut detector = setup_detector();
        // 8 reports when 10 expected = 0.8x (not stimulated)
        for i in 0..8 {
            detector.record_report("warfarin", "bleeding", "2024-Q2", 2000 + i);
        }

        let stim = detector.detect_stimulated("warfarin", "bleeding", "2024-Q2", "none");
        assert!(stim.is_none());
    }

    #[test]
    fn test_unknown_baseline_returns_none() {
        let detector = setup_detector();
        let gap = detector.detect_gap("unknown_drug", "unknown_event", "2024-Q1");
        assert!(gap.is_none());
    }

    #[test]
    fn test_silent_period_duration_days() {
        let sp = SilentPeriod::new(
            DrugKey::new("test"),
            0,
            86400 * 30, // 30 days
            5.0,
        );
        assert!((sp.duration_days() - 30.0).abs() < f64::EPSILON);
    }
}
