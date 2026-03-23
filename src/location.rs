//! # PVGL Location Primitives
//!
//! Spatial reasoning types for pharmacovigilance geographic analysis.
//! Location (λ) is the dominant primitive — all types answer "where?"
//!
//! ## Primitives
//! - λ (Location) — DOMINANT: geographic/topological positioning
//! - μ (Mapping) — spatial index lookups
//! - κ (Comparison) — proximity/distance computation
//! - N (Quantity) — coordinates, distances, counts
//! - ρ (Recursion) — graph traversal, hierarchical regions
//! - σ (Sequence) — path ordering
//! - ∂ (Boundary) — region edges, topology cuts
//! - ∃ (Existence) — reachability, path existence
//! - Σ (Sum) — region aggregation
//!
//! ## Key Insight
//!
//! In pharmacovigilance, *where* a signal originates determines regulatory
//! jurisdiction, reporting obligations, and population exposure. Geographic
//! clustering of adverse events can reveal manufacturing site contamination
//! or regional prescribing patterns invisible to frequency-only analysis.

use std::collections::HashMap;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// SPATIAL INDEX
// ===============================================================

/// Lightweight spatial index for point-in-region and nearest-neighbor queries.
/// Tier: T2-C (λ μ κ N)
///
/// Grounds λ-first: every operation answers "where is this relative to others?"
/// μ provides O(1) cell lookup, κ orders by distance, N measures coordinates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialIndex<V: Clone> {
    /// Grid cell size for spatial hashing.
    cell_size: f64,
    /// Entries: (x, y, value).
    entries: Vec<(f64, f64, V)>,
    /// Grid cells mapping cell coordinates to entry indices.
    grid: HashMap<(i64, i64), Vec<usize>>,
}

impl<V: Clone> SpatialIndex<V> {
    /// Creates an empty spatial index with the given cell size.
    #[must_use]
    pub fn new(cell_size: f64) -> Self {
        let cs = if cell_size <= 0.0 { 1.0 } else { cell_size };
        Self {
            cell_size: cs,
            entries: Vec::new(),
            grid: HashMap::new(),
        }
    }

    /// Converts a coordinate to a grid cell.
    fn cell_of(&self, x: f64, y: f64) -> (i64, i64) {
        let cx = (x / self.cell_size).floor() as i64;
        let cy = (y / self.cell_size).floor() as i64;
        (cx, cy)
    }

    /// Inserts a point with associated value.
    pub fn insert(&mut self, x: f64, y: f64, value: V) {
        let idx = self.entries.len();
        self.entries.push((x, y, value));
        let cell = self.cell_of(x, y);
        self.grid.entry(cell).or_default().push(idx);
    }

    /// Returns the number of indexed entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Finds all entries within `radius` of the query point.
    #[must_use]
    pub fn within_radius(&self, qx: f64, qy: f64, radius: f64) -> Vec<&V> {
        let r2 = radius * radius;
        let min_cell = self.cell_of(qx - radius, qy - radius);
        let max_cell = self.cell_of(qx + radius, qy + radius);
        let mut results = Vec::new();

        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                if let Some(indices) = self.grid.get(&(cx, cy)) {
                    for &idx in indices {
                        let (ex, ey, ref val) = self.entries[idx];
                        let dx = ex - qx;
                        let dy = ey - qy;
                        if dx * dx + dy * dy <= r2 {
                            results.push(val);
                        }
                    }
                }
            }
        }
        results
    }

    /// Finds the nearest entry to the query point, if any.
    #[must_use]
    pub fn nearest(&self, qx: f64, qy: f64) -> Option<(&V, f64)> {
        let mut best: Option<(usize, f64)> = None;
        for (idx, (ex, ey, _)) in self.entries.iter().enumerate() {
            let dx = ex - qx;
            let dy = ey - qy;
            let d2 = dx * dx + dy * dy;
            match best {
                None => best = Some((idx, d2)),
                Some((_, bd)) if d2 < bd => best = Some((idx, d2)),
                _ => {}
            }
        }
        best.map(|(idx, d2)| (&self.entries[idx].2, d2.sqrt()))
    }

    /// Returns the number of distinct grid cells occupied.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.grid.len()
    }
}

impl<V: Clone> GroundsTo for SpatialIndex<V> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Mapping,
            LexPrimitiva::Comparison,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// TOPOLOGY GRAPH
// ===============================================================

/// Directed graph for topological relationships between locations.
/// Tier: T2-C (λ ρ σ ∂)
///
/// Models adjacency, connectivity, and boundary crossings.
/// ρ enables recursive traversal, σ orders paths, ∂ marks graph cuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    /// Adjacency list: node → [(neighbor, weight)].
    adjacency: HashMap<String, Vec<(String, f64)>>,
    /// Node count.
    node_count: usize,
}

impl TopologyGraph {
    /// Creates an empty topology graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            node_count: 0,
        }
    }

    /// Adds a node if it doesn't exist.
    pub fn add_node(&mut self, id: &str) {
        if !self.adjacency.contains_key(id) {
            self.adjacency.insert(id.to_string(), Vec::new());
            self.node_count += 1;
        }
    }

    /// Adds a directed edge from `src` to `dst` with weight.
    pub fn add_edge(&mut self, src: &str, dst: &str, weight: f64) {
        self.add_node(src);
        self.add_node(dst);
        if let Some(neighbors) = self.adjacency.get_mut(src) {
            neighbors.push((dst.to_string(), weight));
        }
    }

    /// Returns the number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Returns the total number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum()
    }

    /// Returns neighbors of a node.
    #[must_use]
    pub fn neighbors(&self, node: &str) -> Vec<(&str, f64)> {
        self.adjacency
            .get(node)
            .map(|n| n.iter().map(|(s, w)| (s.as_str(), *w)).collect())
            .unwrap_or_default()
    }

    /// Checks if `dst` is reachable from `src` via BFS.
    #[must_use]
    pub fn is_reachable(&self, src: &str, dst: &str) -> bool {
        if src == dst {
            return true;
        }
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        visited.insert(src.to_string());
        queue.push_back(src.to_string());

        while let Some(current) = queue.pop_front() {
            for (neighbor, _) in self.neighbors(&current) {
                if neighbor == dst {
                    return true;
                }
                if visited.insert(neighbor.to_string()) {
                    queue.push_back(neighbor.to_string());
                }
            }
        }
        false
    }

    /// Returns all nodes with no incoming edges (source nodes).
    #[must_use]
    pub fn source_nodes(&self) -> Vec<&str> {
        let mut has_incoming: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for neighbors in self.adjacency.values() {
            for (dst, _) in neighbors {
                has_incoming.insert(dst.as_str());
            }
        }
        self.adjacency
            .keys()
            .filter(|k| !has_incoming.contains(k.as_str()))
            .map(|k| k.as_str())
            .collect()
    }
}

impl Default for TopologyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for TopologyGraph {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Recursion,
            LexPrimitiva::Sequence,
            LexPrimitiva::Boundary,
        ])
    }
}

// ===============================================================
// PATH RESOLVER
// ===============================================================

/// Resolves optimal paths between nodes in a weighted graph.
/// Tier: T2-C (λ σ ∃ ρ)
///
/// σ orders the path steps, ∃ validates path existence, ρ recurses
/// through the graph. Returns `None` when no path exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResolver<V: Clone + Eq + Hash> {
    /// Adjacency: node → [(neighbor, cost)].
    edges: HashMap<V, Vec<(V, f64)>>,
}

impl<V: Clone + Eq + Hash> PathResolver<V> {
    /// Creates an empty path resolver.
    #[must_use]
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// Adds a directed edge.
    pub fn add_edge(&mut self, from: V, to: V, cost: f64) {
        self.edges.entry(from).or_default().push((to, cost));
    }

    /// Returns the number of registered nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.edges.len()
    }
}

impl<V: Clone + Eq + Hash> Default for PathResolver<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl PathResolver<String> {
    /// Resolves the shortest path from `start` to `end` using Dijkstra.
    /// Returns `(path, total_cost)` or `None` if unreachable.
    #[must_use]
    pub fn resolve(&self, start: &str, end: &str) -> Option<(Vec<String>, f64)> {
        use std::cmp::Ordering;
        use std::collections::BinaryHeap;

        #[derive(Debug)]
        struct State {
            cost: f64,
            node: String,
        }

        impl PartialEq for State {
            fn eq(&self, other: &Self) -> bool {
                self.cost.to_bits() == other.cost.to_bits() && self.node == other.node
            }
        }
        impl Eq for State {}

        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other
                    .cost
                    .partial_cmp(&self.cost)
                    .unwrap_or(Ordering::Equal)
            }
        }
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut dist: HashMap<String, f64> = HashMap::new();
        let mut prev: HashMap<String, String> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(start.to_string(), 0.0);
        heap.push(State {
            cost: 0.0,
            node: start.to_string(),
        });

        while let Some(State { cost, node }) = heap.pop() {
            if node == end {
                // Reconstruct path
                let mut path = vec![end.to_string()];
                let mut current = end.to_string();
                while let Some(p) = prev.get(&current) {
                    path.push(p.clone());
                    current = p.clone();
                }
                path.reverse();
                return Some((path, cost));
            }

            let best = dist.get(&node).copied().unwrap_or(f64::INFINITY);
            if cost > best {
                continue;
            }

            if let Some(neighbors) = self.edges.get(&node) {
                for (next, edge_cost) in neighbors {
                    let new_cost = cost + edge_cost;
                    let current_best = dist.get(next).copied().unwrap_or(f64::INFINITY);
                    if new_cost < current_best {
                        dist.insert(next.clone(), new_cost);
                        prev.insert(next.clone(), node.clone());
                        heap.push(State {
                            cost: new_cost,
                            node: next.clone(),
                        });
                    }
                }
            }
        }

        None
    }
}

impl<V: Clone + Eq + Hash> GroundsTo for PathResolver<V> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Sequence,
            LexPrimitiva::Existence,
            LexPrimitiva::Recursion,
        ])
    }
}

// ===============================================================
// REGION PARTITIONER
// ===============================================================

/// Divides a space into non-overlapping regions for jurisdiction mapping.
/// Tier: T2-C (λ ∂ N Σ)
///
/// ∂ defines region boundaries, N sizes them, Σ aggregates contents.
/// PV use case: mapping adverse events to regulatory jurisdictions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionPartitioner {
    /// Regions: name → (min_x, min_y, max_x, max_y).
    regions: Vec<(String, f64, f64, f64, f64)>,
}

impl RegionPartitioner {
    /// Creates an empty partitioner.
    #[must_use]
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// Adds a rectangular region.
    pub fn add_region(&mut self, name: &str, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.regions
            .push((name.to_string(), min_x, min_y, max_x, max_y));
    }

    /// Returns the number of defined regions.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Finds which region(s) a point belongs to.
    #[must_use]
    pub fn locate(&self, x: f64, y: f64) -> Vec<&str> {
        self.regions
            .iter()
            .filter(|(_, min_x, min_y, max_x, max_y)| {
                x >= *min_x && x <= *max_x && y >= *min_y && y <= *max_y
            })
            .map(|(name, _, _, _, _)| name.as_str())
            .collect()
    }

    /// Returns the area of a named region, if it exists.
    #[must_use]
    pub fn area(&self, name: &str) -> Option<f64> {
        self.regions
            .iter()
            .find(|(n, _, _, _, _)| n == name)
            .map(|(_, min_x, min_y, max_x, max_y)| (max_x - min_x) * (max_y - min_y))
    }

    /// Total area across all regions.
    #[must_use]
    pub fn total_area(&self) -> f64 {
        self.regions
            .iter()
            .map(|(_, min_x, min_y, max_x, max_y)| (max_x - min_x) * (max_y - min_y))
            .sum()
    }

    /// Checks if two regions overlap.
    #[must_use]
    pub fn overlaps(&self, a: &str, b: &str) -> bool {
        let ra = self.regions.iter().find(|(n, _, _, _, _)| n == a);
        let rb = self.regions.iter().find(|(n, _, _, _, _)| n == b);
        match (ra, rb) {
            (Some((_, ax1, ay1, ax2, ay2)), Some((_, bx1, by1, bx2, by2))) => {
                ax1 <= bx2 && ax2 >= bx1 && ay1 <= by2 && ay2 >= by1
            }
            _ => false,
        }
    }
}

impl Default for RegionPartitioner {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for RegionPartitioner {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Boundary,
            LexPrimitiva::Quantity,
            LexPrimitiva::Sum,
        ])
    }
}

// ===============================================================
// PROXIMITY ENGINE
// ===============================================================

/// Proximity computation engine for nearest-neighbor and distance queries.
/// Tier: T2-C (λ κ N μ)
///
/// κ compares distances, N measures them, μ maps names to coordinates.
/// Highest confidence in the corpus (0.930) — the primitives align perfectly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityEngine<V: Clone> {
    /// Named points: (name, x, y, value).
    points: Vec<(String, f64, f64, V)>,
}

impl<V: Clone> ProximityEngine<V> {
    /// Creates an empty proximity engine.
    #[must_use]
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Registers a named point.
    pub fn add_point(&mut self, name: &str, x: f64, y: f64, value: V) {
        self.points.push((name.to_string(), x, y, value));
    }

    /// Returns the number of registered points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns true if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Euclidean distance between two coordinates.
    #[must_use]
    pub fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        let dx = x2 - x1;
        let dy = y2 - y1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Finds the K nearest points to the query.
    #[must_use]
    pub fn k_nearest(&self, qx: f64, qy: f64, k: usize) -> Vec<(&str, f64, &V)> {
        let mut scored: Vec<(&str, f64, &V)> = self
            .points
            .iter()
            .map(|(name, x, y, v)| (name.as_str(), Self::distance(qx, qy, *x, *y), v))
            .collect();
        scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }

    /// Returns all points within the given distance.
    #[must_use]
    pub fn within(&self, qx: f64, qy: f64, max_distance: f64) -> Vec<(&str, f64, &V)> {
        self.points
            .iter()
            .filter_map(|(name, x, y, v)| {
                let d = Self::distance(qx, qy, *x, *y);
                if d <= max_distance {
                    Some((name.as_str(), d, v))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Average distance from a query point to all registered points.
    #[must_use]
    pub fn mean_distance(&self, qx: f64, qy: f64) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        let total: f64 = self
            .points
            .iter()
            .map(|(_, x, y, _)| Self::distance(qx, qy, *x, *y))
            .sum();
        total / self.points.len() as f64
    }
}

impl<V: Clone> Default for ProximityEngine<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone> GroundsTo for ProximityEngine<V> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Comparison,
            LexPrimitiva::Quantity,
            LexPrimitiva::Mapping,
        ])
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- SpatialIndex ---

    #[test]
    fn spatial_index_insert_and_len() {
        let mut idx = SpatialIndex::new(10.0);
        idx.insert(5.0, 5.0, "a");
        idx.insert(15.0, 15.0, "b");
        assert_eq!(idx.len(), 2);
        assert!(!idx.is_empty());
    }

    #[test]
    fn spatial_index_empty() {
        let idx: SpatialIndex<u32> = SpatialIndex::new(1.0);
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
    }

    #[test]
    fn spatial_index_within_radius() {
        let mut idx = SpatialIndex::new(5.0);
        idx.insert(0.0, 0.0, "origin");
        idx.insert(3.0, 0.0, "near");
        idx.insert(100.0, 100.0, "far");
        let results = idx.within_radius(0.0, 0.0, 5.0);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn spatial_index_nearest() {
        let mut idx = SpatialIndex::new(5.0);
        idx.insert(10.0, 0.0, "ten");
        idx.insert(2.0, 0.0, "two");
        let result = idx.nearest(0.0, 0.0);
        assert!(result.is_some());
        let (val, dist) = result.expect("tested above");
        assert_eq!(*val, "two");
        assert!((dist - 2.0).abs() < 0.001);
    }

    #[test]
    fn spatial_index_cell_count() {
        let mut idx = SpatialIndex::new(10.0);
        idx.insert(0.0, 0.0, 1);
        idx.insert(5.0, 5.0, 2); // same cell
        idx.insert(15.0, 15.0, 3); // different cell
        assert_eq!(idx.cell_count(), 2);
    }

    #[test]
    fn spatial_index_negative_cell_size() {
        let idx: SpatialIndex<u32> = SpatialIndex::new(-5.0);
        assert_eq!(idx.cell_size, 1.0);
    }

    #[test]
    fn spatial_index_grounding() {
        let comp = SpatialIndex::<u32>::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Location);
        assert_eq!(comp.primitives.len(), 4);
    }

    // --- TopologyGraph ---

    #[test]
    fn topology_graph_add_and_count() {
        let mut g = TopologyGraph::new();
        g.add_edge("A", "B", 1.0);
        g.add_edge("B", "C", 2.0);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }

    #[test]
    fn topology_graph_reachable() {
        let mut g = TopologyGraph::new();
        g.add_edge("A", "B", 1.0);
        g.add_edge("B", "C", 1.0);
        assert!(g.is_reachable("A", "C"));
        assert!(!g.is_reachable("C", "A"));
    }

    #[test]
    fn topology_graph_self_reachable() {
        let g = TopologyGraph::new();
        assert!(g.is_reachable("X", "X"));
    }

    #[test]
    fn topology_graph_source_nodes() {
        let mut g = TopologyGraph::new();
        g.add_edge("root", "child1", 1.0);
        g.add_edge("root", "child2", 1.0);
        g.add_edge("child1", "leaf", 1.0);
        let sources = g.source_nodes();
        assert!(sources.contains(&"root"));
        assert!(!sources.contains(&"leaf"));
    }

    #[test]
    fn topology_graph_grounding() {
        let comp = TopologyGraph::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Location);
    }

    // --- PathResolver ---

    #[test]
    fn path_resolver_shortest_path() {
        let mut pr = PathResolver::new();
        pr.add_edge("A".into(), "B".into(), 1.0);
        pr.add_edge("B".into(), "C".into(), 2.0);
        pr.add_edge("A".into(), "C".into(), 10.0);
        let result = pr.resolve("A", "C");
        assert!(result.is_some());
        let (path, cost) = result.expect("tested above");
        assert_eq!(path, vec!["A", "B", "C"]);
        assert!((cost - 3.0).abs() < 0.001);
    }

    #[test]
    fn path_resolver_no_path() {
        let mut pr = PathResolver::new();
        pr.add_edge("A".into(), "B".into(), 1.0);
        assert!(pr.resolve("B", "A").is_none());
    }

    #[test]
    fn path_resolver_direct() {
        let mut pr = PathResolver::new();
        pr.add_edge("X".into(), "Y".into(), 5.0);
        let result = pr.resolve("X", "Y");
        assert!(result.is_some());
        let (path, cost) = result.expect("tested above");
        assert_eq!(path, vec!["X", "Y"]);
        assert!((cost - 5.0).abs() < 0.001);
    }

    #[test]
    fn path_resolver_grounding() {
        let comp = PathResolver::<String>::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Location);
        assert_eq!(comp.primitives.len(), 4);
    }

    // --- RegionPartitioner ---

    #[test]
    fn region_partitioner_locate() {
        let mut rp = RegionPartitioner::new();
        rp.add_region("USA", -125.0, 24.0, -66.0, 49.0);
        rp.add_region("EU", -10.0, 36.0, 40.0, 71.0);
        let regions = rp.locate(-100.0, 40.0);
        assert!(regions.contains(&"USA"));
        assert!(!regions.contains(&"EU"));
    }

    #[test]
    fn region_partitioner_area() {
        let mut rp = RegionPartitioner::new();
        rp.add_region("square", 0.0, 0.0, 10.0, 10.0);
        let area = rp.area("square");
        assert!(area.is_some());
        assert!((area.expect("tested above") - 100.0).abs() < 0.001);
    }

    #[test]
    fn region_partitioner_total_area() {
        let mut rp = RegionPartitioner::new();
        rp.add_region("a", 0.0, 0.0, 5.0, 5.0);
        rp.add_region("b", 10.0, 10.0, 20.0, 20.0);
        assert!((rp.total_area() - 125.0).abs() < 0.001);
    }

    #[test]
    fn region_partitioner_overlaps() {
        let mut rp = RegionPartitioner::new();
        rp.add_region("a", 0.0, 0.0, 10.0, 10.0);
        rp.add_region("b", 5.0, 5.0, 15.0, 15.0);
        rp.add_region("c", 20.0, 20.0, 30.0, 30.0);
        assert!(rp.overlaps("a", "b"));
        assert!(!rp.overlaps("a", "c"));
    }

    #[test]
    fn region_partitioner_grounding() {
        let comp = RegionPartitioner::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Location);
    }

    // --- ProximityEngine ---

    #[test]
    fn proximity_engine_k_nearest() {
        let mut pe = ProximityEngine::new();
        pe.add_point("a", 1.0, 0.0, ());
        pe.add_point("b", 5.0, 0.0, ());
        pe.add_point("c", 10.0, 0.0, ());
        let results = pe.k_nearest(0.0, 0.0, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert_eq!(results[1].0, "b");
    }

    #[test]
    fn proximity_engine_within() {
        let mut pe = ProximityEngine::new();
        pe.add_point("close", 1.0, 0.0, 10);
        pe.add_point("far", 100.0, 0.0, 20);
        let results = pe.within(0.0, 0.0, 5.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "close");
    }

    #[test]
    fn proximity_engine_mean_distance() {
        let mut pe = ProximityEngine::new();
        pe.add_point("a", 3.0, 0.0, ());
        pe.add_point("b", 5.0, 0.0, ());
        let mean = pe.mean_distance(0.0, 0.0);
        assert!((mean - 4.0).abs() < 0.001);
    }

    #[test]
    fn proximity_engine_empty_mean() {
        let pe: ProximityEngine<()> = ProximityEngine::new();
        assert!((pe.mean_distance(0.0, 0.0)).abs() < 0.001);
    }

    #[test]
    fn proximity_engine_distance_fn() {
        let d = ProximityEngine::<()>::distance(0.0, 0.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn proximity_engine_grounding() {
        let comp = ProximityEngine::<()>::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Location);
        assert_eq!(comp.primitives.len(), 4);
        assert!((comp.confidence - 0.85).abs() < 0.2);
    }
}
