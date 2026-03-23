//! # PV∅ Missing Data Detection
//!
//! Scans records for absent required fields and generates data quality
//! reports. In pharmacovigilance, missing data is a compliance signal:
//! an ICSR without a drug name or adverse event term is incomplete.
//!
//! ## Primitives
//! - ∅ (Void) — DOMINANT: detecting absence is the core operation
//! - ∂ (Boundary) — required vs optional field boundaries
//! - N (Quantity) — counting missing fields
//! - σ (Sequence) — patterns of co-missing fields
//!
//! ## ICSR Context
//!
//! E2B(R3) mandatory fields: reporter, patient, drug, adverse event,
//! seriousness criteria. Missing any of these triggers a data quality alert.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::void::{AbsenceReason, FieldRequirement, Maybe};

// ===============================================================
// FIELD SCHEMA
// ===============================================================

/// Describes a single field in a record schema.
/// Tier: T2-P (∂ + ∅)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDescriptor {
    /// Field name (e.g., "patient_age", "drug_name").
    pub name: String,
    /// Whether the field is mandatory, conditional, or optional.
    pub requirement: FieldRequirement,
    /// Human-readable description.
    pub description: String,
}

impl FieldDescriptor {
    /// Creates a new mandatory field descriptor.
    #[must_use]
    pub fn mandatory(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            requirement: FieldRequirement::Mandatory,
            description: description.to_string(),
        }
    }

    /// Creates a new optional field descriptor.
    #[must_use]
    pub fn optional(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            requirement: FieldRequirement::Optional,
            description: description.to_string(),
        }
    }

    /// Creates a new conditional field descriptor.
    #[must_use]
    pub fn conditional(name: &str, condition: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            requirement: FieldRequirement::Conditional(condition.to_string()),
            description: description.to_string(),
        }
    }
}

impl GroundsTo for FieldDescriptor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary, // ∂ — requirement boundary
            LexPrimitiva::Void,     // ∅ — absence specification
        ])
        .with_dominant(LexPrimitiva::Void, 0.75)
    }
}

// ===============================================================
// RECORD SCHEMA
// ===============================================================

/// Schema defining expected fields for a record type.
/// Tier: T2-C (∂ + ∅ + σ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSchema {
    /// Name of the schema (e.g., "ICSR", "PSUR_summary").
    pub name: String,
    /// Fields in this schema.
    pub fields: Vec<FieldDescriptor>,
}

impl RecordSchema {
    /// Creates a new schema.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fields: Vec::new(),
        }
    }

    /// Adds a field to the schema.
    #[must_use]
    pub fn with_field(mut self, field: FieldDescriptor) -> Self {
        self.fields.push(field);
        self
    }

    /// Returns all mandatory field names.
    #[must_use]
    pub fn mandatory_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|f| f.requirement.is_mandatory())
            .map(|f| f.name.as_str())
            .collect()
    }

    /// Returns the total number of fields.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

impl GroundsTo for RecordSchema {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary, // ∂ — field requirement boundaries
            LexPrimitiva::Void,     // ∅ — absence specification
            LexPrimitiva::Sequence, // σ — ordered field list
            LexPrimitiva::Quantity, // N — field count
        ])
        .with_dominant(LexPrimitiva::Void, 0.70)
    }
}

// ===============================================================
// MISSING FIELD ENTRY
// ===============================================================

/// A single missing field entry in a quality report.
/// Tier: T2-P (∅ + ∂)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingField {
    /// Name of the missing field.
    pub field_name: String,
    /// Why the field is absent.
    pub reason: AbsenceReason,
    /// The requirement level of this field.
    pub requirement: FieldRequirement,
}

impl GroundsTo for MissingField {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,     // ∅ — field is absent
            LexPrimitiva::Boundary, // ∂ — requirement level
        ])
        .with_dominant(LexPrimitiva::Void, 0.85)
    }
}

// ===============================================================
// IMPUTATION STRATEGY
// ===============================================================

/// Strategy for filling in missing values.
/// Tier: T2-P (∅ + μ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Imputation {
    /// No imputation — leave the value absent.
    None,
    /// Use a static default value.
    Default,
    /// Derive from other fields in the record.
    Derived,
    /// Use a model-based prediction.
    ModelBased,
}

impl GroundsTo for Imputation {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,    // ∅ — what we're filling
            LexPrimitiva::Mapping, // μ — void → value mapping
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// DATA QUALITY REPORT
// ===============================================================

/// Summary of missing data in a record.
/// Tier: T2-C (∅ + ∂ + N + Σ)
///
/// Provides a completeness score and categorized missing fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityReport {
    /// Schema against which the record was checked.
    pub schema_name: String,
    /// Missing fields found.
    pub missing_fields: Vec<MissingField>,
    /// Total fields checked.
    pub total_fields: usize,
    /// Total present fields.
    pub present_fields: usize,
    /// Timestamp of the check (epoch seconds).
    pub checked_at: u64,
}

impl DataQualityReport {
    /// Creates a new empty report.
    #[must_use]
    pub fn new(schema_name: &str, total_fields: usize, now: u64) -> Self {
        Self {
            schema_name: schema_name.to_string(),
            missing_fields: Vec::new(),
            total_fields,
            present_fields: total_fields,
            checked_at: now,
        }
    }

    /// Adds a missing field to the report.
    pub fn add_missing(
        &mut self,
        field_name: &str,
        reason: AbsenceReason,
        requirement: FieldRequirement,
    ) {
        self.missing_fields.push(MissingField {
            field_name: field_name.to_string(),
            reason,
            requirement,
        });
        self.present_fields = self.total_fields.saturating_sub(self.missing_fields.len());
    }

    /// Returns the completeness ratio (0.0 to 1.0).
    #[must_use]
    pub fn completeness(&self) -> f64 {
        if self.total_fields == 0 {
            return 1.0;
        }
        self.present_fields as f64 / self.total_fields as f64
    }

    /// Returns true if all required fields are present.
    ///
    /// A field is "required" if it is mandatory, or conditional with
    /// its condition met. Only optional fields can be missing without
    /// affecting completeness.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self
            .missing_fields
            .iter()
            .any(|f| !f.requirement.is_optional())
    }

    /// Returns missing fields that are non-optional (mandatory or conditional-met).
    #[must_use]
    pub fn required_missing(&self) -> Vec<&MissingField> {
        self.missing_fields
            .iter()
            .filter(|f| !f.requirement.is_optional())
            .collect()
    }

    /// Returns the count of missing fields.
    #[must_use]
    pub fn missing_count(&self) -> usize {
        self.missing_fields.len()
    }

    /// Returns the count of actionable missing fields.
    #[must_use]
    pub fn actionable_count(&self) -> usize {
        self.missing_fields
            .iter()
            .filter(|f| f.reason.is_actionable())
            .count()
    }
}

impl GroundsTo for DataQualityReport {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,     // ∅ — missing data tracking
            LexPrimitiva::Boundary, // ∂ — requirement boundaries
            LexPrimitiva::Quantity, // N — counts
            LexPrimitiva::Sum,      // Σ — completeness aggregation
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ===============================================================
// MISSING PATTERN
// ===============================================================

/// Tracks which fields are commonly missing together.
/// Tier: T2-C (∅ + σ + κ + N)
///
/// If "reporter_name" and "reporter_country" are always missing together,
/// they form a co-missing pattern — likely the reporter section is skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingPattern {
    /// Fields that co-occur as missing.
    pub field_group: Vec<String>,
    /// Number of records exhibiting this pattern.
    pub occurrence_count: u64,
    /// Proportion of all records with this pattern.
    pub prevalence: f64,
}

impl GroundsTo for MissingPattern {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — absence pattern
            LexPrimitiva::Sequence,   // σ — field group ordering
            LexPrimitiva::Comparison, // κ — pattern matching
            LexPrimitiva::Quantity,   // N — occurrence count
        ])
        .with_dominant(LexPrimitiva::Void, 0.75)
    }
}

// ===============================================================
// MISSING FIELD DETECTOR
// ===============================================================

/// Detects missing fields in records against a schema.
/// Tier: T3 (∅ + ∂ + N + σ + κ + Σ)
///
/// The detector scans records represented as field name → Maybe<value>
/// maps, compares against a schema, and produces quality reports.
/// It also tracks co-missing patterns across multiple records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingFieldDetector {
    /// Schema to check against.
    schema: RecordSchema,
    /// Accumulated pattern counts: sorted field group → count.
    pattern_counts: HashMap<Vec<String>, u64>,
    /// Total records processed.
    records_processed: u64,
}

impl MissingFieldDetector {
    /// Creates a new detector with the given schema.
    #[must_use]
    pub fn new(schema: RecordSchema) -> Self {
        Self {
            schema,
            pattern_counts: HashMap::new(),
            records_processed: 0,
        }
    }

    /// Checks a record for missing fields.
    ///
    /// `record` maps field names to `Maybe<String>` values.
    /// Fields not present in the map are treated as absent (NotProvided).
    pub fn check(
        &mut self,
        record: &HashMap<String, Maybe<String>>,
        conditions: &HashMap<String, bool>,
        now: u64,
    ) -> DataQualityReport {
        let mut report = DataQualityReport::new(&self.schema.name, self.schema.fields.len(), now);

        let mut missing_group: Vec<String> = Vec::new();

        for field in &self.schema.fields {
            let present = record.get(&field.name).map_or(false, |m| m.is_present());
            let condition_met = match &field.requirement {
                FieldRequirement::Conditional(cond) => {
                    conditions.get(cond).copied().unwrap_or(false)
                }
                _ => false, // non-conditional fields don't use this
            };

            if !field.requirement.is_satisfied(present, condition_met) {
                let reason = record
                    .get(&field.name)
                    .and_then(|m| m.absence_reason().cloned())
                    .unwrap_or(AbsenceReason::NotProvided);

                report.add_missing(&field.name, reason, field.requirement.clone());
                missing_group.push(field.name.clone());
            }
        }

        // Track co-missing patterns
        if !missing_group.is_empty() {
            missing_group.sort();
            *self.pattern_counts.entry(missing_group).or_insert(0) += 1;
        }
        self.records_processed += 1;

        report
    }

    /// Returns detected co-missing patterns sorted by prevalence.
    #[must_use]
    pub fn patterns(&self) -> Vec<MissingPattern> {
        if self.records_processed == 0 {
            return Vec::new();
        }

        let mut patterns: Vec<MissingPattern> = self
            .pattern_counts
            .iter()
            .map(|(group, &count)| MissingPattern {
                field_group: group.clone(),
                occurrence_count: count,
                prevalence: count as f64 / self.records_processed as f64,
            })
            .collect();

        patterns.sort_by(|a, b| {
            b.prevalence
                .partial_cmp(&a.prevalence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        patterns
    }

    /// Returns the number of records processed.
    #[must_use]
    pub fn records_processed(&self) -> u64 {
        self.records_processed
    }

    /// Returns the schema name.
    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema.name
    }
}

impl GroundsTo for MissingFieldDetector {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — absence detection
            LexPrimitiva::Boundary,   // ∂ — requirement checking
            LexPrimitiva::Quantity,   // N — missing counts
            LexPrimitiva::Sequence,   // σ — field ordering
            LexPrimitiva::Comparison, // κ — pattern matching
            LexPrimitiva::Sum,        // Σ — completeness aggregation
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

    fn icsr_schema() -> RecordSchema {
        RecordSchema::new("ICSR")
            .with_field(FieldDescriptor::mandatory("drug_name", "Name of the drug"))
            .with_field(FieldDescriptor::mandatory(
                "adverse_event",
                "Adverse event term",
            ))
            .with_field(FieldDescriptor::mandatory(
                "reporter_type",
                "Type of reporter",
            ))
            .with_field(FieldDescriptor::optional("patient_age", "Patient age"))
            .with_field(FieldDescriptor::conditional(
                "pregnancy_status",
                "patient_sex_female",
                "Pregnancy status (required for female patients)",
            ))
    }

    fn complete_record() -> HashMap<String, Maybe<String>> {
        let mut rec = HashMap::new();
        rec.insert("drug_name".into(), Maybe::Present("aspirin".into()));
        rec.insert("adverse_event".into(), Maybe::Present("headache".into()));
        rec.insert("reporter_type".into(), Maybe::Present("physician".into()));
        rec.insert("patient_age".into(), Maybe::Present("45".into()));
        rec
    }

    // --- Grounding tests ---

    #[test]
    fn test_field_descriptor_grounding() {
        let comp = FieldDescriptor::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_record_schema_grounding() {
        let comp = RecordSchema::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
    }

    #[test]
    fn test_data_quality_report_grounding() {
        let comp = DataQualityReport::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_missing_field_detector_grounding() {
        let comp = MissingFieldDetector::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
    }

    #[test]
    fn test_imputation_grounding() {
        let comp = Imputation::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    // --- Detection tests ---

    #[test]
    fn test_complete_record_passes() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);
        let record = complete_record();
        let conditions = HashMap::new();

        let report = detector.check(&record, &conditions, 1000);
        assert!(report.is_complete());
        assert_eq!(report.missing_count(), 0);
        assert!((report.completeness() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_missing_mandatory_field() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);

        let mut record = complete_record();
        record.remove("drug_name");

        let conditions = HashMap::new();
        let report = detector.check(&record, &conditions, 1000);

        assert!(!report.is_complete());
        assert_eq!(report.missing_count(), 1);
        assert_eq!(report.required_missing().len(), 1);
        assert_eq!(report.required_missing()[0].field_name, "drug_name");
    }

    #[test]
    fn test_missing_optional_field_still_complete() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);

        let mut record = complete_record();
        record.remove("patient_age");

        let conditions = HashMap::new();
        let report = detector.check(&record, &conditions, 1000);

        // Missing optional field doesn't affect completeness for mandatory check
        assert!(report.is_complete());
        // But it does appear in the missing list since schema expects it
        // Actually, optional fields that are absent are not flagged by is_satisfied
        assert_eq!(report.missing_count(), 0);
    }

    #[test]
    fn test_conditional_field_required_when_condition_met() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);

        let record = complete_record();
        let mut conditions = HashMap::new();
        // Use the condition name ("patient_sex_female"), not the field name
        conditions.insert("patient_sex_female".into(), true);

        let report = detector.check(&record, &conditions, 1000);
        // pregnancy_status is conditional, condition is met, but field is absent
        assert!(!report.is_complete());
    }

    #[test]
    fn test_conditional_field_not_required_when_condition_not_met() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);

        let record = complete_record();
        let mut conditions = HashMap::new();
        // Condition not met → pregnancy_status not required
        conditions.insert("patient_sex_female".into(), false);

        let report = detector.check(&record, &conditions, 1000);
        assert!(report.is_complete());
    }

    #[test]
    fn test_absent_with_reason() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);

        let mut record = complete_record();
        record.insert("drug_name".into(), Maybe::Absent(AbsenceReason::Redacted));

        let conditions = HashMap::new();
        let report = detector.check(&record, &conditions, 1000);

        assert!(!report.is_complete());
        assert_eq!(report.missing_fields[0].reason, AbsenceReason::Redacted);
    }

    #[test]
    fn test_completeness_ratio() {
        let mut report = DataQualityReport::new("test", 10, 1000);
        assert!((report.completeness() - 1.0).abs() < f64::EPSILON);

        report.add_missing(
            "f1",
            AbsenceReason::NotProvided,
            FieldRequirement::Mandatory,
        );
        report.add_missing("f2", AbsenceReason::Unknown, FieldRequirement::Optional);
        // 8/10 = 0.8
        assert!((report.completeness() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_actionable_count() {
        let mut report = DataQualityReport::new("test", 5, 1000);
        report.add_missing(
            "f1",
            AbsenceReason::NotProvided,
            FieldRequirement::Mandatory,
        );
        report.add_missing("f2", AbsenceReason::Redacted, FieldRequirement::Optional);
        report.add_missing(
            "f3",
            AbsenceReason::Error("timeout".into()),
            FieldRequirement::Mandatory,
        );

        assert_eq!(report.actionable_count(), 2); // NotProvided + Error
    }

    #[test]
    fn test_co_missing_patterns() {
        let schema = icsr_schema();
        let mut detector = MissingFieldDetector::new(schema);
        let conditions = HashMap::new();

        // Record 1: missing drug_name and adverse_event
        let mut r1 = HashMap::new();
        r1.insert("reporter_type".into(), Maybe::Present("physician".into()));
        detector.check(&r1, &conditions, 1000);

        // Record 2: same pattern
        let mut r2 = HashMap::new();
        r2.insert("reporter_type".into(), Maybe::Present("nurse".into()));
        detector.check(&r2, &conditions, 1001);

        let patterns = detector.patterns();
        assert!(!patterns.is_empty());
        // The most prevalent pattern should involve drug_name and adverse_event
        assert!(patterns[0].occurrence_count >= 2);
    }

    #[test]
    fn test_empty_schema() {
        let schema = RecordSchema::new("empty");
        let mut detector = MissingFieldDetector::new(schema);

        let record = HashMap::new();
        let conditions = HashMap::new();
        let report = detector.check(&record, &conditions, 1000);

        assert!(report.is_complete());
        assert_eq!(report.missing_count(), 0);
    }
}
