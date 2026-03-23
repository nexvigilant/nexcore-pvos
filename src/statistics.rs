//! # PVNM Statistics — PV-Specific Statistical Types
//!
//! Type-safe representations of signal detection statistics:
//! PRR, ROR, IC, χ², confidence intervals, and contingency tables.
//! Every statistical calculation in the PVOS flows through these types.
//!
//! ## T1 Grounding (dominant: N Quantity)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | N | Quantity | 0.35 — numeric statistics |
//! | κ | Comparison | 0.30 — signal thresholds |
//! | Σ | Sum | 0.20 — aggregation |
//! | ∂ | Boundary | 0.15 — CI bounds |

use serde::{Deserialize, Serialize};

use super::arithmetic::{NumericError, NumericResult, safe_div_f64, safe_ln, safe_sqrt};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// CONTINGENCY TABLE
// ═══════════════════════════════════════════════════════════

/// 2×2 contingency table for signal detection.
///
/// ```text
///              Event    No Event
/// Drug    [   a    |     b     ]
/// No Drug [   c    |     d     ]
/// ```
///
/// Tier: T2-P (N — structured count)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContingencyTable {
    /// Drug + Event.
    pub a: u64,
    /// Drug + No Event.
    pub b: u64,
    /// No Drug + Event.
    pub c: u64,
    /// No Drug + No Event.
    pub d: u64,
}

impl ContingencyTable {
    /// Creates a new contingency table.
    #[must_use]
    pub fn new(a: u64, b: u64, c: u64, d: u64) -> Self {
        Self { a, b, c, d }
    }

    /// Total count N = a + b + c + d.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.a + self.b + self.c + self.d
    }

    /// Drug row total (a + b).
    #[must_use]
    pub fn drug_total(&self) -> u64 {
        self.a + self.b
    }

    /// No-drug row total (c + d).
    #[must_use]
    pub fn no_drug_total(&self) -> u64 {
        self.c + self.d
    }

    /// Event column total (a + c).
    #[must_use]
    pub fn event_total(&self) -> u64 {
        self.a + self.c
    }

    /// No-event column total (b + d).
    #[must_use]
    pub fn no_event_total(&self) -> u64 {
        self.b + self.d
    }

    /// Expected count for cell (a) under null hypothesis.
    #[must_use]
    pub fn expected_a(&self) -> f64 {
        let n = self.total() as f64;
        if n == 0.0 {
            return 0.0;
        }
        (self.drug_total() as f64 * self.event_total() as f64) / n
    }

    /// Whether the table has sufficient data (all margins > 0).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.drug_total() > 0
            && self.no_drug_total() > 0
            && self.event_total() > 0
            && self.no_event_total() > 0
    }
}

// ═══════════════════════════════════════════════════════════
// CONFIDENCE INTERVAL
// ═══════════════════════════════════════════════════════════

/// A confidence interval with lower, point, and upper estimates.
///
/// Tier: T2-P (N + ∂ — bounded estimate)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// Lower bound of the interval.
    pub lower: f64,
    /// Point estimate.
    pub point: f64,
    /// Upper bound of the interval.
    pub upper: f64,
}

impl ConfidenceInterval {
    /// Creates a new confidence interval.
    #[must_use]
    pub fn new(lower: f64, point: f64, upper: f64) -> Self {
        Self {
            lower,
            point,
            upper,
        }
    }

    /// Width of the interval.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }

    /// Whether the interval excludes a reference value.
    #[must_use]
    pub fn excludes(&self, reference: f64) -> bool {
        self.lower > reference || self.upper < reference
    }

    /// Whether the lower bound exceeds a reference (signal threshold).
    #[must_use]
    pub fn lower_exceeds(&self, reference: f64) -> bool {
        self.lower > reference
    }
}

// ═══════════════════════════════════════════════════════════
// PRR VALUE
// ═══════════════════════════════════════════════════════════

/// Proportional Reporting Ratio with confidence interval.
///
/// PRR = (a/(a+b)) / (c/(c+d))
///
/// Tier: T2-C (N + κ + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PRRValue {
    /// Point estimate.
    pub point: f64,
    /// 95% confidence interval.
    pub ci: ConfidenceInterval,
}

impl PRRValue {
    /// Whether a signal is detected (PRR ≥ threshold AND CI lower > 1).
    #[must_use]
    pub fn is_signal(&self, threshold: f64) -> bool {
        self.point >= threshold && self.ci.lower > 1.0
    }
}

// ═══════════════════════════════════════════════════════════
// ROR VALUE
// ═══════════════════════════════════════════════════════════

/// Reporting Odds Ratio with confidence interval.
///
/// ROR = (a*d) / (b*c)
///
/// Tier: T2-C (N + κ + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RORValue {
    /// Point estimate.
    pub point: f64,
    /// 95% confidence interval.
    pub ci: ConfidenceInterval,
}

impl RORValue {
    /// Whether a signal is detected (ROR lower CI > 1).
    #[must_use]
    pub fn is_signal(&self) -> bool {
        self.ci.lower > 1.0
    }
}

// ═══════════════════════════════════════════════════════════
// IC VALUE (INFORMATION COMPONENT)
// ═══════════════════════════════════════════════════════════

/// Information Component (IC) value.
///
/// IC = log₂(observed / expected)
///
/// Tier: T2-C (N + κ + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ICValue {
    /// IC point estimate.
    pub ic: f64,
    /// IC025 (lower 95% credible limit).
    pub ic025: f64,
}

impl ICValue {
    /// Creates a new IC value.
    #[must_use]
    pub fn new(ic: f64, ic025: f64) -> Self {
        Self { ic, ic025 }
    }

    /// Whether a signal is detected (IC025 > 0).
    #[must_use]
    pub fn is_signal(&self) -> bool {
        self.ic025 > 0.0
    }
}

// ═══════════════════════════════════════════════════════════
// CHI-SQUARE VALUE
// ═══════════════════════════════════════════════════════════

/// Chi-square statistic.
///
/// Tier: T2-P (N + κ — comparative quantity)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChiSquareValue {
    /// χ² statistic.
    pub statistic: f64,
    /// Whether Yates correction was applied.
    pub yates_corrected: bool,
}

impl ChiSquareValue {
    /// Creates a new chi-square value.
    #[must_use]
    pub fn new(statistic: f64, yates_corrected: bool) -> Self {
        Self {
            statistic,
            yates_corrected,
        }
    }

    /// Whether significant at the given threshold (default 3.841 for p<0.05).
    #[must_use]
    pub fn is_significant(&self, threshold: f64) -> bool {
        self.statistic >= threshold
    }

    /// Whether significant at p < 0.05 (χ² ≥ 3.841).
    #[must_use]
    pub fn is_significant_05(&self) -> bool {
        self.is_significant(3.841)
    }
}

// ═══════════════════════════════════════════════════════════
// STATISTICS CALCULATOR
// ═══════════════════════════════════════════════════════════

/// Calculator for PV signal detection statistics.
///
/// Tier: T2-C (N + κ + Σ + ∂ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsCalculator {
    /// Z-score for 95% CI (1.96).
    z_score: f64,
    /// Total calculations performed.
    total_calculations: u64,
}

impl StatisticsCalculator {
    /// Creates a new statistics calculator with 95% CI.
    #[must_use]
    pub fn new() -> Self {
        Self {
            z_score: 1.96,
            total_calculations: 0,
        }
    }

    /// Creates with custom z-score.
    #[must_use]
    pub fn with_z_score(z: f64) -> Self {
        Self {
            z_score: z,
            total_calculations: 0,
        }
    }

    /// Calculates PRR with 95% CI.
    ///
    /// PRR = (a/(a+b)) / (c/(c+d))
    /// ln(SE) = sqrt(1/a - 1/(a+b) + 1/c - 1/(c+d))
    ///
    /// # Errors
    /// Returns error if table has zero margins.
    pub fn prr(&mut self, table: &ContingencyTable) -> NumericResult<PRRValue> {
        self.total_calculations += 1;

        if !table.is_valid() || table.a == 0 || table.c == 0 {
            return Err(NumericError::DivByZero);
        }

        let a = table.a as f64;
        let b = table.b as f64;
        let c = table.c as f64;
        let d = table.d as f64;

        let prr = safe_div_f64(a / (a + b), c / (c + d))?;

        // Standard error of ln(PRR)
        let se_ln = safe_sqrt(1.0 / a - 1.0 / (a + b) + 1.0 / c - 1.0 / (c + d))?;
        let ln_prr = safe_ln(prr)?;

        let lower = (ln_prr - self.z_score * se_ln).exp();
        let upper = (ln_prr + self.z_score * se_ln).exp();

        Ok(PRRValue {
            point: prr,
            ci: ConfidenceInterval::new(lower, prr, upper),
        })
    }

    /// Calculates ROR with 95% CI.
    ///
    /// ROR = (a*d) / (b*c)
    /// ln(SE) = sqrt(1/a + 1/b + 1/c + 1/d)
    ///
    /// # Errors
    /// Returns error if any cell is zero.
    pub fn ror(&mut self, table: &ContingencyTable) -> NumericResult<RORValue> {
        self.total_calculations += 1;

        if table.a == 0 || table.b == 0 || table.c == 0 || table.d == 0 {
            return Err(NumericError::DivByZero);
        }

        let a = table.a as f64;
        let b = table.b as f64;
        let c = table.c as f64;
        let d = table.d as f64;

        let ror = (a * d) / (b * c);

        let se_ln = safe_sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d)?;
        let ln_ror = safe_ln(ror)?;

        let lower = (ln_ror - self.z_score * se_ln).exp();
        let upper = (ln_ror + self.z_score * se_ln).exp();

        Ok(RORValue {
            point: ror,
            ci: ConfidenceInterval::new(lower, ror, upper),
        })
    }

    /// Calculates IC (Information Component).
    ///
    /// IC = log₂(observed / expected)
    ///
    /// # Errors
    /// Returns error if expected is zero or observed is zero.
    pub fn ic(&mut self, table: &ContingencyTable) -> NumericResult<ICValue> {
        self.total_calculations += 1;

        if !table.is_valid() {
            return Err(NumericError::DivByZero);
        }

        let observed = table.a as f64;
        let expected = table.expected_a();

        if expected == 0.0 || observed == 0.0 {
            return Err(NumericError::DivByZero);
        }

        let ic = safe_ln(observed / expected)? / std::f64::consts::LN_2;

        // Simplified IC025 using shrinkage
        let ic025 = ic - self.z_score * (1.0 / safe_sqrt(observed)?);

        Ok(ICValue::new(ic, ic025))
    }

    /// Calculates χ² with Yates correction.
    ///
    /// χ² = Σ (|O - E| - 0.5)² / E
    ///
    /// # Errors
    /// Returns error if expected values are zero.
    pub fn chi_square(&mut self, table: &ContingencyTable) -> NumericResult<ChiSquareValue> {
        self.total_calculations += 1;

        let n = table.total() as f64;
        if n == 0.0 {
            return Err(NumericError::DivByZero);
        }

        let a = table.a as f64;
        let b = table.b as f64;
        let c = table.c as f64;
        let d = table.d as f64;

        // Yates correction: χ² = N(|ad - bc| - N/2)² / ((a+b)(c+d)(a+c)(b+d))
        let numerator = n * (((a * d) - (b * c)).abs() - n / 2.0).powi(2);
        let denominator = (a + b) * (c + d) * (a + c) * (b + d);

        if denominator == 0.0 {
            return Err(NumericError::DivByZero);
        }

        let chi2 = numerator / denominator;

        Ok(ChiSquareValue::new(chi2.max(0.0), true))
    }

    /// Returns total calculations.
    #[must_use]
    pub fn total_calculations(&self) -> u64 {
        self.total_calculations
    }
}

impl Default for StatisticsCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for StatisticsCalculator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,   // N — numeric statistics
            LexPrimitiva::Comparison, // κ — signal thresholds
            LexPrimitiva::Sum,        // Σ — aggregation
            LexPrimitiva::Boundary,   // ∂ — CI bounds
            LexPrimitiva::Existence,  // ∃ — signal present?
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn standard_table() -> ContingencyTable {
        // Classic aspirin-headache example
        ContingencyTable::new(15, 100, 20, 10000)
    }

    #[test]
    fn test_statistics_calculator_grounding() {
        let comp = StatisticsCalculator::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_contingency_table_totals() {
        let t = standard_table();
        assert_eq!(t.total(), 10135);
        assert_eq!(t.drug_total(), 115);
        assert_eq!(t.no_drug_total(), 10020);
        assert_eq!(t.event_total(), 35);
        assert_eq!(t.no_event_total(), 10100);
    }

    #[test]
    fn test_contingency_table_expected() {
        let t = standard_table();
        let expected = t.expected_a();
        // Expected = (115 * 35) / 10135 ≈ 0.397
        assert!(expected > 0.0);
        assert!(expected < 1.0);
    }

    #[test]
    fn test_contingency_table_validity() {
        let valid = standard_table();
        assert!(valid.is_valid());

        let invalid = ContingencyTable::new(0, 0, 0, 0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_confidence_interval() {
        let ci = ConfidenceInterval::new(1.5, 3.0, 4.5);
        assert_eq!(ci.width(), 3.0);
        assert!(ci.excludes(1.0)); // [1.5, 4.5] excludes 1.0
        assert!(!ci.excludes(2.0)); // doesn't exclude 2.0
        assert!(ci.lower_exceeds(1.0));
        assert!(!ci.lower_exceeds(2.0));
    }

    #[test]
    fn test_prr_calculation() {
        let mut calc = StatisticsCalculator::new();
        let table = standard_table();
        let prr = calc.prr(&table);

        assert!(prr.is_ok());
        if let Ok(v) = prr {
            // PRR = (15/115) / (20/10020) ≈ 65.3
            assert!(v.point > 2.0);
            assert!(v.ci.lower > 0.0);
            assert!(v.ci.upper > v.point);
            assert!(v.is_signal(2.0));
        }
    }

    #[test]
    fn test_ror_calculation() {
        let mut calc = StatisticsCalculator::new();
        let table = standard_table();
        let ror = calc.ror(&table);

        assert!(ror.is_ok());
        if let Ok(v) = ror {
            // ROR = (15 * 10000) / (100 * 20) = 75.0
            assert!((v.point - 75.0).abs() < 0.1);
            assert!(v.is_signal());
        }
    }

    #[test]
    fn test_ic_calculation() {
        let mut calc = StatisticsCalculator::new();
        let table = standard_table();
        let ic = calc.ic(&table);

        assert!(ic.is_ok());
        if let Ok(v) = ic {
            // IC should be positive for a clear signal
            assert!(v.ic > 0.0);
            assert!(v.is_signal()); // IC025 > 0
        }
    }

    #[test]
    fn test_chi_square_calculation() {
        let mut calc = StatisticsCalculator::new();
        let table = standard_table();
        let chi2 = calc.chi_square(&table);

        assert!(chi2.is_ok());
        if let Ok(v) = chi2 {
            assert!(v.statistic > 3.841); // significant at p<0.05
            assert!(v.is_significant_05());
            assert!(v.yates_corrected);
        }
    }

    #[test]
    fn test_prr_no_signal() {
        let mut calc = StatisticsCalculator::new();
        // Balanced table (no signal)
        let table = ContingencyTable::new(10, 990, 10, 990);
        let prr = calc.prr(&table);

        assert!(prr.is_ok());
        if let Ok(v) = prr {
            assert!((v.point - 1.0).abs() < 0.1); // PRR ≈ 1.0
            assert!(!v.is_signal(2.0));
        }
    }

    #[test]
    fn test_prr_zero_cell() {
        let mut calc = StatisticsCalculator::new();
        let table = ContingencyTable::new(0, 100, 20, 10000);
        let prr = calc.prr(&table);
        assert!(prr.is_err());
    }

    #[test]
    fn test_chi_square_not_significant() {
        let mut calc = StatisticsCalculator::new();
        // Balanced table
        let table = ContingencyTable::new(10, 990, 10, 990);
        let chi2 = calc.chi_square(&table);

        assert!(chi2.is_ok());
        if let Ok(v) = chi2 {
            assert!(!v.is_significant_05());
        }
    }

    #[test]
    fn test_prr_signal_detection() {
        let prr = PRRValue {
            point: 3.5,
            ci: ConfidenceInterval::new(2.1, 3.5, 5.0),
        };
        assert!(prr.is_signal(2.0));

        let weak = PRRValue {
            point: 1.5,
            ci: ConfidenceInterval::new(0.8, 1.5, 2.2),
        };
        assert!(!weak.is_signal(2.0)); // below threshold
    }

    #[test]
    fn test_calculator_counter() {
        let mut calc = StatisticsCalculator::new();
        let table = standard_table();
        calc.prr(&table).ok();
        calc.ror(&table).ok();
        calc.chi_square(&table).ok();
        assert_eq!(calc.total_calculations(), 3);
    }
}
