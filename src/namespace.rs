//! # PVEX Namespace — Hierarchical Entity Namespaces
//!
//! Organizes entities into hierarchical namespaces with
//! path-based addressing, visibility controls, and
//! cross-namespace resolution.
//!
//! ## T1 Grounding (dominant: ∃ Existence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | ∃ | Existence | 0.30 — entity addressing |
//! | λ | Location | 0.30 — path-based namespace |
//! | μ | Mapping | 0.20 — path→entity |
//! | ∂ | Boundary | 0.20 — visibility controls |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::registry::EntityId;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// NAMESPACE PATH
// ═══════════════════════════════════════════════════════════

/// A hierarchical path within a namespace.
///
/// Tier: T2-P (λ — location)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NamespacePath(pub String);

impl NamespacePath {
    /// Creates a new namespace path.
    #[must_use]
    pub fn new(path: &str) -> Self {
        // Normalize: ensure leading /, remove trailing /
        let normalized = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        let normalized = normalized.trim_end_matches('/').to_string();
        let normalized = if normalized.is_empty() {
            "/".to_string()
        } else {
            normalized
        };
        Self(normalized)
    }

    /// Root namespace.
    #[must_use]
    pub fn root() -> Self {
        Self("/".to_string())
    }

    /// Returns the parent path.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.0 == "/" {
            return None;
        }
        match self.0.rfind('/') {
            Some(0) => Some(Self::root()),
            Some(idx) => Some(Self(self.0[..idx].to_string())),
            None => Some(Self::root()),
        }
    }

    /// Returns the last segment.
    #[must_use]
    pub fn name(&self) -> &str {
        if self.0 == "/" {
            return "/";
        }
        match self.0.rfind('/') {
            Some(idx) => &self.0[idx + 1..],
            None => &self.0,
        }
    }

    /// Joins a child segment.
    #[must_use]
    pub fn join(&self, segment: &str) -> Self {
        if self.0 == "/" {
            Self(format!("/{segment}"))
        } else {
            Self(format!("{}/{segment}", self.0))
        }
    }

    /// Returns path segments.
    #[must_use]
    pub fn segments(&self) -> Vec<&str> {
        self.0.split('/').filter(|s| !s.is_empty()).collect()
    }

    /// Depth of the path (0 = root).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.segments().len()
    }

    /// Whether this path is a prefix of another.
    #[must_use]
    pub fn is_ancestor_of(&self, other: &Self) -> bool {
        if self.0 == "/" {
            return true;
        }
        other.0.starts_with(&self.0)
            && (other.0.len() == self.0.len()
                || other.0.as_bytes().get(self.0.len()) == Some(&b'/'))
    }

    /// Returns the raw path string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ═══════════════════════════════════════════════════════════
// NAMESPACE VISIBILITY
// ═══════════════════════════════════════════════════════════

/// Visibility of a namespace entry.
///
/// Tier: T2-P (∂ — boundary control)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamespaceVisibility {
    /// Visible to all namespaces.
    Public,
    /// Visible only within this namespace.
    Private,
    /// Visible to parent and sibling namespaces.
    Internal,
}

impl NamespaceVisibility {
    /// Whether visible from the given path.
    #[must_use]
    pub fn is_visible_from(&self, entry_path: &NamespacePath, viewer_path: &NamespacePath) -> bool {
        match self {
            Self::Public => true,
            Self::Private => entry_path == viewer_path,
            Self::Internal => {
                // Same parent = siblings
                entry_path.parent() == viewer_path.parent()
                    || viewer_path.is_ancestor_of(entry_path)
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════
// NAMESPACE ENTRY
// ═══════════════════════════════════════════════════════════

/// An entity registered within a namespace.
///
/// Tier: T2-C (∃ + λ + ∂ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceEntry {
    /// Entity ID.
    pub entity_id: EntityId,
    /// Path within the namespace.
    pub path: NamespacePath,
    /// Visibility.
    pub visibility: NamespaceVisibility,
    /// When registered.
    pub registered_at: u64,
    /// Optional alias (human-friendly name).
    pub alias: Option<String>,
}

impl NamespaceEntry {
    /// Creates a new namespace entry.
    #[must_use]
    pub fn new(
        entity_id: EntityId,
        path: NamespacePath,
        visibility: NamespaceVisibility,
        registered_at: u64,
    ) -> Self {
        Self {
            entity_id,
            path,
            visibility,
            registered_at,
            alias: None,
        }
    }

    /// Adds an alias.
    #[must_use]
    pub fn with_alias(mut self, alias: &str) -> Self {
        self.alias = Some(alias.to_string());
        self
    }
}

// ═══════════════════════════════════════════════════════════
// CROSS-NAMESPACE RESULT
// ═══════════════════════════════════════════════════════════

/// Outcome of a cross-namespace resolution.
///
/// Tier: T2-P (∃ + λ)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossNamespaceResult {
    /// Entity found at path.
    Found {
        entity_id: EntityId,
        path: NamespacePath,
    },
    /// Path not found.
    NotFound,
    /// Entity exists but not visible from the requesting namespace.
    NotVisible { path: NamespacePath },
}

impl CrossNamespaceResult {
    /// Whether the entity was found.
    #[must_use]
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found { .. })
    }

    /// Returns the entity ID if found.
    #[must_use]
    pub fn entity_id(&self) -> Option<EntityId> {
        match self {
            Self::Found { entity_id, .. } => Some(*entity_id),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// NAMESPACE REGISTRY
// ═══════════════════════════════════════════════════════════

/// Registry of entity namespaces.
///
/// Tier: T2-C (∃ + λ + μ + ∂ + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceRegistry {
    /// Path → namespace entry.
    entries: HashMap<String, NamespaceEntry>,
    /// Entity ID → path (reverse lookup).
    entity_paths: HashMap<u64, NamespacePath>,
    /// Known namespace prefixes (directories).
    prefixes: Vec<NamespacePath>,
    /// Total registrations.
    total_registered: u64,
    /// Total resolutions attempted.
    total_resolutions: u64,
}

impl NamespaceRegistry {
    /// Creates a new namespace registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            entity_paths: HashMap::new(),
            prefixes: vec![NamespacePath::root()],
            total_registered: 0,
            total_resolutions: 0,
        }
    }

    /// Registers a namespace prefix (creates the "directory").
    pub fn create_namespace(&mut self, path: &NamespacePath) {
        if !self.prefixes.contains(path) {
            self.prefixes.push(path.clone());
        }
    }

    /// Registers an entity in a namespace.
    pub fn register(
        &mut self,
        entity_id: EntityId,
        path: NamespacePath,
        visibility: NamespaceVisibility,
        timestamp: u64,
    ) -> bool {
        if self.entries.contains_key(path.as_str()) {
            return false;
        }

        let entry = NamespaceEntry::new(entity_id, path.clone(), visibility, timestamp);
        self.entries.insert(path.as_str().to_string(), entry);
        self.entity_paths.insert(entity_id.0, path);
        self.total_registered += 1;
        true
    }

    /// Registers with an alias.
    pub fn register_with_alias(
        &mut self,
        entity_id: EntityId,
        path: NamespacePath,
        visibility: NamespaceVisibility,
        timestamp: u64,
        alias: &str,
    ) -> bool {
        if self.entries.contains_key(path.as_str()) {
            return false;
        }

        let entry =
            NamespaceEntry::new(entity_id, path.clone(), visibility, timestamp).with_alias(alias);
        self.entries.insert(path.as_str().to_string(), entry);
        self.entity_paths.insert(entity_id.0, path);
        self.total_registered += 1;
        true
    }

    /// Removes an entity from the namespace.
    pub fn deregister(&mut self, path: &NamespacePath) -> bool {
        if let Some(entry) = self.entries.remove(path.as_str()) {
            self.entity_paths.remove(&entry.entity_id.0);
            true
        } else {
            false
        }
    }

    /// Resolves a path to an entity, respecting visibility.
    pub fn resolve(
        &mut self,
        path: &NamespacePath,
        from_namespace: &NamespacePath,
    ) -> CrossNamespaceResult {
        self.total_resolutions += 1;

        match self.entries.get(path.as_str()) {
            Some(entry) => {
                if entry
                    .visibility
                    .is_visible_from(&entry.path, from_namespace)
                {
                    CrossNamespaceResult::Found {
                        entity_id: entry.entity_id,
                        path: entry.path.clone(),
                    }
                } else {
                    CrossNamespaceResult::NotVisible { path: path.clone() }
                }
            }
            None => CrossNamespaceResult::NotFound,
        }
    }

    /// Looks up path by entity ID (reverse).
    #[must_use]
    pub fn path_of(&self, entity_id: EntityId) -> Option<&NamespacePath> {
        self.entity_paths.get(&entity_id.0)
    }

    /// Lists all entries under a namespace prefix.
    #[must_use]
    pub fn list(&self, prefix: &NamespacePath) -> Vec<&NamespaceEntry> {
        self.entries
            .values()
            .filter(|e| prefix.is_ancestor_of(&e.path))
            .collect()
    }

    /// Lists direct children of a namespace.
    #[must_use]
    pub fn children(&self, parent: &NamespacePath) -> Vec<&NamespaceEntry> {
        let parent_depth = parent.depth();
        self.entries
            .values()
            .filter(|e| parent.is_ancestor_of(&e.path) && e.path.depth() == parent_depth + 1)
            .collect()
    }

    /// Returns total registered entries.
    #[must_use]
    pub fn total_registered(&self) -> u64 {
        self.total_registered
    }

    /// Returns total resolution attempts.
    #[must_use]
    pub fn total_resolutions(&self) -> u64 {
        self.total_resolutions
    }

    /// Returns the number of active entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns all known namespace prefixes.
    #[must_use]
    pub fn namespaces(&self) -> &[NamespacePath] {
        &self.prefixes
    }
}

impl Default for NamespaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for NamespaceRegistry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence, // ∃ — entity addressing
            LexPrimitiva::Location,  // λ — hierarchical paths
            LexPrimitiva::Mapping,   // μ — path→entity
            LexPrimitiva::Boundary,  // ∂ — visibility control
            LexPrimitiva::Sequence,  // σ — path segments
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_namespace_registry_grounding() {
        let comp = NamespaceRegistry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_path_creation() {
        let p = NamespacePath::new("/pv/cases");
        assert_eq!(p.as_str(), "/pv/cases");

        let p2 = NamespacePath::new("pv/signals");
        assert_eq!(p2.as_str(), "/pv/signals");

        let root = NamespacePath::root();
        assert_eq!(root.as_str(), "/");
    }

    #[test]
    fn test_path_parent() {
        let p = NamespacePath::new("/pv/cases/2026");
        let parent = p.parent();
        assert!(parent.is_some());
        assert_eq!(parent.as_ref().map(|p| p.as_str()), Some("/pv/cases"));

        let root_child = NamespacePath::new("/pv");
        assert_eq!(root_child.parent().map(|p| p.0), Some("/".to_string()));

        let root = NamespacePath::root();
        assert!(root.parent().is_none());
    }

    #[test]
    fn test_path_name() {
        let p = NamespacePath::new("/pv/cases/2026");
        assert_eq!(p.name(), "2026");

        let root = NamespacePath::root();
        assert_eq!(root.name(), "/");
    }

    #[test]
    fn test_path_join() {
        let root = NamespacePath::root();
        let child = root.join("pv");
        assert_eq!(child.as_str(), "/pv");

        let nested = child.join("cases");
        assert_eq!(nested.as_str(), "/pv/cases");
    }

    #[test]
    fn test_path_segments() {
        let p = NamespacePath::new("/pv/cases/2026/q1");
        assert_eq!(p.segments(), vec!["pv", "cases", "2026", "q1"]);
        assert_eq!(p.depth(), 4);

        let root = NamespacePath::root();
        assert_eq!(root.depth(), 0);
    }

    #[test]
    fn test_path_ancestor() {
        let parent = NamespacePath::new("/pv/cases");
        let child = NamespacePath::new("/pv/cases/2026");
        let sibling = NamespacePath::new("/pv/signals");

        assert!(parent.is_ancestor_of(&child));
        assert!(!parent.is_ancestor_of(&sibling));

        let root = NamespacePath::root();
        assert!(root.is_ancestor_of(&parent));
        assert!(root.is_ancestor_of(&child));
    }

    #[test]
    fn test_register_and_resolve() {
        let mut reg = NamespaceRegistry::new();
        let path = NamespacePath::new("/pv/cases/case-001");

        assert!(reg.register(EntityId(1), path.clone(), NamespaceVisibility::Public, 1000));
        assert_eq!(reg.len(), 1);

        let result = reg.resolve(&path, &NamespacePath::root());
        assert!(result.is_found());
        assert_eq!(result.entity_id(), Some(EntityId(1)));
    }

    #[test]
    fn test_register_duplicate() {
        let mut reg = NamespaceRegistry::new();
        let path = NamespacePath::new("/pv/cases/c1");

        assert!(reg.register(EntityId(1), path.clone(), NamespaceVisibility::Public, 1000));
        assert!(!reg.register(EntityId(2), path, NamespaceVisibility::Public, 2000));
    }

    #[test]
    fn test_visibility_private() {
        let mut reg = NamespaceRegistry::new();
        let path = NamespacePath::new("/pv/internal/secret");

        reg.register(
            EntityId(1),
            path.clone(),
            NamespaceVisibility::Private,
            1000,
        );

        // Same path → visible
        let same = reg.resolve(&path, &NamespacePath::new("/pv/internal/secret"));
        assert!(same.is_found());

        // Different path → not visible
        let other = reg.resolve(&path, &NamespacePath::new("/pv/external"));
        assert!(matches!(other, CrossNamespaceResult::NotVisible { .. }));
    }

    #[test]
    fn test_resolve_not_found() {
        let mut reg = NamespaceRegistry::new();
        let result = reg.resolve(&NamespacePath::new("/nonexistent"), &NamespacePath::root());
        assert!(matches!(result, CrossNamespaceResult::NotFound));
    }

    #[test]
    fn test_deregister() {
        let mut reg = NamespaceRegistry::new();
        let path = NamespacePath::new("/pv/cases/c1");
        reg.register(EntityId(1), path.clone(), NamespaceVisibility::Public, 1000);

        assert!(reg.deregister(&path));
        assert_eq!(reg.len(), 0);
        assert!(!reg.deregister(&path)); // Already removed
    }

    #[test]
    fn test_path_of_reverse_lookup() {
        let mut reg = NamespaceRegistry::new();
        let path = NamespacePath::new("/pv/signals/s1");
        reg.register(
            EntityId(42),
            path.clone(),
            NamespaceVisibility::Public,
            1000,
        );

        let found = reg.path_of(EntityId(42));
        assert!(found.is_some());
        assert_eq!(found.map(|p| p.as_str()), Some("/pv/signals/s1"));
    }

    #[test]
    fn test_list_under_prefix() {
        let mut reg = NamespaceRegistry::new();
        reg.register(
            EntityId(1),
            NamespacePath::new("/pv/cases/c1"),
            NamespaceVisibility::Public,
            1000,
        );
        reg.register(
            EntityId(2),
            NamespacePath::new("/pv/cases/c2"),
            NamespaceVisibility::Public,
            2000,
        );
        reg.register(
            EntityId(3),
            NamespacePath::new("/pv/signals/s1"),
            NamespaceVisibility::Public,
            3000,
        );

        let under_cases = reg.list(&NamespacePath::new("/pv/cases"));
        assert_eq!(under_cases.len(), 2);

        let under_root = reg.list(&NamespacePath::root());
        assert_eq!(under_root.len(), 3);
    }

    #[test]
    fn test_children() {
        let mut reg = NamespaceRegistry::new();
        reg.register(
            EntityId(1),
            NamespacePath::new("/pv/cases"),
            NamespaceVisibility::Public,
            1000,
        );
        reg.register(
            EntityId(2),
            NamespacePath::new("/pv/signals"),
            NamespaceVisibility::Public,
            2000,
        );
        reg.register(
            EntityId(3),
            NamespacePath::new("/pv/cases/2026"),
            NamespaceVisibility::Public,
            3000,
        );

        let pv_children = reg.children(&NamespacePath::new("/pv"));
        assert_eq!(pv_children.len(), 2); // cases and signals (not cases/2026)
    }

    #[test]
    fn test_registry_counters() {
        let mut reg = NamespaceRegistry::new();
        reg.register(
            EntityId(1),
            NamespacePath::new("/a"),
            NamespaceVisibility::Public,
            1000,
        );
        reg.register(
            EntityId(2),
            NamespacePath::new("/b"),
            NamespaceVisibility::Public,
            2000,
        );

        assert_eq!(reg.total_registered(), 2);

        reg.resolve(&NamespacePath::new("/a"), &NamespacePath::root());
        reg.resolve(&NamespacePath::new("/c"), &NamespacePath::root());
        assert_eq!(reg.total_resolutions(), 2);
    }
}
