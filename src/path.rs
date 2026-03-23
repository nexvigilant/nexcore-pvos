//! # PVSH Path and Navigation
//!
//! Hierarchical path system for the PVOS namespace.
//! Every resource in the PV system is addressable by path.
//!
//! ## Primitives
//! - λ (Location) — paths, navigation, addressing
//! - ∃ (Existence) — path resolution, validation
//! - ς (State) — current location state

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// PATH SEGMENT
// ===============================================================

/// A segment of the PVOS namespace hierarchy.
/// Tier: T2-P (λ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathSegment {
    /// Root of the namespace.
    Root,
    /// Signal detection results.
    Signals,
    /// Case management.
    Cases,
    /// Workflow orchestration.
    Workflows,
    /// ML models and experiments.
    Models,
    /// Reactive streams.
    Streams,
    /// System internals (health, metrics, config).
    System,
    /// Named child node (year, drug, event, etc.).
    Named(String),
}

impl PathSegment {
    /// Parses a string into a path segment.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "" | "/" => Self::Root,
            "signals" => Self::Signals,
            "cases" => Self::Cases,
            "workflows" => Self::Workflows,
            "models" => Self::Models,
            "streams" => Self::Streams,
            "system" => Self::System,
            other => Self::Named(other.to_string()),
        }
    }

    /// Display name of this segment.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Root => "/",
            Self::Signals => "signals",
            Self::Cases => "cases",
            Self::Workflows => "workflows",
            Self::Models => "models",
            Self::Streams => "streams",
            Self::System => "system",
            Self::Named(n) => n,
        }
    }

    /// Whether this segment is a top-level namespace.
    #[must_use]
    pub fn is_namespace(&self) -> bool {
        matches!(
            self,
            Self::Root
                | Self::Signals
                | Self::Cases
                | Self::Workflows
                | Self::Models
                | Self::Streams
                | Self::System
        )
    }
}

impl GroundsTo for PathSegment {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Location])
    }
}

// ===============================================================
// PV PATH
// ===============================================================

/// A hierarchical path in the PVOS namespace.
/// Tier: T2-P (λ + ∃)
///
/// Paths are Unix-like: `/signals/2024/aspirin/headache`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PvPath {
    /// Path segments from root.
    segments: Vec<PathSegment>,
}

impl PvPath {
    /// The root path `/`.
    #[must_use]
    pub fn root() -> Self {
        Self {
            segments: vec![PathSegment::Root],
        }
    }

    /// Parses a path string like `/signals/2024/aspirin`.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed == "/" {
            return Self::root();
        }

        let mut segments = vec![PathSegment::Root];
        for part in trimmed.split('/') {
            if part.is_empty() {
                continue;
            }
            if part == ".." {
                if segments.len() > 1 {
                    segments.pop();
                }
            } else if part != "." {
                segments.push(PathSegment::parse(part));
            }
        }

        Self { segments }
    }

    /// Returns the display string for this path.
    #[must_use]
    pub fn display(&self) -> String {
        if self.segments.len() <= 1 {
            return "/".to_string();
        }

        let parts: Vec<&str> = self.segments.iter().skip(1).map(|s| s.name()).collect();
        format!("/{}", parts.join("/"))
    }

    /// Joins a child segment to this path.
    #[must_use]
    pub fn join(&self, child: &str) -> Self {
        if child.starts_with('/') {
            return Self::parse(child);
        }

        let mut new_segments = self.segments.clone();
        for part in child.split('/') {
            if part.is_empty() {
                continue;
            }
            if part == ".." {
                if new_segments.len() > 1 {
                    new_segments.pop();
                }
            } else if part != "." {
                new_segments.push(PathSegment::parse(part));
            }
        }

        Self {
            segments: new_segments,
        }
    }

    /// Returns the parent path, or self if at root.
    #[must_use]
    pub fn parent(&self) -> Self {
        if self.segments.len() <= 1 {
            return self.clone();
        }
        let mut segs = self.segments.clone();
        segs.pop();
        Self { segments: segs }
    }

    /// Returns the last segment (basename).
    #[must_use]
    pub fn basename(&self) -> &PathSegment {
        self.segments.last().unwrap_or(&PathSegment::Root)
    }

    /// Depth from root.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.segments.len().saturating_sub(1)
    }

    /// Whether this is the root path.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments.len() <= 1
    }

    /// All segments.
    #[must_use]
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    /// Returns the top-level namespace (first segment after root).
    #[must_use]
    pub fn namespace(&self) -> Option<&PathSegment> {
        self.segments.get(1)
    }
}

impl GroundsTo for PvPath {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Location, LexPrimitiva::Existence])
    }
}

// ===============================================================
// NAVIGATOR
// ===============================================================

/// Directory stack entry for pushd/popd.
/// Tier: T2-P (λ + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirStackEntry {
    /// The path that was pushed.
    pub path: PvPath,
}

/// Navigator — manages current location and directory stack.
/// Tier: T2-C (λ + ς + σ + ∃)
///
/// Provides cd, pwd, pushd, popd operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Navigator {
    /// Current working path.
    current: PvPath,
    /// Directory stack for pushd/popd.
    dir_stack: Vec<DirStackEntry>,
    /// Previous path (for `cd -`).
    previous: Option<PvPath>,
    /// Total navigations performed.
    total_navigations: u64,
}

impl Navigator {
    /// Creates a new navigator starting at root.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: PvPath::root(),
            dir_stack: Vec::new(),
            previous: None,
            total_navigations: 0,
        }
    }

    /// Current working path.
    #[must_use]
    pub fn pwd(&self) -> &PvPath {
        &self.current
    }

    /// Changes directory. Supports absolute, relative, `..`, and `-`.
    pub fn cd(&mut self, target: &str) {
        let old = self.current.clone();

        if target == "-" {
            if let Some(prev) = self.previous.take() {
                self.current = prev;
            }
        } else if target.starts_with('/') {
            self.current = PvPath::parse(target);
        } else {
            self.current = self.current.join(target);
        }

        self.previous = Some(old);
        self.total_navigations += 1;
    }

    /// Pushes current directory and changes to target.
    pub fn pushd(&mut self, target: &str) {
        self.dir_stack.push(DirStackEntry {
            path: self.current.clone(),
        });
        self.cd(target);
    }

    /// Pops directory stack and returns to saved location.
    /// Returns `true` if a directory was popped.
    pub fn popd(&mut self) -> bool {
        if let Some(entry) = self.dir_stack.pop() {
            self.previous = Some(self.current.clone());
            self.current = entry.path;
            self.total_navigations += 1;
            true
        } else {
            false
        }
    }

    /// Directory stack depth.
    #[must_use]
    pub fn stack_depth(&self) -> usize {
        self.dir_stack.len()
    }

    /// Previous directory (for `cd -`).
    #[must_use]
    pub fn previous(&self) -> Option<&PvPath> {
        self.previous.as_ref()
    }

    /// Total navigations performed.
    #[must_use]
    pub fn total_navigations(&self) -> u64 {
        self.total_navigations
    }
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for Navigator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::State,
            LexPrimitiva::Sequence,
            LexPrimitiva::Existence,
        ])
        .with_dominant(LexPrimitiva::Location, 0.80)
    }
}

// ===============================================================
// PATH RESOLVER
// ===============================================================

/// Known children of each namespace node.
/// Tier: T2-P (λ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceNode {
    /// Segment this node represents.
    pub segment: PathSegment,
    /// Known children.
    pub children: Vec<String>,
}

/// Resolves paths against a known namespace tree.
/// Tier: T2-C (λ + ∃ + μ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathResolver {
    /// Known namespace nodes.
    nodes: Vec<NamespaceNode>,
}

impl PathResolver {
    /// Creates a resolver with the default PVOS namespace.
    #[must_use]
    pub fn with_defaults() -> Self {
        let nodes = vec![
            NamespaceNode {
                segment: PathSegment::Root,
                children: vec![
                    "signals".into(),
                    "cases".into(),
                    "workflows".into(),
                    "models".into(),
                    "streams".into(),
                    "system".into(),
                ],
            },
            NamespaceNode {
                segment: PathSegment::Cases,
                children: vec!["pending".into(), "processing".into(), "closed".into()],
            },
            NamespaceNode {
                segment: PathSegment::Workflows,
                children: vec!["running".into(), "completed".into(), "patterns".into()],
            },
            NamespaceNode {
                segment: PathSegment::Models,
                children: vec!["current".into(), "versions".into(), "experiments".into()],
            },
            NamespaceNode {
                segment: PathSegment::Streams,
                children: vec!["topics".into(), "monitors".into()],
            },
            NamespaceNode {
                segment: PathSegment::System,
                children: vec!["health".into(), "metrics".into(), "config".into()],
            },
        ];

        Self { nodes }
    }

    /// Lists children of a path.
    #[must_use]
    pub fn children_of(&self, path: &PvPath) -> Vec<String> {
        let target_segment = path.basename();
        self.nodes
            .iter()
            .find(|n| n.segment == *target_segment)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    /// Checks if a path segment exists as a child of its parent.
    #[must_use]
    pub fn exists(&self, path: &PvPath) -> bool {
        if path.is_root() {
            return true;
        }

        let parent = path.parent();
        let children = self.children_of(&parent);
        let basename = path.basename().name();
        children.iter().any(|c| c == basename)
    }

    /// Adds a custom child to a namespace node.
    pub fn add_child(&mut self, parent_segment: PathSegment, child: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.segment == parent_segment) {
            if !node.children.contains(&child.to_string()) {
                node.children.push(child.to_string());
            }
        } else {
            self.nodes.push(NamespaceNode {
                segment: parent_segment,
                children: vec![child.to_string()],
            });
        }
    }
}

impl GroundsTo for PathResolver {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Existence,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Location, 0.75)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_path_parse_root() {
        let p = PvPath::parse("/");
        assert!(p.is_root());
        assert_eq!(p.display(), "/");
        assert_eq!(p.depth(), 0);
    }

    #[test]
    fn test_path_parse_absolute() {
        let p = PvPath::parse("/signals/2024/aspirin");
        assert_eq!(p.display(), "/signals/2024/aspirin");
        assert_eq!(p.depth(), 3);
        assert!(!p.is_root());
    }

    #[test]
    fn test_path_parse_dotdot() {
        let p = PvPath::parse("/signals/2024/../cases");
        assert_eq!(p.display(), "/signals/cases");
    }

    #[test]
    fn test_path_join() {
        let p = PvPath::parse("/signals");
        let child = p.join("2024/aspirin");
        assert_eq!(child.display(), "/signals/2024/aspirin");
    }

    #[test]
    fn test_path_join_absolute() {
        let p = PvPath::parse("/signals/2024");
        let abs = p.join("/cases/pending");
        assert_eq!(abs.display(), "/cases/pending");
    }

    #[test]
    fn test_path_parent() {
        let p = PvPath::parse("/signals/2024/aspirin");
        assert_eq!(p.parent().display(), "/signals/2024");
        assert_eq!(PvPath::root().parent().display(), "/");
    }

    #[test]
    fn test_path_basename() {
        let p = PvPath::parse("/signals/2024");
        assert_eq!(p.basename().name(), "2024");
    }

    #[test]
    fn test_path_namespace() {
        let p = PvPath::parse("/signals/2024/aspirin");
        assert_eq!(p.namespace().map(|s| s.name()), Some("signals"));
    }

    #[test]
    fn test_navigator_cd() {
        let mut nav = Navigator::new();
        assert_eq!(nav.pwd().display(), "/");

        nav.cd("/signals");
        assert_eq!(nav.pwd().display(), "/signals");

        nav.cd("2024");
        assert_eq!(nav.pwd().display(), "/signals/2024");

        nav.cd("..");
        assert_eq!(nav.pwd().display(), "/signals");

        nav.cd("-");
        assert_eq!(nav.pwd().display(), "/signals/2024");
    }

    #[test]
    fn test_navigator_pushd_popd() {
        let mut nav = Navigator::new();
        nav.cd("/signals");

        nav.pushd("/cases/pending");
        assert_eq!(nav.pwd().display(), "/cases/pending");
        assert_eq!(nav.stack_depth(), 1);

        assert!(nav.popd());
        assert_eq!(nav.pwd().display(), "/signals");
        assert_eq!(nav.stack_depth(), 0);

        assert!(!nav.popd()); // Empty stack
    }

    #[test]
    fn test_path_resolver_children() {
        let resolver = PathResolver::with_defaults();

        let root_children = resolver.children_of(&PvPath::root());
        assert!(root_children.contains(&"signals".to_string()));
        assert!(root_children.contains(&"cases".to_string()));
        assert_eq!(root_children.len(), 6);
    }

    #[test]
    fn test_path_resolver_exists() {
        let resolver = PathResolver::with_defaults();
        assert!(resolver.exists(&PvPath::root()));
        assert!(resolver.exists(&PvPath::parse("/signals")));
        assert!(!resolver.exists(&PvPath::parse("/nonexistent")));
    }

    #[test]
    fn test_path_resolver_add_child() {
        let mut resolver = PathResolver::with_defaults();
        resolver.add_child(PathSegment::Signals, "2024");

        let children = resolver.children_of(&PvPath::parse("/signals"));
        assert!(children.contains(&"2024".to_string()));
    }

    #[test]
    fn test_segment_parse() {
        assert_eq!(PathSegment::parse("signals"), PathSegment::Signals);
        assert_eq!(PathSegment::parse("cases"), PathSegment::Cases);
        assert_eq!(
            PathSegment::parse("aspirin"),
            PathSegment::Named("aspirin".into())
        );
    }

    #[test]
    fn test_navigator_grounding() {
        let comp = Navigator::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Location));
    }

    #[test]
    fn test_path_grounding() {
        let comp = PvPath::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_path_resolver_grounding() {
        let comp = PathResolver::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Location));
    }
}
