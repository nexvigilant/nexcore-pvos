//! # PVEX Discovery — Entity Discovery Service
//!
//! Enables searching and finding entities across registries.
//! Supports queries with filters, indexed lookups, and
//! multi-criteria matching for entity existence verification.
//!
//! ## T1 Grounding (dominant: ∃ Existence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | ∃ | Existence | 0.35 — does entity exist? |
//! | μ | Mapping | 0.25 — query→results |
//! | σ | Sequence | 0.20 — ordered results |
//! | κ | Comparison | 0.20 — filter matching |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::registry::{EntityId, EntityKind, RegistryEntry};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// DISCOVERY QUERY
// ═══════════════════════════════════════════════════════════

/// A query to discover entities by criteria.
///
/// Tier: T2-P (∃ + κ — existence-based search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryQuery {
    /// Filter by entity kind.
    pub kind: Option<EntityKind>,
    /// Filter by label substring.
    pub label_contains: Option<String>,
    /// Filter by metadata key existence.
    pub has_metadata_key: Option<String>,
    /// Filter by metadata key-value pair.
    pub metadata_match: Option<(String, String)>,
    /// Filter by active status.
    pub active_only: bool,
    /// Maximum results.
    pub limit: Option<usize>,
    /// Minimum age (timestamp units).
    pub min_age: Option<u64>,
    /// Maximum staleness (timestamp units).
    pub max_staleness: Option<u64>,
}

impl DiscoveryQuery {
    /// Creates an empty query (matches all).
    #[must_use]
    pub fn all() -> Self {
        Self {
            kind: None,
            label_contains: None,
            has_metadata_key: None,
            metadata_match: None,
            active_only: false,
            limit: None,
            min_age: None,
            max_staleness: None,
        }
    }

    /// Filters by entity kind.
    #[must_use]
    pub fn with_kind(mut self, kind: EntityKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Filters by label substring.
    #[must_use]
    pub fn with_label(mut self, label: &str) -> Self {
        self.label_contains = Some(label.to_string());
        self
    }

    /// Only active entities.
    #[must_use]
    pub fn active(mut self) -> Self {
        self.active_only = true;
        self
    }

    /// Filters by metadata key.
    #[must_use]
    pub fn with_metadata_key(mut self, key: &str) -> Self {
        self.has_metadata_key = Some(key.to_string());
        self
    }

    /// Filters by metadata key-value pair.
    #[must_use]
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata_match = Some((key.to_string(), value.to_string()));
        self
    }

    /// Limits results.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Checks if an entry matches this query.
    #[must_use]
    pub fn matches(&self, entry: &RegistryEntry, now: u64) -> bool {
        if let Some(kind) = &self.kind {
            if entry.kind != *kind {
                return false;
            }
        }

        if self.active_only && !entry.active {
            return false;
        }

        if let Some(label) = &self.label_contains {
            if !entry.label.contains(label.as_str()) {
                return false;
            }
        }

        if let Some(key) = &self.has_metadata_key {
            if entry.get_metadata(key).is_none() {
                return false;
            }
        }

        if let Some((key, value)) = &self.metadata_match {
            match entry.get_metadata(key) {
                Some(v) if v == value.as_str() => {}
                _ => return false,
            }
        }

        if let Some(min_age) = self.min_age {
            if entry.age(now) < min_age {
                return false;
            }
        }

        if let Some(max_staleness) = self.max_staleness {
            if entry.staleness(now) > max_staleness {
                return false;
            }
        }

        true
    }
}

// ═══════════════════════════════════════════════════════════
// DISCOVERY RESULT
// ═══════════════════════════════════════════════════════════

/// Result of a discovery query.
///
/// Tier: T2-P (∃ + σ — ordered existence results)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Total entities that matched.
    pub total_matched: usize,
    /// Number returned (may be limited).
    pub returned: usize,
    /// Matched entity IDs.
    pub entity_ids: Vec<EntityId>,
    /// Whether more results were available.
    pub has_more: bool,
}

impl DiscoveryResult {
    /// Whether any entities were found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entity_ids.is_empty()
    }

    /// Number of found entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entity_ids.len()
    }

    /// First matched entity.
    #[must_use]
    pub fn first(&self) -> Option<EntityId> {
        self.entity_ids.first().copied()
    }
}

// ═══════════════════════════════════════════════════════════
// DISCOVERY INDEX
// ═══════════════════════════════════════════════════════════

/// Index for accelerating entity lookups.
///
/// Tier: T2-C (∃ + μ + κ + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryIndex {
    /// Kind → entity IDs.
    by_kind: HashMap<String, Vec<EntityId>>,
    /// Label → entity IDs.
    by_label: HashMap<String, Vec<EntityId>>,
    /// Metadata key → entity IDs.
    by_metadata_key: HashMap<String, Vec<EntityId>>,
    /// Total indexed entries.
    total_indexed: u64,
}

impl DiscoveryIndex {
    /// Creates a new empty index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_kind: HashMap::new(),
            by_label: HashMap::new(),
            by_metadata_key: HashMap::new(),
            total_indexed: 0,
        }
    }

    /// Indexes an entity entry.
    pub fn index(&mut self, entry: &RegistryEntry) {
        let kind_key = entry.kind.name().to_string();
        self.by_kind.entry(kind_key).or_default().push(entry.id);

        self.by_label
            .entry(entry.label.clone())
            .or_default()
            .push(entry.id);

        for key in entry.metadata.keys() {
            self.by_metadata_key
                .entry(key.clone())
                .or_default()
                .push(entry.id);
        }

        self.total_indexed += 1;
    }

    /// Removes an entity from the index.
    pub fn remove(&mut self, entry: &RegistryEntry) {
        let kind_key = entry.kind.name().to_string();
        if let Some(ids) = self.by_kind.get_mut(&kind_key) {
            ids.retain(|id| *id != entry.id);
        }

        if let Some(ids) = self.by_label.get_mut(&entry.label) {
            ids.retain(|id| *id != entry.id);
        }

        for key in entry.metadata.keys() {
            if let Some(ids) = self.by_metadata_key.get_mut(key) {
                ids.retain(|id| *id != entry.id);
            }
        }
    }

    /// Lookup by kind (fast path).
    #[must_use]
    pub fn by_kind(&self, kind: &str) -> Vec<EntityId> {
        self.by_kind.get(kind).cloned().unwrap_or_default()
    }

    /// Lookup by label (fast path).
    #[must_use]
    pub fn by_label(&self, label: &str) -> Vec<EntityId> {
        self.by_label.get(label).cloned().unwrap_or_default()
    }

    /// Lookup by metadata key (fast path).
    #[must_use]
    pub fn by_metadata_key(&self, key: &str) -> Vec<EntityId> {
        self.by_metadata_key.get(key).cloned().unwrap_or_default()
    }

    /// Total indexed entries.
    #[must_use]
    pub fn total_indexed(&self) -> u64 {
        self.total_indexed
    }
}

impl Default for DiscoveryIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for DiscoveryIndex {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,  // ∃ — entity existence lookups
            LexPrimitiva::Mapping,    // μ — key→ids indexing
            LexPrimitiva::Comparison, // κ — filter matching
            LexPrimitiva::Sequence,   // σ — ordered results
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// DISCOVERY SERVICE
// ═══════════════════════════════════════════════════════════

/// Discovery service — searches for entities across registries.
///
/// Tier: T2-C (∃ + μ + σ + κ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryService {
    /// Index for accelerated lookups.
    index: DiscoveryIndex,
    /// Total queries performed.
    total_queries: u64,
    /// Total results returned.
    total_results: u64,
}

impl DiscoveryService {
    /// Creates a new discovery service.
    #[must_use]
    pub fn new() -> Self {
        Self {
            index: DiscoveryIndex::new(),
            total_queries: 0,
            total_results: 0,
        }
    }

    /// Indexes an entity for future discovery.
    pub fn register(&mut self, entry: &RegistryEntry) {
        self.index.index(entry);
    }

    /// Removes an entity from the discovery index.
    pub fn deregister(&mut self, entry: &RegistryEntry) {
        self.index.remove(entry);
    }

    /// Discovers entities matching a query.
    pub fn discover(
        &mut self,
        query: &DiscoveryQuery,
        entries: &[&RegistryEntry],
        now: u64,
    ) -> DiscoveryResult {
        self.total_queries += 1;

        let matched: Vec<EntityId> = entries
            .iter()
            .filter(|e| query.matches(e, now))
            .map(|e| e.id)
            .collect();

        let total_matched = matched.len();
        let (returned_ids, has_more) = if let Some(limit) = query.limit {
            if matched.len() > limit {
                (matched[..limit].to_vec(), true)
            } else {
                (matched, false)
            }
        } else {
            (matched, false)
        };

        let returned = returned_ids.len();
        self.total_results += returned as u64;

        DiscoveryResult {
            total_matched,
            returned,
            entity_ids: returned_ids,
            has_more,
        }
    }

    /// Quick existence check using the index.
    #[must_use]
    pub fn exists_by_kind(&self, kind: &str) -> bool {
        !self.index.by_kind(kind).is_empty()
    }

    /// Quick existence check using the index.
    #[must_use]
    pub fn exists_by_label(&self, label: &str) -> bool {
        !self.index.by_label(label).is_empty()
    }

    /// Returns total queries performed.
    #[must_use]
    pub fn total_queries(&self) -> u64 {
        self.total_queries
    }

    /// Returns total results returned.
    #[must_use]
    pub fn total_results(&self) -> u64 {
        self.total_results
    }

    /// Returns the discovery index.
    #[must_use]
    pub fn index(&self) -> &DiscoveryIndex {
        &self.index
    }
}

impl Default for DiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for DiscoveryService {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,  // ∃ — entity discovery
            LexPrimitiva::Mapping,    // μ — query→results
            LexPrimitiva::Sequence,   // σ — ordered results
            LexPrimitiva::Comparison, // κ — filter matching
            LexPrimitiva::Boundary,   // ∂ — limits, active_only
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: u64, kind: EntityKind, label: &str, ts: u64) -> RegistryEntry {
        RegistryEntry::new(EntityId(id), kind, label, ts)
    }

    #[test]
    fn test_discovery_index_grounding() {
        let comp = DiscoveryIndex::primitive_composition();
        assert_eq!(
            nexcore_lex_primitiva::GroundingTier::classify(&comp),
            nexcore_lex_primitiva::GroundingTier::T2Composite
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
    }

    #[test]
    fn test_discovery_service_grounding() {
        let comp = DiscoveryService::primitive_composition();
        assert_eq!(
            nexcore_lex_primitiva::GroundingTier::classify(&comp),
            nexcore_lex_primitiva::GroundingTier::T2Composite
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_query_all() {
        let q = DiscoveryQuery::all();
        let entry = make_entry(1, EntityKind::Case, "case-001", 1000);
        assert!(q.matches(&entry, 2000));
    }

    #[test]
    fn test_query_by_kind() {
        let q = DiscoveryQuery::all().with_kind(EntityKind::Signal);
        let case = make_entry(1, EntityKind::Case, "c1", 1000);
        let signal = make_entry(2, EntityKind::Signal, "s1", 1000);

        assert!(!q.matches(&case, 2000));
        assert!(q.matches(&signal, 2000));
    }

    #[test]
    fn test_query_by_label() {
        let q = DiscoveryQuery::all().with_label("warfarin");
        let e1 = make_entry(1, EntityKind::Case, "warfarin-bleeding", 1000);
        let e2 = make_entry(2, EntityKind::Case, "aspirin-headache", 1000);

        assert!(q.matches(&e1, 2000));
        assert!(!q.matches(&e2, 2000));
    }

    #[test]
    fn test_query_active_only() {
        let q = DiscoveryQuery::all().active();
        let active = make_entry(1, EntityKind::Case, "c1", 1000);
        let mut inactive = make_entry(2, EntityKind::Case, "c2", 1000);
        inactive.deactivate();

        assert!(q.matches(&active, 2000));
        assert!(!q.matches(&inactive, 2000));
    }

    #[test]
    fn test_query_by_metadata() {
        let q = DiscoveryQuery::all().with_metadata("drug", "aspirin");
        let mut e1 = make_entry(1, EntityKind::Case, "c1", 1000);
        e1.set_metadata("drug", "aspirin");
        let e2 = make_entry(2, EntityKind::Case, "c2", 1000);

        assert!(q.matches(&e1, 2000));
        assert!(!q.matches(&e2, 2000));
    }

    #[test]
    fn test_discovery_service_discover() {
        let mut svc = DiscoveryService::new();
        let e1 = make_entry(1, EntityKind::Case, "case-001", 1000);
        let e2 = make_entry(2, EntityKind::Signal, "sig-001", 1000);
        let e3 = make_entry(3, EntityKind::Case, "case-002", 1000);

        svc.register(&e1);
        svc.register(&e2);
        svc.register(&e3);

        let entries: Vec<&RegistryEntry> = vec![&e1, &e2, &e3];
        let query = DiscoveryQuery::all().with_kind(EntityKind::Case);
        let result = svc.discover(&query, &entries, 2000);

        assert_eq!(result.total_matched, 2);
        assert_eq!(result.returned, 2);
        assert!(!result.has_more);
    }

    #[test]
    fn test_discovery_with_limit() {
        let mut svc = DiscoveryService::new();
        let e1 = make_entry(1, EntityKind::Case, "c1", 1000);
        let e2 = make_entry(2, EntityKind::Case, "c2", 1000);
        let e3 = make_entry(3, EntityKind::Case, "c3", 1000);

        let entries: Vec<&RegistryEntry> = vec![&e1, &e2, &e3];
        let query = DiscoveryQuery::all().with_limit(2);
        let result = svc.discover(&query, &entries, 2000);

        assert_eq!(result.total_matched, 3);
        assert_eq!(result.returned, 2);
        assert!(result.has_more);
    }

    #[test]
    fn test_discovery_index_by_kind() {
        let mut index = DiscoveryIndex::new();
        let e1 = make_entry(1, EntityKind::Case, "c1", 1000);
        let e2 = make_entry(2, EntityKind::Signal, "s1", 1000);
        let e3 = make_entry(3, EntityKind::Case, "c2", 1000);

        index.index(&e1);
        index.index(&e2);
        index.index(&e3);

        let cases = index.by_kind("case");
        assert_eq!(cases.len(), 2);

        let signals = index.by_kind("signal");
        assert_eq!(signals.len(), 1);
    }

    #[test]
    fn test_discovery_index_remove() {
        let mut index = DiscoveryIndex::new();
        let e1 = make_entry(1, EntityKind::Case, "c1", 1000);
        index.index(&e1);
        assert_eq!(index.by_kind("case").len(), 1);

        index.remove(&e1);
        assert_eq!(index.by_kind("case").len(), 0);
    }

    #[test]
    fn test_discovery_exists_shortcuts() {
        let mut svc = DiscoveryService::new();
        let e1 = make_entry(1, EntityKind::Case, "case-001", 1000);
        svc.register(&e1);

        assert!(svc.exists_by_kind("case"));
        assert!(!svc.exists_by_kind("signal"));
        assert!(svc.exists_by_label("case-001"));
    }

    #[test]
    fn test_discovery_counters() {
        let mut svc = DiscoveryService::new();
        let e1 = make_entry(1, EntityKind::Case, "c1", 1000);
        let entries: Vec<&RegistryEntry> = vec![&e1];

        svc.discover(&DiscoveryQuery::all(), &entries, 2000);
        svc.discover(&DiscoveryQuery::all(), &entries, 3000);

        assert_eq!(svc.total_queries(), 2);
        assert_eq!(svc.total_results(), 2);
    }
}
