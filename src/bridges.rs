//! # PVBR Bridge Types
//!
//! Cross-layer integration types that connect PVOS subsystems.
//! These types compose primitives from multiple layers — they are the
//! "nervous system" of the OS, coordinating between specialized organs.
//!
//! ## Primitives
//! - → (Causality) — coordination triggers downstream effects
//! - κ (Comparison) — schema validation, energy threshold evaluation
//! - μ (Mapping) — crate/subsystem address resolution
//! - ρ (Recursion) — recursive coordination patterns
//! - ς (State) — energy state, executor lifecycle
//! - N (Quantity) — energy budgets, token counts
//! - π (Persistence) — immune memory, schema contracts
//! - ν (Frequency) — scan/refresh cadence
//! - ∃ (Existence) — schema existence validation
//! - λ (Location) — coordinate space addressing
//!
//! ## Key Insight
//!
//! Bridge types are where T3 emerges: 6+ primitives compose into
//! domain-specific orchestration. The NeuroendocrineCoordinator has 7
//! primitives — the most complex type in this research corpus. It's the
//! "hypothalamus" of PVOS: slow hormonal signals + fast neural signals.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// NEUROENDOCRINE COORDINATOR
// ===============================================================

/// Cross-crate coordinator modeled on the neuroendocrine system.
/// Tier: T3 (→ μ ρ N λ π ς) — 7 primitives, confidence 0.882
///
/// Combines fast event-driven signals (neural/cytokine) with slow
/// configuration propagation (hormonal). Routes messages based on
/// source location (λ), maintains state (ς), persists routing
/// rules (π), and recurses through dependency chains (ρ).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuroendocrineCoordinator {
    /// Fast signal routes: event_type → [target_crates].
    neural_routes: HashMap<String, Vec<String>>,
    /// Slow config propagation routes: config_key → [target_crates].
    hormonal_routes: HashMap<String, Vec<String>>,
    /// Source location metadata: crate_name → location_tag.
    location_map: HashMap<String, String>,
    /// Signal history depth for recursive pattern detection.
    history_depth: usize,
    /// Recent signals (ring buffer semantics).
    signal_history: Vec<(String, String)>,
    /// Total signals routed.
    total_routed: u64,
    /// Active state.
    active: bool,
}

impl NeuroendocrineCoordinator {
    /// Creates a new coordinator.
    #[must_use]
    pub fn new(history_depth: usize) -> Self {
        Self {
            neural_routes: HashMap::new(),
            hormonal_routes: HashMap::new(),
            location_map: HashMap::new(),
            history_depth,
            signal_history: Vec::new(),
            total_routed: 0,
            active: true,
        }
    }

    /// Registers a fast (neural) route.
    pub fn add_neural_route(&mut self, event_type: &str, targets: Vec<String>) {
        self.neural_routes.insert(event_type.to_string(), targets);
    }

    /// Registers a slow (hormonal) route.
    pub fn add_hormonal_route(&mut self, config_key: &str, targets: Vec<String>) {
        self.hormonal_routes.insert(config_key.to_string(), targets);
    }

    /// Maps a crate to a location tag.
    pub fn set_location(&mut self, crate_name: &str, location: &str) {
        self.location_map
            .insert(crate_name.to_string(), location.to_string());
    }

    /// Routes a neural signal, returning the target crates.
    #[must_use]
    pub fn route_neural(&mut self, event_type: &str, source: &str) -> Vec<&str> {
        self.total_routed += 1;
        self.signal_history
            .push((event_type.to_string(), source.to_string()));
        if self.signal_history.len() > self.history_depth {
            self.signal_history.remove(0);
        }
        self.neural_routes
            .get(event_type)
            .map(|targets| targets.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Routes a hormonal config change, returning the target crates.
    #[must_use]
    pub fn route_hormonal(&mut self, config_key: &str) -> Vec<&str> {
        self.total_routed += 1;
        self.hormonal_routes
            .get(config_key)
            .map(|targets| targets.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Returns the total number of signals routed.
    #[must_use]
    pub fn total_routed(&self) -> u64 {
        self.total_routed
    }

    /// Returns the number of neural route definitions.
    #[must_use]
    pub fn neural_route_count(&self) -> usize {
        self.neural_routes.len()
    }

    /// Returns the number of hormonal route definitions.
    #[must_use]
    pub fn hormonal_route_count(&self) -> usize {
        self.hormonal_routes.len()
    }

    /// Returns the location tag for a crate, if registered.
    #[must_use]
    pub fn location_of(&self, crate_name: &str) -> Option<&str> {
        self.location_map.get(crate_name).map(|s| s.as_str())
    }

    /// Detects if the same event type appears more than `threshold` times
    /// in recent history (recursive pattern).
    #[must_use]
    pub fn detect_storm(&self, event_type: &str, threshold: usize) -> bool {
        self.signal_history
            .iter()
            .filter(|(et, _)| et == event_type)
            .count()
            >= threshold
    }

    /// Returns whether the coordinator is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Deactivates the coordinator (emergency shutdown).
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl GroundsTo for NeuroendocrineCoordinator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,
            LexPrimitiva::Mapping,
            LexPrimitiva::Recursion,
            LexPrimitiva::Quantity,
            LexPrimitiva::Location,
            LexPrimitiva::Persistence,
            LexPrimitiva::State,
        ])
    }
}

// ===============================================================
// ENERGETIC EXECUTOR
// ===============================================================

/// Executor that gates task execution on energy (token) budgets.
/// Tier: T2-C (→ ς κ N)
///
/// → triggers execution, ς tracks energy state, κ compares budget vs cost,
/// N measures the token quantities. Models the nexcore-energy ATP/ADP cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergeticExecutor {
    /// Available energy budget (tokens).
    budget: u64,
    /// Maximum budget capacity.
    capacity: u64,
    /// Total tasks executed.
    tasks_executed: u64,
    /// Total energy consumed.
    energy_consumed: u64,
    /// Whether the executor is in low-energy mode.
    low_energy: bool,
    /// Threshold below which low-energy mode activates.
    low_threshold: u64,
}

impl EnergeticExecutor {
    /// Creates a new executor with the given capacity.
    #[must_use]
    pub fn new(capacity: u64) -> Self {
        let threshold = capacity / 5; // 20% threshold
        Self {
            budget: capacity,
            capacity,
            tasks_executed: 0,
            energy_consumed: 0,
            low_energy: false,
            low_threshold: threshold,
        }
    }

    /// Returns the current budget.
    #[must_use]
    pub fn budget(&self) -> u64 {
        self.budget
    }

    /// Returns the capacity.
    #[must_use]
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Returns the energy charge ratio (0.0 to 1.0).
    #[must_use]
    pub fn charge(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.budget as f64 / self.capacity as f64
    }

    /// Returns whether low-energy mode is active.
    #[must_use]
    pub fn is_low_energy(&self) -> bool {
        self.low_energy
    }

    /// Attempts to execute a task with the given energy cost.
    /// Returns `true` if executed, `false` if insufficient budget.
    pub fn execute(&mut self, cost: u64) -> bool {
        if cost > self.budget {
            return false;
        }
        self.budget -= cost;
        self.tasks_executed += 1;
        self.energy_consumed += cost;
        self.low_energy = self.budget <= self.low_threshold;
        true
    }

    /// Recharges the budget by the given amount (capped at capacity).
    pub fn recharge(&mut self, amount: u64) {
        self.budget = (self.budget + amount).min(self.capacity);
        self.low_energy = self.budget <= self.low_threshold;
    }

    /// Returns the total tasks executed.
    #[must_use]
    pub fn tasks_executed(&self) -> u64 {
        self.tasks_executed
    }

    /// Returns average energy per task.
    #[must_use]
    pub fn avg_energy_per_task(&self) -> f64 {
        if self.tasks_executed == 0 {
            return 0.0;
        }
        self.energy_consumed as f64 / self.tasks_executed as f64
    }
}

impl GroundsTo for EnergeticExecutor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,
            LexPrimitiva::State,
            LexPrimitiva::Comparison,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// SCHEMA IMMUNE SYSTEM
// ===============================================================

/// Schema-aware immune system that detects and responds to contract violations.
/// Tier: T3 (κ ∃ μ π ρ ν) — 6 primitives, confidence 0.870
///
/// κ compares schemas for drift, ∃ validates field existence, μ maps schema
/// to contract, π persists known-good baselines, ρ recurses nested schemas,
/// ν sets scan frequency. The biological analog: adaptive immunity that
/// "remembers" past violations (π) and speeds response on re-encounter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaImmuneSystem {
    /// Known-good schema baselines: schema_name → field list.
    baselines: HashMap<String, Vec<String>>,
    /// Violation history: schema_name → violation count.
    violation_counts: HashMap<String, u64>,
    /// Scan interval in seconds.
    scan_interval_secs: u64,
    /// Total scans performed.
    total_scans: u64,
    /// Total violations detected.
    total_violations: u64,
    /// Active antibodies (response rules).
    antibodies: Vec<(String, String)>, // (pattern, action)
}

impl SchemaImmuneSystem {
    /// Creates a new schema immune system.
    #[must_use]
    pub fn new(scan_interval_secs: u64) -> Self {
        Self {
            baselines: HashMap::new(),
            violation_counts: HashMap::new(),
            scan_interval_secs,
            total_scans: 0,
            total_violations: 0,
            antibodies: Vec::new(),
        }
    }

    /// Registers a known-good schema baseline.
    pub fn register_baseline(&mut self, name: &str, fields: Vec<String>) {
        self.baselines.insert(name.to_string(), fields);
    }

    /// Scans a schema against its baseline. Returns list of missing fields.
    #[must_use]
    pub fn scan(&mut self, name: &str, current_fields: &[String]) -> Vec<String> {
        self.total_scans += 1;
        let missing = match self.baselines.get(name) {
            Some(baseline) => baseline
                .iter()
                .filter(|f| !current_fields.contains(f))
                .cloned()
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };
        if !missing.is_empty() {
            self.total_violations += missing.len() as u64;
            *self.violation_counts.entry(name.to_string()).or_insert(0) += missing.len() as u64;
        }
        missing
    }

    /// Adds an antibody rule (pattern → action).
    pub fn add_antibody(&mut self, pattern: &str, action: &str) {
        self.antibodies
            .push((pattern.to_string(), action.to_string()));
    }

    /// Returns the total number of baselines registered.
    #[must_use]
    pub fn baseline_count(&self) -> usize {
        self.baselines.len()
    }

    /// Returns the total violations detected.
    #[must_use]
    pub fn total_violations(&self) -> u64 {
        self.total_violations
    }

    /// Returns the total scans performed.
    #[must_use]
    pub fn total_scans(&self) -> u64 {
        self.total_scans
    }

    /// Returns the violation count for a specific schema.
    #[must_use]
    pub fn violations_for(&self, name: &str) -> u64 {
        self.violation_counts.get(name).copied().unwrap_or(0)
    }

    /// Returns the number of active antibodies.
    #[must_use]
    pub fn antibody_count(&self) -> usize {
        self.antibodies.len()
    }

    /// Returns the scan interval.
    #[must_use]
    pub fn scan_interval_secs(&self) -> u64 {
        self.scan_interval_secs
    }

    /// Returns matched antibody actions for a violation pattern.
    #[must_use]
    pub fn match_antibodies(&self, violation: &str) -> Vec<&str> {
        self.antibodies
            .iter()
            .filter(|(pattern, _)| violation.contains(pattern.as_str()))
            .map(|(_, action)| action.as_str())
            .collect()
    }
}

impl GroundsTo for SchemaImmuneSystem {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison,
            LexPrimitiva::Existence,
            LexPrimitiva::Mapping,
            LexPrimitiva::Persistence,
            LexPrimitiva::Recursion,
            LexPrimitiva::Frequency,
        ])
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- NeuroendocrineCoordinator ---

    #[test]
    fn neuroendocrine_neural_routing() {
        let mut nc = NeuroendocrineCoordinator::new(10);
        nc.add_neural_route("safety_signal", vec!["guardian".into(), "alerting".into()]);
        let targets = nc.route_neural("safety_signal", "vigilance");
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&"guardian"));
    }

    #[test]
    fn neuroendocrine_hormonal_routing() {
        let mut nc = NeuroendocrineCoordinator::new(10);
        nc.add_hormonal_route("threshold_change", vec!["detection".into()]);
        let targets = nc.route_hormonal("threshold_change");
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn neuroendocrine_location_mapping() {
        let mut nc = NeuroendocrineCoordinator::new(5);
        nc.set_location("nexcore-energy", "foundation");
        assert_eq!(nc.location_of("nexcore-energy"), Some("foundation"));
        assert!(nc.location_of("unknown").is_none());
    }

    #[test]
    fn neuroendocrine_storm_detection() {
        let mut nc = NeuroendocrineCoordinator::new(20);
        nc.add_neural_route("alert", vec!["target".into()]);
        for _ in 0..5 {
            nc.route_neural("alert", "src");
        }
        assert!(nc.detect_storm("alert", 5));
        assert!(!nc.detect_storm("alert", 10));
    }

    #[test]
    fn neuroendocrine_deactivation() {
        let mut nc = NeuroendocrineCoordinator::new(5);
        assert!(nc.is_active());
        nc.deactivate();
        assert!(!nc.is_active());
    }

    #[test]
    fn neuroendocrine_total_routed() {
        let mut nc = NeuroendocrineCoordinator::new(5);
        nc.add_neural_route("e", vec!["t".into()]);
        nc.route_neural("e", "s");
        nc.route_hormonal("k");
        assert_eq!(nc.total_routed(), 2);
    }

    #[test]
    fn neuroendocrine_grounding() {
        let comp = NeuroendocrineCoordinator::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Causality
        );
        assert_eq!(comp.primitives.len(), 7); // T3
    }

    // --- EnergeticExecutor ---

    #[test]
    fn energetic_executor_basic_execute() {
        let mut ee = EnergeticExecutor::new(100);
        assert!(ee.execute(30));
        assert_eq!(ee.budget(), 70);
        assert_eq!(ee.tasks_executed(), 1);
    }

    #[test]
    fn energetic_executor_insufficient_budget() {
        let mut ee = EnergeticExecutor::new(10);
        assert!(!ee.execute(20));
        assert_eq!(ee.budget(), 10);
    }

    #[test]
    fn energetic_executor_low_energy_mode() {
        let mut ee = EnergeticExecutor::new(100); // threshold = 20
        assert!(!ee.is_low_energy());
        ee.execute(85); // budget = 15 < 20
        assert!(ee.is_low_energy());
    }

    #[test]
    fn energetic_executor_recharge() {
        let mut ee = EnergeticExecutor::new(100);
        ee.execute(80);
        ee.recharge(50);
        assert_eq!(ee.budget(), 70); // 20 + 50
    }

    #[test]
    fn energetic_executor_recharge_caps_at_capacity() {
        let mut ee = EnergeticExecutor::new(100);
        ee.recharge(200);
        assert_eq!(ee.budget(), 100);
    }

    #[test]
    fn energetic_executor_charge_ratio() {
        let mut ee = EnergeticExecutor::new(100);
        ee.execute(50);
        assert!((ee.charge() - 0.5).abs() < 0.001);
    }

    #[test]
    fn energetic_executor_avg_energy() {
        let mut ee = EnergeticExecutor::new(1000);
        ee.execute(100);
        ee.execute(200);
        assert!((ee.avg_energy_per_task() - 150.0).abs() < 0.001);
    }

    #[test]
    fn energetic_executor_grounding() {
        let comp = EnergeticExecutor::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Causality
        );
        assert_eq!(comp.primitives.len(), 4);
    }

    // --- SchemaImmuneSystem ---

    #[test]
    fn schema_immune_baseline_and_scan() {
        let mut sis = SchemaImmuneSystem::new(60);
        sis.register_baseline(
            "icsr",
            vec!["patient_id".into(), "drug".into(), "event".into()],
        );
        let current = vec!["patient_id".into(), "drug".into()];
        let missing = sis.scan("icsr", &current);
        assert_eq!(missing, vec!["event"]);
        assert_eq!(sis.total_violations(), 1);
    }

    #[test]
    fn schema_immune_clean_scan() {
        let mut sis = SchemaImmuneSystem::new(60);
        sis.register_baseline("report", vec!["id".into(), "body".into()]);
        let current = vec!["id".into(), "body".into()];
        let missing = sis.scan("report", &current);
        assert!(missing.is_empty());
        assert_eq!(sis.total_violations(), 0);
    }

    #[test]
    fn schema_immune_unknown_schema() {
        let mut sis = SchemaImmuneSystem::new(30);
        let missing = sis.scan("unknown", &["field".into()]);
        assert!(missing.is_empty());
    }

    #[test]
    fn schema_immune_antibody_matching() {
        let mut sis = SchemaImmuneSystem::new(60);
        sis.add_antibody("missing_field", "quarantine");
        sis.add_antibody("type_mismatch", "alert");
        let actions = sis.match_antibodies("missing_field:patient_id");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], "quarantine");
    }

    #[test]
    fn schema_immune_violation_tracking() {
        let mut sis = SchemaImmuneSystem::new(60);
        sis.register_baseline("a", vec!["x".into(), "y".into()]);
        sis.scan("a", &["x".into()]);
        sis.scan("a", &["x".into()]);
        assert_eq!(sis.violations_for("a"), 2);
        assert_eq!(sis.violations_for("b"), 0);
    }

    #[test]
    fn schema_immune_grounding() {
        let comp = SchemaImmuneSystem::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Comparison
        );
        assert_eq!(comp.primitives.len(), 6); // T3
    }
}
