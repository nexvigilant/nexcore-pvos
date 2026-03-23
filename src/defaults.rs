//! # PV∅ Default Value System
//!
//! Systematic default value management with audit trails.
//! When a void is replaced with a default, the substitution is
//! recorded for regulatory compliance.
//!
//! ## Primitives
//! - ∅ (Void) — DOMINANT: defaults replace void
//! - μ (Mapping) — void → value mapping
//! - π (Persistence) — audit trail
//! - ∂ (Boundary) — valid default boundaries
//!
//! ## Regulatory Context
//!
//! In PV, applying defaults must be traceable:
//! - What default was applied and why
//! - What the original absence reason was
//! - Whether the default affects signal detection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::void::AbsenceReason;

// ===============================================================
// DEFAULT STRATEGY
// ===============================================================

/// Strategy for determining a default value.
/// Tier: T2-P (∅ + μ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DefaultStrategy {
    /// Use a fixed static value.
    Static,
    /// Compute from other fields in the record.
    Computed,
    /// Inherit from a parent/template record.
    Inherited,
    /// Use a model-predicted value.
    ModelDerived,
}

impl DefaultStrategy {
    /// Returns true if this strategy always produces the same value.
    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        matches!(self, Self::Static | Self::Inherited)
    }

    /// Returns true if this strategy may produce different values per context.
    #[must_use]
    pub fn is_context_dependent(&self) -> bool {
        matches!(self, Self::Computed | Self::ModelDerived)
    }
}

impl GroundsTo for DefaultStrategy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,    // ∅ — replacing void
            LexPrimitiva::Mapping, // μ — void → value strategy
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// DEFAULT ENTRY
// ===============================================================

/// A registered default value for a specific field.
/// Tier: T2-P (∅ + μ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultEntry {
    /// Field name this default applies to.
    pub field_name: String,
    /// The default value (as string representation).
    pub value: String,
    /// Strategy used to determine this default.
    pub strategy: DefaultStrategy,
    /// Description of why this default was chosen.
    pub rationale: String,
    /// Whether this default affects signal detection results.
    pub affects_detection: bool,
}

impl DefaultEntry {
    /// Creates a new static default entry.
    #[must_use]
    pub fn static_default(field: &str, value: &str, rationale: &str) -> Self {
        Self {
            field_name: field.to_string(),
            value: value.to_string(),
            strategy: DefaultStrategy::Static,
            rationale: rationale.to_string(),
            affects_detection: false,
        }
    }

    /// Creates a default that affects detection.
    #[must_use]
    pub fn detection_affecting(field: &str, value: &str, rationale: &str) -> Self {
        Self {
            field_name: field.to_string(),
            value: value.to_string(),
            strategy: DefaultStrategy::Static,
            rationale: rationale.to_string(),
            affects_detection: true,
        }
    }
}

impl GroundsTo for DefaultEntry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,     // ∅ — what's being replaced
            LexPrimitiva::Mapping,  // μ — void → value
            LexPrimitiva::Boundary, // ∂ — valid range
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// DEFAULT AUDIT RECORD
// ===============================================================

/// Records when a default was applied, for regulatory traceability.
/// Tier: T2-C (∅ + μ + π + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultAudit {
    /// Field that received the default.
    pub field_name: String,
    /// The default value applied.
    pub value_applied: String,
    /// Strategy that produced the default.
    pub strategy: DefaultStrategy,
    /// Original absence reason.
    pub original_reason: AbsenceReason,
    /// Whether this default affects downstream signal detection.
    pub affects_detection: bool,
    /// Timestamp of application (epoch seconds).
    pub applied_at: u64,
    /// Context where the default was applied (e.g., record ID).
    pub context: String,
}

impl GroundsTo for DefaultAudit {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,        // ∅ — void that was filled
            LexPrimitiva::Mapping,     // μ — substitution
            LexPrimitiva::Persistence, // π — audit trail
            LexPrimitiva::Quantity,    // N — timestamp/counting
        ])
        .with_dominant(LexPrimitiva::Void, 0.75)
    }
}

// ===============================================================
// DEFAULT REGISTRY
// ===============================================================

/// Registry of default values with audit trail.
/// Tier: T3 (∅ + μ + π + ∂ + N + σ)
///
/// Manages a catalogue of field defaults and records every
/// application for regulatory compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultRegistry {
    /// Registered defaults by field name.
    defaults: HashMap<String, DefaultEntry>,
    /// Audit trail of applied defaults.
    audit_trail: Vec<DefaultAudit>,
    /// Total defaults applied.
    total_applied: u64,
    /// Detection-affecting defaults applied.
    detection_affecting_applied: u64,
}

impl DefaultRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            defaults: HashMap::new(),
            audit_trail: Vec::new(),
            total_applied: 0,
            detection_affecting_applied: 0,
        }
    }

    /// Registers a default value for a field.
    pub fn register(&mut self, entry: DefaultEntry) {
        self.defaults.insert(entry.field_name.clone(), entry);
    }

    /// Looks up the default for a field name.
    #[must_use]
    pub fn get(&self, field_name: &str) -> Option<&DefaultEntry> {
        self.defaults.get(field_name)
    }

    /// Applies the default for a field, recording an audit entry.
    ///
    /// Returns the default value if one is registered, `None` otherwise.
    pub fn apply(
        &mut self,
        field_name: &str,
        reason: AbsenceReason,
        context: &str,
        now: u64,
    ) -> Option<String> {
        let entry = self.defaults.get(field_name)?;
        let value = entry.value.clone();
        let affects_detection = entry.affects_detection;
        let strategy = entry.strategy.clone();

        let audit = DefaultAudit {
            field_name: field_name.to_string(),
            value_applied: value.clone(),
            strategy,
            original_reason: reason,
            affects_detection,
            applied_at: now,
            context: context.to_string(),
        };

        self.audit_trail.push(audit);
        self.total_applied += 1;
        if affects_detection {
            self.detection_affecting_applied += 1;
        }

        Some(value)
    }

    /// Returns the audit trail.
    #[must_use]
    pub fn audit_trail(&self) -> &[DefaultAudit] {
        &self.audit_trail
    }

    /// Returns the total number of defaults applied.
    #[must_use]
    pub fn total_applied(&self) -> u64 {
        self.total_applied
    }

    /// Returns the count of detection-affecting defaults applied.
    #[must_use]
    pub fn detection_affecting_count(&self) -> u64 {
        self.detection_affecting_applied
    }

    /// Returns the number of registered defaults.
    #[must_use]
    pub fn registered_count(&self) -> usize {
        self.defaults.len()
    }

    /// Returns audit entries for a specific field.
    #[must_use]
    pub fn audits_for_field(&self, field_name: &str) -> Vec<&DefaultAudit> {
        self.audit_trail
            .iter()
            .filter(|a| a.field_name == field_name)
            .collect()
    }

    /// Returns all detection-affecting audit entries.
    #[must_use]
    pub fn detection_affecting_audits(&self) -> Vec<&DefaultAudit> {
        self.audit_trail
            .iter()
            .filter(|a| a.affects_detection)
            .collect()
    }
}

impl Default for DefaultRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for DefaultRegistry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,        // ∅ — void substitution
            LexPrimitiva::Mapping,     // μ — field → default mapping
            LexPrimitiva::Persistence, // π — audit persistence
            LexPrimitiva::Boundary,    // ∂ — valid defaults
            LexPrimitiva::Quantity,    // N — applied counts
            LexPrimitiva::Sequence,    // σ — audit trail ordering
        ])
        .with_dominant(LexPrimitiva::Void, 0.70)
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
    fn test_default_strategy_grounding() {
        let comp = DefaultStrategy::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_default_entry_grounding() {
        let comp = DefaultEntry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_default_audit_grounding() {
        let comp = DefaultAudit::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_default_registry_grounding() {
        let comp = DefaultRegistry::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    // --- Strategy tests ---

    #[test]
    fn test_strategy_properties() {
        assert!(DefaultStrategy::Static.is_deterministic());
        assert!(DefaultStrategy::Inherited.is_deterministic());
        assert!(!DefaultStrategy::Computed.is_deterministic());
        assert!(!DefaultStrategy::ModelDerived.is_deterministic());

        assert!(DefaultStrategy::Computed.is_context_dependent());
        assert!(DefaultStrategy::ModelDerived.is_context_dependent());
    }

    // --- Registry tests ---

    #[test]
    fn test_register_and_get() {
        let mut registry = DefaultRegistry::new();
        registry.register(DefaultEntry::static_default(
            "patient_age",
            "unknown",
            "E2B R3 allows unknown age",
        ));

        let entry = registry.get("patient_age");
        assert!(entry.is_some());
        if let Some(e) = entry {
            assert_eq!(e.value, "unknown");
            assert_eq!(e.strategy, DefaultStrategy::Static);
        }

        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_apply_default() {
        let mut registry = DefaultRegistry::new();
        registry.register(DefaultEntry::static_default(
            "reporter_country",
            "unknown",
            "Default when reporter country not provided",
        ));

        let result = registry.apply(
            "reporter_country",
            AbsenceReason::NotProvided,
            "ICSR-001",
            1000,
        );
        assert_eq!(result, Some("unknown".to_string()));
        assert_eq!(registry.total_applied(), 1);
    }

    #[test]
    fn test_apply_unknown_field_returns_none() {
        let mut registry = DefaultRegistry::new();
        let result = registry.apply("unknown_field", AbsenceReason::NotProvided, "ctx", 1000);
        assert!(result.is_none());
        assert_eq!(registry.total_applied(), 0);
    }

    #[test]
    fn test_audit_trail() {
        let mut registry = DefaultRegistry::new();
        registry.register(DefaultEntry::static_default("age", "0", "default"));

        registry.apply("age", AbsenceReason::NotProvided, "ICSR-001", 1000);
        registry.apply("age", AbsenceReason::Unknown, "ICSR-002", 1001);

        let trail = registry.audit_trail();
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].context, "ICSR-001");
        assert_eq!(trail[1].context, "ICSR-002");
        assert_eq!(trail[0].original_reason, AbsenceReason::NotProvided);
        assert_eq!(trail[1].original_reason, AbsenceReason::Unknown);
    }

    #[test]
    fn test_detection_affecting_tracking() {
        let mut registry = DefaultRegistry::new();
        registry.register(DefaultEntry::detection_affecting(
            "seriousness",
            "non_serious",
            "Defaults to non-serious when not specified",
        ));
        registry.register(DefaultEntry::static_default(
            "reporter_name",
            "anonymous",
            "Privacy default",
        ));

        registry.apply("seriousness", AbsenceReason::NotProvided, "ICSR-001", 1000);
        registry.apply("reporter_name", AbsenceReason::Redacted, "ICSR-001", 1001);

        assert_eq!(registry.total_applied(), 2);
        assert_eq!(registry.detection_affecting_count(), 1);

        let det_audits = registry.detection_affecting_audits();
        assert_eq!(det_audits.len(), 1);
        assert_eq!(det_audits[0].field_name, "seriousness");
    }

    #[test]
    fn test_audits_for_field() {
        let mut registry = DefaultRegistry::new();
        registry.register(DefaultEntry::static_default("age", "0", "default"));
        registry.register(DefaultEntry::static_default("sex", "unknown", "default"));

        registry.apply("age", AbsenceReason::NotProvided, "r1", 1000);
        registry.apply("sex", AbsenceReason::NotProvided, "r1", 1001);
        registry.apply("age", AbsenceReason::Unknown, "r2", 1002);

        let age_audits = registry.audits_for_field("age");
        assert_eq!(age_audits.len(), 2);

        let sex_audits = registry.audits_for_field("sex");
        assert_eq!(sex_audits.len(), 1);
    }

    #[test]
    fn test_empty_registry() {
        let registry = DefaultRegistry::new();
        assert_eq!(registry.registered_count(), 0);
        assert_eq!(registry.total_applied(), 0);
        assert!(registry.audit_trail().is_empty());
    }

    #[test]
    fn test_default_entry_creation() {
        let entry = DefaultEntry::static_default("field", "val", "reason");
        assert_eq!(entry.strategy, DefaultStrategy::Static);
        assert!(!entry.affects_detection);

        let det_entry = DefaultEntry::detection_affecting("field", "val", "reason");
        assert!(det_entry.affects_detection);
    }
}
