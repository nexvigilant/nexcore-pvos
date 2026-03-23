//! # PV∅G Gap Analysis Types
//!
//! Types for detecting, measuring, and modeling gaps in pharmacovigilance data.
//! Gaps are where "absence IS signal" — missing reports, underreporting regions,
//! and decaying signal strength all carry semantic weight.
//!
//! ## Primitives
//! - ∅ (Void) — dominant for absence-measuring types
//! - ν (Frequency) — dominant for temporal decay patterns
//! - κ (Comparison) — rate comparison, threshold evaluation
//! - N (Quantity) — counts, rates, damping coefficients
//! - π (Persistence) — tombstone durability guarantees
//! - ∃ (Existence) — pre-existence verification before deletion
//! - ∝ (Irreversibility) — tombstone permanence
//! - ρ (Recursion) — oscillation cycles
//! - ∂ (Boundary) — damping bounds, decay floors
//!
//! ## Key Insight (from research)
//!
//! ∅ (Void) is the strongest identity primitive in the corpus.
//! It appears in exactly 2 types and is dominant in both.
//! When void shows up, it IS the point — no type has void as
//! background noise. This is the "conscience layer" of PVOS.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// ABSENCE RATE DETECTOR
// ===============================================================

/// Detects statistically significant gaps in reporting rates.
/// Tier: T2-C (∅ ν κ N)
///
/// ∅ is the point: we're measuring *what isn't there*.
/// ν provides temporal rate computation, κ compares observed vs expected,
/// N measures the counts. PV application: detecting underreporting
/// of adverse events in specific populations or regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsenceRateDetector {
    /// Expected reporting rate (events per time period).
    expected_rate: f64,
    /// Observed counts per period.
    observed_periods: Vec<u64>,
    /// Threshold multiplier for gap detection (e.g., 0.5 = 50% of expected).
    gap_threshold: f64,
    /// Name of the monitored signal.
    signal_name: String,
}

impl AbsenceRateDetector {
    /// Creates a new detector.
    #[must_use]
    pub fn new(signal_name: &str, expected_rate: f64, gap_threshold: f64) -> Self {
        Self {
            expected_rate: if expected_rate > 0.0 {
                expected_rate
            } else {
                1.0
            },
            observed_periods: Vec::new(),
            gap_threshold: gap_threshold.clamp(0.0, 1.0),
            signal_name: signal_name.to_string(),
        }
    }

    /// Records an observed count for a period.
    pub fn observe(&mut self, count: u64) {
        self.observed_periods.push(count);
    }

    /// Returns the signal name.
    #[must_use]
    pub fn signal_name(&self) -> &str {
        &self.signal_name
    }

    /// Returns the number of observed periods.
    #[must_use]
    pub fn period_count(&self) -> usize {
        self.observed_periods.len()
    }

    /// Returns the observed mean rate.
    #[must_use]
    pub fn observed_rate(&self) -> f64 {
        if self.observed_periods.is_empty() {
            return 0.0;
        }
        let total: u64 = self.observed_periods.iter().sum();
        total as f64 / self.observed_periods.len() as f64
    }

    /// Returns the ratio of observed to expected rate (< 1.0 = gap).
    #[must_use]
    pub fn rate_ratio(&self) -> f64 {
        if self.expected_rate == 0.0 {
            return 0.0;
        }
        self.observed_rate() / self.expected_rate
    }

    /// Returns true if a statistically significant gap is detected.
    #[must_use]
    pub fn gap_detected(&self) -> bool {
        self.rate_ratio() < self.gap_threshold
    }

    /// Returns the gap severity: 1.0 = complete absence, 0.0 = no gap.
    #[must_use]
    pub fn gap_severity(&self) -> f64 {
        let ratio = self.rate_ratio();
        if ratio >= 1.0 { 0.0 } else { 1.0 - ratio }
    }

    /// Returns periods where the count was zero (complete gaps).
    #[must_use]
    pub fn zero_periods(&self) -> usize {
        self.observed_periods.iter().filter(|&&c| c == 0).count()
    }

    /// Returns the zero-period ratio.
    #[must_use]
    pub fn zero_period_ratio(&self) -> f64 {
        if self.observed_periods.is_empty() {
            return 0.0;
        }
        self.zero_periods() as f64 / self.observed_periods.len() as f64
    }
}

impl GroundsTo for AbsenceRateDetector {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,
            LexPrimitiva::Frequency,
            LexPrimitiva::Comparison,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// TOMBSTONE
// ===============================================================

/// Persistent marker for a deleted or invalidated entity.
/// Tier: T2-C (∅ π ∃ ∝)
///
/// ∅ marks what's gone. π ensures the tombstone persists (you can't
/// forget that something was deleted). ∃ validates pre-existence
/// before tombstoning. ∝ makes it irreversible — tombstones are final.
///
/// PV application: withdrawn drug products, retracted case reports,
/// revoked marketing authorizations. The deletion itself IS the signal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tombstone {
    /// ID of the tombstoned entity.
    entity_id: String,
    /// Entity type (drug, case, report, etc.).
    entity_type: String,
    /// Reason for tombstoning.
    reason: String,
    /// Timestamp as unix epoch seconds.
    timestamp_epoch: u64,
    /// Whether this tombstone can be reversed (default: false).
    reversible: bool,
    /// Chain of custody: who authorized the deletion.
    authorized_by: String,
}

impl Tombstone {
    /// Creates a new irreversible tombstone.
    #[must_use]
    pub fn new(entity_id: &str, entity_type: &str, reason: &str, authorized_by: &str) -> Self {
        Self {
            entity_id: entity_id.to_string(),
            entity_type: entity_type.to_string(),
            reason: reason.to_string(),
            timestamp_epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            reversible: false,
            authorized_by: authorized_by.to_string(),
        }
    }

    /// Creates a reversible tombstone (soft delete).
    #[must_use]
    pub fn soft(entity_id: &str, entity_type: &str, reason: &str, authorized_by: &str) -> Self {
        let mut ts = Self::new(entity_id, entity_type, reason, authorized_by);
        ts.reversible = true;
        ts
    }

    /// Returns the entity ID.
    #[must_use]
    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    /// Returns the entity type.
    #[must_use]
    pub fn entity_type(&self) -> &str {
        &self.entity_type
    }

    /// Returns the reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Returns whether this tombstone is reversible.
    #[must_use]
    pub fn is_reversible(&self) -> bool {
        self.reversible
    }

    /// Returns who authorized the deletion.
    #[must_use]
    pub fn authorized_by(&self) -> &str {
        &self.authorized_by
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub fn timestamp_epoch(&self) -> u64 {
        self.timestamp_epoch
    }

    /// Returns a display key for this tombstone.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}", self.entity_type, self.entity_id)
    }
}

impl std::fmt::Display for Tombstone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TOMBSTONE[{}:{}] reason={} by={}",
            self.entity_type, self.entity_id, self.reason, self.authorized_by
        )
    }
}

impl GroundsTo for Tombstone {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,
            LexPrimitiva::Persistence,
            LexPrimitiva::Existence,
            LexPrimitiva::Irreversibility,
        ])
    }
}

// ===============================================================
// DAMPED OSCILLATOR
// ===============================================================

/// Models signal strength decay with oscillatory behavior.
/// Tier: T2-C (ν ρ N ∂)
///
/// ν drives the oscillation frequency, ρ enables recursive cycle computation,
/// N measures amplitude/phase, ∂ enforces decay bounds (signal floor).
///
/// PV application: modeling how safety signals decay in relevance over time
/// after the initial detection. Some signals exhibit "echo" patterns —
/// periodic re-emergence that a simple exponential decay misses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DampedOscillator {
    /// Initial amplitude (signal strength at t=0).
    amplitude: f64,
    /// Angular frequency (oscillation speed).
    omega: f64,
    /// Damping coefficient (decay rate).
    gamma: f64,
    /// Phase offset in radians.
    phase: f64,
    /// Minimum signal floor (below this = noise).
    floor: f64,
}

impl DampedOscillator {
    /// Creates a new damped oscillator.
    #[must_use]
    pub fn new(amplitude: f64, omega: f64, gamma: f64) -> Self {
        Self {
            amplitude: amplitude.abs(),
            omega: omega.abs(),
            gamma: gamma.abs(),
            phase: 0.0,
            floor: 0.01,
        }
    }

    /// Sets the phase offset.
    #[must_use]
    pub fn with_phase(mut self, phase: f64) -> Self {
        self.phase = phase;
        self
    }

    /// Sets the signal floor.
    #[must_use]
    pub fn with_floor(mut self, floor: f64) -> Self {
        self.floor = floor.abs();
        self
    }

    /// Evaluates the oscillator at time t.
    /// Formula: A * exp(-γt) * cos(ωt + φ)
    #[must_use]
    pub fn evaluate(&self, t: f64) -> f64 {
        let decay = (-self.gamma * t).exp();
        let oscillation = (self.omega * t + self.phase).cos();
        let value = self.amplitude * decay * oscillation;
        if value.abs() < self.floor { 0.0 } else { value }
    }

    /// Returns the envelope (ignoring oscillation) at time t.
    #[must_use]
    pub fn envelope(&self, t: f64) -> f64 {
        self.amplitude * (-self.gamma * t).exp()
    }

    /// Returns the time at which the envelope drops below the floor.
    #[must_use]
    pub fn decay_time(&self) -> f64 {
        if self.gamma <= 0.0 || self.amplitude <= self.floor {
            return 0.0;
        }
        (self.amplitude / self.floor).ln() / self.gamma
    }

    /// Returns the oscillation period.
    #[must_use]
    pub fn period(&self) -> f64 {
        if self.omega == 0.0 {
            return f64::INFINITY;
        }
        2.0 * std::f64::consts::PI / self.omega
    }

    /// Returns the half-life (time for envelope to halve).
    #[must_use]
    pub fn half_life(&self) -> f64 {
        if self.gamma <= 0.0 {
            return f64::INFINITY;
        }
        (2.0_f64).ln() / self.gamma
    }

    /// Returns the quality factor Q = ω / (2γ).
    /// Higher Q = more oscillatory, lower Q = more damped.
    #[must_use]
    pub fn quality_factor(&self) -> f64 {
        if self.gamma == 0.0 {
            return f64::INFINITY;
        }
        self.omega / (2.0 * self.gamma)
    }

    /// Returns the damping coefficient.
    #[must_use]
    pub fn gamma(&self) -> f64 {
        self.gamma
    }

    /// Returns the amplitude.
    #[must_use]
    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }
}

impl GroundsTo for DampedOscillator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Recursion,
            LexPrimitiva::Quantity,
            LexPrimitiva::Boundary,
        ])
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- AbsenceRateDetector ---

    #[test]
    fn absence_rate_gap_detected() {
        let mut d = AbsenceRateDetector::new("aspirin_gi", 10.0, 0.5);
        d.observe(3);
        d.observe(2);
        d.observe(4);
        // Mean = 3.0, expected = 10.0, ratio = 0.3 < 0.5
        assert!(d.gap_detected());
    }

    #[test]
    fn absence_rate_no_gap() {
        let mut d = AbsenceRateDetector::new("ibuprofen_gi", 10.0, 0.5);
        d.observe(9);
        d.observe(11);
        d.observe(10);
        assert!(!d.gap_detected());
    }

    #[test]
    fn absence_rate_severity() {
        let mut d = AbsenceRateDetector::new("test", 100.0, 0.5);
        d.observe(50);
        assert!((d.gap_severity() - 0.5).abs() < 0.01);
    }

    #[test]
    fn absence_rate_zero_periods() {
        let mut d = AbsenceRateDetector::new("test", 10.0, 0.5);
        d.observe(5);
        d.observe(0);
        d.observe(0);
        d.observe(8);
        assert_eq!(d.zero_periods(), 2);
        assert!((d.zero_period_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn absence_rate_empty_detector() {
        let d = AbsenceRateDetector::new("empty", 10.0, 0.5);
        assert_eq!(d.observed_rate(), 0.0);
        assert!(d.gap_detected()); // 0.0 < 0.5
    }

    #[test]
    fn absence_rate_grounding() {
        let comp = AbsenceRateDetector::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Void);
        assert_eq!(comp.primitives.len(), 4);
    }

    // --- Tombstone ---

    #[test]
    fn tombstone_creation() {
        let ts = Tombstone::new("DRUG-001", "drug", "safety_withdrawal", "FDA");
        assert_eq!(ts.entity_id(), "DRUG-001");
        assert_eq!(ts.entity_type(), "drug");
        assert!(!ts.is_reversible());
    }

    #[test]
    fn tombstone_soft_delete() {
        let ts = Tombstone::soft("CASE-042", "case", "duplicate", "reviewer");
        assert!(ts.is_reversible());
    }

    #[test]
    fn tombstone_key() {
        let ts = Tombstone::new("R-100", "report", "retracted", "sponsor");
        assert_eq!(ts.key(), "report:R-100");
    }

    #[test]
    fn tombstone_display() {
        let ts = Tombstone::new("X", "drug", "recall", "EMA");
        let display = format!("{ts}");
        assert!(display.contains("TOMBSTONE"));
        assert!(display.contains("drug:X"));
    }

    #[test]
    fn tombstone_authorized_by() {
        let ts = Tombstone::new("A", "case", "error", "system");
        assert_eq!(ts.authorized_by(), "system");
    }

    #[test]
    fn tombstone_grounding() {
        let comp = Tombstone::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Void);
        assert_eq!(comp.primitives.len(), 4);
    }

    // --- DampedOscillator ---

    #[test]
    fn damped_oscillator_initial_value() {
        let osc = DampedOscillator::new(10.0, 1.0, 0.1);
        let val = osc.evaluate(0.0);
        assert!((val - 10.0).abs() < 0.01);
    }

    #[test]
    fn damped_oscillator_decays() {
        let osc = DampedOscillator::new(10.0, 1.0, 0.5);
        let v0 = osc.envelope(0.0);
        let v10 = osc.envelope(10.0);
        assert!(v10 < v0);
    }

    #[test]
    fn damped_oscillator_floor_cutoff() {
        let osc = DampedOscillator::new(1.0, 1.0, 1.0).with_floor(0.5);
        // At large t, envelope < floor → returns 0
        let val = osc.evaluate(100.0);
        assert!((val).abs() < 0.001);
    }

    #[test]
    fn damped_oscillator_half_life() {
        let osc = DampedOscillator::new(10.0, 1.0, 0.1);
        let hl = osc.half_life();
        let envelope_at_hl = osc.envelope(hl);
        assert!((envelope_at_hl - 5.0).abs() < 0.1);
    }

    #[test]
    fn damped_oscillator_quality_factor() {
        let osc = DampedOscillator::new(1.0, 10.0, 1.0);
        let q = osc.quality_factor();
        assert!((q - 5.0).abs() < 0.001);
    }

    #[test]
    fn damped_oscillator_period() {
        let osc = DampedOscillator::new(1.0, std::f64::consts::PI, 0.1);
        let period = osc.period();
        assert!((period - 2.0).abs() < 0.01);
    }

    #[test]
    fn damped_oscillator_decay_time() {
        let osc = DampedOscillator::new(10.0, 1.0, 0.5).with_floor(0.01);
        let dt = osc.decay_time();
        assert!(dt > 0.0);
        let envelope_at_dt = osc.envelope(dt);
        assert!((envelope_at_dt - 0.01).abs() < 0.01);
    }

    #[test]
    fn damped_oscillator_grounding() {
        let comp = DampedOscillator::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Frequency
        );
        assert_eq!(comp.primitives.len(), 4);
    }
}
