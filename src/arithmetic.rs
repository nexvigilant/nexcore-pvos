//! # PVNM Arithmetic — Safe Numeric Operations
//!
//! Overflow-checked arithmetic, rounding strategies, and error
//! handling for all numeric operations in the PVOS.
//!
//! ## T1 Grounding (dominant: N Quantity)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | N | Quantity | 0.40 — numeric values |
//! | ∂ | Boundary | 0.30 — overflow/underflow bounds |
//! | κ | Comparison | 0.15 — numeric comparison |
//! | ∃ | Existence | 0.15 — valid result check |

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// NUMERIC ERROR
// ═══════════════════════════════════════════════════════════

/// Error types for numeric operations.
///
/// Tier: T2-P (N + ∅ — numeric absence)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumericError {
    /// Integer overflow.
    Overflow,
    /// Integer underflow.
    Underflow,
    /// Division by zero.
    DivByZero,
    /// Value not a number.
    NotANumber,
    /// Value is infinite.
    Infinite,
    /// Value out of valid range.
    OutOfRange { reason: String },
}

impl NumericError {
    /// Human-readable description.
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::Overflow => "numeric overflow",
            Self::Underflow => "numeric underflow",
            Self::DivByZero => "division by zero",
            Self::NotANumber => "not a number",
            Self::Infinite => "infinite value",
            Self::OutOfRange { .. } => "value out of range",
        }
    }
}

impl std::fmt::Display for NumericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange { reason } => write!(f, "out of range: {reason}"),
            other => write!(f, "{}", other.description()),
        }
    }
}

impl std::error::Error for NumericError {}

/// Result type for numeric operations.
pub type NumericResult<T> = Result<T, NumericError>;

// ═══════════════════════════════════════════════════════════
// ROUNDING STRATEGY
// ═══════════════════════════════════════════════════════════

/// Rounding strategy for numeric operations.
///
/// Tier: T2-P (N — numeric transformation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rounding {
    /// Round to nearest (half up).
    Round,
    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
    /// Round toward zero.
    Truncate,
}

impl Rounding {
    /// Applies the rounding strategy to a value.
    #[must_use]
    pub fn apply(&self, value: f64) -> f64 {
        match self {
            Self::Round => value.round(),
            Self::Floor => value.floor(),
            Self::Ceil => value.ceil(),
            Self::Truncate => value.trunc(),
        }
    }

    /// Applies rounding to a specific number of decimal places.
    #[must_use]
    pub fn apply_decimals(&self, value: f64, decimals: u32) -> f64 {
        let factor = 10_f64.powi(decimals as i32);
        self.apply(value * factor) / factor
    }
}

// ═══════════════════════════════════════════════════════════
// SAFE INTEGER OPERATIONS
// ═══════════════════════════════════════════════════════════

/// Checked addition for u64.
///
/// # Errors
/// Returns `NumericError::Overflow` if the result exceeds u64::MAX.
pub fn safe_add_u64(a: u64, b: u64) -> NumericResult<u64> {
    a.checked_add(b).ok_or(NumericError::Overflow)
}

/// Checked subtraction for u64.
///
/// # Errors
/// Returns `NumericError::Underflow` if b > a.
pub fn safe_sub_u64(a: u64, b: u64) -> NumericResult<u64> {
    a.checked_sub(b).ok_or(NumericError::Underflow)
}

/// Checked multiplication for u64.
///
/// # Errors
/// Returns `NumericError::Overflow` if the result exceeds u64::MAX.
pub fn safe_mul_u64(a: u64, b: u64) -> NumericResult<u64> {
    a.checked_mul(b).ok_or(NumericError::Overflow)
}

/// Checked division for u64.
///
/// # Errors
/// Returns `NumericError::DivByZero` if b is 0.
pub fn safe_div_u64(a: u64, b: u64) -> NumericResult<u64> {
    if b == 0 {
        return Err(NumericError::DivByZero);
    }
    Ok(a / b)
}

// ═══════════════════════════════════════════════════════════
// SAFE FLOAT OPERATIONS
// ═══════════════════════════════════════════════════════════

/// Safe division for f64 (returns error on zero divisor or NaN).
///
/// # Errors
/// Returns `NumericError::DivByZero` if b is 0.0.
/// Returns `NumericError::NotANumber` if result is NaN.
pub fn safe_div_f64(a: f64, b: f64) -> NumericResult<f64> {
    if b == 0.0 {
        return Err(NumericError::DivByZero);
    }
    let result = a / b;
    if result.is_nan() {
        return Err(NumericError::NotANumber);
    }
    if result.is_infinite() {
        return Err(NumericError::Infinite);
    }
    Ok(result)
}

/// Validates that a float is finite and non-NaN.
///
/// # Errors
/// Returns appropriate error for invalid floats.
pub fn validate_f64(value: f64) -> NumericResult<f64> {
    if value.is_nan() {
        Err(NumericError::NotANumber)
    } else if value.is_infinite() {
        Err(NumericError::Infinite)
    } else {
        Ok(value)
    }
}

/// Safe natural logarithm (returns error for non-positive).
///
/// # Errors
/// Returns `NumericError::OutOfRange` if value <= 0.
pub fn safe_ln(value: f64) -> NumericResult<f64> {
    if value <= 0.0 {
        return Err(NumericError::OutOfRange {
            reason: "ln requires positive value".into(),
        });
    }
    let result = value.ln();
    validate_f64(result)
}

/// Safe square root (returns error for negative).
///
/// # Errors
/// Returns `NumericError::OutOfRange` if value < 0.
pub fn safe_sqrt(value: f64) -> NumericResult<f64> {
    if value < 0.0 {
        return Err(NumericError::OutOfRange {
            reason: "sqrt requires non-negative value".into(),
        });
    }
    Ok(value.sqrt())
}

// ═══════════════════════════════════════════════════════════
// ARITHMETIC ENGINE
// ═══════════════════════════════════════════════════════════

/// Engine for safe arithmetic operations with configurable rounding.
///
/// Tier: T2-C (N + ∂ + κ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArithmeticEngine {
    /// Default rounding strategy.
    rounding: Rounding,
    /// Default decimal places for output.
    decimals: u32,
    /// Total operations performed.
    total_ops: u64,
    /// Total errors encountered.
    total_errors: u64,
}

impl ArithmeticEngine {
    /// Creates a new arithmetic engine.
    #[must_use]
    pub fn new(rounding: Rounding, decimals: u32) -> Self {
        Self {
            rounding,
            decimals,
            total_ops: 0,
            total_errors: 0,
        }
    }

    /// Creates with default settings (Round, 4 decimals).
    #[must_use]
    pub fn default_engine() -> Self {
        Self::new(Rounding::Round, 4)
    }

    /// Safe divide with automatic rounding.
    pub fn divide(&mut self, a: f64, b: f64) -> NumericResult<f64> {
        self.total_ops += 1;
        match safe_div_f64(a, b) {
            Ok(result) => Ok(self.rounding.apply_decimals(result, self.decimals)),
            Err(e) => {
                self.total_errors += 1;
                Err(e)
            }
        }
    }

    /// Compute a ratio (a/b) with validation.
    pub fn ratio(&mut self, numerator: f64, denominator: f64) -> NumericResult<f64> {
        self.divide(numerator, denominator)
    }

    /// Sum a slice of f64 values with validation.
    pub fn sum(&mut self, values: &[f64]) -> NumericResult<f64> {
        self.total_ops += 1;
        let total: f64 = values.iter().sum();
        validate_f64(total)
    }

    /// Mean of a slice of f64 values.
    pub fn mean(&mut self, values: &[f64]) -> NumericResult<f64> {
        if values.is_empty() {
            return Err(NumericError::DivByZero);
        }
        self.total_ops += 1;
        let sum: f64 = values.iter().sum();
        safe_div_f64(sum, values.len() as f64)
    }

    /// Returns total operations.
    #[must_use]
    pub fn total_ops(&self) -> u64 {
        self.total_ops
    }

    /// Returns total errors.
    #[must_use]
    pub fn total_errors(&self) -> u64 {
        self.total_errors
    }

    /// Returns the rounding strategy.
    #[must_use]
    pub fn rounding(&self) -> Rounding {
        self.rounding
    }
}

impl Default for ArithmeticEngine {
    fn default() -> Self {
        Self::default_engine()
    }
}

impl GroundsTo for ArithmeticEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,   // N — numeric values
            LexPrimitiva::Boundary,   // ∂ — overflow/underflow bounds
            LexPrimitiva::Comparison, // κ — numeric comparison
            LexPrimitiva::Existence,  // ∃ — valid result check
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_arithmetic_engine_grounding() {
        let comp = ArithmeticEngine::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }

    #[test]
    fn test_safe_add_u64() {
        assert_eq!(safe_add_u64(10, 20), Ok(30));
        assert_eq!(safe_add_u64(u64::MAX, 1), Err(NumericError::Overflow));
    }

    #[test]
    fn test_safe_sub_u64() {
        assert_eq!(safe_sub_u64(20, 10), Ok(10));
        assert_eq!(safe_sub_u64(5, 10), Err(NumericError::Underflow));
    }

    #[test]
    fn test_safe_mul_u64() {
        assert_eq!(safe_mul_u64(10, 20), Ok(200));
        assert_eq!(safe_mul_u64(u64::MAX, 2), Err(NumericError::Overflow));
    }

    #[test]
    fn test_safe_div_u64() {
        assert_eq!(safe_div_u64(20, 10), Ok(2));
        assert_eq!(safe_div_u64(10, 0), Err(NumericError::DivByZero));
    }

    #[test]
    fn test_safe_div_f64() {
        assert!(safe_div_f64(10.0, 3.0).is_ok());
        assert_eq!(safe_div_f64(10.0, 0.0), Err(NumericError::DivByZero));
    }

    #[test]
    fn test_validate_f64() {
        assert!(validate_f64(3.14).is_ok());
        assert_eq!(validate_f64(f64::NAN), Err(NumericError::NotANumber));
        assert_eq!(validate_f64(f64::INFINITY), Err(NumericError::Infinite));
    }

    #[test]
    fn test_safe_ln() {
        assert!(safe_ln(1.0).is_ok());
        assert!((safe_ln(std::f64::consts::E).unwrap_or(0.0) - 1.0).abs() < 0.001);
        assert!(safe_ln(0.0).is_err());
        assert!(safe_ln(-1.0).is_err());
    }

    #[test]
    fn test_safe_sqrt() {
        assert!((safe_sqrt(4.0).unwrap_or(0.0) - 2.0).abs() < f64::EPSILON);
        assert!(safe_sqrt(-1.0).is_err());
        assert!((safe_sqrt(0.0).unwrap_or(-1.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rounding_strategies() {
        assert_eq!(Rounding::Round.apply(2.5), 3.0);
        assert_eq!(Rounding::Floor.apply(2.9), 2.0);
        assert_eq!(Rounding::Ceil.apply(2.1), 3.0);
        assert_eq!(Rounding::Truncate.apply(2.9), 2.0);
        assert_eq!(Rounding::Truncate.apply(-2.9), -2.0);
    }

    #[test]
    fn test_rounding_decimals() {
        let r = Rounding::Round;
        assert!((r.apply_decimals(3.14159, 2) - 3.14).abs() < f64::EPSILON);
        assert!((r.apply_decimals(3.145, 2) - 3.15).abs() < 0.001);
    }

    #[test]
    fn test_arithmetic_engine_divide() {
        let mut engine = ArithmeticEngine::new(Rounding::Round, 4);
        let result = engine.divide(10.0, 3.0);
        assert!(result.is_ok());
        let val = result.unwrap_or(0.0);
        assert!((val - 3.3333).abs() < 0.0001);
    }

    #[test]
    fn test_arithmetic_engine_mean() {
        let mut engine = ArithmeticEngine::default_engine();
        let values = vec![2.0, 4.0, 6.0, 8.0];
        let mean = engine.mean(&values);
        assert!(mean.is_ok());
        assert!((mean.unwrap_or(0.0) - 5.0).abs() < f64::EPSILON);

        let empty_mean = engine.mean(&[]);
        assert!(empty_mean.is_err());
    }

    #[test]
    fn test_arithmetic_engine_sum() {
        let mut engine = ArithmeticEngine::default_engine();
        let values = vec![1.0, 2.0, 3.0, 4.0];
        let sum = engine.sum(&values);
        assert!(sum.is_ok());
        assert!((sum.unwrap_or(0.0) - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_arithmetic_engine_counters() {
        let mut engine = ArithmeticEngine::default_engine();
        engine.divide(10.0, 3.0).ok();
        engine.divide(10.0, 0.0).ok();
        assert_eq!(engine.total_ops(), 2);
        assert_eq!(engine.total_errors(), 1);
    }

    #[test]
    fn test_numeric_error_display() {
        assert_eq!(NumericError::Overflow.to_string(), "numeric overflow");
        assert_eq!(NumericError::DivByZero.to_string(), "division by zero");
        let oor = NumericError::OutOfRange {
            reason: "too big".into(),
        };
        assert_eq!(oor.to_string(), "out of range: too big");
    }
}
