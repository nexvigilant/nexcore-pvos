//! # PVML Model Training
//!
//! Epoch-based training loop with checkpointing and early stopping.
//! The training loop is the pure ρ-operation: iterate epochs until
//! convergence or resource exhaustion.
//!
//! ## Primitives
//! - ρ (Recursion) — the training loop itself
//! - σ (Sequence) — training data batches
//! - π (Persistence) — model checkpoints
//! - N (Quantity) — loss, accuracy metrics
//! - ∂ (Boundary) — early stopping convergence threshold

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// TRAINING TYPES
// ===============================================================

/// Unique model identifier.
/// Tier: T2-P (π)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub u64);

impl GroundsTo for ModelId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Persistence])
    }
}

/// Epoch counter.
/// Tier: T2-P (ρ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Epoch(pub u32);

impl GroundsTo for Epoch {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Recursion])
    }
}

/// Learning rate parameter.
/// Tier: T2-P (N)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LearningRate(pub f64);

impl LearningRate {
    /// Default learning rate.
    #[must_use]
    pub fn default_rate() -> Self {
        Self(0.01)
    }
}

impl GroundsTo for LearningRate {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
    }
}

/// Loss value from training.
/// Tier: T2-P (N + κ)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Loss(pub f64);

impl GroundsTo for Loss {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Comparison])
    }
}

/// Training data sample (prediction statistic + whether it was a true signal).
/// Tier: T2-P (σ + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Predicted statistic value.
    pub statistic: f64,
    /// Whether this was truly a signal.
    pub is_signal: bool,
    /// Algorithm used.
    pub algorithm: String,
}

impl GroundsTo for TrainingSample {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Comparison])
    }
}

// ===============================================================
// TRAINING CONFIGURATION
// ===============================================================

/// Configuration for a training run.
/// Tier: T2-C (ρ + σ + N + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Maximum epochs.
    pub max_epochs: u32,
    /// Mini-batch size.
    pub batch_size: usize,
    /// Learning rate.
    pub learning_rate: LearningRate,
    /// Early stopping patience (epochs without improvement).
    pub patience: u32,
    /// Minimum improvement to reset patience counter.
    pub min_delta: f64,
}

impl TrainingConfig {
    /// Default training configuration.
    #[must_use]
    pub fn default_config() -> Self {
        Self {
            max_epochs: 100,
            batch_size: 32,
            learning_rate: LearningRate::default_rate(),
            patience: 10,
            min_delta: 0.001,
        }
    }

    /// Fast training (fewer epochs, larger batches).
    #[must_use]
    pub fn fast() -> Self {
        Self {
            max_epochs: 20,
            batch_size: 64,
            learning_rate: LearningRate(0.05),
            patience: 5,
            min_delta: 0.01,
        }
    }
}

impl GroundsTo for TrainingConfig {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Sequence,
            LexPrimitiva::Quantity,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.75)
    }
}

// ===============================================================
// CHECKPOINT
// ===============================================================

/// A saved model state at a particular epoch.
/// Tier: T2-P (π + ρ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Model identifier.
    pub model_id: ModelId,
    /// Epoch at which checkpoint was taken.
    pub epoch: Epoch,
    /// Loss at checkpoint.
    pub loss: Loss,
    /// Threshold parameters (the "model weights").
    pub thresholds: Vec<(String, f64)>,
    /// When the checkpoint was created.
    pub created_at: SystemTime,
}

impl Checkpoint {
    /// Creates a new checkpoint.
    #[must_use]
    pub fn new(
        model_id: ModelId,
        epoch: Epoch,
        loss: Loss,
        thresholds: Vec<(String, f64)>,
    ) -> Self {
        Self {
            model_id,
            epoch,
            loss,
            thresholds,
            created_at: SystemTime::now(),
        }
    }
}

impl GroundsTo for Checkpoint {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Persistence, LexPrimitiva::Recursion])
    }
}

// ===============================================================
// EARLY STOPPING
// ===============================================================

/// Early stopping monitor.
/// Tier: T2-P (∂ + ρ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlyStopping {
    /// Best loss seen so far.
    best_loss: f64,
    /// Epochs since last improvement.
    epochs_without_improvement: u32,
    /// Patience threshold.
    patience: u32,
    /// Minimum improvement to count.
    min_delta: f64,
}

impl EarlyStopping {
    /// Creates a new early stopping monitor.
    #[must_use]
    pub fn new(patience: u32, min_delta: f64) -> Self {
        Self {
            best_loss: f64::MAX,
            epochs_without_improvement: 0,
            patience,
            min_delta,
        }
    }

    /// Records a loss and returns true if training should stop.
    pub fn check(&mut self, loss: f64) -> bool {
        if self.best_loss - loss > self.min_delta {
            self.best_loss = loss;
            self.epochs_without_improvement = 0;
            false
        } else {
            self.epochs_without_improvement += 1;
            self.epochs_without_improvement >= self.patience
        }
    }

    /// Best loss seen.
    #[must_use]
    pub fn best_loss(&self) -> f64 {
        self.best_loss
    }

    /// Resets the early stopping state.
    pub fn reset(&mut self) {
        self.best_loss = f64::MAX;
        self.epochs_without_improvement = 0;
    }
}

impl GroundsTo for EarlyStopping {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Recursion])
    }
}

// ===============================================================
// TRAINING RESULT
// ===============================================================

/// Result of a training run.
/// Tier: T2-C (ρ + N + π + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    /// Model identifier.
    pub model_id: ModelId,
    /// Final epoch reached.
    pub final_epoch: Epoch,
    /// Final loss.
    pub final_loss: Loss,
    /// Best loss achieved.
    pub best_loss: Loss,
    /// Whether early stopping triggered.
    pub early_stopped: bool,
    /// Loss history per epoch.
    pub loss_history: Vec<f64>,
    /// Final checkpoint.
    pub checkpoint: Option<Checkpoint>,
}

impl GroundsTo for TrainingResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Quantity,
            LexPrimitiva::Persistence,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.80)
    }
}

// ===============================================================
// TRAINING LOOP
// ===============================================================

/// The training loop — epoch-based iterative model improvement.
/// Tier: T2-C (ρ + σ + π + N + ∂)
///
/// This is the pure ρ-structure: iterate epochs, compute loss,
/// checkpoint when improving, stop when converged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoop {
    /// Configuration.
    config: TrainingConfig,
    /// Early stopping monitor.
    early_stopping: EarlyStopping,
    /// Checkpoints (best models saved).
    checkpoints: Vec<Checkpoint>,
    /// Next model ID.
    next_model_id: u64,
    /// Total training runs completed.
    total_runs: u64,
}

impl TrainingLoop {
    /// Creates a new training loop.
    #[must_use]
    pub fn new(config: TrainingConfig) -> Self {
        let patience = config.patience;
        let min_delta = config.min_delta;
        Self {
            config,
            early_stopping: EarlyStopping::new(patience, min_delta),
            checkpoints: Vec::new(),
            next_model_id: 1,
            total_runs: 0,
        }
    }

    /// Runs a training loop on the given data.
    ///
    /// The training "model" here is simple: find the optimal threshold
    /// that minimizes classification error on the training data.
    /// This is intentionally simple — the ρ-structure (loop, checkpoint,
    /// early-stop) is what matters, not ML complexity.
    pub fn train(&mut self, data: &[TrainingSample]) -> TrainingResult {
        let model_id = ModelId(self.next_model_id);
        self.next_model_id += 1;
        self.early_stopping.reset();
        self.total_runs += 1;

        let mut loss_history = Vec::new();
        let mut best_loss = f64::MAX;
        let mut best_threshold = 2.0;
        let mut final_epoch = Epoch(0);
        let mut early_stopped = false;
        let mut best_checkpoint = None;

        // The ρ-core: iterate epochs
        for epoch_num in 0..self.config.max_epochs {
            final_epoch = Epoch(epoch_num);

            // Compute loss for current threshold candidate
            let threshold = 1.0 + (epoch_num as f64) * 0.1 * self.config.learning_rate.0;
            let loss = self.compute_loss(data, threshold);
            loss_history.push(loss);

            // Check for improvement
            if loss < best_loss {
                best_loss = loss;
                best_threshold = threshold;

                // Checkpoint (π)
                let cp = Checkpoint::new(
                    model_id,
                    Epoch(epoch_num),
                    Loss(loss),
                    vec![("threshold".to_string(), threshold)],
                );
                best_checkpoint = Some(cp.clone());
                self.checkpoints.push(cp);
            }

            // Early stopping check (∂)
            if self.early_stopping.check(loss) {
                early_stopped = true;
                break;
            }
        }

        // Store best threshold in final checkpoint if not already there
        if best_checkpoint.is_none() {
            best_checkpoint = Some(Checkpoint::new(
                model_id,
                final_epoch,
                Loss(best_loss),
                vec![("threshold".to_string(), best_threshold)],
            ));
        }

        TrainingResult {
            model_id,
            final_epoch,
            final_loss: Loss(*loss_history.last().unwrap_or(&best_loss)),
            best_loss: Loss(best_loss),
            early_stopped,
            loss_history,
            checkpoint: best_checkpoint,
        }
    }

    /// Computes classification loss for a threshold on training data.
    fn compute_loss(&self, data: &[TrainingSample], threshold: f64) -> f64 {
        if data.is_empty() {
            return 1.0;
        }

        let errors: usize = data
            .iter()
            .filter(|s| {
                let predicted = s.statistic >= threshold;
                predicted != s.is_signal
            })
            .count();

        errors as f64 / data.len() as f64
    }

    /// Returns all checkpoints.
    #[must_use]
    pub fn checkpoints(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Total training runs.
    #[must_use]
    pub fn total_runs(&self) -> u64 {
        self.total_runs
    }

    /// Training configuration.
    #[must_use]
    pub fn config(&self) -> &TrainingConfig {
        &self.config
    }
}

impl GroundsTo for TrainingLoop {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Sequence,
            LexPrimitiva::Persistence,
            LexPrimitiva::Quantity,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_data() -> Vec<TrainingSample> {
        let mut data = Vec::new();
        for i in 0..20 {
            let stat = 0.5 + (i as f64) * 0.3;
            data.push(TrainingSample {
                statistic: stat,
                is_signal: stat >= 3.0,
                algorithm: "PRR".to_string(),
            });
        }
        data
    }

    #[test]
    fn test_training_loop_runs() {
        let config = TrainingConfig::fast();
        let mut tl = TrainingLoop::new(config);
        let data = make_data();

        let result = tl.train(&data);
        assert!(result.final_epoch.0 > 0);
        assert!(result.best_loss.0 >= 0.0);
        assert!(result.best_loss.0 <= 1.0);
    }

    #[test]
    fn test_training_produces_checkpoint() {
        let config = TrainingConfig::fast();
        let mut tl = TrainingLoop::new(config);
        let data = make_data();

        let result = tl.train(&data);
        assert!(result.checkpoint.is_some());
    }

    #[test]
    fn test_training_loss_history() {
        let config = TrainingConfig {
            max_epochs: 10,
            batch_size: 32,
            learning_rate: LearningRate(0.01),
            patience: 100,
            min_delta: 0.0001,
        };
        let mut tl = TrainingLoop::new(config);
        let data = make_data();

        let result = tl.train(&data);
        assert_eq!(result.loss_history.len(), 10);
    }

    #[test]
    fn test_early_stopping() {
        let mut es = EarlyStopping::new(3, 0.01);

        assert!(!es.check(1.0)); // New best (delta=MAX-1.0 > 0.01)
        assert!(!es.check(0.5)); // Better (delta=0.5 > 0.01)
        assert!(!es.check(0.5)); // No improvement → epochs=1
        assert!(!es.check(0.5)); // No improvement → epochs=2
        assert!(es.check(0.5)); // No improvement → epochs=3 ≥ 3 → stop
    }

    #[test]
    fn test_early_stopping_reset() {
        let mut es = EarlyStopping::new(2, 0.01);

        es.check(1.0);
        es.check(1.0);
        assert!(es.check(1.0)); // Would stop

        es.reset();
        assert!(!es.check(1.0)); // Reset: starts fresh
    }

    #[test]
    fn test_training_early_stop() {
        let config = TrainingConfig {
            max_epochs: 1000,
            batch_size: 32,
            learning_rate: LearningRate(0.001),
            patience: 5,
            min_delta: 0.001,
        };
        let mut tl = TrainingLoop::new(config);

        // Constant data where loss won't change much
        let data = vec![
            TrainingSample {
                statistic: 3.0,
                is_signal: true,
                algorithm: "PRR".into(),
            },
            TrainingSample {
                statistic: 1.0,
                is_signal: false,
                algorithm: "PRR".into(),
            },
        ];

        let result = tl.train(&data);
        assert!(result.early_stopped);
        assert!(result.final_epoch.0 < 1000);
    }

    #[test]
    fn test_training_empty_data() {
        let config = TrainingConfig::fast();
        let mut tl = TrainingLoop::new(config);

        let result = tl.train(&[]);
        assert!((result.best_loss.0 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_checkpoint_creation() {
        let cp = Checkpoint::new(
            ModelId(1),
            Epoch(5),
            Loss(0.15),
            vec![("threshold".into(), 2.5)],
        );
        assert_eq!(cp.model_id, ModelId(1));
        assert_eq!(cp.epoch, Epoch(5));
    }

    #[test]
    fn test_training_total_runs() {
        let config = TrainingConfig::fast();
        let mut tl = TrainingLoop::new(config);
        let data = make_data();

        tl.train(&data);
        tl.train(&data);
        assert_eq!(tl.total_runs(), 2);
    }

    #[test]
    fn test_training_loop_grounding() {
        let comp = TrainingLoop::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }

    #[test]
    fn test_training_result_grounding() {
        let comp = TrainingResult::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }
}
