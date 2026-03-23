//! # PVEX Exploratory Types
//!
//! Experimental types for state-space exploration, resource topology,
//! and schema-guided data partitioning. These types push the primitive
//! composition into T3 territory where 6+ primitives compose into
//! domain-specific reasoning engines.
//!
//! ## Primitives
//! - ς (State) — state-space enumeration, superposition
//! - Σ (Sum) — resource aggregation across cloud regions
//! - κ (Comparison) — splitting criteria, state evaluation
//! - → (Causality) — state transitions, consequence modeling
//! - ∂ (Boundary) — partition boundaries, state-space limits
//! - ρ (Recursion) — recursive exploration, tree traversal
//! - N (Quantity) — resource counts, partition sizes, state dimensions
//! - σ (Sequence) — ordered splitting, schema field ordering
//! - μ (Mapping) — field-to-type mapping in schemas
//! - λ (Location) — cloud resource placement
//!
//! ## Key Insight
//!
//! QuantumStateSpace has the lowest confidence (0.844) and highest primitive
//! count (7) in the corpus. This is the complexity frontier — where T3
//! emerges and the type becomes irreducibly domain-specific.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// QUANTUM STATE SPACE
// ===============================================================

/// Multi-dimensional state space for exploring decision landscapes.
/// Tier: T3 (ς Σ κ → ∂ ρ N) — 7 primitives, confidence 0.844
///
/// The most complex type in the research corpus. Models a space where
/// each dimension represents a decision variable, and states can be
/// in superposition (multiple possibilities evaluated simultaneously).
///
/// PV application: multi-criteria benefit-risk assessment where drug
/// safety decisions involve simultaneous evaluation across efficacy,
/// risk, population exposure, regulatory context, and market factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumStateSpace {
    /// Dimensions of the state space.
    dimensions: Vec<Dimension>,
    /// Explored states: state_key → (value, explored).
    states: HashMap<String, (f64, bool)>,
    /// Transition rules: from_state → [(to_state, probability)].
    transitions: HashMap<String, Vec<(String, f64)>>,
    /// Maximum states to explore before halting.
    max_states: usize,
    /// States explored so far.
    explored_count: usize,
}

/// A dimension in the state space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    /// Dimension name.
    pub name: String,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// Step size for discretization.
    pub step: f64,
}

impl QuantumStateSpace {
    /// Creates a new state space.
    #[must_use]
    pub fn new(max_states: usize) -> Self {
        Self {
            dimensions: Vec::new(),
            states: HashMap::new(),
            transitions: HashMap::new(),
            max_states,
            explored_count: 0,
        }
    }

    /// Adds a dimension to the state space.
    pub fn add_dimension(&mut self, name: &str, min: f64, max: f64, step: f64) {
        self.dimensions.push(Dimension {
            name: name.to_string(),
            min,
            max,
            step: if step <= 0.0 { 1.0 } else { step },
        });
    }

    /// Returns the number of dimensions.
    #[must_use]
    pub fn dimension_count(&self) -> usize {
        self.dimensions.len()
    }

    /// Returns the theoretical state space size (product of dimension sizes).
    #[must_use]
    pub fn theoretical_size(&self) -> usize {
        self.dimensions
            .iter()
            .map(|d| ((d.max - d.min) / d.step).ceil() as usize + 1)
            .product()
    }

    /// Registers a state with a value.
    pub fn register_state(&mut self, key: &str, value: f64) {
        self.states.insert(key.to_string(), (value, false));
    }

    /// Marks a state as explored.
    pub fn explore(&mut self, key: &str) -> Option<f64> {
        if self.explored_count >= self.max_states {
            return None;
        }
        if let Some(state) = self.states.get_mut(key) {
            if !state.1 {
                state.1 = true;
                self.explored_count += 1;
            }
            Some(state.0)
        } else {
            None
        }
    }

    /// Adds a transition between states.
    pub fn add_transition(&mut self, from: &str, to: &str, probability: f64) {
        self.transitions
            .entry(from.to_string())
            .or_default()
            .push((to.to_string(), probability.clamp(0.0, 1.0)));
    }

    /// Returns the number of registered states.
    #[must_use]
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Returns the number of explored states.
    #[must_use]
    pub fn explored_count(&self) -> usize {
        self.explored_count
    }

    /// Returns the exploration ratio.
    #[must_use]
    pub fn exploration_ratio(&self) -> f64 {
        if self.states.is_empty() {
            return 0.0;
        }
        self.explored_count as f64 / self.states.len() as f64
    }

    /// Returns the best (highest value) explored state.
    #[must_use]
    pub fn best_state(&self) -> Option<(&str, f64)> {
        self.states
            .iter()
            .filter(|(_, (_, explored))| *explored)
            .max_by(|(_, (a, _)), (_, (b, _))| {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(key, (val, _))| (key.as_str(), *val))
    }

    /// Returns whether the exploration budget is exhausted.
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.explored_count >= self.max_states
    }
}

impl GroundsTo for QuantumStateSpace {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,
            LexPrimitiva::Sum,
            LexPrimitiva::Comparison,
            LexPrimitiva::Causality,
            LexPrimitiva::Boundary,
            LexPrimitiva::Recursion,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// CLOUD RESOURCE GRAPH
// ===============================================================

/// Graph of cloud resources with cost, location, and dependency edges.
/// Tier: T2-C (Σ ρ κ N λ) — 5 primitives, confidence 0.890
///
/// Σ aggregates resource costs, ρ recurses through dependency chains,
/// κ compares costs for optimization, N measures resource quantities,
/// λ tracks geographic placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudResourceGraph {
    /// Resources: id → (name, region, cost_per_unit).
    resources: HashMap<String, (String, String, f64)>,
    /// Dependencies: resource_id → [dependency_ids].
    dependencies: HashMap<String, Vec<String>>,
}

impl CloudResourceGraph {
    /// Creates an empty resource graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Adds a resource.
    pub fn add_resource(&mut self, id: &str, name: &str, region: &str, cost: f64) {
        self.resources
            .insert(id.to_string(), (name.to_string(), region.to_string(), cost));
    }

    /// Adds a dependency: `from` depends on `to`.
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.dependencies
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
    }

    /// Returns the number of resources.
    #[must_use]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Returns the total cost across all resources.
    #[must_use]
    pub fn total_cost(&self) -> f64 {
        self.resources.values().map(|(_, _, c)| c).sum()
    }

    /// Returns the cost for a specific region.
    #[must_use]
    pub fn cost_by_region(&self, region: &str) -> f64 {
        self.resources
            .values()
            .filter(|(_, r, _)| r == region)
            .map(|(_, _, c)| c)
            .sum()
    }

    /// Returns unique regions.
    #[must_use]
    pub fn regions(&self) -> Vec<String> {
        let mut regions: Vec<String> = self.resources.values().map(|(_, r, _)| r.clone()).collect();
        regions.sort();
        regions.dedup();
        regions
    }

    /// Returns direct dependencies for a resource.
    #[must_use]
    pub fn dependencies_of(&self, id: &str) -> Vec<&str> {
        self.dependencies
            .get(id)
            .map(|deps| deps.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Returns the transitive dependency count (recursive).
    #[must_use]
    pub fn transitive_dependency_count(&self, id: &str) -> usize {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(current) = stack.pop() {
            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    if visited.insert(dep.clone()) {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        visited.len()
    }

    /// Returns resources with no dependencies (leaf resources).
    #[must_use]
    pub fn leaf_resources(&self) -> Vec<&str> {
        self.resources
            .keys()
            .filter(|id| {
                self.dependencies
                    .get(id.as_str())
                    .map_or(true, |d| d.is_empty())
            })
            .map(|s| s.as_str())
            .collect()
    }
}

impl Default for CloudResourceGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for CloudResourceGraph {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Recursion,
            LexPrimitiva::Comparison,
            LexPrimitiva::Quantity,
            LexPrimitiva::Location,
        ])
    }
}

// ===============================================================
// SCHEMA GUIDED SPLITTER
// ===============================================================

/// Splits datasets based on schema field types and value distributions.
/// Tier: T2-C (κ σ μ ∂ N) — 5 primitives, confidence 0.888
///
/// κ evaluates split quality (like Gini/entropy in decision trees),
/// σ maintains field ordering, μ maps fields to types, ∂ defines
/// split boundaries, N counts split sizes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaGuidedSplitter {
    /// Schema fields: name → type_hint.
    fields: Vec<(String, FieldType)>,
    /// Split history: field_name → split_count.
    split_counts: HashMap<String, u32>,
    /// Total rows processed.
    total_rows: u64,
}

/// Schema field type for split strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// Numeric: split by threshold.
    Numeric,
    /// Categorical: split by value set.
    Categorical,
    /// Temporal: split by time boundary.
    Temporal,
    /// Text: split by pattern/regex.
    Text,
    /// Boolean: binary split.
    Boolean,
}

impl SchemaGuidedSplitter {
    /// Creates a new splitter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            split_counts: HashMap::new(),
            total_rows: 0,
        }
    }

    /// Adds a field to the schema.
    pub fn add_field(&mut self, name: &str, field_type: FieldType) {
        self.fields.push((name.to_string(), field_type));
    }

    /// Returns the number of schema fields.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns the fields in order.
    #[must_use]
    pub fn fields(&self) -> Vec<(&str, FieldType)> {
        self.fields.iter().map(|(n, t)| (n.as_str(), *t)).collect()
    }

    /// Records a split on a field.
    pub fn record_split(&mut self, field_name: &str, rows: u64) {
        *self.split_counts.entry(field_name.to_string()).or_insert(0) += 1;
        self.total_rows += rows;
    }

    /// Returns the split count for a field.
    #[must_use]
    pub fn splits_for(&self, field_name: &str) -> u32 {
        self.split_counts.get(field_name).copied().unwrap_or(0)
    }

    /// Returns the total rows processed.
    #[must_use]
    pub fn total_rows(&self) -> u64 {
        self.total_rows
    }

    /// Returns the field most frequently used for splits.
    #[must_use]
    pub fn most_split_field(&self) -> Option<(&str, u32)> {
        let mut best: Option<(&String, &u32)> = None;
        for (k, v) in &self.split_counts {
            match best {
                None => best = Some((k, v)),
                Some((_, bv)) if v > bv => best = Some((k, v)),
                _ => {}
            }
        }
        best.map(|(k, v)| (k.as_str(), *v))
    }

    /// Returns the recommended split strategy for a field type.
    #[must_use]
    pub fn recommended_strategy(field_type: FieldType) -> &'static str {
        match field_type {
            FieldType::Numeric => "threshold_bisect",
            FieldType::Categorical => "value_partition",
            FieldType::Temporal => "time_window",
            FieldType::Text => "pattern_match",
            FieldType::Boolean => "binary_split",
        }
    }

    /// Returns numeric fields (candidates for threshold splits).
    #[must_use]
    pub fn numeric_fields(&self) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|(_, t)| matches!(t, FieldType::Numeric))
            .map(|(n, _)| n.as_str())
            .collect()
    }
}

impl Default for SchemaGuidedSplitter {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for SchemaGuidedSplitter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison,
            LexPrimitiva::Sequence,
            LexPrimitiva::Mapping,
            LexPrimitiva::Boundary,
            LexPrimitiva::Quantity,
        ])
    }
}

impl GroundsTo for FieldType {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- QuantumStateSpace ---

    #[test]
    fn qss_dimension_management() {
        let mut qss = QuantumStateSpace::new(100);
        qss.add_dimension("efficacy", 0.0, 1.0, 0.1);
        qss.add_dimension("risk", 0.0, 1.0, 0.1);
        assert_eq!(qss.dimension_count(), 2);
    }

    #[test]
    fn qss_theoretical_size() {
        let mut qss = QuantumStateSpace::new(1000);
        qss.add_dimension("x", 0.0, 10.0, 1.0);
        qss.add_dimension("y", 0.0, 5.0, 1.0);
        assert_eq!(qss.theoretical_size(), 66); // 11 * 6
    }

    #[test]
    fn qss_explore_state() {
        let mut qss = QuantumStateSpace::new(10);
        qss.register_state("s1", 0.8);
        qss.register_state("s2", 0.6);
        let val = qss.explore("s1");
        assert_eq!(val, Some(0.8));
        assert_eq!(qss.explored_count(), 1);
    }

    #[test]
    fn qss_exploration_budget() {
        let mut qss = QuantumStateSpace::new(2);
        qss.register_state("a", 1.0);
        qss.register_state("b", 2.0);
        qss.register_state("c", 3.0);
        assert!(qss.explore("a").is_some());
        assert!(qss.explore("b").is_some());
        assert!(qss.explore("c").is_none()); // budget exhausted
        assert!(qss.is_exhausted());
    }

    #[test]
    fn qss_best_state() {
        let mut qss = QuantumStateSpace::new(10);
        qss.register_state("low", 0.2);
        qss.register_state("high", 0.9);
        qss.explore("low");
        qss.explore("high");
        let best = qss.best_state();
        assert!(best.is_some());
        let (key, val) = best.expect("tested above");
        assert_eq!(key, "high");
        assert!((val - 0.9).abs() < 0.001);
    }

    #[test]
    fn qss_exploration_ratio() {
        let mut qss = QuantumStateSpace::new(100);
        qss.register_state("a", 1.0);
        qss.register_state("b", 2.0);
        qss.explore("a");
        assert!((qss.exploration_ratio() - 0.5).abs() < 0.001);
    }

    #[test]
    fn qss_grounding() {
        let comp = QuantumStateSpace::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::State);
        assert_eq!(comp.primitives.len(), 7); // T3
    }

    // --- CloudResourceGraph ---

    #[test]
    fn crg_resource_management() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("vm-1", "gateway", "us-east-1", 10.0);
        crg.add_resource("vm-2", "worker", "eu-west-1", 15.0);
        assert_eq!(crg.resource_count(), 2);
    }

    #[test]
    fn crg_total_cost() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("a", "svc-a", "us", 10.0);
        crg.add_resource("b", "svc-b", "us", 20.0);
        crg.add_resource("c", "svc-c", "eu", 5.0);
        assert!((crg.total_cost() - 35.0).abs() < 0.001);
    }

    #[test]
    fn crg_cost_by_region() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("a", "x", "us", 10.0);
        crg.add_resource("b", "y", "us", 20.0);
        crg.add_resource("c", "z", "eu", 5.0);
        assert!((crg.cost_by_region("us") - 30.0).abs() < 0.001);
    }

    #[test]
    fn crg_regions() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("a", "x", "us-east-1", 1.0);
        crg.add_resource("b", "y", "eu-west-1", 2.0);
        let regions = crg.regions();
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn crg_dependencies() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("api", "api", "us", 10.0);
        crg.add_resource("db", "db", "us", 20.0);
        crg.add_resource("cache", "cache", "us", 5.0);
        crg.add_dependency("api", "db");
        crg.add_dependency("api", "cache");
        assert_eq!(crg.dependencies_of("api").len(), 2);
    }

    #[test]
    fn crg_transitive_deps() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("a", "a", "us", 1.0);
        crg.add_resource("b", "b", "us", 1.0);
        crg.add_resource("c", "c", "us", 1.0);
        crg.add_dependency("a", "b");
        crg.add_dependency("b", "c");
        assert_eq!(crg.transitive_dependency_count("a"), 2); // b and c
    }

    #[test]
    fn crg_leaf_resources() {
        let mut crg = CloudResourceGraph::new();
        crg.add_resource("api", "api", "us", 10.0);
        crg.add_resource("db", "db", "us", 20.0);
        crg.add_dependency("api", "db");
        let leaves = crg.leaf_resources();
        assert!(leaves.contains(&"db"));
    }

    #[test]
    fn crg_grounding() {
        let comp = CloudResourceGraph::primitive_composition();
        assert_eq!(comp.dominant.expect("has dominant"), LexPrimitiva::Sum);
        assert_eq!(comp.primitives.len(), 5);
    }

    // --- SchemaGuidedSplitter ---

    #[test]
    fn sgs_field_management() {
        let mut sgs = SchemaGuidedSplitter::new();
        sgs.add_field("age", FieldType::Numeric);
        sgs.add_field("country", FieldType::Categorical);
        sgs.add_field("active", FieldType::Boolean);
        assert_eq!(sgs.field_count(), 3);
    }

    #[test]
    fn sgs_split_tracking() {
        let mut sgs = SchemaGuidedSplitter::new();
        sgs.add_field("age", FieldType::Numeric);
        sgs.record_split("age", 1000);
        sgs.record_split("age", 500);
        assert_eq!(sgs.splits_for("age"), 2);
        assert_eq!(sgs.total_rows(), 1500);
    }

    #[test]
    fn sgs_most_split_field() {
        let mut sgs = SchemaGuidedSplitter::new();
        sgs.record_split("a", 100);
        sgs.record_split("b", 100);
        sgs.record_split("b", 100);
        let best = sgs.most_split_field();
        assert!(best.is_some());
        let (field, count) = best.expect("tested above");
        assert_eq!(field, "b");
        assert_eq!(count, 2);
    }

    #[test]
    fn sgs_recommended_strategies() {
        assert_eq!(
            SchemaGuidedSplitter::recommended_strategy(FieldType::Numeric),
            "threshold_bisect"
        );
        assert_eq!(
            SchemaGuidedSplitter::recommended_strategy(FieldType::Boolean),
            "binary_split"
        );
    }

    #[test]
    fn sgs_numeric_fields() {
        let mut sgs = SchemaGuidedSplitter::new();
        sgs.add_field("age", FieldType::Numeric);
        sgs.add_field("name", FieldType::Text);
        sgs.add_field("score", FieldType::Numeric);
        assert_eq!(sgs.numeric_fields().len(), 2);
    }

    #[test]
    fn sgs_grounding() {
        let comp = SchemaGuidedSplitter::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Comparison
        );
        assert_eq!(comp.primitives.len(), 5);
    }

    #[test]
    fn field_type_grounding() {
        let comp = FieldType::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Comparison
        );
        assert_eq!(comp.primitives.len(), 1); // T1
    }
}
