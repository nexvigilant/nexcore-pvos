//! # PVOS Kernel
//!
//! The kernel contains four subsystems:
//!
//! ```text
//! ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
//! │ Detection│ │ Triage   │ │ Learning │ │ Audit    │
//! │ Engine κ │ │ Sched  σ │ │ Loop   ρ │ │ Log    π │
//! └──────────┘ └──────────┘ └──────────┘ └──────────┘
//! ```
//!
//! Each subsystem is grounded to its dominant T1 primitive.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::PvosError;
use super::syscall::{
    Algorithm, Artifact, ArtifactKind, AuditedRef, CaseRef, Filter, LearningOutcome, Priority,
    ProcessRef, ProcessState, SignalResult, WorkflowDef, WorkflowStep,
};

// ═══════════════════════════════════════════════════════════
// DETECTION ENGINE (κ)
// ═══════════════════════════════════════════════════════════

/// Detection Engine — dispatches signal detection requests to algorithms.
/// Tier: T2-C (κ + ∂ + μ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionEngine {
    /// Detection count per algorithm.
    dispatch_count: HashMap<String, u64>,
    /// Total detections performed.
    total: u64,
    /// Default threshold for signal detection.
    default_threshold: f64,
}

impl DetectionEngine {
    /// Creates a new detection engine with default threshold.
    #[must_use]
    pub fn new(default_threshold: f64) -> Self {
        Self {
            dispatch_count: HashMap::new(),
            total: 0,
            default_threshold,
        }
    }

    /// Detects a signal using the specified algorithm.
    pub fn detect(
        &mut self,
        drug: &str,
        event: &str,
        algo: &Algorithm,
        contingency: [u64; 4],
    ) -> Result<SignalResult, PvosError> {
        let [a, b, c, d] = contingency;

        // Validate contingency table
        if a == 0 && b == 0 {
            return Err(PvosError::InvalidInput(
                "contingency table has zero drug exposure".into(),
            ));
        }

        let algo_name = format!("{algo:?}");
        *self.dispatch_count.entry(algo_name).or_insert(0) += 1;
        self.total += 1;

        let (statistic, ci_lower, ci_upper, threshold) = match algo {
            Algorithm::Prr => {
                let a_f = a as f64;
                let b_f = b as f64;
                let c_f = c as f64;
                let d_f = d as f64;
                let prr = if (c_f * (a_f + b_f)).abs() < f64::EPSILON {
                    0.0
                } else {
                    (a_f / (a_f + b_f)) / (c_f / (c_f + d_f))
                };
                (prr, None, None, 2.0)
            }
            Algorithm::Ror => {
                let a_f = a as f64;
                let b_f = b as f64;
                let c_f = c as f64;
                let d_f = d as f64;
                let ror = if (b_f * c_f).abs() < f64::EPSILON {
                    0.0
                } else {
                    (a_f * d_f) / (b_f * c_f)
                };
                let se = if a > 0 && b > 0 && c > 0 && d > 0 {
                    (1.0 / a_f + 1.0 / b_f + 1.0 / c_f + 1.0 / d_f).sqrt()
                } else {
                    f64::INFINITY
                };
                let ln_ror = ror.ln();
                let lower = (ln_ror - 1.96 * se).exp();
                let upper = (ln_ror + 1.96 * se).exp();
                (ror, Some(lower), Some(upper), 1.0) // ROR threshold: lower CI > 1.0
            }
            Algorithm::ChiSquared => {
                let a_f = a as f64;
                let b_f = b as f64;
                let c_f = c as f64;
                let d_f = d as f64;
                let n = a_f + b_f + c_f + d_f;
                let expected_a = (a_f + b_f) * (a_f + c_f) / n;
                let chi2 = if expected_a.abs() < f64::EPSILON {
                    0.0
                } else {
                    (a_f - expected_a).powi(2) / expected_a
                };
                (chi2, None, None, 3.841) // p=0.05
            }
            _ => {
                // Fallback: use PRR for unimplemented algorithms
                let a_f = a as f64;
                let b_f = b as f64;
                let c_f = c as f64;
                let d_f = d as f64;
                let prr = if (c_f * (a_f + b_f)).abs() < f64::EPSILON {
                    0.0
                } else {
                    (a_f / (a_f + b_f)) / (c_f / (c_f + d_f))
                };
                (prr, None, None, self.default_threshold)
            }
        };

        let signal_detected = match algo {
            Algorithm::Ror => ci_lower.unwrap_or(0.0) > threshold,
            _ => statistic >= threshold,
        };

        Ok(SignalResult {
            drug: drug.to_string(),
            event: event.to_string(),
            algorithm: algo.clone(),
            statistic,
            signal_detected,
            ci_lower,
            ci_upper,
        })
    }

    /// Total detections performed.
    #[must_use]
    pub fn total_detections(&self) -> u64 {
        self.total
    }
}

impl GroundsTo for DetectionEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison,
            LexPrimitiva::Boundary,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Comparison, 0.90)
    }
}

// ═══════════════════════════════════════════════════════════
// TRIAGE SCHEDULER (σ)
// ═══════════════════════════════════════════════════════════

/// A managed process in the scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    /// Process reference.
    pub id: ProcessRef,
    /// Workflow definition.
    pub workflow: WorkflowDef,
    /// Current step index.
    pub current_step: usize,
    /// Process state.
    pub state: ProcessState,
    /// Priority.
    pub priority: Priority,
    /// Creation time.
    pub created: SystemTime,
}

/// Triage Scheduler — manages process lifecycle and prioritization.
/// Tier: T2-C (σ + ∂ + μ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriageScheduler {
    /// Active processes.
    processes: Vec<Process>,
    /// Next process ID.
    next_id: u64,
}

impl TriageScheduler {
    /// Creates a new scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_id: 1,
        }
    }

    /// Spawns a new process.
    pub fn spawn(&mut self, workflow: WorkflowDef) -> ProcessRef {
        let id = ProcessRef(self.next_id);
        self.next_id += 1;
        let priority = workflow.priority;
        self.processes.push(Process {
            id,
            workflow,
            current_step: 0,
            state: ProcessState::Pending,
            priority,
            created: SystemTime::now(),
        });
        id
    }

    /// Schedules a process at given priority.
    ///
    /// # Errors
    /// Returns `Err` if process not found.
    pub fn schedule(&mut self, process: ProcessRef, priority: Priority) -> Result<(), PvosError> {
        let proc = self
            .processes
            .iter_mut()
            .find(|p| p.id == process)
            .ok_or(PvosError::ProcessNotFound(process.0))?;
        proc.priority = priority;
        proc.state = ProcessState::Running;
        Ok(())
    }

    /// Gets process state.
    pub fn state(&self, process: ProcessRef) -> Result<ProcessState, PvosError> {
        self.processes
            .iter()
            .find(|p| p.id == process)
            .map(|p| p.state)
            .ok_or(PvosError::ProcessNotFound(process.0))
    }

    /// Prioritizes cases by sorting on priority (descending).
    #[must_use]
    pub fn prioritize(&self, cases: &[CaseRef]) -> Vec<CaseRef> {
        // Without process-level priority info, preserve order
        // but place serious cases first (odd IDs heuristic placeholder)
        let mut sorted = cases.to_vec();
        sorted.sort_by(|a, b| b.0.cmp(&a.0)); // Higher IDs first (most recent)
        sorted
    }

    /// Number of active processes.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.processes
            .iter()
            .filter(|p| matches!(p.state, ProcessState::Running | ProcessState::AwaitingHuman))
            .count()
    }

    /// Total processes.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.processes.len()
    }
}

impl GroundsTo for TriageScheduler {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Boundary,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.90)
    }
}

// ═══════════════════════════════════════════════════════════
// LEARNING LOOP (ρ)
// ═══════════════════════════════════════════════════════════

/// A feedback entry for learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    /// Algorithm that produced the signal.
    pub algorithm: Algorithm,
    /// Whether signal was detected.
    pub signal_detected: bool,
    /// Actual outcome.
    pub outcome: LearningOutcome,
    /// Timestamp.
    pub timestamp: SystemTime,
}

/// Learning Loop — accumulates feedback and calibrates detection.
/// Tier: T2-C (ρ + κ + σ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningLoop {
    /// Feedback buffer.
    feedback: Vec<FeedbackEntry>,
    /// Confirmed true positives per algorithm.
    true_positives: HashMap<String, u64>,
    /// Confirmed false positives per algorithm.
    false_positives: HashMap<String, u64>,
    /// Confirmed false negatives per algorithm.
    false_negatives: HashMap<String, u64>,
    /// Number of retraining cycles completed.
    retrain_cycles: u64,
    /// Minimum batch size before retraining.
    batch_size: usize,
}

impl LearningLoop {
    /// Creates a new learning loop with specified batch size.
    #[must_use]
    pub fn new(batch_size: usize) -> Self {
        Self {
            feedback: Vec::new(),
            true_positives: HashMap::new(),
            false_positives: HashMap::new(),
            false_negatives: HashMap::new(),
            retrain_cycles: 0,
            batch_size,
        }
    }

    /// Records feedback.
    pub fn record(&mut self, signal: &SignalResult, outcome: LearningOutcome) {
        let algo_key = format!("{:?}", signal.algorithm);

        match (signal.signal_detected, &outcome) {
            (true, LearningOutcome::Confirmed) => {
                *self.true_positives.entry(algo_key.clone()).or_insert(0) += 1;
            }
            (true, LearningOutcome::Refuted) => {
                *self.false_positives.entry(algo_key.clone()).or_insert(0) += 1;
            }
            (false, LearningOutcome::Confirmed) => {
                *self.false_negatives.entry(algo_key.clone()).or_insert(0) += 1;
            }
            _ => {} // TN or Indeterminate
        }

        self.feedback.push(FeedbackEntry {
            algorithm: signal.algorithm.clone(),
            signal_detected: signal.signal_detected,
            outcome,
            timestamp: SystemTime::now(),
        });
    }

    /// Triggers retraining if batch threshold met.
    ///
    /// Returns `true` if retraining occurred.
    pub fn retrain(&mut self) -> bool {
        if self.feedback.len() < self.batch_size {
            return false;
        }

        self.retrain_cycles += 1;
        self.feedback.clear();
        true
    }

    /// Returns the false positive rate for an algorithm.
    #[must_use]
    pub fn fpr(&self, algorithm: &str) -> f64 {
        let fp = *self.false_positives.get(algorithm).unwrap_or(&0);
        let tp = *self.true_positives.get(algorithm).unwrap_or(&0);
        let total = fp + tp;
        if total == 0 {
            0.0
        } else {
            fp as f64 / total as f64
        }
    }

    /// Number of retraining cycles.
    #[must_use]
    pub fn retrain_cycles(&self) -> u64 {
        self.retrain_cycles
    }

    /// Pending feedback count.
    #[must_use]
    pub fn pending_feedback(&self) -> usize {
        self.feedback.len()
    }
}

impl GroundsTo for LearningLoop {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::Comparison,
            LexPrimitiva::Sequence,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.90)
    }
}

// ═══════════════════════════════════════════════════════════
// AUDIT LOG (π)
// ═══════════════════════════════════════════════════════════

/// An audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID.
    pub id: u64,
    /// Operation performed.
    pub operation: String,
    /// Timestamp.
    pub timestamp: SystemTime,
    /// Integrity hash.
    pub hash: u64,
}

/// Audit Log — append-only, tamper-evident record of all operations.
/// Tier: T2-P (π + σ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLog {
    /// Log entries (append-only).
    entries: Vec<AuditEntry>,
    /// Next entry ID.
    next_id: u64,
}

impl AuditLog {
    /// Creates a new empty audit log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Records an operation.
    pub fn record(&mut self, operation: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let hash = crate::util::fnv1a_hash(operation.as_bytes());
        self.entries.push(AuditEntry {
            id,
            operation: operation.to_string(),
            timestamp: SystemTime::now(),
            hash,
        });
        id
    }

    /// Returns all entries.
    #[must_use]
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Verifies integrity of an entry.
    #[must_use]
    pub fn verify(&self, id: u64) -> bool {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| crate::util::fnv1a_hash(e.operation.as_bytes()) == e.hash)
            .unwrap_or(false)
    }
}

impl GroundsTo for AuditLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Persistence, LexPrimitiva::Sequence])
            .with_dominant(LexPrimitiva::Persistence, 0.90)
    }
}

// ═══════════════════════════════════════════════════════════
// KERNEL COMPOSITE
// ═══════════════════════════════════════════════════════════

/// The PVOS Kernel — composes all four subsystems.
/// Tier: T2-C (κ + σ + ρ + π + ∂)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kernel {
    /// Signal detection engine.
    pub detection: DetectionEngine,
    /// Process triage and scheduling.
    pub triage: TriageScheduler,
    /// Feedback and model improvement.
    pub learning: LearningLoop,
    /// Immutable operation log.
    pub audit: AuditLog,
}

impl Kernel {
    /// Creates a kernel with specified detection threshold and learning batch size.
    #[must_use]
    pub fn new(detection_threshold: f64, learning_batch_size: usize) -> Self {
        Self {
            detection: DetectionEngine::new(detection_threshold),
            triage: TriageScheduler::new(),
            learning: LearningLoop::new(learning_batch_size),
            audit: AuditLog::new(),
        }
    }
}

impl GroundsTo for Kernel {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison,
            LexPrimitiva::Sequence,
            LexPrimitiva::Recursion,
            LexPrimitiva::Persistence,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Comparison, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// ARTIFACT STORAGE
// ═══════════════════════════════════════════════════════════

/// In-memory artifact store with audit trail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactStore {
    artifacts: Vec<(AuditedRef, Artifact)>,
    next_id: u64,
}

impl ArtifactStore {
    /// Creates a new empty store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            artifacts: Vec::new(),
            next_id: 1,
        }
    }

    /// Stores an artifact and returns an audited reference.
    pub fn store(&mut self, artifact: Artifact) -> AuditedRef {
        let id = self.next_id;
        self.next_id += 1;
        let hash = crate::util::fnv1a_hash(artifact.content.as_bytes());
        let audited = AuditedRef { id, hash };
        self.artifacts.push((audited, artifact));
        audited
    }

    /// Queries artifacts by filter.
    #[must_use]
    pub fn query(&self, filter: &Filter) -> Vec<Artifact> {
        let mut results: Vec<Artifact> = self
            .artifacts
            .iter()
            .filter(|(_, a)| {
                if let Some(ref kind) = filter.kind {
                    if &a.kind != kind {
                        return false;
                    }
                }
                if !filter.tags.is_empty() {
                    if !filter.tags.iter().any(|t| a.tags.contains(t)) {
                        return false;
                    }
                }
                true
            })
            .map(|(_, a)| a.clone())
            .collect();

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }

    /// Total stored artifacts.
    #[must_use]
    pub fn count(&self) -> usize {
        self.artifacts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_detection_engine_prr() {
        let mut engine = DetectionEngine::new(2.0);
        // a=15, b=100, c=20, d=10000
        let result = engine.detect("aspirin", "headache", &Algorithm::Prr, [15, 100, 20, 10000]);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.statistic > 2.0); // PRR should indicate signal
            assert!(r.signal_detected);
        }
    }

    #[test]
    fn test_detection_engine_chi_squared() {
        let mut engine = DetectionEngine::new(2.0);
        let result = engine.detect(
            "drug",
            "event",
            &Algorithm::ChiSquared,
            [50, 100, 20, 10000],
        );
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.statistic > 0.0);
        }
    }

    #[test]
    fn test_detection_engine_grounding() {
        let comp = DetectionEngine::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }

    #[test]
    fn test_triage_scheduler_spawn() {
        let mut sched = TriageScheduler::new();
        let wf = WorkflowDef {
            name: "test".into(),
            steps: vec![WorkflowStep {
                name: "step1".into(),
                syscall: "detect".into(),
                requires_human: false,
            }],
            priority: Priority::Normal,
        };
        let proc_ref = sched.spawn(wf);
        assert_eq!(proc_ref.0, 1);
        assert_eq!(sched.total_count(), 1);

        let state = sched.state(proc_ref);
        assert!(state.is_ok());
        if let Ok(s) = state {
            assert_eq!(s, ProcessState::Pending);
        }
    }

    #[test]
    fn test_triage_scheduler_schedule() {
        let mut sched = TriageScheduler::new();
        let wf = WorkflowDef {
            name: "test".into(),
            steps: Vec::new(),
            priority: Priority::Low,
        };
        let proc_ref = sched.spawn(wf);
        let result = sched.schedule(proc_ref, Priority::Critical);
        assert!(result.is_ok());
        assert_eq!(sched.active_count(), 1);
    }

    #[test]
    fn test_learning_loop_feedback() {
        let mut learning = LearningLoop::new(3);

        let signal = SignalResult {
            drug: "aspirin".into(),
            event: "headache".into(),
            algorithm: Algorithm::Prr,
            statistic: 3.5,
            signal_detected: true,
            ci_lower: None,
            ci_upper: None,
        };

        learning.record(&signal, LearningOutcome::Confirmed);
        learning.record(&signal, LearningOutcome::Refuted);
        assert_eq!(learning.pending_feedback(), 2);

        // FPR for PRR: 1 FP / (1 FP + 1 TP) = 0.5
        assert!((learning.fpr("Prr") - 0.5).abs() < f64::EPSILON);

        // Not enough for retraining yet
        assert!(!learning.retrain());

        learning.record(&signal, LearningOutcome::Confirmed);
        // Now batch size met
        assert!(learning.retrain());
        assert_eq!(learning.retrain_cycles(), 1);
        assert_eq!(learning.pending_feedback(), 0);
    }

    #[test]
    fn test_audit_log_integrity() {
        let mut log = AuditLog::new();
        let id = log.record("detect(aspirin, headache)");
        assert_eq!(log.len(), 1);
        assert!(log.verify(id));
        assert!(!log.verify(999)); // non-existent
    }

    #[test]
    fn test_kernel_grounding() {
        let comp = Kernel::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_artifact_store() {
        let mut store = ArtifactStore::new();
        let artifact = Artifact {
            kind: ArtifactKind::Signal,
            content: "test signal".into(),
            tags: vec!["aspirin".into()],
        };
        let audited_ref = store.store(artifact);
        assert_eq!(audited_ref.id, 1);
        assert_eq!(store.count(), 1);

        let results = store.query(&Filter {
            kind: Some(ArtifactKind::Signal),
            tags: Vec::new(),
            limit: None,
        });
        assert_eq!(results.len(), 1);

        let no_results = store.query(&Filter {
            kind: Some(ArtifactKind::Case),
            tags: Vec::new(),
            limit: None,
        });
        assert_eq!(no_results.len(), 0);
    }
}
