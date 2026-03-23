//! # PVOS Drivers
//!
//! Data source adapters — the bottom layer of the OS stack.
//! Drivers translate external data formats into the PVOS internal model.
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────┐
//! │                    DRIVERS                         │
//! │   FAERS │ VigiBase │ EudraVigilance │ Sponsor DBs │
//! └────────────────────────────────────────────────────┘
//! ```
//!
//! Each driver implements the `DataSourceDriver` trait,
//! providing a unified `ingest()` interface regardless of source format.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::PvosError;

/// Known pharmacovigilance data sources.
/// Tier: T2-P (λ + μ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataSourceKind {
    /// FDA Adverse Event Reporting System.
    Faers,
    /// WHO Global Individual Case Safety Reports.
    VigiBase,
    /// EU Adverse Drug Reaction Reporting System.
    EudraVigilance,
    /// Sponsor/company proprietary database.
    SponsorDb(String),
    /// Custom data source.
    Custom(String),
}

impl DataSourceKind {
    /// Returns a human-readable name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Faers => "FAERS",
            Self::VigiBase => "VigiBase",
            Self::EudraVigilance => "EudraVigilance",
            Self::SponsorDb(name) => name,
            Self::Custom(name) => name,
        }
    }
}

impl GroundsTo for DataSourceKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Location, LexPrimitiva::Mapping])
    }
}

/// A raw record ingested from a data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRecord {
    /// Source this record came from.
    pub source: DataSourceKind,
    /// Key-value fields (normalized from source format).
    pub fields: HashMap<String, String>,
    /// Raw content if structured parsing failed.
    pub raw: Option<String>,
}

/// A normalized case record after driver transformation.
/// Tier: T2-C (μ + σ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedCase {
    /// Drug name(s).
    pub drugs: Vec<String>,
    /// Adverse event(s).
    pub events: Vec<String>,
    /// Patient age (if available).
    pub patient_age: Option<u32>,
    /// Patient sex (if available).
    pub patient_sex: Option<String>,
    /// Seriousness criteria met.
    pub serious_criteria: Vec<String>,
    /// Reporter type.
    pub reporter: Option<String>,
    /// Source data origin.
    pub source: DataSourceKind,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

impl GroundsTo for NormalizedCase {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Mapping,
            LexPrimitiva::Sequence,
            LexPrimitiva::Persistence,
        ])
        .with_dominant(LexPrimitiva::Mapping, 0.90)
    }
}

/// Data source driver trait.
///
/// Drivers transform raw data from external sources into
/// normalized case records. This is the μ-abstraction that
/// makes all data sources look the same to the kernel.
pub trait DataSourceDriver {
    /// Returns the data source kind this driver handles.
    fn source_kind(&self) -> DataSourceKind;

    /// Transforms raw input into a normalized case.
    ///
    /// # Errors
    /// Returns `Err` if the input cannot be parsed.
    fn normalize(&self, raw: &str) -> Result<NormalizedCase, PvosError>;

    /// Validates that a raw record meets minimum quality standards.
    fn validate(&self, record: &NormalizedCase) -> bool {
        // Default: require at least one drug and one event
        !record.drugs.is_empty() && !record.events.is_empty()
    }
}

/// Built-in FAERS driver.
/// Maps FDA FAERS JSON/CSV fields to normalized cases.
#[derive(Debug, Clone, Default)]
pub struct FaersDriver;

impl DataSourceDriver for FaersDriver {
    fn source_kind(&self) -> DataSourceKind {
        DataSourceKind::Faers
    }

    fn normalize(&self, raw: &str) -> Result<NormalizedCase, PvosError> {
        // Parse as JSON key-value pairs
        let fields: HashMap<String, String> = serde_json::from_str(raw)
            .map_err(|e| PvosError::DriverError(format!("FAERS parse error: {e}")))?;

        let drugs = fields
            .get("drugname")
            .map(|d| d.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let events = fields
            .get("reactions")
            .or(fields.get("event"))
            .map(|e| e.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let patient_age = fields.get("patient_age").and_then(|a| a.parse().ok());

        let patient_sex = fields.get("patient_sex").cloned();

        let serious_criteria = fields
            .get("serious")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let reporter = fields.get("reporter_type").cloned();

        Ok(NormalizedCase {
            drugs,
            events,
            patient_age,
            patient_sex,
            serious_criteria,
            reporter,
            source: DataSourceKind::Faers,
            metadata: fields,
        })
    }
}

/// Generic JSON driver for arbitrary sources.
#[derive(Debug, Clone)]
pub struct GenericJsonDriver {
    source: DataSourceKind,
    drug_field: String,
    event_field: String,
}

impl GenericJsonDriver {
    /// Creates a generic driver with field mappings.
    #[must_use]
    pub fn new(source: DataSourceKind, drug_field: &str, event_field: &str) -> Self {
        Self {
            source,
            drug_field: drug_field.to_string(),
            event_field: event_field.to_string(),
        }
    }
}

impl DataSourceDriver for GenericJsonDriver {
    fn source_kind(&self) -> DataSourceKind {
        self.source.clone()
    }

    fn normalize(&self, raw: &str) -> Result<NormalizedCase, PvosError> {
        let fields: HashMap<String, String> = serde_json::from_str(raw)
            .map_err(|e| PvosError::DriverError(format!("JSON parse error: {e}")))?;

        let drugs = fields
            .get(&self.drug_field)
            .map(|d| d.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let events = fields
            .get(&self.event_field)
            .map(|e| e.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(NormalizedCase {
            drugs,
            events,
            patient_age: None,
            patient_sex: None,
            serious_criteria: Vec::new(),
            reporter: None,
            source: self.source.clone(),
            metadata: fields,
        })
    }
}

/// Driver registry — maps data sources to their drivers.
/// Tier: T2-P (μ)
#[derive(Default)]
pub struct DriverRegistry {
    drivers: Vec<Box<dyn DataSourceDriver>>,
}

impl DriverRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    /// Creates a registry with default drivers (FAERS).
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(FaersDriver));
        reg
    }

    /// Registers a new driver.
    pub fn register(&mut self, driver: Box<dyn DataSourceDriver>) {
        self.drivers.push(driver);
    }

    /// Finds a driver for the given source kind.
    pub fn driver_for(&self, source: &DataSourceKind) -> Option<&dyn DataSourceDriver> {
        self.drivers
            .iter()
            .find(|d| &d.source_kind() == source)
            .map(|d| d.as_ref())
    }

    /// Normalizes raw data using the appropriate driver.
    ///
    /// # Errors
    /// Returns `Err` if no driver is registered for the source.
    pub fn normalize(
        &self,
        source: &DataSourceKind,
        raw: &str,
    ) -> Result<NormalizedCase, PvosError> {
        let driver = self
            .driver_for(source)
            .ok_or_else(|| PvosError::NoDriver(source.name().to_string()))?;
        driver.normalize(raw)
    }

    /// Number of registered drivers.
    #[must_use]
    pub fn count(&self) -> usize {
        self.drivers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_data_source_kind_grounding() {
        let comp = DataSourceKind::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_normalized_case_grounding() {
        let comp = NormalizedCase::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Mapping));
    }

    #[test]
    fn test_faers_driver_normalize() {
        let driver = FaersDriver;
        let raw = r#"{"drugname": "aspirin", "reactions": "headache, nausea"}"#;
        let result = driver.normalize(raw);
        assert!(result.is_ok());
        if let Ok(case) = result {
            assert_eq!(case.drugs, vec!["aspirin"]);
            assert_eq!(case.events, vec!["headache", "nausea"]);
            assert_eq!(case.source, DataSourceKind::Faers);
        }
    }

    #[test]
    fn test_faers_driver_validate() {
        let driver = FaersDriver;
        let valid = NormalizedCase {
            drugs: vec!["aspirin".into()],
            events: vec!["headache".into()],
            patient_age: None,
            patient_sex: None,
            serious_criteria: Vec::new(),
            reporter: None,
            source: DataSourceKind::Faers,
            metadata: HashMap::new(),
        };
        assert!(driver.validate(&valid));

        let invalid = NormalizedCase {
            drugs: Vec::new(),
            events: vec!["headache".into()],
            patient_age: None,
            patient_sex: None,
            serious_criteria: Vec::new(),
            reporter: None,
            source: DataSourceKind::Faers,
            metadata: HashMap::new(),
        };
        assert!(!driver.validate(&invalid));
    }

    #[test]
    fn test_generic_json_driver() {
        let driver = GenericJsonDriver::new(
            DataSourceKind::SponsorDb("test".into()),
            "medication",
            "adverse_event",
        );
        let raw = r#"{"medication": "ibuprofen", "adverse_event": "rash"}"#;
        let result = driver.normalize(raw);
        assert!(result.is_ok());
        if let Ok(case) = result {
            assert_eq!(case.drugs, vec!["ibuprofen"]);
            assert_eq!(case.events, vec!["rash"]);
        }
    }

    #[test]
    fn test_driver_registry() {
        let registry = DriverRegistry::with_defaults();
        assert_eq!(registry.count(), 1);
        assert!(registry.driver_for(&DataSourceKind::Faers).is_some());
        assert!(registry.driver_for(&DataSourceKind::VigiBase).is_none());
    }

    #[test]
    fn test_registry_normalize() {
        let registry = DriverRegistry::with_defaults();
        let raw = r#"{"drugname": "metformin", "reactions": "diarrhea"}"#;
        let result = registry.normalize(&DataSourceKind::Faers, raw);
        assert!(result.is_ok());

        let missing = registry.normalize(&DataSourceKind::VigiBase, raw);
        assert!(missing.is_err());
    }
}
