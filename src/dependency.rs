//! # PVOC Dependency Graph
//!
//! DAG-based dependency management with topological sorting
//! and cycle detection. Determines execution order when
//! operations depend on other operations completing first.
//!
//! ## Primitives
//! - → (Causality) — dependency IS a causal relationship (A must complete before B)
//! - ρ (Recursion) — graph traversal, DFS/BFS
//! - σ (Sequence) — topological ordering

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Node identifier in the dependency graph.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl GroundsTo for NodeId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

// ═══════════════════════════════════════════════════════════
// DEPENDENCY NODE
// ═══════════════════════════════════════════════════════════

/// A vertex in the dependency graph with metadata.
/// Tier: T2-P (→ + ς)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyNode {
    /// Node ID
    pub id: NodeId,
    /// Human-readable label
    pub label: String,
    /// Node execution state
    pub state: NodeState,
}

/// Execution state of a dependency node.
/// Tier: T2-P (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeState {
    /// Not yet started
    Pending,
    /// Currently executing
    Running,
    /// Completed successfully
    Complete,
    /// Failed during execution
    Failed,
    /// Skipped (dependency failed)
    Skipped,
}

impl DependencyNode {
    /// Creates a new pending node.
    #[must_use]
    pub fn new(id: NodeId, label: &str) -> Self {
        Self {
            id,
            label: label.into(),
            state: NodeState::Pending,
        }
    }

    /// Returns true if the node has completed (success or failure).
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        matches!(
            self.state,
            NodeState::Complete | NodeState::Failed | NodeState::Skipped
        )
    }

    /// Returns true if the node completed successfully.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state == NodeState::Complete
    }
}

impl GroundsTo for DependencyNode {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — node is a causal actor
            LexPrimitiva::State,     // ς — execution state
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// DEPENDENCY EDGE
// ═══════════════════════════════════════════════════════════

/// A directed edge: `dependent` depends on `dependency`.
/// Meaning: `dependency` must complete before `dependent` can start.
///
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// The node that waits (downstream)
    pub dependent: NodeId,
    /// The node that must complete first (upstream)
    pub dependency: NodeId,
}

impl DependencyEdge {
    /// Creates a new edge: `dependent` depends on `dependency`.
    #[must_use]
    pub fn new(dependent: NodeId, dependency: NodeId) -> Self {
        Self {
            dependent,
            dependency,
        }
    }
}

impl GroundsTo for DependencyEdge {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

// ═══════════════════════════════════════════════════════════
// CYCLE DETECTION RESULT
// ═══════════════════════════════════════════════════════════

/// Result of cycle detection.
/// Tier: T2-P (ρ + ∂)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleResult {
    /// Graph is acyclic (valid DAG)
    Acyclic,
    /// Cycle detected, contains participating node IDs
    CycleDetected(Vec<NodeId>),
}

impl CycleResult {
    /// Returns true if no cycles found.
    #[must_use]
    pub fn is_acyclic(&self) -> bool {
        matches!(self, Self::Acyclic)
    }
}

// ═══════════════════════════════════════════════════════════
// DEPENDENCY GRAPH
// ═══════════════════════════════════════════════════════════

/// Directed acyclic graph of dependencies.
/// Provides topological sorting for execution order
/// and cycle detection to prevent infinite loops.
///
/// Tier: T2-C (→ + ρ + σ + ∂ + ς)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All nodes in the graph
    nodes: HashMap<NodeId, DependencyNode>,
    /// Adjacency list: node → set of nodes it depends on
    dependencies: HashMap<NodeId, HashSet<NodeId>>,
    /// Reverse adjacency: node → set of nodes that depend on it
    dependents: HashMap<NodeId, HashSet<NodeId>>,
}

impl DependencyGraph {
    /// Creates an empty dependency graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, node: DependencyNode) {
        let id = node.id;
        self.nodes.insert(id, node);
        self.dependencies.entry(id).or_default();
        self.dependents.entry(id).or_default();
    }

    /// Adds a dependency edge: `dependent` depends on `dependency`.
    /// Both nodes must already exist in the graph.
    /// Returns false if either node doesn't exist.
    pub fn add_edge(&mut self, dependent: NodeId, dependency: NodeId) -> bool {
        if !self.nodes.contains_key(&dependent) || !self.nodes.contains_key(&dependency) {
            return false;
        }

        self.dependencies
            .entry(dependent)
            .or_default()
            .insert(dependency);
        self.dependents
            .entry(dependency)
            .or_default()
            .insert(dependent);
        true
    }

    /// Returns the number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.dependencies.values().map(|deps| deps.len()).sum()
    }

    /// Returns the dependencies of a node (what it depends on).
    #[must_use]
    pub fn dependencies_of(&self, id: NodeId) -> Vec<NodeId> {
        self.dependencies
            .get(&id)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Returns the dependents of a node (what depends on it).
    #[must_use]
    pub fn dependents_of(&self, id: NodeId) -> Vec<NodeId> {
        self.dependents
            .get(&id)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Returns nodes with no dependencies (roots/entry points).
    #[must_use]
    pub fn roots(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|id| {
                self.dependencies
                    .get(id)
                    .map_or(true, |deps| deps.is_empty())
            })
            .copied()
            .collect()
    }

    /// Returns nodes with no dependents (leaves/exit points).
    #[must_use]
    pub fn leaves(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|id| self.dependents.get(id).map_or(true, |deps| deps.is_empty()))
            .copied()
            .collect()
    }

    /// Returns a node by ID.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<&DependencyNode> {
        self.nodes.get(&id)
    }

    /// Returns a mutable node by ID.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut DependencyNode> {
        self.nodes.get_mut(&id)
    }

    /// Performs topological sort using Kahn's algorithm.
    /// Returns nodes in valid execution order (dependencies before dependents).
    /// Returns None if the graph contains a cycle.
    #[must_use]
    pub fn topological_sort(&self) -> Option<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(*id, self.dependencies.get(id).map_or(0, |deps| deps.len()));
        }

        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(id, _)| *id)
            .collect();

        // Sort the initial queue for deterministic output
        let mut sorted_queue: Vec<NodeId> = queue.drain(..).collect();
        sorted_queue.sort_by_key(|n| n.0);
        queue.extend(sorted_queue);

        let mut result = Vec::with_capacity(self.nodes.len());

        while let Some(node) = queue.pop_front() {
            result.push(node);

            if let Some(deps) = self.dependents.get(&node) {
                let mut next_nodes: Vec<NodeId> = Vec::new();
                for &dependent in deps {
                    if let Some(deg) = in_degree.get_mut(&dependent) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            next_nodes.push(dependent);
                        }
                    }
                }
                // Sort for deterministic ordering
                next_nodes.sort_by_key(|n| n.0);
                queue.extend(next_nodes);
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // Cycle detected
        }
    }

    /// Detects cycles in the graph using DFS.
    #[must_use]
    pub fn detect_cycles(&self) -> CycleResult {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut cycle_nodes = Vec::new();

        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id)
                && self.dfs_cycle_detect(node_id, &mut visited, &mut rec_stack, &mut cycle_nodes)
            {
                return CycleResult::CycleDetected(cycle_nodes);
            }
        }

        CycleResult::Acyclic
    }

    /// DFS helper for cycle detection.
    fn dfs_cycle_detect(
        &self,
        node: NodeId,
        visited: &mut HashSet<NodeId>,
        rec_stack: &mut HashSet<NodeId>,
        cycle_nodes: &mut Vec<NodeId>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(deps) = self.dependencies.get(&node) {
            for &dep in deps {
                if !visited.contains(&dep) {
                    if self.dfs_cycle_detect(dep, visited, rec_stack, cycle_nodes) {
                        cycle_nodes.push(node);
                        return true;
                    }
                } else if rec_stack.contains(&dep) {
                    cycle_nodes.push(dep);
                    cycle_nodes.push(node);
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }

    /// Returns execution levels: groups of nodes that can execute in parallel.
    /// Level 0 = roots (no deps), Level 1 = depends only on level 0, etc.
    #[must_use]
    pub fn execution_levels(&self) -> Option<Vec<Vec<NodeId>>> {
        let sorted = self.topological_sort()?;
        let mut node_level: HashMap<NodeId, usize> = HashMap::new();
        let mut max_level = 0_usize;

        for &node in &sorted {
            let level = self
                .dependencies
                .get(&node)
                .map(|deps| {
                    deps.iter()
                        .filter_map(|d| node_level.get(d))
                        .max()
                        .map_or(0, |max| max + 1)
                })
                .unwrap_or(0);

            node_level.insert(node, level);
            if level > max_level {
                max_level = level;
            }
        }

        let mut levels = vec![Vec::new(); max_level + 1];
        for (node, level) in &node_level {
            levels[*level].push(*node);
        }

        // Sort within each level for determinism
        for level in &mut levels {
            level.sort_by_key(|n| n.0);
        }

        Some(levels)
    }

    /// Marks a node as complete and returns newly unblocked nodes
    /// (nodes whose all dependencies are now complete).
    pub fn complete_node(&mut self, id: NodeId) -> Vec<NodeId> {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.state = NodeState::Complete;
        }

        let mut unblocked = Vec::new();
        if let Some(deps) = self.dependents.get(&id) {
            for &dependent in deps {
                let all_deps_complete = self.dependencies.get(&dependent).map_or(true, |deps| {
                    deps.iter().all(|d| {
                        self.nodes
                            .get(d)
                            .map_or(false, |n| n.state == NodeState::Complete)
                    })
                });

                if all_deps_complete {
                    unblocked.push(dependent);
                }
            }
        }

        unblocked.sort_by_key(|n| n.0);
        unblocked
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for DependencyGraph {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — dependencies ARE causal relationships
            LexPrimitiva::Recursion, // ρ — graph traversal (DFS/BFS)
            LexPrimitiva::Sequence,  // σ — topological ordering
            LexPrimitiva::Boundary,  // ∂ — cycle detection prevents infinite loops
            LexPrimitiva::State,     // ς — node execution states
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn build_diamond_graph() -> DependencyGraph {
        // Diamond: A → B, A → C, B → D, C → D
        let mut g = DependencyGraph::new();
        g.add_node(DependencyNode::new(NodeId(1), "A"));
        g.add_node(DependencyNode::new(NodeId(2), "B"));
        g.add_node(DependencyNode::new(NodeId(3), "C"));
        g.add_node(DependencyNode::new(NodeId(4), "D"));

        g.add_edge(NodeId(2), NodeId(1)); // B depends on A
        g.add_edge(NodeId(3), NodeId(1)); // C depends on A
        g.add_edge(NodeId(4), NodeId(2)); // D depends on B
        g.add_edge(NodeId(4), NodeId(3)); // D depends on C
        g
    }

    #[test]
    fn test_node_id_grounding() {
        let comp = NodeId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
    }

    #[test]
    fn test_dependency_node_states() {
        let mut node = DependencyNode::new(NodeId(1), "test");
        assert_eq!(node.state, NodeState::Pending);
        assert!(!node.is_resolved());
        assert!(!node.is_complete());

        node.state = NodeState::Complete;
        assert!(node.is_resolved());
        assert!(node.is_complete());

        node.state = NodeState::Failed;
        assert!(node.is_resolved());
        assert!(!node.is_complete());
    }

    #[test]
    fn test_empty_graph() {
        let g = DependencyGraph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        assert!(g.roots().is_empty());
        assert!(g.leaves().is_empty());

        let sorted = g.topological_sort();
        assert!(sorted.is_some());
        if let Some(s) = sorted {
            assert!(s.is_empty());
        }
    }

    #[test]
    fn test_single_node() {
        let mut g = DependencyGraph::new();
        g.add_node(DependencyNode::new(NodeId(1), "single"));

        assert_eq!(g.node_count(), 1);
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.roots(), vec![NodeId(1)]);
        assert_eq!(g.leaves(), vec![NodeId(1)]);
    }

    #[test]
    fn test_linear_chain() {
        // A → B → C (B depends on A, C depends on B)
        let mut g = DependencyGraph::new();
        g.add_node(DependencyNode::new(NodeId(1), "A"));
        g.add_node(DependencyNode::new(NodeId(2), "B"));
        g.add_node(DependencyNode::new(NodeId(3), "C"));

        g.add_edge(NodeId(2), NodeId(1));
        g.add_edge(NodeId(3), NodeId(2));

        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
        assert_eq!(g.roots(), vec![NodeId(1)]);
        assert_eq!(g.leaves(), vec![NodeId(3)]);

        let sorted = g.topological_sort();
        assert!(sorted.is_some());
        if let Some(s) = sorted {
            assert_eq!(s, vec![NodeId(1), NodeId(2), NodeId(3)]);
        }
    }

    #[test]
    fn test_diamond_graph_topo_sort() {
        let g = build_diamond_graph();
        let sorted = g.topological_sort();
        assert!(sorted.is_some());
        if let Some(s) = sorted {
            assert_eq!(s.len(), 4);
            // A must come first
            assert_eq!(s[0], NodeId(1));
            // D must come last
            assert_eq!(s[3], NodeId(4));
        }
    }

    #[test]
    fn test_diamond_graph_execution_levels() {
        let g = build_diamond_graph();
        let levels = g.execution_levels();
        assert!(levels.is_some());
        if let Some(lvls) = levels {
            assert_eq!(lvls.len(), 3);
            assert_eq!(lvls[0], vec![NodeId(1)]); // Level 0: A (root)
            assert_eq!(lvls[1], vec![NodeId(2), NodeId(3)]); // Level 1: B, C (parallel)
            assert_eq!(lvls[2], vec![NodeId(4)]); // Level 2: D (leaf)
        }
    }

    #[test]
    fn test_cycle_detection_no_cycle() {
        let g = build_diamond_graph();
        let result = g.detect_cycles();
        assert!(result.is_acyclic());
    }

    #[test]
    fn test_cycle_detection_with_cycle() {
        let mut g = DependencyGraph::new();
        g.add_node(DependencyNode::new(NodeId(1), "A"));
        g.add_node(DependencyNode::new(NodeId(2), "B"));
        g.add_node(DependencyNode::new(NodeId(3), "C"));

        // A → B → C → A (cycle!)
        g.add_edge(NodeId(2), NodeId(1));
        g.add_edge(NodeId(3), NodeId(2));
        g.add_edge(NodeId(1), NodeId(3));

        let result = g.detect_cycles();
        assert!(!result.is_acyclic());

        // Topological sort should fail
        assert!(g.topological_sort().is_none());
    }

    #[test]
    fn test_complete_node_unblocks() {
        let mut g = build_diamond_graph();

        // Complete A → unblocks B and C
        let unblocked = g.complete_node(NodeId(1));
        assert_eq!(unblocked, vec![NodeId(2), NodeId(3)]);

        // Complete B → doesn't unblock D yet (C not complete)
        let unblocked = g.complete_node(NodeId(2));
        assert!(unblocked.is_empty());

        // Complete C → unblocks D
        let unblocked = g.complete_node(NodeId(3));
        assert_eq!(unblocked, vec![NodeId(4)]);
    }

    #[test]
    fn test_add_edge_invalid_node() {
        let mut g = DependencyGraph::new();
        g.add_node(DependencyNode::new(NodeId(1), "A"));

        // Edge to non-existent node
        assert!(!g.add_edge(NodeId(1), NodeId(99)));
        assert!(!g.add_edge(NodeId(99), NodeId(1)));
    }

    #[test]
    fn test_dependencies_and_dependents() {
        let g = build_diamond_graph();

        assert!(g.dependencies_of(NodeId(1)).is_empty()); // A has no deps
        let d_deps = g.dependencies_of(NodeId(4));
        assert_eq!(d_deps.len(), 2); // D depends on B and C

        let a_dependents = g.dependents_of(NodeId(1));
        assert_eq!(a_dependents.len(), 2); // B and C depend on A
    }

    #[test]
    fn test_dependency_graph_grounding() {
        let comp = DependencyGraph::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 5);
    }
}
