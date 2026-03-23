//! # PVNM Range — Numeric Ranges and Bounds
//!
//! Provides range types, bound checking, value clamping,
//! and threshold comparisons for PV numeric validation.
//!
//! ## T1 Grounding (dominant: N Quantity)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | N | Quantity | 0.35 — numeric values |
//! | ∂ | Boundary | 0.35 — range bounds |
//! | κ | Comparison | 0.20 — bound checking |
//! | ∃ | Existence | 0.10 — valid/invalid |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// BOUND
// ═══════════════════════════════════════════════════════════

/// A bound for a range endpoint.
///
/// Tier: T2-P (N + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Bound {
    /// Value is included in the range.
    Inclusive(f64),
    /// Value is excluded from the range.
    Exclusive(f64),
    /// No bound (extends to infinity).
    Unbounded,
}

impl Bound {
    /// Whether a value satisfies this bound as a lower bound.
    #[must_use]
    pub fn satisfies_lower(&self, value: f64) -> bool {
        match self {
            Self::Inclusive(min) => value >= *min,
            Self::Exclusive(min) => value > *min,
            Self::Unbounded => true,
        }
    }

    /// Whether a value satisfies this bound as an upper bound.
    #[must_use]
    pub fn satisfies_upper(&self, value: f64) -> bool {
        match self {
            Self::Inclusive(max) => value <= *max,
            Self::Exclusive(max) => value < *max,
            Self::Unbounded => true,
        }
    }

    /// Returns the bound value if present.
    #[must_use]
    pub fn value(&self) -> Option<f64> {
        match self {
            Self::Inclusive(v) | Self::Exclusive(v) => Some(*v),
            Self::Unbounded => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// RANGE CHECK RESULT
// ═══════════════════════════════════════════════════════════

/// Result of checking a value against a range.
///
/// Tier: T2-P (N + κ — comparison result)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RangeCheck {
    /// Value is within the range.
    InRange,
    /// Value is below the minimum.
    BelowMin,
    /// Value is above the maximum.
    AboveMax,
}

impl RangeCheck {
    /// Whether the value was in range.
    #[must_use]
    pub fn is_in_range(&self) -> bool {
        matches!(self, Self::InRange)
    }

    /// Whether the value was out of range.
    #[must_use]
    pub fn is_out_of_range(&self) -> bool {
        !self.is_in_range()
    }
}

// ═══════════════════════════════════════════════════════════
// NUMERIC RANGE
// ═══════════════════════════════════════════════════════════

/// A numeric range with configurable bounds.
///
/// Tier: T2-C (N + ∂ + κ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericRange {
    /// Lower bound.
    pub lower: Bound,
    /// Upper bound.
    pub upper: Bound,
    /// Optional label for this range.
    pub label: Option<String>,
}

impl NumericRange {
    /// Creates an inclusive range [min, max].
    #[must_use]
    pub fn inclusive(min: f64, max: f64) -> Self {
        Self {
            lower: Bound::Inclusive(min),
            upper: Bound::Inclusive(max),
            label: None,
        }
    }

    /// Creates an exclusive range (min, max).
    #[must_use]
    pub fn exclusive(min: f64, max: f64) -> Self {
        Self {
            lower: Bound::Exclusive(min),
            upper: Bound::Exclusive(max),
            label: None,
        }
    }

    /// Creates a range with only a lower bound [min, ∞).
    #[must_use]
    pub fn at_least(min: f64) -> Self {
        Self {
            lower: Bound::Inclusive(min),
            upper: Bound::Unbounded,
            label: None,
        }
    }

    /// Creates a range with only an upper bound (-∞, max].
    #[must_use]
    pub fn at_most(max: f64) -> Self {
        Self {
            lower: Bound::Unbounded,
            upper: Bound::Inclusive(max),
            label: None,
        }
    }

    /// Creates the non-negative range [0, ∞).
    #[must_use]
    pub fn non_negative() -> Self {
        Self::at_least(0.0)
    }

    /// Creates a positive range (0, ∞).
    #[must_use]
    pub fn positive() -> Self {
        Self {
            lower: Bound::Exclusive(0.0),
            upper: Bound::Unbounded,
            label: None,
        }
    }

    /// Adds a label.
    #[must_use]
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Checks whether a value is in range.
    #[must_use]
    pub fn check(&self, value: f64) -> RangeCheck {
        if !self.lower.satisfies_lower(value) {
            RangeCheck::BelowMin
        } else if !self.upper.satisfies_upper(value) {
            RangeCheck::AboveMax
        } else {
            RangeCheck::InRange
        }
    }

    /// Whether the value is in range.
    #[must_use]
    pub fn contains(&self, value: f64) -> bool {
        self.check(value).is_in_range()
    }

    /// Clamps a value to this range.
    #[must_use]
    pub fn clamp(&self, value: f64) -> f64 {
        let lower_clamped = match self.lower {
            Bound::Inclusive(min) | Bound::Exclusive(min) => {
                if value < min {
                    min
                } else {
                    value
                }
            }
            Bound::Unbounded => value,
        };

        match self.upper {
            Bound::Inclusive(max) | Bound::Exclusive(max) => {
                if lower_clamped > max {
                    max
                } else {
                    lower_clamped
                }
            }
            Bound::Unbounded => lower_clamped,
        }
    }
}

impl GroundsTo for NumericRange {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,   // N — numeric values
            LexPrimitiva::Boundary,   // ∂ — range bounds
            LexPrimitiva::Comparison, // κ — bound checking
            LexPrimitiva::Existence,  // ∃ — in/out of range
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// THRESHOLD
// ═══════════════════════════════════════════════════════════

/// A comparison threshold for signal detection.
///
/// Tier: T2-P (N + κ — comparative quantity)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Threshold {
    /// Threshold value.
    pub value: f64,
    /// Whether the comparison is inclusive (>=) or exclusive (>).
    pub inclusive: bool,
}

impl Threshold {
    /// Creates an inclusive threshold (value >= threshold).
    #[must_use]
    pub fn inclusive(value: f64) -> Self {
        Self {
            value,
            inclusive: true,
        }
    }

    /// Creates an exclusive threshold (value > threshold).
    #[must_use]
    pub fn exclusive(value: f64) -> Self {
        Self {
            value,
            inclusive: false,
        }
    }

    /// Whether a value exceeds this threshold.
    #[must_use]
    pub fn exceeded_by(&self, value: f64) -> bool {
        if self.inclusive {
            value >= self.value
        } else {
            value > self.value
        }
    }

    /// Whether a value is below this threshold.
    #[must_use]
    pub fn below(&self, value: f64) -> bool {
        !self.exceeded_by(value)
    }
}

// ═══════════════════════════════════════════════════════════
// RANGE CHECKER — BATCH VALIDATION
// ═══════════════════════════════════════════════════════════

/// Batch range checking engine for named ranges.
///
/// Tier: T2-C (N + ∂ + κ + μ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeChecker {
    /// Named ranges.
    ranges: Vec<(String, NumericRange)>,
    /// Total checks performed.
    total_checks: u64,
    /// Total violations found.
    total_violations: u64,
}

impl RangeChecker {
    /// Creates a new range checker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
            total_checks: 0,
            total_violations: 0,
        }
    }

    /// Adds a named range.
    pub fn add_range(&mut self, name: &str, range: NumericRange) {
        self.ranges.push((name.to_string(), range));
    }

    /// Checks a value against a named range.
    pub fn check(&mut self, name: &str, value: f64) -> Option<RangeCheck> {
        self.total_checks += 1;
        let result = self
            .ranges
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r.check(value));

        if let Some(ref r) = result {
            if r.is_out_of_range() {
                self.total_violations += 1;
            }
        }

        result
    }

    /// Returns total checks.
    #[must_use]
    pub fn total_checks(&self) -> u64 {
        self.total_checks
    }

    /// Returns total violations.
    #[must_use]
    pub fn total_violations(&self) -> u64 {
        self.total_violations
    }

    /// Returns the number of registered ranges.
    #[must_use]
    pub fn range_count(&self) -> usize {
        self.ranges.len()
    }
}

impl Default for RangeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for RangeChecker {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,   // N — numeric values
            LexPrimitiva::Boundary,   // ∂ — range definitions
            LexPrimitiva::Comparison, // κ — bound checking
            LexPrimitiva::Mapping,    // μ — name→range
            LexPrimitiva::Existence,  // ∃ — in/out
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_numeric_range_grounding() {
        let comp = NumericRange::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }

    #[test]
    fn test_range_checker_grounding() {
        let comp = RangeChecker::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_inclusive_range() {
        let range = NumericRange::inclusive(0.0, 100.0);
        assert_eq!(range.check(50.0), RangeCheck::InRange);
        assert_eq!(range.check(0.0), RangeCheck::InRange);
        assert_eq!(range.check(100.0), RangeCheck::InRange);
        assert_eq!(range.check(-1.0), RangeCheck::BelowMin);
        assert_eq!(range.check(101.0), RangeCheck::AboveMax);
    }

    #[test]
    fn test_exclusive_range() {
        let range = NumericRange::exclusive(0.0, 100.0);
        assert_eq!(range.check(50.0), RangeCheck::InRange);
        assert_eq!(range.check(0.0), RangeCheck::BelowMin); // exclusive
        assert_eq!(range.check(100.0), RangeCheck::AboveMax); // exclusive
    }

    #[test]
    fn test_half_bounded_ranges() {
        let at_least = NumericRange::at_least(0.0);
        assert!(at_least.contains(0.0));
        assert!(at_least.contains(1000.0));
        assert!(!at_least.contains(-1.0));

        let at_most = NumericRange::at_most(100.0);
        assert!(at_most.contains(-1000.0));
        assert!(at_most.contains(100.0));
        assert!(!at_most.contains(101.0));
    }

    #[test]
    fn test_range_clamp() {
        let range = NumericRange::inclusive(0.0, 100.0);
        assert_eq!(range.clamp(50.0), 50.0);
        assert_eq!(range.clamp(-10.0), 0.0);
        assert_eq!(range.clamp(200.0), 100.0);
    }

    #[test]
    fn test_non_negative_range() {
        let range = NumericRange::non_negative();
        assert!(range.contains(0.0));
        assert!(range.contains(42.0));
        assert!(!range.contains(-1.0));
    }

    #[test]
    fn test_positive_range() {
        let range = NumericRange::positive();
        assert!(!range.contains(0.0)); // exclusive
        assert!(range.contains(0.001));
    }

    #[test]
    fn test_threshold_inclusive() {
        let t = Threshold::inclusive(2.0);
        assert!(t.exceeded_by(2.0));
        assert!(t.exceeded_by(3.0));
        assert!(!t.exceeded_by(1.9));
    }

    #[test]
    fn test_threshold_exclusive() {
        let t = Threshold::exclusive(2.0);
        assert!(!t.exceeded_by(2.0)); // exclusive
        assert!(t.exceeded_by(2.001));
        assert!(t.below(2.0));
    }

    #[test]
    fn test_range_checker() {
        let mut checker = RangeChecker::new();
        checker.add_range("prr", NumericRange::non_negative());
        checker.add_range("confidence", NumericRange::inclusive(0.0, 1.0));

        assert_eq!(checker.check("prr", 3.5), Some(RangeCheck::InRange));
        assert_eq!(checker.check("prr", -1.0), Some(RangeCheck::BelowMin));
        assert_eq!(checker.check("confidence", 0.95), Some(RangeCheck::InRange));
        assert_eq!(checker.check("confidence", 1.5), Some(RangeCheck::AboveMax));
        assert!(checker.check("unknown", 1.0).is_none());

        assert_eq!(checker.total_checks(), 5);
        assert_eq!(checker.total_violations(), 2);
    }

    #[test]
    fn test_bound_values() {
        assert_eq!(Bound::Inclusive(5.0).value(), Some(5.0));
        assert_eq!(Bound::Exclusive(3.0).value(), Some(3.0));
        assert_eq!(Bound::Unbounded.value(), None);
    }
}
