//! # PVEX Enumeration — Entity Enumeration & Pagination
//!
//! Provides systematic enumeration over entity collections with
//! pagination, ordering, scoping, and live cursor-based iteration.
//!
//! ## T1 Grounding (dominant: ∃ Existence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | ∃ | Existence | 0.30 — counting existing entities |
//! | σ | Sequence | 0.30 — ordered enumeration |
//! | κ | Comparison | 0.20 — sort/filter |
//! | N | Quantity | 0.20 — page sizes, offsets |

use serde::{Deserialize, Serialize};

use super::registry::{EntityId, EntityKind, RegistryEntry};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// ENUMERATION SCOPE
// ═══════════════════════════════════════════════════════════

/// Scope of enumeration.
///
/// Tier: T2-P (∃ + ∂ — existence boundary)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumerationScope {
    /// All entities.
    All,
    /// Only entities of a specific kind.
    ByKind(EntityKind),
    /// Only active entities.
    ActiveOnly,
    /// Only inactive entities.
    InactiveOnly,
    /// Custom ID set.
    Subset(Vec<EntityId>),
}

impl EnumerationScope {
    /// Checks if an entry is within this scope.
    #[must_use]
    pub fn includes(&self, entry: &RegistryEntry) -> bool {
        match self {
            Self::All => true,
            Self::ByKind(kind) => entry.kind == *kind,
            Self::ActiveOnly => entry.active,
            Self::InactiveOnly => !entry.active,
            Self::Subset(ids) => ids.contains(&entry.id),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// ENUMERATION ORDER
// ═══════════════════════════════════════════════════════════

/// Ordering for enumerated results.
///
/// Tier: T2-P (σ + κ — ordered comparison)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumerationOrder {
    /// By entity ID ascending.
    IdAsc,
    /// By entity ID descending.
    IdDesc,
    /// By registration time ascending (oldest first).
    OldestFirst,
    /// By registration time descending (newest first).
    NewestFirst,
    /// By last seen ascending (stalest first).
    StalestFirst,
    /// By last seen descending (freshest first).
    FreshestFirst,
}

impl EnumerationOrder {
    /// Applies ordering to a collection of entries.
    pub fn sort(&self, entries: &mut [&RegistryEntry]) {
        match self {
            Self::IdAsc => entries.sort_by_key(|e| e.id.0),
            Self::IdDesc => entries.sort_by(|a, b| b.id.0.cmp(&a.id.0)),
            Self::OldestFirst => entries.sort_by_key(|e| e.registered_at),
            Self::NewestFirst => entries.sort_by(|a, b| b.registered_at.cmp(&a.registered_at)),
            Self::StalestFirst => entries.sort_by(|a, b| a.last_seen_at.cmp(&b.last_seen_at)),
            Self::FreshestFirst => entries.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at)),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// ENUMERATION PAGE
// ═══════════════════════════════════════════════════════════

/// A page of enumerated entity IDs.
///
/// Tier: T2-C (∃ + σ + N + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumerationPage {
    /// Entity IDs on this page.
    pub items: Vec<EntityId>,
    /// Page number (0-indexed).
    pub page: usize,
    /// Page size.
    pub page_size: usize,
    /// Total items across all pages.
    pub total_items: usize,
    /// Total number of pages.
    pub total_pages: usize,
    /// Whether there is a next page.
    pub has_next: bool,
    /// Whether there is a previous page.
    pub has_prev: bool,
}

impl EnumerationPage {
    /// Number of items on this page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether this page is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether this is the first page.
    #[must_use]
    pub fn is_first(&self) -> bool {
        self.page == 0
    }

    /// Whether this is the last page.
    #[must_use]
    pub fn is_last(&self) -> bool {
        !self.has_next
    }
}

impl GroundsTo for EnumerationPage {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,  // ∃ — entity existence
            LexPrimitiva::Sequence,   // σ — ordered page items
            LexPrimitiva::Quantity,   // N — page size, totals
            LexPrimitiva::Comparison, // κ — ordering
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// ENUMERATOR
// ═══════════════════════════════════════════════════════════

/// Enumerates entities with scope, order, and pagination.
///
/// Tier: T2-C (∃ + σ + κ + N + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enumerator {
    /// Enumeration scope.
    scope: EnumerationScope,
    /// Sort order.
    order: EnumerationOrder,
    /// Page size.
    page_size: usize,
    /// Total enumerations performed.
    total_enumerations: u64,
}

impl Enumerator {
    /// Creates a new enumerator.
    #[must_use]
    pub fn new(scope: EnumerationScope, order: EnumerationOrder, page_size: usize) -> Self {
        Self {
            scope,
            order,
            page_size: if page_size == 0 { 50 } else { page_size },
            total_enumerations: 0,
        }
    }

    /// Creates an enumerator with default settings.
    #[must_use]
    pub fn default_enumerator() -> Self {
        Self::new(EnumerationScope::All, EnumerationOrder::IdAsc, 50)
    }

    /// Enumerate a specific page from a collection of entries.
    pub fn enumerate(&mut self, entries: &[&RegistryEntry], page: usize) -> EnumerationPage {
        self.total_enumerations += 1;

        // Filter by scope
        let mut filtered: Vec<&RegistryEntry> = entries
            .iter()
            .filter(|e| self.scope.includes(e))
            .copied()
            .collect();

        // Sort
        self.order.sort(&mut filtered);

        let total_items = filtered.len();
        let total_pages = if total_items == 0 {
            0
        } else {
            (total_items + self.page_size - 1) / self.page_size
        };

        // Extract page
        let start = page * self.page_size;
        let items: Vec<EntityId> = if start < total_items {
            filtered[start..]
                .iter()
                .take(self.page_size)
                .map(|e| e.id)
                .collect()
        } else {
            Vec::new()
        };

        EnumerationPage {
            items,
            page,
            page_size: self.page_size,
            total_items,
            total_pages,
            has_next: page + 1 < total_pages,
            has_prev: page > 0,
        }
    }

    /// Enumerate all matching items (no pagination).
    pub fn enumerate_all(&mut self, entries: &[&RegistryEntry]) -> Vec<EntityId> {
        self.total_enumerations += 1;

        let mut filtered: Vec<&RegistryEntry> = entries
            .iter()
            .filter(|e| self.scope.includes(e))
            .copied()
            .collect();

        self.order.sort(&mut filtered);
        filtered.iter().map(|e| e.id).collect()
    }

    /// Returns total enumerations performed.
    #[must_use]
    pub fn total_enumerations(&self) -> u64 {
        self.total_enumerations
    }

    /// Returns the page size.
    #[must_use]
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// Returns the scope.
    #[must_use]
    pub fn scope(&self) -> &EnumerationScope {
        &self.scope
    }
}

impl Default for Enumerator {
    fn default() -> Self {
        Self::default_enumerator()
    }
}

impl GroundsTo for Enumerator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence,  // ∃ — entity existence
            LexPrimitiva::Sequence,   // σ — ordered iteration
            LexPrimitiva::Comparison, // κ — sorting
            LexPrimitiva::Quantity,   // N — page sizes
            LexPrimitiva::Boundary,   // ∂ — scope filtering
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// LIVE ENUMERATION
// ═══════════════════════════════════════════════════════════

/// Cursor-based live enumeration for streaming results.
///
/// Tier: T2-C (∃ + σ + N + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveEnumeration {
    /// Cursor: last seen entity ID.
    cursor: Option<EntityId>,
    /// Batch size per iteration.
    batch_size: usize,
    /// Total items yielded.
    total_yielded: u64,
    /// Whether enumeration is exhausted.
    exhausted: bool,
}

impl LiveEnumeration {
    /// Creates a new live enumeration.
    #[must_use]
    pub fn new(batch_size: usize) -> Self {
        Self {
            cursor: None,
            batch_size: if batch_size == 0 { 100 } else { batch_size },
            total_yielded: 0,
            exhausted: false,
        }
    }

    /// Gets the next batch of entity IDs.
    pub fn next_batch(&mut self, sorted_ids: &[EntityId]) -> Vec<EntityId> {
        if self.exhausted {
            return Vec::new();
        }

        let start_idx = match self.cursor {
            Some(cursor_id) => sorted_ids
                .iter()
                .position(|id| id.0 > cursor_id.0)
                .unwrap_or(sorted_ids.len()),
            None => 0,
        };

        let batch: Vec<EntityId> = sorted_ids[start_idx..]
            .iter()
            .take(self.batch_size)
            .copied()
            .collect();

        if batch.is_empty() {
            self.exhausted = true;
        } else {
            self.cursor = batch.last().copied();
            self.total_yielded += batch.len() as u64;
        }

        batch
    }

    /// Resets the cursor to the beginning.
    pub fn reset(&mut self) {
        self.cursor = None;
        self.exhausted = false;
    }

    /// Returns whether enumeration is exhausted.
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Returns total items yielded.
    #[must_use]
    pub fn total_yielded(&self) -> u64 {
        self.total_yielded
    }

    /// Returns the current cursor position.
    #[must_use]
    pub fn cursor(&self) -> Option<EntityId> {
        self.cursor
    }
}

impl GroundsTo for LiveEnumeration {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence, // ∃ — entity existence
            LexPrimitiva::Sequence,  // σ — cursor iteration
            LexPrimitiva::Quantity,  // N — batch sizes
            LexPrimitiva::Boundary,  // ∂ — cursor bounds
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_entry(id: u64, kind: EntityKind, label: &str, ts: u64) -> RegistryEntry {
        RegistryEntry::new(EntityId(id), kind, label, ts)
    }

    #[test]
    fn test_enumeration_page_grounding() {
        let comp = EnumerationPage::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
    }

    #[test]
    fn test_enumerator_grounding() {
        let comp = Enumerator::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_live_enumeration_grounding() {
        let comp = LiveEnumeration::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
    }

    #[test]
    fn test_scope_all() {
        let scope = EnumerationScope::All;
        let entry = make_entry(1, EntityKind::Case, "c1", 1000);
        assert!(scope.includes(&entry));
    }

    #[test]
    fn test_scope_by_kind() {
        let scope = EnumerationScope::ByKind(EntityKind::Signal);
        let case = make_entry(1, EntityKind::Case, "c1", 1000);
        let signal = make_entry(2, EntityKind::Signal, "s1", 1000);
        assert!(!scope.includes(&case));
        assert!(scope.includes(&signal));
    }

    #[test]
    fn test_scope_active_only() {
        let scope = EnumerationScope::ActiveOnly;
        let active = make_entry(1, EntityKind::Case, "c1", 1000);
        let mut inactive = make_entry(2, EntityKind::Case, "c2", 1000);
        inactive.deactivate();
        assert!(scope.includes(&active));
        assert!(!scope.includes(&inactive));
    }

    #[test]
    fn test_enumeration_order_id() {
        let e1 = make_entry(3, EntityKind::Case, "c3", 1000);
        let e2 = make_entry(1, EntityKind::Case, "c1", 2000);
        let e3 = make_entry(2, EntityKind::Case, "c2", 3000);

        let mut refs: Vec<&RegistryEntry> = vec![&e1, &e2, &e3];
        EnumerationOrder::IdAsc.sort(&mut refs);
        assert_eq!(refs[0].id, EntityId(1));
        assert_eq!(refs[1].id, EntityId(2));
        assert_eq!(refs[2].id, EntityId(3));
    }

    #[test]
    fn test_enumerator_paginate() {
        let entries: Vec<RegistryEntry> = (1..=5)
            .map(|i| make_entry(i, EntityKind::Case, &format!("c{i}"), i * 1000))
            .collect();
        let refs: Vec<&RegistryEntry> = entries.iter().collect();

        let mut enumerator = Enumerator::new(EnumerationScope::All, EnumerationOrder::IdAsc, 2);

        let page0 = enumerator.enumerate(&refs, 0);
        assert_eq!(page0.items.len(), 2);
        assert_eq!(page0.total_items, 5);
        assert_eq!(page0.total_pages, 3);
        assert!(page0.has_next);
        assert!(!page0.has_prev);
        assert!(page0.is_first());

        let page1 = enumerator.enumerate(&refs, 1);
        assert_eq!(page1.items.len(), 2);
        assert!(page1.has_next);
        assert!(page1.has_prev);

        let page2 = enumerator.enumerate(&refs, 2);
        assert_eq!(page2.items.len(), 1);
        assert!(!page2.has_next);
        assert!(page2.is_last());
    }

    #[test]
    fn test_enumerator_scoped() {
        let e1 = make_entry(1, EntityKind::Case, "c1", 1000);
        let e2 = make_entry(2, EntityKind::Signal, "s1", 2000);
        let e3 = make_entry(3, EntityKind::Case, "c2", 3000);
        let refs: Vec<&RegistryEntry> = vec![&e1, &e2, &e3];

        let mut enumerator = Enumerator::new(
            EnumerationScope::ByKind(EntityKind::Case),
            EnumerationOrder::IdAsc,
            50,
        );

        let page = enumerator.enumerate(&refs, 0);
        assert_eq!(page.total_items, 2);
        assert_eq!(page.items.len(), 2);
    }

    #[test]
    fn test_enumerator_enumerate_all() {
        let entries: Vec<RegistryEntry> = (1..=3)
            .map(|i| make_entry(i, EntityKind::Case, &format!("c{i}"), i * 1000))
            .collect();
        let refs: Vec<&RegistryEntry> = entries.iter().collect();

        let mut enumerator = Enumerator::default();
        let all = enumerator.enumerate_all(&refs);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_live_enumeration() {
        let ids: Vec<EntityId> = (1..=5).map(EntityId).collect();

        let mut live = LiveEnumeration::new(2);
        assert!(!live.is_exhausted());

        let batch1 = live.next_batch(&ids);
        assert_eq!(batch1.len(), 2);
        assert_eq!(batch1[0], EntityId(1));
        assert_eq!(batch1[1], EntityId(2));

        let batch2 = live.next_batch(&ids);
        assert_eq!(batch2.len(), 2);
        assert_eq!(batch2[0], EntityId(3));

        let batch3 = live.next_batch(&ids);
        assert_eq!(batch3.len(), 1);
        assert_eq!(batch3[0], EntityId(5));

        let batch4 = live.next_batch(&ids);
        assert!(batch4.is_empty());
        assert!(live.is_exhausted());

        assert_eq!(live.total_yielded(), 5);
    }

    #[test]
    fn test_live_enumeration_reset() {
        let ids: Vec<EntityId> = (1..=3).map(EntityId).collect();

        let mut live = LiveEnumeration::new(10);
        live.next_batch(&ids);
        live.next_batch(&ids);
        assert!(live.is_exhausted());

        live.reset();
        assert!(!live.is_exhausted());
        let batch = live.next_batch(&ids);
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn test_enumeration_page_properties() {
        let page = EnumerationPage {
            items: vec![EntityId(1), EntityId(2)],
            page: 0,
            page_size: 10,
            total_items: 5,
            total_pages: 1,
            has_next: false,
            has_prev: false,
        };
        assert_eq!(page.len(), 2);
        assert!(!page.is_empty());
        assert!(page.is_first());
        assert!(page.is_last());
    }
}
