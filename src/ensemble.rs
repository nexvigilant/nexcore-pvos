//! # PVML Model Ensemble & Selection
//!
//! Manages multiple model versions, combines their predictions,
//! and selects the best-performing models through A/B testing.
//!
//! ## Primitives
//! - ρ (Recursion) — selection improves over time
//! - κ (Comparison) — model vs model comparison
//! - π (Persistence) — model version registry
//! - N (Quantity) — performance scores

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::trainer::ModelId;

// ===============================================================
// ENSEMBLE TYPES
// ===============================================================

/// Strategy for combining multiple model predictions.
/// Tier: T2-P (ρ + κ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Use the single best-performing model.
    Best,
    /// Majority vote among models.
    Voting,
    /// Weighted average based on performance.
    WeightedAverage,
    /// Multi-armed bandit: explore/exploit.
    Bandit,
}

impl GroundsTo for SelectionStrategy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Recursion, LexPrimitiva::Comparison])
    }
}

/// Performance record for a model version.
/// Tier: T2-P (N + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    /// Total predictions made.
    pub predictions: u64,
    /// Correct predictions.
    pub correct: u64,
    /// False positives.
    pub false_positives: u64,
    /// False negatives.
    pub false_negatives: u64,
}

impl ModelPerformance {
    /// Creates new empty performance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            predictions: 0,
            correct: 0,
            false_positives: 0,
            false_negatives: 0,
        }
    }

    /// Accuracy.
    #[must_use]
    pub fn accuracy(&self) -> f64 {
        if self.predictions == 0 {
            return 0.0;
        }
        self.correct as f64 / self.predictions as f64
    }

    /// F1 score.
    #[must_use]
    pub fn f1(&self) -> f64 {
        let tp = self.correct;
        let fp = self.false_positives;
        let r#fn = self.false_negatives;

        let precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            0.0
        };
        let recall = if tp + r#fn > 0 {
            tp as f64 / (tp + r#fn) as f64
        } else {
            0.0
        };

        if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        }
    }

    /// Records a correct prediction.
    pub fn record_correct(&mut self) {
        self.predictions += 1;
        self.correct += 1;
    }

    /// Records a false positive.
    pub fn record_false_positive(&mut self) {
        self.predictions += 1;
        self.false_positives += 1;
    }

    /// Records a false negative.
    pub fn record_false_negative(&mut self) {
        self.predictions += 1;
        self.false_negatives += 1;
    }
}

impl Default for ModelPerformance {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for ModelPerformance {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Comparison])
    }
}

// ===============================================================
// MODEL VERSION
// ===============================================================

/// A registered model version with its threshold and performance.
/// Tier: T2-C (π + N + ρ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Model identifier.
    pub id: ModelId,
    /// Version name/label.
    pub name: String,
    /// Detection threshold this model uses.
    pub threshold: f64,
    /// Performance tracking.
    pub performance: ModelPerformance,
    /// Whether this model is active (eligible for selection).
    pub active: bool,
    /// When this version was registered.
    pub registered_at: SystemTime,
}

impl ModelVersion {
    /// Creates a new model version.
    #[must_use]
    pub fn new(id: ModelId, name: &str, threshold: f64) -> Self {
        Self {
            id,
            name: name.to_string(),
            threshold,
            performance: ModelPerformance::new(),
            active: true,
            registered_at: SystemTime::now(),
        }
    }
}

impl GroundsTo for ModelVersion {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence,
            LexPrimitiva::Quantity,
            LexPrimitiva::Recursion,
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.75)
    }
}

// ===============================================================
// A/B TEST
// ===============================================================

/// A/B test comparing two model versions.
/// Tier: T2-C (κ + ρ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    /// Control model (existing).
    pub control: ModelId,
    /// Treatment model (challenger).
    pub treatment: ModelId,
    /// Traffic fraction to treatment (0.0-1.0).
    pub treatment_fraction: f64,
    /// Minimum observations before declaring winner.
    pub min_observations: u64,
    /// Current observation count.
    pub observations: u64,
    /// Whether the test is concluded.
    pub concluded: bool,
    /// Winner (if concluded).
    pub winner: Option<ModelId>,
}

impl ABTest {
    /// Creates a new A/B test.
    #[must_use]
    pub fn new(control: ModelId, treatment: ModelId, treatment_fraction: f64) -> Self {
        Self {
            control,
            treatment,
            treatment_fraction: treatment_fraction.clamp(0.0, 1.0),
            min_observations: 100,
            observations: 0,
            concluded: false,
            winner: None,
        }
    }

    /// Sets minimum observations before conclusion.
    #[must_use]
    pub fn with_min_observations(mut self, min: u64) -> Self {
        self.min_observations = min;
        self
    }

    /// Selects which model to use for this request.
    /// Uses deterministic routing based on a hash-like counter.
    #[must_use]
    pub fn select(&self) -> ModelId {
        // Simple: use observation count modulo to route
        let frac = (self.observations % 100) as f64 / 100.0;
        if frac < self.treatment_fraction {
            self.treatment
        } else {
            self.control
        }
    }

    /// Records an observation and checks if test should conclude.
    pub fn observe(&mut self, used_model: ModelId, correct: bool) {
        self.observations += 1;

        if self.observations >= self.min_observations && !self.concluded {
            self.concluded = true;
            // Winner is determined externally by comparing performance
        }

        let _ = (used_model, correct); // Used by registry to update performance
    }

    /// Sets the winner.
    pub fn conclude(&mut self, winner: ModelId) {
        self.concluded = true;
        self.winner = Some(winner);
    }
}

impl GroundsTo for ABTest {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison,
            LexPrimitiva::Recursion,
            LexPrimitiva::Quantity,
        ])
        .with_dominant(LexPrimitiva::Comparison, 0.75)
    }
}

// ===============================================================
// MODEL REGISTRY
// ===============================================================

/// Registry of model versions.
/// Tier: T2-C (π + ρ + κ + N)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelRegistry {
    /// Registered model versions.
    versions: Vec<ModelVersion>,
    /// Currently selected model.
    current: Option<ModelId>,
    /// A/B tests.
    ab_tests: Vec<ABTest>,
}

impl ModelRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new model version.
    pub fn register(&mut self, version: ModelVersion) {
        if self.current.is_none() {
            self.current = Some(version.id);
        }
        self.versions.push(version);
    }

    /// Gets the current (selected) model ID.
    #[must_use]
    pub fn current(&self) -> Option<ModelId> {
        self.current
    }

    /// Sets the current model.
    pub fn set_current(&mut self, id: ModelId) {
        self.current = Some(id);
    }

    /// Gets a model version by ID.
    #[must_use]
    pub fn get(&self, id: ModelId) -> Option<&ModelVersion> {
        self.versions.iter().find(|v| v.id == id)
    }

    /// Gets a mutable model version by ID.
    pub fn get_mut(&mut self, id: ModelId) -> Option<&mut ModelVersion> {
        self.versions.iter_mut().find(|v| v.id == id)
    }

    /// Returns the best-performing active model.
    #[must_use]
    pub fn best(&self) -> Option<&ModelVersion> {
        self.versions
            .iter()
            .filter(|v| v.active && v.performance.predictions > 0)
            .max_by(|a, b| {
                a.performance
                    .accuracy()
                    .partial_cmp(&b.performance.accuracy())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Number of registered versions.
    #[must_use]
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }

    /// All registered versions.
    #[must_use]
    pub fn versions(&self) -> &[ModelVersion] {
        &self.versions
    }

    /// Starts an A/B test.
    pub fn start_ab_test(&mut self, test: ABTest) {
        self.ab_tests.push(test);
    }

    /// Active A/B tests.
    #[must_use]
    pub fn ab_tests(&self) -> &[ABTest] {
        &self.ab_tests
    }

    /// Deactivates a model version.
    pub fn deactivate(&mut self, id: ModelId) {
        if let Some(v) = self.versions.iter_mut().find(|v| v.id == id) {
            v.active = false;
        }
    }
}

impl GroundsTo for ModelRegistry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence,
            LexPrimitiva::Recursion,
            LexPrimitiva::Comparison,
            LexPrimitiva::Quantity,
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.80)
    }
}

// ===============================================================
// ENSEMBLE
// ===============================================================

/// Model ensemble combining multiple models.
/// Tier: T2-C (ρ + κ + π + N)
///
/// Combines predictions from multiple model versions using
/// a configurable selection strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ensemble {
    /// Selection strategy.
    strategy: SelectionStrategy,
    /// Model registry.
    registry: ModelRegistry,
    /// Total ensemble predictions.
    total_predictions: u64,
}

impl Ensemble {
    /// Creates a new ensemble.
    #[must_use]
    pub fn new(strategy: SelectionStrategy) -> Self {
        Self {
            strategy,
            registry: ModelRegistry::new(),
            total_predictions: 0,
        }
    }

    /// Registers a model version.
    pub fn register(&mut self, version: ModelVersion) {
        self.registry.register(version);
    }

    /// Predicts using the ensemble: returns the threshold to apply.
    #[must_use]
    pub fn predict(&self) -> Option<f64> {
        match self.strategy {
            SelectionStrategy::Best => self
                .registry
                .best()
                .or_else(|| self.registry.current().and_then(|id| self.registry.get(id)))
                .map(|v| v.threshold),
            SelectionStrategy::WeightedAverage => {
                let active: Vec<&ModelVersion> = self
                    .registry
                    .versions()
                    .iter()
                    .filter(|v| v.active)
                    .collect();

                if active.is_empty() {
                    return None;
                }

                let total_accuracy: f64 = active
                    .iter()
                    .map(|v| v.performance.accuracy().max(0.001))
                    .sum();

                let weighted_sum: f64 = active
                    .iter()
                    .map(|v| v.threshold * v.performance.accuracy().max(0.001))
                    .sum();

                Some(weighted_sum / total_accuracy)
            }
            SelectionStrategy::Voting | SelectionStrategy::Bandit => {
                // For Voting/Bandit, fall back to best
                self.registry
                    .current()
                    .and_then(|id| self.registry.get(id))
                    .map(|v| v.threshold)
            }
        }
    }

    /// Records a prediction outcome for the given model.
    pub fn record_outcome(&mut self, model_id: ModelId, correct: bool) {
        self.total_predictions += 1;
        if let Some(version) = self.registry.get_mut(model_id) {
            if correct {
                version.performance.record_correct();
            } else {
                version.performance.record_false_positive();
            }
        }
    }

    /// Model registry reference.
    #[must_use]
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Mutable registry reference.
    pub fn registry_mut(&mut self) -> &mut ModelRegistry {
        &mut self.registry
    }

    /// Total predictions made.
    #[must_use]
    pub fn total_predictions(&self) -> u64 {
        self.total_predictions
    }

    /// Selection strategy.
    #[must_use]
    pub fn strategy(&self) -> SelectionStrategy {
        self.strategy
    }
}

impl GroundsTo for Ensemble {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Comparison,
            LexPrimitiva::Persistence,
            LexPrimitiva::Quantity,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_version(id: u64, name: &str, threshold: f64) -> ModelVersion {
        ModelVersion::new(ModelId(id), name, threshold)
    }

    #[test]
    fn test_model_performance_accuracy() {
        let mut perf = ModelPerformance::new();
        for _ in 0..8 {
            perf.record_correct();
        }
        perf.record_false_positive();
        perf.record_false_negative();

        assert!((perf.accuracy() - 0.8).abs() < f64::EPSILON);
        assert_eq!(perf.predictions, 10);
    }

    #[test]
    fn test_model_performance_empty() {
        let perf = ModelPerformance::new();
        assert!((perf.accuracy() - 0.0).abs() < f64::EPSILON);
        assert!((perf.f1() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_model_registry_register() {
        let mut reg = ModelRegistry::new();
        reg.register(make_version(1, "v1", 2.0));
        reg.register(make_version(2, "v2", 2.5));

        assert_eq!(reg.version_count(), 2);
        assert_eq!(reg.current(), Some(ModelId(1))); // First registered becomes current
    }

    #[test]
    fn test_model_registry_best() {
        let mut reg = ModelRegistry::new();

        let mut v1 = make_version(1, "v1", 2.0);
        for _ in 0..5 {
            v1.performance.record_correct();
        }
        v1.performance.record_false_positive();

        let mut v2 = make_version(2, "v2", 2.5);
        for _ in 0..9 {
            v2.performance.record_correct();
        }
        v2.performance.record_false_positive();

        reg.register(v1);
        reg.register(v2);

        let best = reg.best();
        assert!(best.is_some());
        assert_eq!(best.map(|v| v.id), Some(ModelId(2))); // v2 has higher accuracy
    }

    #[test]
    fn test_model_registry_deactivate() {
        let mut reg = ModelRegistry::new();
        reg.register(make_version(1, "v1", 2.0));
        reg.deactivate(ModelId(1));

        assert!(reg.get(ModelId(1)).map(|v| !v.active).unwrap_or(false));
    }

    #[test]
    fn test_ensemble_best_strategy() {
        let mut ensemble = Ensemble::new(SelectionStrategy::Best);

        let mut v1 = make_version(1, "v1", 2.0);
        v1.performance.record_correct();
        ensemble.register(v1);

        let threshold = ensemble.predict();
        assert!(threshold.is_some());
    }

    #[test]
    fn test_ensemble_weighted_average() {
        let mut ensemble = Ensemble::new(SelectionStrategy::WeightedAverage);

        let mut v1 = make_version(1, "low", 1.0);
        v1.performance.record_correct();
        v1.performance.record_correct();

        let mut v2 = make_version(2, "high", 3.0);
        v2.performance.record_correct();
        v2.performance.record_correct();

        ensemble.register(v1);
        ensemble.register(v2);

        let threshold = ensemble.predict();
        assert!(threshold.is_some());
        // Both have same accuracy, so weighted average = (1.0 + 3.0) / 2 = 2.0
        if let Some(t) = threshold {
            assert!((t - 2.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_ensemble_record_outcome() {
        let mut ensemble = Ensemble::new(SelectionStrategy::Best);
        ensemble.register(make_version(1, "v1", 2.0));

        ensemble.record_outcome(ModelId(1), true);
        ensemble.record_outcome(ModelId(1), false);

        assert_eq!(ensemble.total_predictions(), 2);
        let v = ensemble.registry().get(ModelId(1));
        assert!(v.is_some());
        if let Some(v) = v {
            assert_eq!(v.performance.predictions, 2);
        }
    }

    #[test]
    fn test_ab_test_selection() {
        let test = ABTest::new(ModelId(1), ModelId(2), 0.5);
        let selected = test.select();
        // Should select either control or treatment
        assert!(selected == ModelId(1) || selected == ModelId(2));
    }

    #[test]
    fn test_ab_test_conclude() {
        let mut test = ABTest::new(ModelId(1), ModelId(2), 0.3).with_min_observations(5);

        for _ in 0..5 {
            test.observe(ModelId(1), true);
        }

        assert!(test.concluded);
        test.conclude(ModelId(1));
        assert_eq!(test.winner, Some(ModelId(1)));
    }

    #[test]
    fn test_ensemble_grounding() {
        let comp = Ensemble::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }

    #[test]
    fn test_model_registry_grounding() {
        let comp = ModelRegistry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }
}
