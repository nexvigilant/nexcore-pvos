//! # PVEX Registry — Core Entity Registration
//!
//! Foundational registry for tracking entity existence in the PVOS.
//! Every entity that "exists" in the system is registered here with
//! metadata, kind classification, and lifecycle tracking.
//!
//! ## T1 Grounding (dominant: ∃ Existence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | ∃ | Existence | 0.40 — entity present/absent |
//! | μ | Mapping | 0.25 — id→entity |
//! | π | Persistence | 0.20 — durable registration |
//! | σ | Sequence | 0.15 — registration order |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique identifier for a registered entity.
///
/// Tier: T2-P (∃ newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Unique identifier for a registry instance.
///
/// Tier: T2-P (∃ newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegistryId(pub u64);

// ═══════════════════════════════════════════════════════════
// ENTITY KIND
// ═══════════════════════════════════════════════════════════

/// Classification of entity types in the PVOS.
///
/// Tier: T2-P (∃ + ∂ — bounded existence categories)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    /// Safety case / ICSR.
    Case,
    /// Detected signal.
    Signal,
    /// Workflow instance.
    Workflow,
    /// Regulatory submission.
    Submission,
    /// ML model.
    Model,
    /// Reactive stream.
    Stream,
    /// Metric collector.
    Metric,
    /// Custom entity type.
    Custom,
}

impl EntityKind {
    /// Returns the kind name as a string.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Case => "case",
            Self::Signal => "signal",
            Self::Workflow => "workflow",
            Self::Submission => "submission",
            Self::Model => "model",
            Self::Stream => "stream",
            Self::Metric => "metric",
            Self::Custom => "custom",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// REGISTRY ENTRY
// ═══════════════════════════════════════════════════════════

/// A registered entity with existence metadata.
///
/// Tier: T2-C (∃ + μ + π + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Entity identifier.
    pub id: EntityId,
    /// Entity kind.
    pub kind: EntityKind,
    /// Human-readable label.
    pub label: String,
    /// When the entity was registered.
    pub registered_at: u64,
    /// When the entity was last seen.
    pub last_seen_at: u64,
    /// Whether the entity is currently active.
    pub active: bool,
    /// Optional parent entity.
    pub parent: Option<EntityId>,
    /// Metadata key-value pairs.
    pub metadata: HashMap<String, String>,
}

impl RegistryEntry {
    /// Creates a new registry entry.
    #[must_use]
    pub fn new(id: EntityId, kind: EntityKind, label: &str, timestamp: u64) -> Self {
        Self {
            id,
            kind,
            label: label.to_string(),
            registered_at: timestamp,
            last_seen_at: timestamp,
            active: true,
            parent: None,
            metadata: HashMap::new(),
        }
    }

    /// Sets the parent entity.
    #[must_use]
    pub fn with_parent(mut self, parent: EntityId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Adds a metadata entry.
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Gets a metadata value.
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|v| v.as_str())
    }

    /// Updates the last-seen timestamp.
    pub fn touch(&mut self, timestamp: u64) {
        self.last_seen_at = timestamp;
    }

    /// Marks the entity as inactive.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Returns how long since registration.
    #[must_use]
    pub fn age(&self, now: u64) -> u64 {
        now.saturating_sub(self.registered_at)
    }

    /// Returns how long since last seen.
    #[must_use]
    pub fn staleness(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_seen_at)
    }
}

impl GroundsTo for RegistryEntry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,   // ∃ — entity exists
            LexPrimitiva::Mapping,     // μ — id→metadata
            LexPrimitiva::Persistence, // π — durable registration
            LexPrimitiva::Sequence,    // σ — temporal ordering
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// REGISTRATION RESULT
// ═══════════════════════════════════════════════════════════

/// Outcome of an entity registration attempt.
///
/// Tier: T2-P (∃ — existence outcome)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrationResult {
    /// Entity was registered successfully.
    Registered { id: EntityId },
    /// Entity already exists.
    AlreadyExists { id: EntityId },
    /// Registration was rejected.
    Rejected { reason: String },
    /// Registry is at capacity.
    AtCapacity,
}

impl RegistrationResult {
    /// Whether registration succeeded.
    #[must_use]
    pub fn is_registered(&self) -> bool {
        matches!(self, Self::Registered { .. })
    }

    /// Returns the entity ID if registered or already exists.
    #[must_use]
    pub fn entity_id(&self) -> Option<EntityId> {
        match self {
            Self::Registered { id } | Self::AlreadyExists { id } => Some(*id),
            _ => None,
        }
    }
}

/// Outcome of an entity deregistration attempt.
///
/// Tier: T2-P (∃ → ∅)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Deregistration {
    /// Entity was removed.
    Removed { id: EntityId },
    /// Entity not found.
    NotFound,
    /// Entity is protected from removal.
    Protected { reason: String },
}

impl Deregistration {
    /// Whether the entity was removed.
    #[must_use]
    pub fn is_removed(&self) -> bool {
        matches!(self, Self::Removed { .. })
    }
}

// ═══════════════════════════════════════════════════════════
// ENTITY REGISTRY
// ═══════════════════════════════════════════════════════════

/// Core entity registry — manages existence of all PVOS entities.
///
/// Tier: T2-C (∃ + μ + π + σ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRegistry {
    /// Registry identifier.
    pub id: RegistryId,
    /// Registry name.
    pub name: String,
    /// Registered entities indexed by ID.
    entries: HashMap<u64, RegistryEntry>,
    /// Kind filter (None = all kinds allowed).
    kind_filter: Option<EntityKind>,
    /// Maximum capacity (0 = unlimited).
    max_capacity: usize,
    /// Next entity ID.
    next_id: u64,
    /// Total registrations ever.
    total_registered: u64,
    /// Total deregistrations ever.
    total_deregistered: u64,
}

impl EntityRegistry {
    /// Creates a new entity registry.
    #[must_use]
    pub fn new(id: RegistryId, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            entries: HashMap::new(),
            kind_filter: None,
            max_capacity: 0,
            next_id: 1,
            total_registered: 0,
            total_deregistered: 0,
        }
    }

    /// Restricts the registry to a single entity kind.
    #[must_use]
    pub fn with_kind_filter(mut self, kind: EntityKind) -> Self {
        self.kind_filter = Some(kind);
        self
    }

    /// Sets maximum capacity.
    #[must_use]
    pub fn with_capacity(mut self, max: usize) -> Self {
        self.max_capacity = max;
        self
    }

    /// Registers a new entity. Returns auto-assigned ID.
    pub fn register(
        &mut self,
        kind: EntityKind,
        label: &str,
        timestamp: u64,
    ) -> RegistrationResult {
        // Check kind filter
        if let Some(allowed) = &self.kind_filter {
            if *allowed != kind {
                return RegistrationResult::Rejected {
                    reason: format!(
                        "registry '{}' only accepts {:?}, got {:?}",
                        self.name, allowed, kind
                    ),
                };
            }
        }

        // Check capacity
        if self.max_capacity > 0 && self.entries.len() >= self.max_capacity {
            return RegistrationResult::AtCapacity;
        }

        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.total_registered += 1;

        let entry = RegistryEntry::new(id, kind, label, timestamp);
        self.entries.insert(id.0, entry);

        RegistrationResult::Registered { id }
    }

    /// Registers with a specific ID.
    pub fn register_with_id(
        &mut self,
        id: EntityId,
        kind: EntityKind,
        label: &str,
        timestamp: u64,
    ) -> RegistrationResult {
        if self.entries.contains_key(&id.0) {
            return RegistrationResult::AlreadyExists { id };
        }

        if self.max_capacity > 0 && self.entries.len() >= self.max_capacity {
            return RegistrationResult::AtCapacity;
        }

        self.total_registered += 1;
        let entry = RegistryEntry::new(id, kind, label, timestamp);
        self.entries.insert(id.0, entry);

        // Update next_id if needed
        if id.0 >= self.next_id {
            self.next_id = id.0 + 1;
        }

        RegistrationResult::Registered { id }
    }

    /// Deregisters an entity.
    pub fn deregister(&mut self, id: EntityId) -> Deregistration {
        if self.entries.remove(&id.0).is_some() {
            self.total_deregistered += 1;
            Deregistration::Removed { id }
        } else {
            Deregistration::NotFound
        }
    }

    /// Checks whether an entity exists (the fundamental ∃ operation).
    #[must_use]
    pub fn exists(&self, id: EntityId) -> bool {
        self.entries.contains_key(&id.0)
    }

    /// Gets an entry by ID.
    #[must_use]
    pub fn get(&self, id: EntityId) -> Option<&RegistryEntry> {
        self.entries.get(&id.0)
    }

    /// Gets a mutable entry by ID.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut RegistryEntry> {
        self.entries.get_mut(&id.0)
    }

    /// Returns all active entries.
    #[must_use]
    pub fn active_entries(&self) -> Vec<&RegistryEntry> {
        self.entries.values().filter(|e| e.active).collect()
    }

    /// Returns entries of a specific kind.
    #[must_use]
    pub fn entries_of_kind(&self, kind: EntityKind) -> Vec<&RegistryEntry> {
        self.entries.values().filter(|e| e.kind == kind).collect()
    }

    /// Returns the number of registered entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all entity IDs.
    #[must_use]
    pub fn ids(&self) -> Vec<EntityId> {
        self.entries.keys().map(|&k| EntityId(k)).collect()
    }

    /// Total registrations ever.
    #[must_use]
    pub fn total_registered(&self) -> u64 {
        self.total_registered
    }

    /// Total deregistrations ever.
    #[must_use]
    pub fn total_deregistered(&self) -> u64 {
        self.total_deregistered
    }

    /// Returns all entries (for iteration).
    #[must_use]
    pub fn all_entries(&self) -> Vec<&RegistryEntry> {
        self.entries.values().collect()
    }
}

impl GroundsTo for EntityRegistry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,   // ∃ — entity existence
            LexPrimitiva::Mapping,     // μ — id→entity
            LexPrimitiva::Persistence, // π — durable registration
            LexPrimitiva::Sequence,    // σ — registration order
            LexPrimitiva::Boundary,    // ∂ — kind filter, capacity
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_registry_entry_grounding() {
        let comp = RegistryEntry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
    }

    #[test]
    fn test_entity_registry_grounding() {
        let comp = EntityRegistry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_entity_kind_names() {
        assert_eq!(EntityKind::Case.name(), "case");
        assert_eq!(EntityKind::Signal.name(), "signal");
        assert_eq!(EntityKind::Workflow.name(), "workflow");
    }

    #[test]
    fn test_registry_entry_lifecycle() {
        let mut entry = RegistryEntry::new(EntityId(1), EntityKind::Case, "case-001", 1000);
        assert!(entry.active);
        assert_eq!(entry.age(2000), 1000);
        assert_eq!(entry.staleness(2000), 1000);

        entry.touch(1500);
        assert_eq!(entry.staleness(2000), 500);

        entry.set_metadata("drug", "aspirin");
        assert_eq!(entry.get_metadata("drug"), Some("aspirin"));

        entry.deactivate();
        assert!(!entry.active);
    }

    #[test]
    fn test_registry_register() {
        let mut reg = EntityRegistry::new(RegistryId(1), "cases");
        let result = reg.register(EntityKind::Case, "case-001", 1000);
        assert!(result.is_registered());
        assert_eq!(result.entity_id(), Some(EntityId(1)));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_registry_exists() {
        let mut reg = EntityRegistry::new(RegistryId(1), "signals");
        let result = reg.register(EntityKind::Signal, "sig-001", 1000);
        let id = result.entity_id().unwrap_or(EntityId(0));

        assert!(reg.exists(id));
        assert!(!reg.exists(EntityId(999)));
    }

    #[test]
    fn test_registry_deregister() {
        let mut reg = EntityRegistry::new(RegistryId(1), "test");
        let result = reg.register(EntityKind::Case, "c1", 1000);
        let id = result.entity_id().unwrap_or(EntityId(0));

        let dereg = reg.deregister(id);
        assert!(dereg.is_removed());
        assert!(!reg.exists(id));

        let dereg2 = reg.deregister(id);
        assert!(!dereg2.is_removed());
    }

    #[test]
    fn test_registry_kind_filter() {
        let mut reg =
            EntityRegistry::new(RegistryId(1), "cases-only").with_kind_filter(EntityKind::Case);

        let ok = reg.register(EntityKind::Case, "c1", 1000);
        assert!(ok.is_registered());

        let rejected = reg.register(EntityKind::Signal, "s1", 2000);
        assert!(!rejected.is_registered());
    }

    #[test]
    fn test_registry_capacity() {
        let mut reg = EntityRegistry::new(RegistryId(1), "bounded").with_capacity(2);

        reg.register(EntityKind::Case, "c1", 1000);
        reg.register(EntityKind::Case, "c2", 2000);

        let result = reg.register(EntityKind::Case, "c3", 3000);
        assert!(matches!(result, RegistrationResult::AtCapacity));
    }

    #[test]
    fn test_registry_register_with_id() {
        let mut reg = EntityRegistry::new(RegistryId(1), "test");

        let result = reg.register_with_id(EntityId(42), EntityKind::Case, "c42", 1000);
        assert!(result.is_registered());
        assert!(reg.exists(EntityId(42)));

        let dup = reg.register_with_id(EntityId(42), EntityKind::Case, "c42-dup", 2000);
        assert!(matches!(dup, RegistrationResult::AlreadyExists { .. }));
    }

    #[test]
    fn test_registry_entries_of_kind() {
        let mut reg = EntityRegistry::new(RegistryId(1), "mixed");
        reg.register(EntityKind::Case, "c1", 1000);
        reg.register(EntityKind::Signal, "s1", 2000);
        reg.register(EntityKind::Case, "c2", 3000);

        let cases = reg.entries_of_kind(EntityKind::Case);
        assert_eq!(cases.len(), 2);

        let signals = reg.entries_of_kind(EntityKind::Signal);
        assert_eq!(signals.len(), 1);
    }

    #[test]
    fn test_registry_counters() {
        let mut reg = EntityRegistry::new(RegistryId(1), "test");
        let r1 = reg.register(EntityKind::Case, "c1", 1000);
        reg.register(EntityKind::Case, "c2", 2000);

        let id1 = r1.entity_id().unwrap_or(EntityId(0));
        reg.deregister(id1);

        assert_eq!(reg.total_registered(), 2);
        assert_eq!(reg.total_deregistered(), 1);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_registration_result_properties() {
        let registered = RegistrationResult::Registered { id: EntityId(1) };
        assert!(registered.is_registered());
        assert_eq!(registered.entity_id(), Some(EntityId(1)));

        let already = RegistrationResult::AlreadyExists { id: EntityId(2) };
        assert!(!already.is_registered());
        assert_eq!(already.entity_id(), Some(EntityId(2)));

        let rejected = RegistrationResult::Rejected {
            reason: "nope".into(),
        };
        assert!(rejected.entity_id().is_none());
    }
}
