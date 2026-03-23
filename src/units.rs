//! # PVNM Units — PV-Specific Unit System
//!
//! Defines units of measurement for pharmacovigilance quantities:
//! time intervals, frequency, count categories, and rate units.
//! Supports conversion between compatible unit types.
//!
//! ## T1 Grounding (dominant: N Quantity)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | N | Quantity | 0.40 — numeric measurement |
//! | μ | Mapping | 0.30 — unit conversions |
//! | ∂ | Boundary | 0.15 — valid unit ranges |
//! | κ | Comparison | 0.15 — unit comparison |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// TIME UNIT
// ═══════════════════════════════════════════════════════════

/// Unit of time measurement.
///
/// Tier: T2-P (N + μ — time quantity mapping)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeUnit {
    /// Days.
    Days,
    /// Weeks (7 days).
    Weeks,
    /// Months (30 days nominal).
    Months,
    /// Quarters (90 days nominal).
    Quarters,
    /// Years (365 days nominal).
    Years,
}

impl TimeUnit {
    /// Nominal days per unit.
    #[must_use]
    pub fn days(&self) -> f64 {
        match self {
            Self::Days => 1.0,
            Self::Weeks => 7.0,
            Self::Months => 30.0,
            Self::Quarters => 90.0,
            Self::Years => 365.0,
        }
    }

    /// Conversion factor from self to target.
    #[must_use]
    pub fn convert_to(&self, target: &Self) -> f64 {
        self.days() / target.days()
    }

    /// Unit name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Days => "days",
            Self::Weeks => "weeks",
            Self::Months => "months",
            Self::Quarters => "quarters",
            Self::Years => "years",
        }
    }

    /// Abbreviation.
    #[must_use]
    pub fn abbrev(&self) -> &str {
        match self {
            Self::Days => "d",
            Self::Weeks => "w",
            Self::Months => "mo",
            Self::Quarters => "q",
            Self::Years => "y",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// FREQUENCY UNIT
// ═══════════════════════════════════════════════════════════

/// Unit of frequency measurement (events per time period).
///
/// Tier: T2-P (N + ν — frequency quantity)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrequencyUnit {
    /// Events per day.
    PerDay,
    /// Events per week.
    PerWeek,
    /// Events per month.
    PerMonth,
    /// Events per quarter.
    PerQuarter,
    /// Events per year.
    PerYear,
}

impl FrequencyUnit {
    /// Corresponding time unit.
    #[must_use]
    pub fn time_unit(&self) -> TimeUnit {
        match self {
            Self::PerDay => TimeUnit::Days,
            Self::PerWeek => TimeUnit::Weeks,
            Self::PerMonth => TimeUnit::Months,
            Self::PerQuarter => TimeUnit::Quarters,
            Self::PerYear => TimeUnit::Years,
        }
    }

    /// Conversion factor from self to target.
    #[must_use]
    pub fn convert_to(&self, target: &Self) -> f64 {
        // Inverse of time conversion (higher freq = more per smaller period)
        target.time_unit().days() / self.time_unit().days()
    }

    /// Unit name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::PerDay => "per day",
            Self::PerWeek => "per week",
            Self::PerMonth => "per month",
            Self::PerQuarter => "per quarter",
            Self::PerYear => "per year",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// COUNT UNIT
// ═══════════════════════════════════════════════════════════

/// Unit of count measurement.
///
/// Tier: T2-P (N — count category)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CountUnit {
    /// Individual safety cases (ICSRs).
    Cases,
    /// Adverse events.
    Events,
    /// Reports submitted.
    Reports,
    /// Exposed patients.
    Patients,
    /// Drug exposures.
    Exposures,
    /// Prescriptions.
    Prescriptions,
}

impl CountUnit {
    /// Unit name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Cases => "cases",
            Self::Events => "events",
            Self::Reports => "reports",
            Self::Patients => "patients",
            Self::Exposures => "exposures",
            Self::Prescriptions => "prescriptions",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// RATE UNIT
// ═══════════════════════════════════════════════════════════

/// Unit for rate expressions.
///
/// Tier: T2-P (N + μ — rate mapping)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RateUnit {
    /// Cases per 1000 patients.
    CasesPerThousand,
    /// Events per 10000 patient-years.
    EventsPerTenThousandPY,
    /// Reports per million doses.
    ReportsPerMillion,
    /// Per patient-year.
    PerPatientYear,
    /// Per 1000 patient-years.
    PerThousandPY,
}

impl RateUnit {
    /// Denominator value for normalization.
    #[must_use]
    pub fn denominator(&self) -> f64 {
        match self {
            Self::CasesPerThousand => 1_000.0,
            Self::EventsPerTenThousandPY => 10_000.0,
            Self::ReportsPerMillion => 1_000_000.0,
            Self::PerPatientYear => 1.0,
            Self::PerThousandPY => 1_000.0,
        }
    }

    /// Conversion factor from self to target.
    #[must_use]
    pub fn convert_to(&self, target: &Self) -> f64 {
        target.denominator() / self.denominator()
    }

    /// Unit name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::CasesPerThousand => "cases/1000",
            Self::EventsPerTenThousandPY => "events/10000 PY",
            Self::ReportsPerMillion => "reports/million",
            Self::PerPatientYear => "per PY",
            Self::PerThousandPY => "per 1000 PY",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// UNIT CONVERTER
// ═══════════════════════════════════════════════════════════

/// Type-safe unit conversion engine.
///
/// Tier: T2-C (N + μ + κ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitConverter {
    /// Total conversions performed.
    total_conversions: u64,
}

impl UnitConverter {
    /// Creates a new unit converter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_conversions: 0,
        }
    }

    /// Converts a time value between units.
    pub fn convert_time(&mut self, value: f64, from: TimeUnit, to: TimeUnit) -> f64 {
        self.total_conversions += 1;
        value * from.convert_to(&to)
    }

    /// Converts a frequency value between units.
    pub fn convert_frequency(&mut self, value: f64, from: FrequencyUnit, to: FrequencyUnit) -> f64 {
        self.total_conversions += 1;
        value * from.convert_to(&to)
    }

    /// Converts a rate value between units.
    pub fn convert_rate(&mut self, value: f64, from: RateUnit, to: RateUnit) -> f64 {
        self.total_conversions += 1;
        value * from.convert_to(&to)
    }

    /// Returns total conversions performed.
    #[must_use]
    pub fn total_conversions(&self) -> u64 {
        self.total_conversions
    }
}

impl Default for UnitConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for UnitConverter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,   // N — numeric values
            LexPrimitiva::Mapping,    // μ — unit→unit mapping
            LexPrimitiva::Comparison, // κ — factor comparison
            LexPrimitiva::Boundary,   // ∂ — valid conversions
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_unit_converter_grounding() {
        let comp = UnitConverter::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }

    #[test]
    fn test_time_unit_days() {
        assert_eq!(TimeUnit::Days.days(), 1.0);
        assert_eq!(TimeUnit::Weeks.days(), 7.0);
        assert_eq!(TimeUnit::Months.days(), 30.0);
        assert_eq!(TimeUnit::Years.days(), 365.0);
    }

    #[test]
    fn test_time_conversion() {
        let mut conv = UnitConverter::new();
        let weeks = conv.convert_time(30.0, TimeUnit::Days, TimeUnit::Weeks);
        assert!((weeks - 30.0 / 7.0).abs() < 0.001);

        let days = conv.convert_time(1.0, TimeUnit::Years, TimeUnit::Days);
        assert!((days - 365.0).abs() < 0.001);
    }

    #[test]
    fn test_frequency_conversion() {
        let mut conv = UnitConverter::new();
        // 5 per month → per year (should be more)
        let per_year = conv.convert_frequency(5.0, FrequencyUnit::PerMonth, FrequencyUnit::PerYear);
        // PerYear.days()/PerMonth.days() = 365/30 ≈ 12.17
        assert!((per_year - 5.0 * 365.0 / 30.0).abs() < 0.01);

        // 365 per year → per day
        let per_day = conv.convert_frequency(365.0, FrequencyUnit::PerYear, FrequencyUnit::PerDay);
        assert!((per_day - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rate_unit_denominator() {
        assert_eq!(RateUnit::CasesPerThousand.denominator(), 1_000.0);
        assert_eq!(RateUnit::ReportsPerMillion.denominator(), 1_000_000.0);
    }

    #[test]
    fn test_rate_conversion() {
        let mut conv = UnitConverter::new();
        // 5 per 1000 → per million
        let per_million =
            conv.convert_rate(5.0, RateUnit::CasesPerThousand, RateUnit::ReportsPerMillion);
        assert!((per_million - 5000.0).abs() < 0.001);
    }

    #[test]
    fn test_count_unit_names() {
        assert_eq!(CountUnit::Cases.name(), "cases");
        assert_eq!(CountUnit::Events.name(), "events");
        assert_eq!(CountUnit::Patients.name(), "patients");
    }

    #[test]
    fn test_time_unit_names() {
        assert_eq!(TimeUnit::Days.name(), "days");
        assert_eq!(TimeUnit::Days.abbrev(), "d");
        assert_eq!(TimeUnit::Years.name(), "years");
        assert_eq!(TimeUnit::Years.abbrev(), "y");
    }

    #[test]
    fn test_frequency_unit_time_mapping() {
        assert_eq!(FrequencyUnit::PerDay.time_unit(), TimeUnit::Days);
        assert_eq!(FrequencyUnit::PerYear.time_unit(), TimeUnit::Years);
    }

    #[test]
    fn test_converter_counter() {
        let mut conv = UnitConverter::new();
        conv.convert_time(1.0, TimeUnit::Days, TimeUnit::Weeks);
        conv.convert_time(1.0, TimeUnit::Days, TimeUnit::Months);
        assert_eq!(conv.total_conversions(), 2);
    }

    #[test]
    fn test_time_roundtrip() {
        let mut conv = UnitConverter::new();
        let original = 365.0;
        let in_years = conv.convert_time(original, TimeUnit::Days, TimeUnit::Years);
        let back = conv.convert_time(in_years, TimeUnit::Years, TimeUnit::Days);
        assert!((back - original).abs() < 0.001);
    }

    #[test]
    fn test_rate_unit_name() {
        assert_eq!(RateUnit::PerPatientYear.name(), "per PY");
        assert_eq!(RateUnit::CasesPerThousand.name(), "cases/1000");
    }
}
