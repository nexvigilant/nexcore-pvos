//! # PV∅ Core Void Types
//!
//! Explicit absence representation for pharmacovigilance data.
//! In PV, absence IS signal: a missing field, an unreported event,
//! a null patient ID all carry semantic weight.
//!
//! ## Primitives
//! - ∅ (Void) — DOMINANT: represents absence, missing data, nothingness
//! - ∃ (Existence) — complement of void: presence checks
//! - ∂ (Boundary) — required vs optional field boundaries
//!
//! ## Key Insight
//!
//! Rust's `Option<T>` distinguishes `Some` from `None`, but `None` has
//! no semantics — it doesn't say *why* the value is absent. `Maybe<T>`
//! enriches absence with `AbsenceReason`, turning void into information.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// ABSENCE REASON
// ===============================================================

/// Why a value is absent.
/// Tier: T2-P (∅)
///
/// In pharmacovigilance, the *reason* for absence determines the action:
/// - `NotProvided` → request from reporter
/// - `NotApplicable` → skip validation
/// - `Unknown` → flag for review
/// - `Redacted` → privacy-compliant handling
/// - `Error` → system recovery needed
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbsenceReason {
    /// Value was not provided by the reporter.
    NotProvided,
    /// Value does not apply to this context (e.g., pregnancy status for males).
    NotApplicable,
    /// Value exists but is unknown to the reporter.
    Unknown,
    /// Value was intentionally removed for privacy/regulatory reasons.
    Redacted,
    /// Value could not be obtained due to a system error.
    Error(String),
}

impl AbsenceReason {
    /// Returns true if the absence is actionable (requires follow-up).
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::NotProvided | Self::Unknown | Self::Error(_))
    }

    /// Returns true if the absence is expected/acceptable.
    #[must_use]
    pub fn is_expected(&self) -> bool {
        matches!(self, Self::NotApplicable | Self::Redacted)
    }
}

impl std::fmt::Display for AbsenceReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotProvided => write!(f, "not provided"),
            Self::NotApplicable => write!(f, "not applicable"),
            Self::Unknown => write!(f, "unknown"),
            Self::Redacted => write!(f, "redacted"),
            Self::Error(msg) => write!(f, "error: {msg}"),
        }
    }
}

impl GroundsTo for AbsenceReason {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Void])
    }
}

// ===============================================================
// MAYBE — OPTION WITH SEMANTICS
// ===============================================================

/// A value that may be present or absent with a reason.
/// Tier: T2-P (∅ + ∃)
///
/// Unlike `Option<T>`, `Maybe<T>` carries semantic information about
/// *why* the value is absent. This turns void into actionable data.
///
/// ```text
/// Option<T>:  Some(value) | None
/// Maybe<T>:   Present(value) | Absent(reason)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Maybe<T> {
    /// Value is present.
    Present(T),
    /// Value is absent, with a reason.
    Absent(AbsenceReason),
}

impl<T> Maybe<T> {
    /// Returns true if the value is present.
    #[must_use]
    pub fn is_present(&self) -> bool {
        matches!(self, Self::Present(_))
    }

    /// Returns true if the value is absent.
    #[must_use]
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent(_))
    }

    /// Returns the absence reason, if absent.
    #[must_use]
    pub fn absence_reason(&self) -> Option<&AbsenceReason> {
        match self {
            Self::Absent(reason) => Some(reason),
            Self::Present(_) => None,
        }
    }

    /// Converts to `Option<T>`, discarding the absence reason.
    #[must_use]
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Present(v) => Some(v),
            Self::Absent(_) => None,
        }
    }

    /// Returns a reference to the contained value, if present.
    #[must_use]
    pub fn as_ref(&self) -> Maybe<&T> {
        match self {
            Self::Present(v) => Maybe::Present(v),
            Self::Absent(r) => Maybe::Absent(r.clone()),
        }
    }

    /// Maps a `Maybe<T>` to `Maybe<U>` by applying a function.
    #[must_use]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Maybe<U> {
        match self {
            Self::Present(v) => Maybe::Present(f(v)),
            Self::Absent(r) => Maybe::Absent(r),
        }
    }

    /// Returns the contained value or a default.
    #[must_use]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Present(v) => v,
            Self::Absent(_) => default,
        }
    }

    /// Returns the contained value or computes it from a closure.
    #[must_use]
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Self::Present(v) => v,
            Self::Absent(_) => f(),
        }
    }
}

impl<T> From<Option<T>> for Maybe<T> {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => Self::Present(v),
            None => Self::Absent(AbsenceReason::NotProvided),
        }
    }
}

impl<T> From<Maybe<T>> for Option<T> {
    fn from(maybe: Maybe<T>) -> Self {
        maybe.into_option()
    }
}

impl<T: Clone> GroundsTo for Maybe<T> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,      // ∅ — the absent branch
            LexPrimitiva::Existence, // ∃ — the present branch
        ])
        .with_dominant(LexPrimitiva::Void, 0.85)
    }
}

// ===============================================================
// FIELD REQUIREMENT
// ===============================================================

/// Specifies whether a field is required, conditional, or optional.
/// Tier: T2-P (∂ + ∅)
///
/// In ICSR processing, field requirements determine data quality:
/// - Mandatory: must be present (E2B R3: drug name, adverse event)
/// - Conditional: required when a condition is met
/// - Optional: nice to have, absence is acceptable
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldRequirement {
    /// Field must always be present.
    Mandatory,
    /// Field required when condition holds (condition name stored).
    Conditional(String),
    /// Field is optional — absence is acceptable.
    Optional,
}

impl FieldRequirement {
    /// Returns true if the field is mandatory.
    #[must_use]
    pub fn is_mandatory(&self) -> bool {
        matches!(self, Self::Mandatory)
    }

    /// Returns true if the field is always optional.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional)
    }

    /// Checks if this requirement is satisfied given presence status
    /// and condition evaluation result.
    #[must_use]
    pub fn is_satisfied(&self, present: bool, condition_met: bool) -> bool {
        match self {
            Self::Mandatory => present,
            Self::Conditional(_) => !condition_met || present,
            Self::Optional => true,
        }
    }
}

impl GroundsTo for FieldRequirement {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary, // ∂ — boundary between required/optional
            LexPrimitiva::Void,     // ∅ — the absent case
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// VOID SAFE — TRACKED VOID PROPAGATION
// ===============================================================

/// A value wrapper that tracks whether void was ever encountered.
/// Tier: T2-C (∅ + ∃ + σ + N)
///
/// When values flow through a pipeline, `VoidSafe<T>` records how
/// many void encounters happened and whether the final value was
/// derived from a default or from actual data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoidSafe<T> {
    /// The current value.
    value: T,
    /// Whether the value came from a default substitution.
    from_default: bool,
    /// Number of void encounters during derivation.
    void_count: u64,
    /// The original absence reason, if any.
    original_reason: Option<AbsenceReason>,
}

impl<T> VoidSafe<T> {
    /// Creates a VoidSafe from an actual value (no void encountered).
    #[must_use]
    pub fn from_value(value: T) -> Self {
        Self {
            value,
            from_default: false,
            void_count: 0,
            original_reason: None,
        }
    }

    /// Creates a VoidSafe from a default substitution (void was present).
    #[must_use]
    pub fn from_default(value: T, reason: AbsenceReason) -> Self {
        Self {
            value,
            from_default: true,
            void_count: 1,
            original_reason: Some(reason),
        }
    }

    /// Returns the contained value.
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Returns the contained value, consuming self.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// Returns true if the value came from a default substitution.
    #[must_use]
    pub fn is_from_default(&self) -> bool {
        self.from_default
    }

    /// Returns the number of void encounters.
    #[must_use]
    pub fn void_count(&self) -> u64 {
        self.void_count
    }

    /// Returns the original absence reason, if any.
    #[must_use]
    pub fn original_reason(&self) -> Option<&AbsenceReason> {
        self.original_reason.as_ref()
    }

    /// Maps the inner value, preserving void tracking metadata.
    #[must_use]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> VoidSafe<U> {
        VoidSafe {
            value: f(self.value),
            from_default: self.from_default,
            void_count: self.void_count,
            original_reason: self.original_reason,
        }
    }

    /// Increments the void counter (used when propagating through chains).
    pub fn record_void(&mut self) {
        self.void_count += 1;
    }
}

impl<T: Clone> GroundsTo for VoidSafe<T> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,      // ∅ — void tracking
            LexPrimitiva::Existence, // ∃ — value presence
            LexPrimitiva::Sequence,  // σ — propagation chain
            LexPrimitiva::Quantity,  // N — void counter
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// NULL COALESCE — FALLBACK CHAIN
// ===============================================================

/// A chain of fallback values, returning the first non-void result.
/// Tier: T2-P (∅ + σ)
///
/// Equivalent to SQL's `COALESCE(a, b, c, ...)` — returns the first
/// present value from a sequence of candidates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NullCoalesce<T> {
    /// Ordered fallback candidates.
    candidates: Vec<Maybe<T>>,
}

impl<T: Clone> NullCoalesce<T> {
    /// Creates a new empty coalesce chain.
    #[must_use]
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    /// Adds a candidate to the fallback chain.
    #[must_use]
    pub fn or(mut self, candidate: Maybe<T>) -> Self {
        self.candidates.push(candidate);
        self
    }

    /// Adds a definite value as fallback.
    #[must_use]
    pub fn or_value(mut self, value: T) -> Self {
        self.candidates.push(Maybe::Present(value));
        self
    }

    /// Resolves the chain, returning the first present value.
    #[must_use]
    pub fn resolve(self) -> Maybe<T> {
        for candidate in self.candidates {
            if candidate.is_present() {
                return candidate;
            }
        }
        Maybe::Absent(AbsenceReason::NotProvided)
    }

    /// Returns the number of candidates in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Returns true if the chain has no candidates.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

impl<T: Clone> Default for NullCoalesce<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> GroundsTo for NullCoalesce<T> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,     // ∅ — what we're resolving
            LexPrimitiva::Sequence, // σ — ordered fallback chain
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    // --- Grounding tests ---

    #[test]
    fn test_absence_reason_grounding() {
        let comp = AbsenceReason::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
    }

    #[test]
    fn test_maybe_grounding() {
        let comp = Maybe::<String>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_field_requirement_grounding() {
        let comp = FieldRequirement::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_void_safe_grounding() {
        let comp = VoidSafe::<String>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_null_coalesce_grounding() {
        let comp = NullCoalesce::<String>::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    // --- Maybe tests ---

    #[test]
    fn test_maybe_present() {
        let m: Maybe<u32> = Maybe::Present(42);
        assert!(m.is_present());
        assert!(!m.is_absent());
        assert!(m.absence_reason().is_none());
        assert_eq!(m.unwrap_or(0), 42);
    }

    #[test]
    fn test_maybe_absent() {
        let m: Maybe<u32> = Maybe::Absent(AbsenceReason::NotProvided);
        assert!(m.is_absent());
        assert!(!m.is_present());
        assert_eq!(m.absence_reason(), Some(&AbsenceReason::NotProvided));
    }

    #[test]
    fn test_maybe_unwrap_or() {
        let absent: Maybe<u32> = Maybe::Absent(AbsenceReason::Unknown);
        assert_eq!(absent.unwrap_or(99), 99);

        let present: Maybe<u32> = Maybe::Present(42);
        assert_eq!(present.unwrap_or(99), 42);
    }

    #[test]
    fn test_maybe_map() {
        let m: Maybe<u32> = Maybe::Present(10);
        let doubled = m.map(|v| v * 2);
        assert_eq!(doubled, Maybe::Present(20));

        let absent: Maybe<u32> = Maybe::Absent(AbsenceReason::Redacted);
        let mapped = absent.map(|v| v * 2);
        assert!(mapped.is_absent());
    }

    #[test]
    fn test_maybe_from_option() {
        let some: Maybe<u32> = Some(42).into();
        assert_eq!(some, Maybe::Present(42));

        let none: Maybe<u32> = None.into();
        assert!(none.is_absent());
    }

    #[test]
    fn test_maybe_into_option() {
        let present: Option<u32> = Maybe::Present(42).into();
        assert_eq!(present, Some(42));

        let absent: Option<u32> = Maybe::<u32>::Absent(AbsenceReason::Unknown).into();
        assert_eq!(absent, None);
    }

    // --- AbsenceReason tests ---

    #[test]
    fn test_absence_reason_actionable() {
        assert!(AbsenceReason::NotProvided.is_actionable());
        assert!(AbsenceReason::Unknown.is_actionable());
        assert!(AbsenceReason::Error("timeout".into()).is_actionable());
        assert!(!AbsenceReason::NotApplicable.is_actionable());
        assert!(!AbsenceReason::Redacted.is_actionable());
    }

    #[test]
    fn test_absence_reason_expected() {
        assert!(AbsenceReason::NotApplicable.is_expected());
        assert!(AbsenceReason::Redacted.is_expected());
        assert!(!AbsenceReason::NotProvided.is_expected());
    }

    // --- FieldRequirement tests ---

    #[test]
    fn test_field_requirement_satisfaction() {
        // Mandatory: must be present
        assert!(FieldRequirement::Mandatory.is_satisfied(true, false));
        assert!(!FieldRequirement::Mandatory.is_satisfied(false, false));

        // Conditional: required only when condition met
        assert!(FieldRequirement::Conditional("serious".into()).is_satisfied(true, true));
        assert!(!FieldRequirement::Conditional("serious".into()).is_satisfied(false, true));
        assert!(FieldRequirement::Conditional("serious".into()).is_satisfied(false, false));

        // Optional: always satisfied
        assert!(FieldRequirement::Optional.is_satisfied(false, false));
        assert!(FieldRequirement::Optional.is_satisfied(true, true));
    }

    // --- VoidSafe tests ---

    #[test]
    fn test_void_safe_from_value() {
        let vs = VoidSafe::from_value(42u32);
        assert_eq!(*vs.value(), 42);
        assert!(!vs.is_from_default());
        assert_eq!(vs.void_count(), 0);
        assert!(vs.original_reason().is_none());
    }

    #[test]
    fn test_void_safe_from_default() {
        let vs = VoidSafe::from_default(0u32, AbsenceReason::NotProvided);
        assert_eq!(*vs.value(), 0);
        assert!(vs.is_from_default());
        assert_eq!(vs.void_count(), 1);
        assert_eq!(vs.original_reason(), Some(&AbsenceReason::NotProvided));
    }

    #[test]
    fn test_void_safe_map() {
        let vs = VoidSafe::from_default(10u32, AbsenceReason::Unknown);
        let doubled = vs.map(|v| v * 2);
        assert_eq!(*doubled.value(), 20);
        assert!(doubled.is_from_default());
        assert_eq!(doubled.void_count(), 1);
    }

    #[test]
    fn test_void_safe_record_void() {
        let mut vs = VoidSafe::from_value(42u32);
        assert_eq!(vs.void_count(), 0);
        vs.record_void();
        assert_eq!(vs.void_count(), 1);
        vs.record_void();
        assert_eq!(vs.void_count(), 2);
    }

    // --- NullCoalesce tests ---

    #[test]
    fn test_null_coalesce_first_present() {
        let result = NullCoalesce::new()
            .or(Maybe::Absent(AbsenceReason::NotProvided))
            .or(Maybe::Present(42u32))
            .or(Maybe::Present(99u32))
            .resolve();
        assert_eq!(result, Maybe::Present(42));
    }

    #[test]
    fn test_null_coalesce_all_absent() {
        let result = NullCoalesce::<u32>::new()
            .or(Maybe::Absent(AbsenceReason::NotProvided))
            .or(Maybe::Absent(AbsenceReason::Unknown))
            .resolve();
        assert!(result.is_absent());
    }

    #[test]
    fn test_null_coalesce_or_value() {
        let result = NullCoalesce::new()
            .or(Maybe::Absent(AbsenceReason::Redacted))
            .or_value(100u32)
            .resolve();
        assert_eq!(result, Maybe::Present(100));
    }

    #[test]
    fn test_null_coalesce_empty() {
        let chain = NullCoalesce::<u32>::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        let result = chain.resolve();
        assert!(result.is_absent());
    }
}
