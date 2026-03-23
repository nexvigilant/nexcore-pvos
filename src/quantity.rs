//! # PVNM Quantity — Core Numeric Types
//!
//! Foundational quantity types for the PVOS numeric layer.
//! Every measured value in PV operations is represented as a
//! typed quantity with explicit units and precision.
//!
//! ## T1 Grounding (dominant: N Quantity)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | N | Quantity | 0.50 — numeric measurement |
//! | ∂ | Boundary | 0.20 — bounded values |
//! | κ | Comparison | 0.15 — numeric ordering |
//! | ∃ | Existence | 0.15 — value validity |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// COUNT — INTEGER QUANTITY
// ═══════════════════════════════════════════════════════════

/// A non-negative integer count (cases, events, reports).
///
/// Tier: T2-P (N newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Count(pub u64);

impl Count {
    /// Creates a new count.
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Zero count.
    #[must_use]
    pub fn zero() -> Self {
        Self(0)
    }

    /// Returns the raw value.
    #[must_use]
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Whether this count is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Checked addition.
    #[must_use]
    pub fn checked_add(&self, other: Count) -> Option<Count> {
        self.0.checked_add(other.0).map(Count)
    }

    /// Checked subtraction.
    #[must_use]
    pub fn checked_sub(&self, other: Count) -> Option<Count> {
        self.0.checked_sub(other.0).map(Count)
    }

    /// Checked multiplication.
    #[must_use]
    pub fn checked_mul(&self, factor: u64) -> Option<Count> {
        self.0.checked_mul(factor).map(Count)
    }

    /// Saturating addition.
    #[must_use]
    pub fn saturating_add(&self, other: Count) -> Count {
        Count(self.0.saturating_add(other.0))
    }

    /// Converts to f64 for calculations.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.0 as f64
    }
}

impl GroundsTo for Count {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
            .with_dominant(LexPrimitiva::Quantity, 0.95)
    }
}

// ═══════════════════════════════════════════════════════════
// PV RATE — RATIO QUANTITY
// ═══════════════════════════════════════════════════════════

/// A non-negative ratio value (PRR, ROR, incidence rates).
///
/// Named `PvRate` to avoid conflict with `stream::Rate`.
///
/// Tier: T2-P (N newtype)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PvRate(pub f64);

impl PvRate {
    /// Creates a new rate value.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(if value < 0.0 { 0.0 } else { value })
    }

    /// Zero rate.
    #[must_use]
    pub fn zero() -> Self {
        Self(0.0)
    }

    /// Returns the raw value.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Whether this rate is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    /// Whether this rate is finite and non-NaN.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.0.is_finite()
    }

    /// Rounds to given decimal places.
    #[must_use]
    pub fn round_to(&self, decimals: u32) -> Self {
        let factor = 10_f64.powi(decimals as i32);
        Self((self.0 * factor).round() / factor)
    }
}

impl GroundsTo for PvRate {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
            .with_dominant(LexPrimitiva::Quantity, 0.95)
    }
}

// ═══════════════════════════════════════════════════════════
// PERCENTAGE
// ═══════════════════════════════════════════════════════════

/// A percentage value bounded to 0.0..=100.0.
///
/// Tier: T2-P (N + ∂ — bounded quantity)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Percentage(f64);

impl Percentage {
    /// Creates a percentage, clamped to [0, 100].
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 100.0))
    }

    /// Creates from a fraction (0.0..=1.0 → 0..=100).
    #[must_use]
    pub fn from_fraction(fraction: f64) -> Self {
        Self::new(fraction * 100.0)
    }

    /// Returns the percentage value (0..=100).
    #[must_use]
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Returns as a fraction (0.0..=1.0).
    #[must_use]
    pub fn as_fraction(&self) -> f64 {
        self.0 / 100.0
    }

    /// Whether this is 100%.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        (self.0 - 100.0).abs() < f64::EPSILON
    }

    /// Whether this is 0%.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.abs() < f64::EPSILON
    }
}

impl GroundsTo for Percentage {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity, // N — numeric value
            LexPrimitiva::Boundary, // ∂ — 0..100 bounds
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// CONFIDENCE — PROBABILITY VALUE
// ═══════════════════════════════════════════════════════════

/// Tier: T2-P (N + ∂ — bounded probability)
///
/// Canonical confidence score in [0.0, 1.0].
/// Re-exported from `nexcore-constants` to eliminate F2 equivocation.
///
/// GroundsTo impl is in `nexcore-constants::grounding` (canonical source).
pub use nexcore_constants::Confidence;

/// PVOS-specific extension methods for [`Confidence`].
pub trait ConfidenceExt {
    /// Converts to percentage.
    fn as_percentage(&self) -> Percentage;
}

impl ConfidenceExt for Confidence {
    fn as_percentage(&self) -> Percentage {
        Percentage::from_fraction(self.value())
    }
}

// ═══════════════════════════════════════════════════════════
// PRECISION
// ═══════════════════════════════════════════════════════════

/// Precision specification for numeric operations.
///
/// Tier: T2-P (N — numeric property)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Precision {
    /// Number of decimal places.
    pub decimal_places: u32,
    /// Number of significant figures (0 = no limit).
    pub significant_figures: u32,
}

impl Precision {
    /// Creates a precision spec by decimal places.
    #[must_use]
    pub fn decimals(places: u32) -> Self {
        Self {
            decimal_places: places,
            significant_figures: 0,
        }
    }

    /// Creates a precision spec by significant figures.
    #[must_use]
    pub fn sig_figs(figures: u32) -> Self {
        Self {
            decimal_places: 0,
            significant_figures: figures,
        }
    }

    /// Default PV precision (4 decimal places).
    #[must_use]
    pub fn pv_default() -> Self {
        Self::decimals(4)
    }

    /// Rounds a value to this precision.
    #[must_use]
    pub fn round(&self, value: f64) -> f64 {
        if self.decimal_places > 0 {
            let factor = 10_f64.powi(self.decimal_places as i32);
            (value * factor).round() / factor
        } else if self.significant_figures > 0 {
            if value == 0.0 {
                return 0.0;
            }
            let digits = value.abs().log10().floor() as i32 + 1;
            let factor = 10_f64.powi(self.significant_figures as i32 - digits);
            (value * factor).round() / factor
        } else {
            value
        }
    }
}

impl Default for Precision {
    fn default() -> Self {
        Self::pv_default()
    }
}

// ═══════════════════════════════════════════════════════════
// DIMENSIONLESS — UNITLESS QUANTITY
// ═══════════════════════════════════════════════════════════

/// A dimensionless quantity (ratios, indices, scores).
///
/// Tier: T2-P (N newtype)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dimensionless(pub f64);

impl Dimensionless {
    /// Creates a new dimensionless value.
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// Returns the raw value.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Whether the value is valid (finite, non-NaN).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.0.is_finite()
    }
}

impl GroundsTo for Dimensionless {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
            .with_dominant(LexPrimitiva::Quantity, 0.95)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_count_grounding() {
        let comp = Count::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }

    #[test]
    fn test_count_operations() {
        let a = Count::new(10);
        let b = Count::new(5);

        assert_eq!(a.checked_add(b), Some(Count::new(15)));
        assert_eq!(a.checked_sub(b), Some(Count::new(5)));
        assert_eq!(b.checked_sub(a), None); // underflow
        assert_eq!(a.checked_mul(3), Some(Count::new(30)));
        assert_eq!(a.as_f64(), 10.0);
        assert!(!a.is_zero());
        assert!(Count::zero().is_zero());
    }

    #[test]
    fn test_count_overflow() {
        let max = Count::new(u64::MAX);
        assert!(max.checked_add(Count::new(1)).is_none());
        assert_eq!(max.saturating_add(Count::new(1)), Count::new(u64::MAX));
    }

    #[test]
    fn test_pv_rate() {
        let rate = PvRate::new(3.5);
        assert_eq!(rate.value(), 3.5);
        assert!(rate.is_valid());
        assert!(!rate.is_zero());

        let negative = PvRate::new(-1.0);
        assert_eq!(negative.value(), 0.0); // clamped

        let rounded = PvRate::new(3.14159).round_to(2);
        assert!((rounded.value() - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn test_percentage() {
        let pct = Percentage::new(75.0);
        assert_eq!(pct.value(), 75.0);
        assert_eq!(pct.as_fraction(), 0.75);

        let from_frac = Percentage::from_fraction(0.5);
        assert_eq!(from_frac.value(), 50.0);

        let clamped = Percentage::new(150.0);
        assert_eq!(clamped.value(), 100.0);

        assert!(Percentage::new(100.0).is_complete());
        assert!(Percentage::new(0.0).is_zero());
    }

    #[test]
    fn test_percentage_grounding() {
        let comp = Percentage::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }

    #[test]
    fn test_confidence() {
        let conf = Confidence::new(0.95);
        assert_eq!(conf.value(), 0.95);
        assert_eq!(conf.as_percentage().value(), 95.0);

        let clamped = Confidence::new(1.5);
        assert_eq!(clamped.value(), 1.0);

        assert_eq!(Confidence::ninety_five().value(), 0.95);
        assert_eq!(Confidence::ninety_nine().value(), 0.99);
        assert!(Confidence::new(1.0).is_certain());
    }

    #[test]
    fn test_precision_decimals() {
        let prec = Precision::decimals(2);
        assert!((prec.round(3.14159) - 3.14).abs() < f64::EPSILON);
        assert!((prec.round(2.005) - 2.01).abs() < 0.001);
    }

    #[test]
    fn test_precision_sig_figs() {
        let prec = Precision::sig_figs(3);
        assert!((prec.round(3.14159) - 3.14).abs() < 0.01);
        assert!((prec.round(0.001234) - 0.00123).abs() < 0.0001);
        assert_eq!(prec.round(0.0), 0.0);
    }

    #[test]
    fn test_precision_default() {
        let prec = Precision::pv_default();
        assert_eq!(prec.decimal_places, 4);
    }

    #[test]
    fn test_dimensionless() {
        let d = Dimensionless::new(2.5);
        assert_eq!(d.value(), 2.5);
        assert!(d.is_valid());

        let inf = Dimensionless::new(f64::INFINITY);
        assert!(!inf.is_valid());
    }

    #[test]
    fn test_dimensionless_grounding() {
        let comp = Dimensionless::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }
}
