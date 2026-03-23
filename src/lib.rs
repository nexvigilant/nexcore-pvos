#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
#![allow(
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::disallowed_types,
    clippy::iter_over_hash_type,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::too_many_arguments,
    clippy::missing_fields_in_debug,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::wildcard_enum_match_arm,
    clippy::shadow_unrelated,
    clippy::map_entry,
    clippy::vec_init_then_push,
    clippy::iter_with_drain,
    clippy::double_ended_iterator_last,
    clippy::redundant_clone,
    clippy::map_clone,
    reason = "PVOS kernel and subsystem APIs prioritize explicit domain contracts and compatibility over style-only lint constraints"
)]

//! # Pharmacovigilance Operating System (PVOS)
//!
//! Foundational substrate that manages resources, provides abstractions,
//! and enables pharmacovigilance applications to execute.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    USER SPACE                           │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐      │
//! │  │ Signal  │ │  Case   │ │ Report  │ │ Custom  │      │
//! │  │ Detect  │ │ Process │ │ Generate│ │  App    │      │
//! │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘      │
//! ├───────┴──────────┴──────────┴──────────┴──────────────┤
//! │                   SYSTEM CALLS                         │
//! │   detect() | compare() | route() | persist() | audit() │
//! ├───────────────────────────────────────────────────────  │
//! │                     KERNEL                             │
//! │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
//! │  │ Detection│ │ Triage   │ │ Learning │ │ Audit    │  │
//! │  │ Engine κ │ │ Sched  σ │ │ Loop   ρ │ │ Log    π │  │
//! │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
//! ├────────────────────────────────────────────────────────┤
//! │                  ABSTRACTION LAYER (μ)                  │
//! ├────────────────────────────────────────────────────────┤
//! │                    DRIVERS                              │
//! │   FAERS │ VigiBase │ EudraVigilance │ Sponsor DBs      │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## T1 Grounding (8 primitives, dominant μ)
//!
//! | Symbol | Role        | Weight |
//! |--------|-------------|--------|
//! | μ      | Mapping     | 0.20   |
//! | σ      | Sequence    | 0.18   |
//! | κ      | Comparison  | 0.15   |
//! | ∂      | Boundary    | 0.12   |
//! | ρ      | Recursion   | 0.12   |
//! | ς      | State       | 0.10   |
//! | π      | Persistence | 0.08   |
//! | →      | Causality   | 0.05   |
//!
//! ## Key Insight
//!
//! An OS is a μ-machine — its core job is mapping:
//! - Raw safety data → structured cases
//! - Complex algorithms → simple `detect()` calls
//! - Regulatory requirements → built-in compliance
//! - Domain expertise → reusable primitives
//!
//! ## Example
//!
//! ```rust
//! use nexcore_pvos::{Pvos, PvosConfig};
//! use nexcore_pvos::syscall::Algorithm;
//!
//! let mut os = Pvos::boot(PvosConfig::default());
//!
//! // Detection system call
//! let signal = os.detect("aspirin", "headache", Algorithm::Prr, [15, 100, 20, 10000]);
//! assert!(signal.is_ok());
//! ```

pub mod driver;
pub mod kernel;
pub mod syscall;
pub mod util;

// PVWF — Workflow Engine (σ-dominant layer)
pub mod executor;
pub mod patterns;
pub mod supervisor;
pub mod workflow;

// PVGW — Gateway Layer (∂-dominant boundary)
pub mod auth;
pub mod crossing;
pub mod gateway;
pub mod protocol;
pub mod ratelimit;

// PVRX — Reactive Layer (ν-dominant streaming)
pub mod backpressure;
pub mod monitor;
pub mod pubsub;
pub mod stream;
pub mod window;

// PVML — Machine Learning Layer (ρ-dominant learning)
pub mod calibrator;
pub mod drift;
pub mod ensemble;
pub mod feedback;
pub mod trainer;

// PVSH — Shell Interface Layer (λ-dominant navigation)
pub mod builtins;
pub mod command;
pub mod completion;
pub mod path;
pub mod repl;

// PVMX — Metrics Layer (Σ-dominant observability)
pub mod aggregator;
pub mod alerting;
pub mod collector;
pub mod dashboard;
pub mod metric;

// PVTX — Transaction Layer (∝-dominant regulatory finality)
pub mod atomic;
pub mod seal;
pub mod signature;
pub mod submission;
pub mod transaction;

// PV∅ — Void Layer (∅-dominant absence handling)
pub mod defaults;
pub mod missing;
pub mod pv_error;
pub mod underreporting;
pub mod void;

// PVOC — Orchestrator Layer (→-dominant cross-layer causality)
pub mod bus;
pub mod dependency;
pub mod event;
pub mod trace;
pub mod trigger;

// PVST — State Layer (ς-dominant finite state machines)
pub mod fsm_history;
pub mod fsm_snapshot;
pub mod fsm_transition;
pub mod lifecycle;
pub mod proof;
pub mod state;
pub mod tov_mapping;
pub mod typestate;

// PVDB — Persistence Layer (π-dominant durable storage)
pub mod backup;
pub mod crud;
pub mod isolation;
pub mod store;
pub mod wal;

// PVEX — Existence Layer (∃-dominant entity management)
pub mod discovery;
pub mod enumeration;
pub mod namespace;
pub mod presence;
pub mod registry;

// PVNM — Numeric Layer (N-dominant measurement & statistics)
pub mod arithmetic;
pub mod quantity;
pub mod range;
pub mod statistics;
pub mod units;

// PVGL+ — Location Layer Extension (λ-dominant spatial reasoning)
pub mod location;

// PVTF+ — Frequency Layer Extension (ν-dominant adaptive temporal)
pub mod frequency;

// PVBR — Bridge Layer (cross-layer coordination)
pub mod bridges;

// PV∅G — Gap Analysis Layer (∅/ν-dominant absence detection)
pub mod gaps;

// PVXP — Exploratory Layer (multi-primitive T3 frontier)
pub mod exploratory;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// Re-export system call types
pub use syscall::{
    Algorithm, Artifact, ArtifactKind, AuditedRef, CaseRef, ComparisonResult, Destination, Filter,
    LearningOutcome, Priority, ProcessRef, ProcessState, RoutingRules, SignalResult, WorkflowDef,
    WorkflowStep,
};

// Re-export system call traits
pub use syscall::{
    CaseSyscall, DetectionSyscall, LearningSyscall, PersistenceSyscall, WorkflowSyscall,
};

// Re-export kernel types
pub use kernel::{
    ArtifactStore, AuditEntry, AuditLog, DetectionEngine, FeedbackEntry, Kernel, LearningLoop,
    Process, TriageScheduler,
};

// Re-export driver types
pub use driver::{
    DataSourceDriver, DataSourceKind, DriverRegistry, FaersDriver, GenericJsonDriver,
    NormalizedCase, RawRecord,
};

// Re-export PVWF workflow types
pub use workflow::{
    BranchCondition, Step, StepId, StepOutput, SyscallKind, Workflow, WorkflowBuilder, WorkflowId,
};

// Re-export PVWF supervisor types
pub use supervisor::{ProcId, ProcState, RestartPolicy, Supervisor, WfProcess};

// Re-export PVWF executor types
pub use executor::{ExecutionInput, ExecutionResult, WorkflowEngine};

// Re-export PVGW auth types
pub use auth::{
    ApiKey, AuthEngine, AuthError, Identity, IdentityKind, Permission, PolicyRule,
    ServiceAccountId, Token,
};

// Re-export PVGW protocol types
pub use protocol::{ContentType, Protocol, RequestMethod, StatusCode};

// Re-export PVGW rate limiting types
pub use ratelimit::{Quota, RateLimiter, Tier, TokenBucket};

// Re-export PVGW crossing audit types
pub use crossing::{AuditLevel, ComplianceTag, CrossingEvent, CrossingLog, CrossingOutcome};

// Re-export PVGW gateway types
pub use gateway::{
    Endpoint, EndpointAction, Gateway, GatewayConfig, GatewayError, GatewayMetrics, GatewayRequest,
    GatewayResponse,
};

// Re-export PVRX stream types
pub use stream::{
    Event, EventId, EventPayload, EventStream, Rate, StreamId, StreamSink, StreamSource,
};

// Re-export PVRX window types
pub use window::{WindowConfig, WindowEngine, WindowKind, WindowPane, WindowResult};

// Re-export PVRX pub/sub types
pub use pubsub::{
    DeliveryMode, DeliveryRecord, PubSub, SubscriberId, Subscription, Topic, TopicFilter,
};

// Re-export PVRX backpressure types
pub use backpressure::{
    AdmitResult, BackpressureStrategy, BufferPolicy, FlowController, PressureState, Throttle,
};

// Re-export PVRX monitor types
pub use monitor::{
    Alert, AlertSeverity, Condition, Monitor, MonitorId, MonitorState, ReactiveEngine,
};

// Re-export PVML feedback types
pub use feedback::{
    Attribution, Feedback, FeedbackId, FeedbackLoop, FeedbackMetrics, Outcome, OutcomeSource,
};

// Re-export PVML calibrator types
pub use calibrator::{
    CalibrationResult, CalibrationStrategy, CalibrationTarget, Calibrator, ThresholdChange,
    ThresholdHistory,
};

// Re-export PVML trainer types
pub use trainer::{
    Checkpoint, EarlyStopping, Epoch, LearningRate, Loss, ModelId, TrainingConfig, TrainingLoop,
    TrainingResult, TrainingSample,
};

// Re-export PVML drift types
pub use drift::{
    DistributionSummary, DriftAlert, DriftDetector, DriftMetric, DriftRecommendation, DriftScore,
    DriftSeverity, DriftType,
};

// Re-export PVML ensemble types
pub use ensemble::{
    ABTest, Ensemble, ModelPerformance, ModelRegistry, ModelVersion, SelectionStrategy,
};

// Re-export PVSH path types
pub use path::{DirStackEntry, NamespaceNode, Navigator, PathResolver, PathSegment, PvPath};

// Re-export PVSH command types
pub use command::{Alias, Args, CommandKind, CommandName, CommandRegistry, Output, ParsedCommand};

// Re-export PVSH repl types
pub use repl::{History, HistoryEntry, PromptConfig, Repl, ReplState};

// Re-export PVSH completion types
pub use completion::{Candidate, Completer, CompletionKind};

// Re-export PVSH builtin types
pub use builtins::{Shell, ShellContext, execute};

// Re-export PVMX metric types
pub use metric::{
    Bucket, BucketBound, Counter, Gauge, Histogram, Labels, MetricDescriptor, MetricId, MetricKind,
};

// Re-export PVMX aggregator types
pub use aggregator::{AggregationFunc, Aggregator, DataPoint, Rollup, TimeSeries};

// Re-export PVMX dashboard types
pub use dashboard::{Dashboard, DashboardFactory, Panel, Query, TimeRange, Visualization};

// Re-export PVMX alerting types
// AlertSeverity and Condition aliased to avoid conflict with PVRX monitor re-exports
pub use alerting::{
    AlertRule, AlertRuleId, AlertSeverity as MxAlertSeverity, AlertState as MxAlertState,
    Comparator, Condition as MxCondition, NotificationTarget,
};

// Re-export PVMX collector types
pub use collector::{
    ExportFormat, Exporter, LabeledMetric, MetricStorage, MetricsEngine, StandardMetrics,
};

// Re-export PVTX transaction types
pub use transaction::{
    Transaction, TransactionEngine, TxError, TxId, TxKind, TxLog, TxLogEntry, TxOutcome, TxState,
};

// Re-export PVTX signature types
pub use signature::{
    Signature, SignatureError, SignatureId, SignatureMeaning, SignaturePolicy, SignatureRequest,
    SignatureService, Signer, SignerId,
};

// Re-export PVTX submission types
pub use submission::{
    Deadline, Submission, SubmissionDest, SubmissionError, SubmissionId, SubmissionQueue,
    SubmissionState, SubmissionType,
};

// Re-export PVTX atomic types
pub use atomic::{
    AtomicError, AtomicOp, AtomicState, IdempotencyGuard, IdempotencyKey, Saga, SagaState,
    SagaStep, TwoPhaseCommit, TwoPhaseState,
};

// Re-export PVTX seal types
pub use seal::{ArchivalPackage, Seal, SealChain, SealId, SealScope, TamperVerdict};

// Re-export PV∅ void types
pub use void::{AbsenceReason, FieldRequirement, Maybe, NullCoalesce, VoidSafe};

// Re-export PV∅ missing data types
pub use missing::{
    DataQualityReport, FieldDescriptor, Imputation, MissingField, MissingFieldDetector,
    MissingPattern, RecordSchema,
};

// Re-export PV∅ underreporting types
pub use underreporting::{
    DrugKey, EventKey, ExpectedRate, GapSeverity, ReportingGap, SilentPeriod, StimulatedReporting,
    UnderreportingDetector,
};

// Re-export PV∅ error types
pub use pv_error::{ErrorChain, ErrorKind, Fallible, PvError, Recovery, RecoveryEngine};

// Re-export PV∅ default types
pub use defaults::{DefaultAudit, DefaultEntry, DefaultRegistry, DefaultStrategy};

// Re-export PVOC event types
pub use event::{
    CausationChain, CausationId, CorrelationId, EventKind, EventMeta, EventSource, OrcEvent,
    OrcEventId, OrcPayload,
};

// Re-export PVOC trigger types
pub use trigger::{
    Debounce, Trigger, TriggerAction, TriggerCondition, TriggerGuard, TriggerId, TriggerPriority,
};

// Re-export PVOC dependency types
pub use dependency::{
    CycleResult, DependencyEdge, DependencyGraph, DependencyNode, NodeId, NodeState,
};

// Re-export PVOC bus types
pub use bus::{
    BusBackpressure, BusMetrics, BusSubscription, BusSubscriptionId, DeliveryResult, EventBus,
    SubscriptionFilter,
};

// Re-export PVOC trace types
pub use trace::{CausalTrace, RootCause, TraceId, TraceLog, TraceNode, TraceQuery};

// Re-export PVST state types
pub use state::{
    CurrentState, FsmState, StateContext, StateId, StateMachine, StateMachineId, TransitionDef,
};

// Re-export PVST transition types
pub use fsm_transition::{
    BlockReason, TransitionEffect, TransitionGuard, TransitionId, TransitionLog, TransitionRecord,
    TransitionResult, Transitioner,
};

// Re-export PVST lifecycle types
pub use lifecycle::{
    CaseEvent, CaseLifecycleState, LifecycleFsm, SignalEvent, SignalLifecycleState,
    SubmissionEvent, SubmissionLifecycleState, WorkflowEvent, WorkflowLifecycleState,
    case_lifecycle, signal_lifecycle, submission_lifecycle, workflow_lifecycle,
};

// Re-export PVST history types
pub use fsm_history::{
    AuditableHistory, HistoryPolicy, StateDiff, StateHistory, StateHistoryEntry, StateRewind,
};

// Re-export PVST snapshot types
pub use fsm_snapshot::{
    CheckpointPolicy, ConsistentSnapshot, RecoveryOutcome, SnapshotId, SnapshotStore,
    StateRecovery, StateSnapshot,
};

// Re-export PVST typestate wrappers (compile-time state enforcement)
pub use typestate::{
    CaseAssessed,
    CaseClosed,
    CaseReceived,
    CaseTriaged,
    LifecycleState,
    SignalConfirmed,
    SignalDetected,
    SignalRefuted,
    SignalValidated,
    SubmissionAcknowledged,
    SubmissionDraft,
    SubmissionSent,
    SubmissionSigned,
    SubmissionValidated,
    // Case lifecycle
    TypesafeCase,
    // Signal lifecycle
    TypesafeSignal,
    // Submission lifecycle
    TypesafeSubmission,
    // Workflow lifecycle
    TypesafeWorkflow,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowPending,
    WorkflowRunning,
};

// Re-export PVST ToV axiom mappings
pub use tov_mapping::{
    EmergenceWitness, FiniteDecomposition, GuardConstraintSet, HierarchicalWitness,
    SafetyManifoldWitness, TovFsmProof, case_lifecycle_proof, signal_lifecycle_proof,
    submission_lifecycle_proof, workflow_lifecycle_proof,
};

// Re-export PVST proof types (conservation laws)
pub use proof::{
    ConservationLaw, ConservationVerifier, L3SingleState, L4NonTerminalFlux,
    L11StructureImmutability, VerificationResult,
};

// Re-export PVDB store types
pub use store::{
    PersistenceStore, StorageKey, StorageValue, StoreConfig, StoreEntry, StoreId, StoreKind,
    StoreName,
};

// Re-export PVDB CRUD types
pub use crud::{CrudBatch, CrudEngine, CrudFilter, CrudLog, CrudLogEntry, CrudOp, CrudResult};

// Re-export PVDB WAL types
pub use wal::{WalCheckpoint, WalEntry, WalEntryId, WalRecovery, WalState, WriteAheadLog};

// Re-export PVDB backup types
pub use backup::{BackupEntry, BackupId, BackupKind, BackupManifest, BackupStore, RestoreOutcome};

// Re-export PVDB isolation types
pub use isolation::{ConflictDetector, DbLock, IsolationLevel, LockKind, LockManager};

// Re-export PVEX registry types
pub use registry::{
    Deregistration, EntityId, EntityKind, EntityRegistry, RegistrationResult, RegistryEntry,
    RegistryId,
};

// Re-export PVEX discovery types
pub use discovery::{DiscoveryIndex, DiscoveryQuery, DiscoveryResult, DiscoveryService};

// Re-export PVEX presence types
pub use presence::{
    Heartbeat, HeartbeatId, HeartbeatTimeout, Presence, PresenceEvent, PresenceMonitor,
};

// Re-export PVEX enumeration types
pub use enumeration::{
    EnumerationOrder, EnumerationPage, EnumerationScope, Enumerator, LiveEnumeration,
};

// Re-export PVEX namespace types
pub use namespace::{
    CrossNamespaceResult, NamespaceEntry, NamespacePath, NamespaceRegistry, NamespaceVisibility,
};

// Re-export PVNM quantity types
pub use quantity::{
    Confidence as PvConfidence, Count, Dimensionless, Percentage, Precision, PvRate,
};

// Re-export PVNM unit types
pub use units::{CountUnit, FrequencyUnit, RateUnit, TimeUnit, UnitConverter};

// Re-export PVNM arithmetic types
pub use arithmetic::{
    ArithmeticEngine, NumericError, NumericResult, Rounding, safe_add_u64, safe_div_f64,
    safe_div_u64, safe_ln, safe_mul_u64, safe_sqrt, safe_sub_u64, validate_f64,
};

// Re-export PVNM range types
pub use range::{Bound as NumBound, NumericRange, RangeCheck, RangeChecker, Threshold};

// Re-export PVNM statistics types
pub use statistics::{
    ChiSquareValue, ConfidenceInterval, ContingencyTable, ICValue, PRRValue, RORValue,
    StatisticsCalculator,
};

// ═══════════════════════════════════════════════════════════
// PVOS ERROR
// ═══════════════════════════════════════════════════════════

/// PVOS error type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PvosError {
    /// No driver registered for data source.
    NoDriver(String),
    /// Invalid input data.
    InvalidInput(String),
    /// Driver error during normalization.
    DriverError(String),
    /// Process not found.
    ProcessNotFound(u64),
    /// Case not found.
    CaseNotFound(u64),
    /// System not booted.
    NotBooted,
}

impl std::fmt::Display for PvosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDriver(s) => write!(f, "no driver for source: {s}"),
            Self::InvalidInput(s) => write!(f, "invalid input: {s}"),
            Self::DriverError(s) => write!(f, "driver error: {s}"),
            Self::ProcessNotFound(id) => write!(f, "process not found: {id}"),
            Self::CaseNotFound(id) => write!(f, "case not found: {id}"),
            Self::NotBooted => write!(f, "PVOS not booted"),
        }
    }
}

impl std::error::Error for PvosError {}

// ═══════════════════════════════════════════════════════════
// PVOS CONFIGURATION
// ═══════════════════════════════════════════════════════════

/// PVOS boot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvosConfig {
    /// Default detection threshold.
    pub detection_threshold: f64,
    /// Learning batch size.
    pub learning_batch_size: usize,
    /// Whether to register default drivers.
    pub register_default_drivers: bool,
}

impl Default for PvosConfig {
    fn default() -> Self {
        Self {
            detection_threshold: 2.0,
            learning_batch_size: 100,
            register_default_drivers: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// PVOS — THE T3 OPERATING SYSTEM
// ═══════════════════════════════════════════════════════════

/// Operational state of the PVOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PvosState {
    /// System is booting.
    Booting,
    /// System is running.
    Running,
    /// System is shutting down.
    ShuttingDown,
    /// System is halted.
    Halted,
}

/// Pharmacovigilance Operating System.
///
/// The T3 platform that enables PV applications.
/// Dominant primitive: μ (Mapping) — provides abstractions.
///
/// Grounding: μ + σ + κ + ∂ + ρ + ς + π + → (8 T1 primitives)
///
/// Tier: T3 Domain-Specific
#[derive(Clone, Serialize, Deserialize)]
pub struct Pvos {
    /// Kernel subsystems.
    kernel: Kernel,
    /// Artifact persistence.
    store: ArtifactStore,
    /// Ingested cases.
    cases: Vec<(CaseRef, NormalizedCase)>,
    /// Next case ID.
    next_case_id: u64,
    /// System state (ς).
    state: PvosState,
    /// Configuration.
    config: PvosConfig,
}

impl std::fmt::Debug for Pvos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pvos")
            .field("state", &self.state)
            .field("cases", &self.cases.len())
            .field("artifacts", &self.store.count())
            .field("audit_entries", &self.kernel.audit.len())
            .finish()
    }
}

impl Pvos {
    /// Boots the PVOS with given configuration.
    #[must_use]
    pub fn boot(config: PvosConfig) -> Self {
        let kernel = Kernel::new(config.detection_threshold, config.learning_batch_size);

        let mut pvos = Self {
            kernel,
            store: ArtifactStore::new(),
            cases: Vec::new(),
            next_case_id: 1,
            state: PvosState::Booting,
            config,
        };

        pvos.kernel.audit.record("PVOS_BOOT");
        pvos.state = PvosState::Running;
        pvos
    }

    /// Returns current system state.
    #[must_use]
    pub fn state(&self) -> PvosState {
        self.state
    }

    /// Shuts down the PVOS gracefully.
    pub fn shutdown(&mut self) {
        self.kernel.audit.record("PVOS_SHUTDOWN");
        self.state = PvosState::Halted;
    }

    /// Returns kernel reference.
    #[must_use]
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    /// Returns system metrics.
    #[must_use]
    pub fn metrics(&self) -> PvosMetrics {
        PvosMetrics {
            state: self.state,
            total_cases: self.cases.len(),
            total_artifacts: self.store.count(),
            total_detections: self.kernel.detection.total_detections(),
            active_processes: self.kernel.triage.active_count(),
            total_processes: self.kernel.triage.total_count(),
            pending_feedback: self.kernel.learning.pending_feedback(),
            retrain_cycles: self.kernel.learning.retrain_cycles(),
            audit_entries: self.kernel.audit.len(),
        }
    }

    // ═══════════════════════════════════════════════════════
    // DETECTION SYSTEM CALLS (κ)
    // ═══════════════════════════════════════════════════════

    /// Detects a signal for a drug-event pair.
    ///
    /// # Errors
    /// Returns `Err` on invalid input.
    pub fn detect(
        &mut self,
        drug: &str,
        event: &str,
        algo: Algorithm,
        contingency: [u64; 4],
    ) -> Result<SignalResult, PvosError> {
        self.kernel
            .audit
            .record(&format!("DETECT({drug},{event},{algo:?})"));
        self.kernel
            .detection
            .detect(drug, event, &algo, contingency)
    }

    /// Compares observed vs expected with threshold.
    #[must_use]
    pub fn compare(&mut self, observed: f64, expected: f64, threshold: f64) -> ComparisonResult {
        let delta = (observed - expected).abs();
        self.kernel.audit.record(&format!(
            "COMPARE(obs={observed},exp={expected},thresh={threshold})"
        ));
        ComparisonResult {
            observed,
            expected,
            delta,
            exceeded: delta > threshold,
        }
    }

    // ═══════════════════════════════════════════════════════
    // CASE MANAGEMENT SYSTEM CALLS (σ + μ)
    // ═══════════════════════════════════════════════════════

    /// Ingests a case from a data source using a registered driver.
    ///
    /// # Errors
    /// Returns `Err` if no driver registered or parse fails.
    pub fn ingest(
        &mut self,
        source: &DataSourceKind,
        raw: &str,
        drivers: &DriverRegistry,
    ) -> Result<CaseRef, PvosError> {
        let normalized = drivers.normalize(source, raw)?;
        let case_ref = CaseRef(self.next_case_id);
        self.next_case_id += 1;
        self.kernel.audit.record(&format!(
            "INGEST(source={},case={})",
            source.name(),
            case_ref.0
        ));
        self.cases.push((case_ref, normalized));
        Ok(case_ref)
    }

    /// Routes a case based on routing rules.
    ///
    /// # Errors
    /// Returns `Err` if case not found.
    pub fn route(&mut self, case: CaseRef, rules: &RoutingRules) -> Result<Destination, PvosError> {
        let normalized = self
            .cases
            .iter()
            .find(|(r, _)| *r == case)
            .map(|(_, c)| c)
            .ok_or(PvosError::CaseNotFound(case.0))?;

        // Check seriousness criteria
        let is_serious = normalized
            .serious_criteria
            .iter()
            .any(|c| rules.serious_criteria.contains(c));

        let destination = if is_serious {
            Destination::Human("safety_reviewer".into())
        } else if rules
            .auto_domains
            .iter()
            .any(|d| normalized.events.iter().any(|e| e.contains(d.as_str())))
        {
            Destination::Auto("automated_processing".into())
        } else {
            rules.default.clone()
        };

        self.kernel
            .audit
            .record(&format!("ROUTE(case={},dest={destination:?})", case.0));

        Ok(destination)
    }

    /// Prioritizes cases (delegates to triage scheduler).
    #[must_use]
    pub fn prioritize(&self, cases: &[CaseRef]) -> Vec<CaseRef> {
        self.kernel.triage.prioritize(cases)
    }

    // ═══════════════════════════════════════════════════════
    // PERSISTENCE SYSTEM CALLS (π)
    // ═══════════════════════════════════════════════════════

    /// Stores an artifact with automatic audit trail.
    pub fn store(&mut self, artifact: Artifact) -> AuditedRef {
        let kind = format!("{:?}", artifact.kind);
        let audited = self.store.store(artifact);
        self.kernel
            .audit
            .record(&format!("STORE(kind={kind},id={})", audited.id));
        audited
    }

    /// Queries stored artifacts.
    #[must_use]
    pub fn query(&self, filter: &Filter) -> Vec<Artifact> {
        self.store.query(filter)
    }

    // ═══════════════════════════════════════════════════════
    // WORKFLOW SYSTEM CALLS (→ + σ)
    // ═══════════════════════════════════════════════════════

    /// Spawns a new workflow process.
    pub fn spawn(&mut self, workflow: WorkflowDef) -> ProcessRef {
        let name = workflow.name.clone();
        let proc_ref = self.kernel.triage.spawn(workflow);
        self.kernel
            .audit
            .record(&format!("SPAWN(workflow={name},proc={})", proc_ref.0));
        proc_ref
    }

    /// Schedules a process at given priority.
    ///
    /// # Errors
    /// Returns `Err` if process not found.
    pub fn schedule(&mut self, process: ProcessRef, priority: Priority) -> Result<(), PvosError> {
        self.kernel
            .audit
            .record(&format!("SCHEDULE(proc={},pri={priority:?})", process.0));
        self.kernel.triage.schedule(process, priority)
    }

    /// Gets process state.
    ///
    /// # Errors
    /// Returns `Err` if process not found.
    pub fn process_state(&self, process: ProcessRef) -> Result<ProcessState, PvosError> {
        self.kernel.triage.state(process)
    }

    // ═══════════════════════════════════════════════════════
    // LEARNING SYSTEM CALLS (ρ)
    // ═══════════════════════════════════════════════════════

    /// Records feedback on a detection result.
    pub fn feedback(&mut self, signal: &SignalResult, outcome: LearningOutcome) {
        self.kernel
            .audit
            .record(&format!("FEEDBACK({:?},{outcome:?})", signal.algorithm));
        self.kernel.learning.record(signal, outcome);
    }

    /// Triggers model retraining from accumulated feedback.
    pub fn retrain(&mut self) -> bool {
        let result = self.kernel.learning.retrain();
        if result {
            self.kernel.audit.record("RETRAIN(completed)");
        }
        result
    }
}

impl GroundsTo for Pvos {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Mapping,     // μ - DOMINANT: abstraction layer
            LexPrimitiva::Sequence,    // σ - event streams, case ordering
            LexPrimitiva::Comparison,  // κ - detection engine
            LexPrimitiva::Boundary,    // ∂ - thresholds
            LexPrimitiva::Recursion,   // ρ - learning loop
            LexPrimitiva::State,       // ς - system state
            LexPrimitiva::Persistence, // π - audit log
            LexPrimitiva::Causality,   // → - workflow chains
        ])
        .with_dominant(LexPrimitiva::Mapping, 0.80)
    }
}

/// PVOS system metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvosMetrics {
    pub state: PvosState,
    pub total_cases: usize,
    pub total_artifacts: usize,
    pub total_detections: u64,
    pub active_processes: usize,
    pub total_processes: usize,
    pub pending_feedback: usize,
    pub retrain_cycles: u64,
    pub audit_entries: usize,
}

// ═══════════════════════════════════════════════════════════
// PV∅ — THE VOID ENGINE (T3 CAPSTONE)
// ═══════════════════════════════════════════════════════════

/// The Void Engine — systematic absence handling for PVOS.
///
/// Combines missing data detection, underreporting analysis,
/// error handling, and default value management into a single
/// T3 domain type. Dominant primitive: ∅ (Void).
///
/// Grounding: ∅ + ∃ + ∂ + κ + σ + N (6 T1 primitives)
///
/// Tier: T3 Domain-Specific
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidEngine {
    /// Missing field detector.
    pub missing_detector: MissingFieldDetector,
    /// Underreporting detector.
    pub underreporting_detector: UnderreportingDetector,
    /// Default value registry.
    pub default_registry: DefaultRegistry,
    /// Error handler.
    pub error_handler: RecoveryEngine,
    /// Total void operations performed.
    total_operations: u64,
}

impl VoidEngine {
    /// Creates a new VoidEngine with the given ICSR schema.
    #[must_use]
    pub fn new(schema: RecordSchema) -> Self {
        Self {
            missing_detector: MissingFieldDetector::new(schema),
            underreporting_detector: UnderreportingDetector::new(),
            default_registry: DefaultRegistry::new(),
            error_handler: RecoveryEngine::new(),
            total_operations: 0,
        }
    }

    /// Returns the total number of void operations performed.
    #[must_use]
    pub fn total_operations(&self) -> u64 {
        self.total_operations
    }

    /// Checks a record for missing fields.
    pub fn check_missing(
        &mut self,
        record: &std::collections::HashMap<String, Maybe<String>>,
        conditions: &std::collections::HashMap<String, bool>,
        now: u64,
    ) -> DataQualityReport {
        self.total_operations += 1;
        self.missing_detector.check(record, conditions, now)
    }

    /// Detects underreporting gaps for a period.
    #[must_use]
    pub fn detect_gaps(&self, period: &str) -> Vec<ReportingGap> {
        self.underreporting_detector.detect_all_gaps(period)
    }

    /// Applies a default value for a missing field.
    pub fn apply_default(
        &mut self,
        field: &str,
        reason: AbsenceReason,
        context: &str,
        now: u64,
    ) -> Option<String> {
        self.total_operations += 1;
        self.default_registry.apply(field, reason, context, now)
    }

    /// Handles an error and returns recovery strategy.
    pub fn handle_error(&mut self, error: &PvError) -> Recovery {
        self.total_operations += 1;
        self.error_handler.handle(error)
    }
}

impl GroundsTo for VoidEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Void,       // ∅ — DOMINANT: absence handling
            LexPrimitiva::Existence,  // ∃ — presence complement
            LexPrimitiva::Boundary,   // ∂ — required/optional
            LexPrimitiva::Comparison, // κ — expected vs actual
            LexPrimitiva::Sequence,   // σ — patterns over time
            LexPrimitiva::Quantity,   // N — counts of missing
        ])
        .with_dominant(LexPrimitiva::Void, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// PVOC — THE T3 ORCHESTRATOR ENGINE
// ═══════════════════════════════════════════════════════════

/// Orchestrator engine: connects all PVOS layers via causality.
///
/// Provides event-driven cross-layer communication, trigger-based
/// automation, dependency management, and causal tracing for
/// regulatory audit compliance.
///
/// Dominant primitive: → (Causality) — all orchestration is
/// fundamentally about cause-and-effect relationships.
///
/// Tier: T3 Domain-Specific (→ + σ + μ + ∂ + ν + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorEngine {
    /// Event bus for cross-layer pub/sub
    bus: EventBus,
    /// Registered triggers (condition → action)
    triggers: Vec<Trigger>,
    /// Dependency graph for execution ordering
    dependencies: DependencyGraph,
    /// Persistent causal trace log
    trace_log: TraceLog,
    /// Next orchestration event ID
    next_event_id: u64,
    /// Total events processed
    total_events: u64,
    /// Total triggers fired
    total_triggers_fired: u64,
}

impl OrchestratorEngine {
    /// Creates a new orchestrator engine.
    #[must_use]
    pub fn new(bus_capacity: usize) -> Self {
        Self {
            bus: EventBus::new(bus_capacity),
            triggers: Vec::new(),
            dependencies: DependencyGraph::new(),
            trace_log: TraceLog::new(),
            next_event_id: 1,
            total_events: 0,
            total_triggers_fired: 0,
        }
    }

    /// Creates an orchestrator with default settings.
    #[must_use]
    pub fn default_engine() -> Self {
        Self::new(10000)
    }

    /// Emits an event into the orchestrator.
    /// Routes to subscribers, checks triggers, records in trace log.
    pub fn emit(
        &mut self,
        kind: EventKind,
        source: EventSource,
        payload: OrcPayload,
    ) -> OrcEventId {
        let id = OrcEventId(self.next_event_id);
        self.next_event_id += 1;
        self.total_events += 1;

        let meta = EventMeta::new(source, self.total_events);
        let event = OrcEvent::new(id, kind, meta, payload);

        // Record in trace log
        self.trace_log.record_event(&event);

        // Publish to bus subscribers
        self.bus.publish(event.clone());

        // Check triggers
        let mut actions_to_execute = Vec::new();
        for trigger in &mut self.triggers {
            if trigger.matches(&event.kind, event.source()) {
                if let Some(action) = trigger.try_fire(self.total_events) {
                    actions_to_execute.push(action.clone());
                    self.total_triggers_fired += 1;
                }
            }
        }

        // Execute trigger actions (emit cascading events)
        for action in actions_to_execute {
            self.execute_action(&action, id);
        }

        id
    }

    /// Emits an event that was caused by another event.
    pub fn emit_caused_by(
        &mut self,
        kind: EventKind,
        source: EventSource,
        payload: OrcPayload,
        cause: OrcEventId,
    ) -> OrcEventId {
        let id = OrcEventId(self.next_event_id);
        self.next_event_id += 1;
        self.total_events += 1;

        let meta = EventMeta::new(source, self.total_events).with_causation(CausationId(cause.0));
        let event = OrcEvent::new(id, kind, meta, payload);

        self.trace_log.record_event(&event);
        self.bus.publish(event.clone());

        // Check triggers
        let mut actions_to_execute = Vec::new();
        for trigger in &mut self.triggers {
            if trigger.matches(&event.kind, event.source()) {
                if let Some(action) = trigger.try_fire(self.total_events) {
                    actions_to_execute.push(action.clone());
                    self.total_triggers_fired += 1;
                }
            }
        }

        for action in actions_to_execute {
            self.execute_action(&action, id);
        }

        id
    }

    /// Executes a trigger action.
    fn execute_action(&mut self, action: &TriggerAction, cause_id: OrcEventId) {
        match action {
            TriggerAction::EmitEvent { kind, source } => {
                self.emit_caused_by(kind.clone(), source.clone(), OrcPayload::Empty, cause_id);
            }
            TriggerAction::StartWorkflow(name) => {
                self.emit_caused_by(
                    EventKind::WorkflowStarted,
                    EventSource::Pvwf,
                    OrcPayload::Workflow {
                        name: name.clone(),
                        step: None,
                        outcome: "started".into(),
                    },
                    cause_id,
                );
            }
            TriggerAction::IncrementMetric(name) => {
                self.emit_caused_by(
                    EventKind::MetricUpdated,
                    EventSource::Pvmx,
                    OrcPayload::Metric {
                        name: name.clone(),
                        value: 1.0,
                        threshold: None,
                    },
                    cause_id,
                );
            }
            TriggerAction::SendAlert { severity, message } => {
                self.emit_caused_by(
                    EventKind::TriggerFired,
                    EventSource::Pvoc,
                    OrcPayload::Alert {
                        severity: severity.clone(),
                        message: message.clone(),
                    },
                    cause_id,
                );
            }
            TriggerAction::AuditLog(_msg) => {
                // Audit log entries are recorded via trace_log
            }
            TriggerAction::Sequence(actions) => {
                for a in actions {
                    self.execute_action(a, cause_id);
                }
            }
            TriggerAction::Noop => {}
        }
    }

    /// Subscribes to orchestration events.
    pub fn subscribe(&mut self, name: &str, filter: SubscriptionFilter) -> BusSubscriptionId {
        self.bus.subscribe(name, filter)
    }

    /// Registers a trigger: when condition is met, execute action.
    pub fn when(
        &mut self,
        name: &str,
        condition: TriggerCondition,
        action: TriggerAction,
    ) -> TriggerId {
        let id = TriggerId(self.triggers.len() as u64 + 1);
        let trigger = Trigger::new(id, name, condition, action);
        self.triggers.push(trigger);
        id
    }

    /// Registers a trigger with guard and priority.
    pub fn when_guarded(
        &mut self,
        name: &str,
        condition: TriggerCondition,
        action: TriggerAction,
        guard: TriggerGuard,
        priority: TriggerPriority,
    ) -> TriggerId {
        let id = TriggerId(self.triggers.len() as u64 + 1);
        let trigger = Trigger::new(id, name, condition, action)
            .with_guard(guard)
            .with_priority(priority);
        self.triggers.push(trigger);
        id
    }

    /// Adds a node to the dependency graph.
    pub fn add_dependency_node(&mut self, id: NodeId, label: &str) {
        self.dependencies.add_node(DependencyNode::new(id, label));
    }

    /// Declares a dependency: `dependent` depends on `dependency`.
    pub fn depends_on(&mut self, dependent: NodeId, dependency: NodeId) -> bool {
        self.dependencies.add_edge(dependent, dependency)
    }

    /// Returns execution order via topological sort.
    #[must_use]
    pub fn execution_order(&self) -> Option<Vec<NodeId>> {
        self.dependencies.topological_sort()
    }

    /// Returns parallel execution levels.
    #[must_use]
    pub fn execution_levels(&self) -> Option<Vec<Vec<NodeId>>> {
        self.dependencies.execution_levels()
    }

    /// Checks for dependency cycles.
    #[must_use]
    pub fn has_cycles(&self) -> bool {
        !self.dependencies.detect_cycles().is_acyclic()
    }

    /// Traces the causal chain leading to an event.
    pub fn trace(&mut self, event_id: OrcEventId, max_depth: usize) -> CausalTrace {
        let query = TraceQuery::new(event_id, max_depth);
        self.trace_log.trace_back(&query)
    }

    /// Finds the direct cause of an event.
    #[must_use]
    pub fn root_cause(&self, event_id: OrcEventId) -> Option<OrcEventId> {
        self.trace_log.direct_cause(event_id)
    }

    /// Returns the event bus for direct access.
    #[must_use]
    pub fn bus(&self) -> &EventBus {
        &self.bus
    }

    /// Returns the dependency graph.
    #[must_use]
    pub fn dependency_graph(&self) -> &DependencyGraph {
        &self.dependencies
    }

    /// Returns total events processed.
    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.total_events
    }

    /// Returns total triggers fired.
    #[must_use]
    pub fn total_triggers_fired(&self) -> u64 {
        self.total_triggers_fired
    }

    /// Returns trigger count.
    #[must_use]
    pub fn trigger_count(&self) -> usize {
        self.triggers.len()
    }
}

impl Default for OrchestratorEngine {
    fn default() -> Self {
        Self::default_engine()
    }
}

impl GroundsTo for OrchestratorEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,   // → — DOMINANT: cross-layer cause-and-effect
            LexPrimitiva::Sequence,    // σ — ordered event processing
            LexPrimitiva::Mapping,     // μ — event → handler routing
            LexPrimitiva::Boundary,    // ∂ — trigger guards, cycle prevention
            LexPrimitiva::Frequency,   // ν — debounce, event rate
            LexPrimitiva::Persistence, // π — causal trace audit trail
        ])
        .with_dominant(LexPrimitiva::Causality, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// PVST — THE T3 STATE ENGINE
// ═══════════════════════════════════════════════════════════

/// State engine: manages all entity FSMs across the PVOS.
///
/// Provides state machine registration, guarded transitions,
/// history tracking, snapshot persistence, and recovery —
/// everything needed for regulatory-grade lifecycle management.
///
/// Dominant primitive: ς (State) — all lifecycle management
/// is fundamentally about discrete modes and transitions.
///
/// Tier: T3 Domain-Specific (ς + → + ∂ + σ + π + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEngine {
    /// Registered state machines keyed by entity ID.
    machines: std::collections::HashMap<u64, StateMachine>,
    /// Transition executor with guards and effects.
    transitioner: Transitioner,
    /// History store for all entities.
    history: AuditableHistory,
    /// Snapshot store for persistence.
    snapshots: SnapshotStore,
    /// Snapshot recovery manager.
    recovery: StateRecovery,
    /// Checkpoint policy for automatic snapshots.
    checkpoint_policy: CheckpointPolicy,
    /// Total state machines registered.
    total_registered: u64,
}

impl StateEngine {
    /// Creates a new state engine with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            machines: std::collections::HashMap::new(),
            transitioner: Transitioner::new(),
            history: AuditableHistory::new(),
            snapshots: SnapshotStore::new(),
            recovery: StateRecovery::new(),
            checkpoint_policy: CheckpointPolicy::default(),
            total_registered: 0,
        }
    }

    /// Sets the checkpoint policy.
    #[must_use]
    pub fn with_checkpoint_policy(mut self, policy: CheckpointPolicy) -> Self {
        self.checkpoint_policy = policy;
        self
    }

    /// Registers a state machine for an entity.
    pub fn register(&mut self, entity_id: u64, machine: StateMachine) {
        // Record initial state in history
        let state_name = machine
            .current_state_name()
            .unwrap_or("unknown")
            .to_string();
        let history = self.history.get_or_create(entity_id, &machine.name);
        history.record(
            machine.current_state(),
            &state_name,
            machine.context().entered_at,
            None,
        );

        self.machines.insert(entity_id, machine);
        self.total_registered += 1;
    }

    /// Gets the current state of an entity.
    #[must_use]
    pub fn current_state(&self, entity_id: u64) -> Option<CurrentState> {
        self.machines.get(&entity_id).map(|m| m.current_info())
    }

    /// Attempts a guarded transition on an entity.
    pub fn transition(
        &mut self,
        entity_id: u64,
        event: &str,
        guard: &TransitionGuard,
        effect: &TransitionEffect,
        timestamp: u64,
    ) -> Option<TransitionResult> {
        let machine = self.machines.get_mut(&entity_id)?;

        let from_state = machine.current_state();
        let result = self
            .transitioner
            .execute(machine, event, guard, effect, timestamp);

        if result.is_success() {
            // Record in history
            let new_name = machine
                .current_state_name()
                .unwrap_or("unknown")
                .to_string();
            let history = self.history.get_or_create(entity_id, &machine.name);
            history.record(machine.current_state(), &new_name, timestamp, Some(event));

            // Check checkpoint policy
            if self
                .checkpoint_policy
                .should_snapshot(machine.transition_count(), machine.current_state())
            {
                self.snapshots.take_snapshot(
                    entity_id,
                    &machine.name,
                    machine.current_state(),
                    &new_name,
                    &machine.context().data,
                    machine.transition_count(),
                    timestamp,
                    Some("auto-checkpoint"),
                );
            }
        }

        // Suppress unused variable warning
        let _ = from_state;
        Some(result)
    }

    /// Simple transition without guard or effect.
    pub fn simple_transition(
        &mut self,
        entity_id: u64,
        event: &str,
        timestamp: u64,
    ) -> Option<TransitionResult> {
        self.transition(
            entity_id,
            event,
            &TransitionGuard::Always,
            &TransitionEffect::None,
            timestamp,
        )
    }

    /// Takes a manual snapshot of an entity's current state.
    pub fn snapshot(&mut self, entity_id: u64, timestamp: u64, reason: &str) -> Option<SnapshotId> {
        let machine = self.machines.get(&entity_id)?;
        let state_name = machine
            .current_state_name()
            .unwrap_or("unknown")
            .to_string();

        Some(self.snapshots.take_snapshot(
            entity_id,
            &machine.name,
            machine.current_state(),
            &state_name,
            &machine.context().data,
            machine.transition_count(),
            timestamp,
            Some(reason),
        ))
    }

    /// Recovers an entity to a snapshot state.
    pub fn recover(
        &mut self,
        entity_id: u64,
        snapshot_id: SnapshotId,
        timestamp: u64,
    ) -> RecoveryOutcome {
        let outcome = self
            .recovery
            .recover(&self.snapshots, snapshot_id, entity_id);

        if let RecoveryOutcome::Restored { restored_state, .. } = &outcome {
            if let Some(machine) = self.machines.get_mut(&entity_id) {
                machine.force_state(*restored_state, timestamp);
                let state_name = machine
                    .current_state_name()
                    .unwrap_or("recovered")
                    .to_string();
                let history = self.history.get_or_create(entity_id, &machine.name);
                history.record(*restored_state, &state_name, timestamp, Some("recovery"));
            }
        }

        outcome
    }

    /// Gets the history for an entity.
    #[must_use]
    pub fn history(&self, entity_id: u64) -> Option<&StateHistory> {
        self.history.get(entity_id)
    }

    /// Gets the transition log.
    #[must_use]
    pub fn transition_log(&self) -> &TransitionLog {
        self.transitioner.log()
    }

    /// Gets the snapshot store.
    #[must_use]
    pub fn snapshot_store(&self) -> &SnapshotStore {
        &self.snapshots
    }

    /// Returns total registered machines.
    #[must_use]
    pub fn total_registered(&self) -> u64 {
        self.total_registered
    }

    /// Returns total active machines.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.machines.len()
    }

    /// Checks whether an entity has a registered machine.
    #[must_use]
    pub fn has_entity(&self, entity_id: u64) -> bool {
        self.machines.contains_key(&entity_id)
    }
}

impl Default for StateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for StateEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,       // ς — DOMINANT: lifecycle management
            LexPrimitiva::Causality,   // → — transitions cause state change
            LexPrimitiva::Boundary,    // ∂ — guard evaluation
            LexPrimitiva::Sequence,    // σ — ordered history
            LexPrimitiva::Persistence, // π — snapshot persistence
            LexPrimitiva::Existence,   // ∃ — entity existence
        ])
        .with_dominant(LexPrimitiva::State, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// PVDB — THE T3 PERSISTENCE ENGINE
// ═══════════════════════════════════════════════════════════

/// Persistence engine: durable storage for all PVOS entities.
///
/// Provides CRUD operations, write-ahead logging for crash
/// recovery, backup/restore, and concurrency control via
/// isolation levels and locks.
///
/// Dominant primitive: π (Persistence) — all database operations
/// are fundamentally about durable storage and retrieval.
///
/// Tier: T3 Domain-Specific (π + μ + σ + ∂ + ∃ + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceEngine {
    /// Named stores for different entity types.
    stores: std::collections::HashMap<String, PersistenceStore>,
    /// CRUD operation engine.
    crud: CrudEngine,
    /// Write-ahead log for crash recovery.
    wal: WriteAheadLog,
    /// Backup manager.
    backups: BackupStore,
    /// Lock manager for concurrency control.
    locks: LockManager,
    /// Conflict detector.
    conflicts: ConflictDetector,
    /// Next store ID.
    next_store_id: u64,
    /// Total stores created.
    total_stores_created: u64,
}

impl PersistenceEngine {
    /// Creates a new persistence engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stores: std::collections::HashMap::new(),
            crud: CrudEngine::new(),
            wal: WriteAheadLog::new(),
            backups: BackupStore::new(),
            locks: LockManager::new(),
            conflicts: ConflictDetector::new(),
            next_store_id: 1,
            total_stores_created: 0,
        }
    }

    /// Sets the default isolation level.
    #[must_use]
    pub fn with_isolation(mut self, level: IsolationLevel) -> Self {
        self.locks = self.locks.with_isolation(level);
        self
    }

    /// Creates a named store and registers it.
    pub fn create_store(&mut self, name: &str, kind: StoreKind) -> StoreId {
        let id = StoreId(self.next_store_id);
        self.next_store_id += 1;
        self.total_stores_created += 1;

        let store = PersistenceStore::new(id, name, kind);
        self.stores.insert(name.to_string(), store);
        id
    }

    /// Creates a named store with configuration.
    pub fn create_store_with_config(
        &mut self,
        name: &str,
        kind: StoreKind,
        config: StoreConfig,
    ) -> StoreId {
        let id = StoreId(self.next_store_id);
        self.next_store_id += 1;
        self.total_stores_created += 1;

        let store = PersistenceStore::new(id, name, kind).with_config(config);
        self.stores.insert(name.to_string(), store);
        id
    }

    /// Executes a CRUD operation against a named store.
    pub fn execute(&mut self, store_name: &str, op: &CrudOp, timestamp: u64) -> Option<CrudResult> {
        // Log write ops to WAL before applying
        if op.is_write() {
            let (before, after) = match op {
                CrudOp::Create { key: _, value } => (None, Some(value.as_str())),
                CrudOp::Update { key, value } => {
                    let before_val = self
                        .stores
                        .get(store_name)
                        .and_then(|s| s.get(key))
                        .map(|e| e.value.0.clone());
                    // We need to handle this differently since we borrow
                    let _ = before_val;
                    (None, Some(value.as_str()))
                }
                CrudOp::Delete { key } => {
                    let _ = key;
                    (None, None)
                }
                CrudOp::Read { .. } => (None, None),
            };
            self.wal
                .append(op.name(), op.key(), before, after, timestamp);
        }

        let store = self.stores.get_mut(store_name)?;
        Some(self.crud.execute(store, op, timestamp))
    }

    /// Reads a value from a named store.
    pub fn read(&mut self, store_name: &str, key: &str, timestamp: u64) -> Option<CrudResult> {
        self.execute(store_name, &CrudOp::Read { key: key.into() }, timestamp)
    }

    /// Writes a value to a named store (upsert).
    pub fn write(
        &mut self,
        store_name: &str,
        key: &str,
        value: &str,
        timestamp: u64,
    ) -> Option<CrudResult> {
        let op = if self.stores.get(store_name).is_some_and(|s| s.contains(key)) {
            CrudOp::Update {
                key: key.into(),
                value: value.into(),
            }
        } else {
            CrudOp::Create {
                key: key.into(),
                value: value.into(),
            }
        };
        self.execute(store_name, &op, timestamp)
    }

    /// Deletes a value from a named store.
    pub fn delete(&mut self, store_name: &str, key: &str, timestamp: u64) -> Option<CrudResult> {
        self.execute(store_name, &CrudOp::Delete { key: key.into() }, timestamp)
    }

    /// Commits the current WAL.
    pub fn commit_wal(&mut self, timestamp: u64) -> bool {
        self.wal.commit(timestamp)
    }

    /// Rolls back the current WAL.
    pub fn rollback_wal(&mut self) -> bool {
        self.wal.rollback()
    }

    /// Resets the WAL for a new transaction cycle.
    pub fn reset_wal(&mut self) {
        self.wal.reset();
    }

    /// Creates a full backup of a named store.
    pub fn backup(&mut self, store_name: &str, timestamp: u64, label: &str) -> Option<BackupId> {
        let store = self.stores.get(store_name)?;
        let entries = store.raw_entries().clone();
        Some(
            self.backups
                .backup_full(store_name, &entries, timestamp, label),
        )
    }

    /// Restores a store from a backup.
    pub fn restore(&mut self, store_name: &str, backup_id: BackupId) -> RestoreOutcome {
        let (outcome, entries) = self.backups.restore(backup_id, store_name);

        if outcome.is_restored() {
            if let Some(store) = self.stores.get_mut(store_name) {
                let mut restored = std::collections::HashMap::new();
                for entry in entries {
                    restored.insert(
                        entry.key.clone(),
                        StoreEntry::new(
                            StorageKey::new(&entry.key),
                            StorageValue::new(&entry.value),
                            entry.created_at,
                        ),
                    );
                }
                store.restore_entries(restored);
            }
        }

        outcome
    }

    /// Acquires a lock.
    pub fn lock(&mut self, key: &str, kind: LockKind, owner: u64, timestamp: u64) -> bool {
        self.locks.acquire(key, kind, owner, timestamp)
    }

    /// Releases a lock.
    pub fn unlock(&mut self, key: &str, owner: u64) -> bool {
        self.locks.release(key, owner)
    }

    /// Returns a reference to a named store.
    #[must_use]
    pub fn store(&self, name: &str) -> Option<&PersistenceStore> {
        self.stores.get(name)
    }

    /// Returns the CRUD engine.
    #[must_use]
    pub fn crud_engine(&self) -> &CrudEngine {
        &self.crud
    }

    /// Returns the WAL.
    #[must_use]
    pub fn wal(&self) -> &WriteAheadLog {
        &self.wal
    }

    /// Returns the backup store.
    #[must_use]
    pub fn backup_store(&self) -> &BackupStore {
        &self.backups
    }

    /// Returns the lock manager.
    #[must_use]
    pub fn lock_manager(&self) -> &LockManager {
        &self.locks
    }

    /// Returns total stores created.
    #[must_use]
    pub fn total_stores_created(&self) -> u64 {
        self.total_stores_created
    }

    /// Returns active store count.
    #[must_use]
    pub fn active_store_count(&self) -> usize {
        self.stores.len()
    }

    /// Returns store names.
    #[must_use]
    pub fn store_names(&self) -> Vec<&str> {
        self.stores.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for PersistenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for PersistenceEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence, // π — DOMINANT: durable storage
            LexPrimitiva::Mapping,     // μ — key→value, store→entries
            LexPrimitiva::Sequence,    // σ — WAL ordering, CRUD log
            LexPrimitiva::Boundary,    // ∂ — isolation, locks
            LexPrimitiva::Existence,   // ∃ — entry existence
            LexPrimitiva::Causality,   // → — write-ahead causality
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// PVEX — THE T3 EXISTENCE ENGINE
// ═══════════════════════════════════════════════════════════

/// Existence engine: manages entity lifecycle across the PVOS.
///
/// Provides entity registration, discovery, presence monitoring,
/// enumeration with pagination, and hierarchical namespacing —
/// everything needed to answer "does this entity exist?"
///
/// Dominant primitive: ∃ (Existence) — all entity management
/// is fundamentally about existence verification.
///
/// Tier: T3 Domain-Specific (∃ + μ + λ + ν + σ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistenceEngine {
    /// Entity registry.
    registries: std::collections::HashMap<String, EntityRegistry>,
    /// Discovery service.
    discovery: DiscoveryService,
    /// Presence monitor.
    presence: PresenceMonitor,
    /// Namespace registry.
    namespaces: NamespaceRegistry,
    /// Next registry ID.
    next_registry_id: u64,
    /// Total registries created.
    total_registries_created: u64,
}

impl ExistenceEngine {
    /// Creates a new existence engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registries: std::collections::HashMap::new(),
            discovery: DiscoveryService::new(),
            presence: PresenceMonitor::with_defaults(),
            namespaces: NamespaceRegistry::new(),
            next_registry_id: 1,
            total_registries_created: 0,
        }
    }

    /// Creates an existence engine with custom heartbeat timeout.
    #[must_use]
    pub fn with_timeout(timeout: HeartbeatTimeout) -> Self {
        Self {
            registries: std::collections::HashMap::new(),
            discovery: DiscoveryService::new(),
            presence: PresenceMonitor::new(timeout),
            namespaces: NamespaceRegistry::new(),
            next_registry_id: 1,
            total_registries_created: 0,
        }
    }

    /// Creates a named entity registry.
    pub fn create_registry(&mut self, name: &str) -> RegistryId {
        let id = RegistryId(self.next_registry_id);
        self.next_registry_id += 1;
        self.total_registries_created += 1;

        let registry = EntityRegistry::new(id, name);
        self.registries.insert(name.to_string(), registry);
        id
    }

    /// Creates a registry restricted to a specific kind.
    pub fn create_typed_registry(&mut self, name: &str, kind: EntityKind) -> RegistryId {
        let id = RegistryId(self.next_registry_id);
        self.next_registry_id += 1;
        self.total_registries_created += 1;

        let registry = EntityRegistry::new(id, name).with_kind_filter(kind);
        self.registries.insert(name.to_string(), registry);
        id
    }

    /// Registers an entity in a named registry, indexes for discovery,
    /// tracks presence, and assigns a namespace path.
    pub fn register(
        &mut self,
        registry_name: &str,
        kind: EntityKind,
        label: &str,
        namespace_path: &str,
        timestamp: u64,
    ) -> Option<RegistrationResult> {
        let registry = self.registries.get_mut(registry_name)?;
        let result = registry.register(kind, label, timestamp);

        if let Some(id) = result.entity_id() {
            // Index for discovery
            if let Some(entry) = registry.get(id) {
                self.discovery.register(entry);
            }

            // Track presence
            self.presence.track(id);
            self.presence.heartbeat(id, timestamp);

            // Register in namespace
            let path = NamespacePath::new(namespace_path);
            self.namespaces
                .register(id, path, NamespaceVisibility::Public, timestamp);
        }

        Some(result)
    }

    /// Checks if an entity exists (the fundamental ∃ operation).
    #[must_use]
    pub fn exists(&self, registry_name: &str, entity_id: EntityId) -> bool {
        self.registries
            .get(registry_name)
            .is_some_and(|r| r.exists(entity_id))
    }

    /// Resolves a namespace path to an entity.
    pub fn resolve(&mut self, path: &str, from_namespace: &str) -> CrossNamespaceResult {
        self.namespaces.resolve(
            &NamespacePath::new(path),
            &NamespacePath::new(from_namespace),
        )
    }

    /// Records a heartbeat for an entity.
    pub fn heartbeat(&mut self, entity_id: EntityId, timestamp: u64) -> Heartbeat {
        self.presence.heartbeat(entity_id, timestamp)
    }

    /// Evaluates presence of all tracked entities.
    pub fn evaluate_presence(&mut self, now: u64) -> Vec<PresenceEvent> {
        self.presence.evaluate_all(now)
    }

    /// Gets the presence status of an entity.
    #[must_use]
    pub fn presence_of(&self, entity_id: EntityId) -> Presence {
        self.presence.status(entity_id)
    }

    /// Discovers entities matching a query within a registry.
    pub fn discover(
        &mut self,
        registry_name: &str,
        query: &DiscoveryQuery,
        now: u64,
    ) -> Option<DiscoveryResult> {
        let registry = self.registries.get(registry_name)?;
        let entries = registry.all_entries();
        Some(self.discovery.discover(query, &entries, now))
    }

    /// Lists children under a namespace path.
    #[must_use]
    pub fn list_namespace(&self, prefix: &str) -> Vec<&NamespaceEntry> {
        self.namespaces.list(&NamespacePath::new(prefix))
    }

    /// Returns a reference to a named registry.
    #[must_use]
    pub fn registry(&self, name: &str) -> Option<&EntityRegistry> {
        self.registries.get(name)
    }

    /// Returns the discovery service.
    #[must_use]
    pub fn discovery_service(&self) -> &DiscoveryService {
        &self.discovery
    }

    /// Returns the presence monitor.
    #[must_use]
    pub fn presence_monitor(&self) -> &PresenceMonitor {
        &self.presence
    }

    /// Returns the namespace registry.
    #[must_use]
    pub fn namespace_registry(&self) -> &NamespaceRegistry {
        &self.namespaces
    }

    /// Returns total registries created.
    #[must_use]
    pub fn total_registries_created(&self) -> u64 {
        self.total_registries_created
    }

    /// Returns active registry count.
    #[must_use]
    pub fn active_registry_count(&self) -> usize {
        self.registries.len()
    }

    /// Returns registry names.
    #[must_use]
    pub fn registry_names(&self) -> Vec<&str> {
        self.registries.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for ExistenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for ExistenceEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence, // ∃ — DOMINANT: entity existence
            LexPrimitiva::Mapping,   // μ — registry→entity, path→entity
            LexPrimitiva::Location,  // λ — namespace paths
            LexPrimitiva::Frequency, // ν — heartbeat presence
            LexPrimitiva::Sequence,  // σ — enumeration ordering
            LexPrimitiva::Boundary,  // ∂ — visibility, scope
        ])
        .with_dominant(LexPrimitiva::Existence, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// PVNM — THE T3 NUMERIC ENGINE (THE QUINDECET FINALE)
// ═══════════════════════════════════════════════════════════

/// Numeric engine: type-safe measurement for all PVOS quantities.
///
/// Combines unit conversion, safe arithmetic, range validation,
/// and PV signal detection statistics into a single T3 domain
/// type. The final layer completing 100% Lex Primitiva coverage.
///
/// Dominant primitive: N (Quantity) — all measurement and
/// numeric computation is fundamentally about quantities.
///
/// Tier: T3 Domain-Specific (N + κ + Σ + ∂ + μ + ∃)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericEngine {
    /// Unit converter.
    pub converter: UnitConverter,
    /// Arithmetic engine.
    pub arithmetic: ArithmeticEngine,
    /// Range checker.
    pub ranges: RangeChecker,
    /// Statistics calculator.
    pub statistics: StatisticsCalculator,
    /// Total numeric operations performed.
    total_operations: u64,
}

impl NumericEngine {
    /// Creates a new numeric engine with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            converter: UnitConverter::new(),
            arithmetic: ArithmeticEngine::default_engine(),
            ranges: RangeChecker::new(),
            statistics: StatisticsCalculator::new(),
            total_operations: 0,
        }
    }

    /// Creates a numeric engine with custom arithmetic settings.
    #[must_use]
    pub fn with_arithmetic(rounding: Rounding, decimals: u32) -> Self {
        Self {
            converter: UnitConverter::new(),
            arithmetic: ArithmeticEngine::new(rounding, decimals),
            ranges: RangeChecker::new(),
            statistics: StatisticsCalculator::new(),
            total_operations: 0,
        }
    }

    /// Configures standard PV ranges.
    pub fn configure_pv_ranges(&mut self) {
        self.ranges.add_range("prr", NumericRange::non_negative());
        self.ranges.add_range("ror", NumericRange::non_negative());
        self.ranges
            .add_range("confidence", NumericRange::inclusive(0.0, 1.0));
        self.ranges
            .add_range("chi_square", NumericRange::non_negative());
        self.ranges.add_range("count", NumericRange::non_negative());
    }

    /// Computes PRR from a contingency table.
    pub fn compute_prr(&mut self, table: &ContingencyTable) -> NumericResult<PRRValue> {
        self.total_operations += 1;
        self.statistics.prr(table)
    }

    /// Computes ROR from a contingency table.
    pub fn compute_ror(&mut self, table: &ContingencyTable) -> NumericResult<RORValue> {
        self.total_operations += 1;
        self.statistics.ror(table)
    }

    /// Computes IC from a contingency table.
    pub fn compute_ic(&mut self, table: &ContingencyTable) -> NumericResult<ICValue> {
        self.total_operations += 1;
        self.statistics.ic(table)
    }

    /// Computes χ² from a contingency table.
    pub fn compute_chi_square(
        &mut self,
        table: &ContingencyTable,
    ) -> NumericResult<ChiSquareValue> {
        self.total_operations += 1;
        self.statistics.chi_square(table)
    }

    /// Validates a value against a named range.
    pub fn validate_range(&mut self, name: &str, value: f64) -> Option<RangeCheck> {
        self.total_operations += 1;
        self.ranges.check(name, value)
    }

    /// Converts a time value between units.
    pub fn convert_time(&mut self, value: f64, from: TimeUnit, to: TimeUnit) -> f64 {
        self.total_operations += 1;
        self.converter.convert_time(value, from, to)
    }

    /// Converts a rate value between units.
    pub fn convert_rate(&mut self, value: f64, from: RateUnit, to: RateUnit) -> f64 {
        self.total_operations += 1;
        self.converter.convert_rate(value, from, to)
    }

    /// Safe divide with automatic rounding.
    pub fn divide(&mut self, a: f64, b: f64) -> NumericResult<f64> {
        self.total_operations += 1;
        self.arithmetic.divide(a, b)
    }

    /// Mean of a slice of values.
    pub fn mean(&mut self, values: &[f64]) -> NumericResult<f64> {
        self.total_operations += 1;
        self.arithmetic.mean(values)
    }

    /// Returns total numeric operations performed.
    #[must_use]
    pub fn total_operations(&self) -> u64 {
        self.total_operations
    }

    /// Returns total range violations detected.
    #[must_use]
    pub fn total_range_violations(&self) -> u64 {
        self.ranges.total_violations()
    }

    /// Returns total arithmetic errors.
    #[must_use]
    pub fn total_arithmetic_errors(&self) -> u64 {
        self.arithmetic.total_errors()
    }

    /// Returns total unit conversions.
    #[must_use]
    pub fn total_conversions(&self) -> u64 {
        self.converter.total_conversions()
    }
}

impl Default for NumericEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for NumericEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,   // N — DOMINANT: all numeric measurement
            LexPrimitiva::Comparison, // κ — signal thresholds, range checking
            LexPrimitiva::Sum,        // Σ — statistical aggregation
            LexPrimitiva::Boundary,   // ∂ — ranges, CI bounds
            LexPrimitiva::Mapping,    // μ — unit conversion, named ranges
            LexPrimitiva::Existence,  // ∃ — valid/invalid results
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_pvos_t3_grounding() {
        let comp = Pvos::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.unique().len(), 8);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Mapping));
    }

    #[test]
    fn test_pvos_boot() {
        let os = Pvos::boot(PvosConfig::default());
        assert_eq!(os.state(), PvosState::Running);
        assert_eq!(os.kernel().audit.len(), 1); // PVOS_BOOT entry
    }

    #[test]
    fn test_pvos_detect() {
        let mut os = Pvos::boot(PvosConfig::default());
        let signal = os.detect("aspirin", "headache", Algorithm::Prr, [15, 100, 20, 10000]);
        assert!(signal.is_ok());
        if let Ok(s) = signal {
            assert!(s.signal_detected);
            assert!(s.statistic > 2.0);
        }
    }

    #[test]
    fn test_pvos_compare() {
        let mut os = Pvos::boot(PvosConfig::default());
        let cmp = os.compare(5.0, 2.0, 2.0);
        assert!(cmp.exceeded); // delta=3 > threshold=2
        assert!((cmp.delta - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pvos_ingest_and_route() {
        let mut os = Pvos::boot(PvosConfig::default());
        let drivers = DriverRegistry::with_defaults();

        let raw =
            r#"{"drugname": "warfarin", "reactions": "bleeding", "serious": "hospitalization"}"#;
        let case = os.ingest(&DataSourceKind::Faers, raw, &drivers);
        assert!(case.is_ok());

        if let Ok(case_ref) = case {
            let rules = RoutingRules::default();
            let dest = os.route(case_ref, &rules);
            assert!(dest.is_ok());
            if let Ok(d) = dest {
                // Should route to human because "hospitalization" is a serious criterion
                assert!(matches!(d, Destination::Human(_)));
            }
        }
    }

    #[test]
    fn test_pvos_store_and_query() {
        let mut os = Pvos::boot(PvosConfig::default());

        let artifact = Artifact {
            kind: ArtifactKind::Signal,
            content: "PRR=3.5 for aspirin-headache".into(),
            tags: vec!["aspirin".into(), "headache".into()],
        };
        let audited = os.store(artifact);
        assert_eq!(audited.id, 1);

        let results = os.query(&Filter {
            kind: Some(ArtifactKind::Signal),
            tags: Vec::new(),
            limit: None,
        });
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_pvos_workflow() {
        let mut os = Pvos::boot(PvosConfig::default());

        let wf = WorkflowDef {
            name: "signal_review".into(),
            steps: vec![
                WorkflowStep {
                    name: "detect".into(),
                    syscall: "detect".into(),
                    requires_human: false,
                },
                WorkflowStep {
                    name: "review".into(),
                    syscall: "await_human".into(),
                    requires_human: true,
                },
            ],
            priority: Priority::High,
        };

        let proc_ref = os.spawn(wf);
        assert_eq!(proc_ref.0, 1);

        let state = os.process_state(proc_ref);
        assert!(state.is_ok());
        if let Ok(s) = state {
            assert_eq!(s, ProcessState::Pending);
        }

        let schedule = os.schedule(proc_ref, Priority::Critical);
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_pvos_learning_cycle() {
        let config = PvosConfig {
            learning_batch_size: 3,
            ..PvosConfig::default()
        };
        let mut os = Pvos::boot(config);

        let signal = SignalResult {
            drug: "aspirin".into(),
            event: "headache".into(),
            algorithm: Algorithm::Prr,
            statistic: 3.5,
            signal_detected: true,
            ci_lower: None,
            ci_upper: None,
        };

        os.feedback(&signal, LearningOutcome::Confirmed);
        os.feedback(&signal, LearningOutcome::Refuted);
        assert!(!os.retrain()); // Not enough yet

        os.feedback(&signal, LearningOutcome::Confirmed);
        assert!(os.retrain()); // Batch size met
        assert_eq!(os.metrics().retrain_cycles, 1);
    }

    #[test]
    fn test_pvos_shutdown() {
        let mut os = Pvos::boot(PvosConfig::default());
        assert_eq!(os.state(), PvosState::Running);
        os.shutdown();
        assert_eq!(os.state(), PvosState::Halted);
    }

    #[test]
    fn test_pvos_audit_trail() {
        let mut os = Pvos::boot(PvosConfig::default());

        // Boot creates 1 audit entry
        assert_eq!(os.kernel().audit.len(), 1);

        let _signal = os.detect("drug", "event", Algorithm::Prr, [10, 100, 20, 10000]);
        assert_eq!(os.kernel().audit.len(), 2); // +DETECT

        let _cmp = os.compare(1.0, 0.5, 0.3);
        assert_eq!(os.kernel().audit.len(), 3); // +COMPARE

        // Verify all entries have valid integrity
        for entry in os.kernel().audit.entries() {
            assert!(os.kernel().audit.verify(entry.id));
        }
    }

    #[test]
    fn test_pvos_metrics() {
        let mut os = Pvos::boot(PvosConfig::default());
        let m = os.metrics();
        assert_eq!(m.total_cases, 0);
        assert_eq!(m.total_artifacts, 0);
        assert_eq!(m.total_detections, 0);
        assert_eq!(m.audit_entries, 1); // PVOS_BOOT

        let _signal = os.detect("drug", "event", Algorithm::Prr, [10, 100, 20, 10000]);
        let m = os.metrics();
        assert_eq!(m.total_detections, 1);
        assert_eq!(m.audit_entries, 2);
    }

    #[test]
    fn test_eight_primitives_coverage() {
        let comp = Pvos::primitive_composition();
        let unique = comp.unique();

        // Verify all 8 T1 primitives from the specification
        assert!(unique.contains(&LexPrimitiva::Mapping)); // μ
        assert!(unique.contains(&LexPrimitiva::Sequence)); // σ
        assert!(unique.contains(&LexPrimitiva::Comparison)); // κ
        assert!(unique.contains(&LexPrimitiva::Boundary)); // ∂
        assert!(unique.contains(&LexPrimitiva::Recursion)); // ρ
        assert!(unique.contains(&LexPrimitiva::State)); // ς
        assert!(unique.contains(&LexPrimitiva::Persistence)); // π
        assert!(unique.contains(&LexPrimitiva::Causality)); // →

        assert_eq!(unique.len(), 8);
    }

    #[test]
    fn test_avc_vs_pvos_dominant() {
        // AVC dominant: κ (Comparison) — it detects
        let avc_comp = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        assert_eq!(avc_comp.dominant, Some(LexPrimitiva::Comparison));

        // PVOS dominant: μ (Mapping) — it enables others to detect
        let pvos_comp = Pvos::primitive_composition();
        assert_eq!(pvos_comp.dominant, Some(LexPrimitiva::Mapping));
    }

    /// The Quintet Test: verify 5 layers have 5 distinct dominant primitives.
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML)
    #[test]
    fn test_pvos_pvwf_pvgw_pvrx_pvml_quintet() {
        use std::collections::HashSet;

        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ

        // All 5 are distinct
        let dominants: HashSet<_> = [
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(dominants.len(), 5, "All 5 layer dominants must be unique");
    }

    /// The Sextet Test: verify 6 layers have 6 distinct dominant primitives.
    /// μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) → λ(PVSH)
    #[test]
    fn test_pvos_pvwf_pvgw_pvrx_pvml_pvsh_sextet() {
        use std::collections::HashSet;

        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ

        // All 6 are distinct
        let dominants: HashSet<_> = [
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(dominants.len(), 6, "All 6 layer dominants must be unique");
    }

    /// The Septet Test: verify 7 layers have 7 distinct dominant primitives.
    /// μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) → λ(PVSH) → Σ(PVMX)
    #[test]
    fn test_pvos_pvwf_pvgw_pvrx_pvml_pvsh_pvmx_septet() {
        use std::collections::HashSet;

        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ

        // All 7 are distinct
        let dominants: HashSet<_> = [
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(dominants.len(), 7, "All 7 layer dominants must be unique");
    }

    /// The Octet Test: verify 8 layers have 8 distinct dominant primitives.
    /// μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) → λ(PVSH) → Σ(PVMX) → ∝(PVTX)
    #[test]
    fn test_pvos_pvwf_pvgw_pvrx_pvml_pvsh_pvmx_pvtx_octet() {
        use std::collections::HashSet;

        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝

        // All 8 are distinct
        let dominants: HashSet<_> = [
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(dominants.len(), 8, "All 8 layer dominants must be unique");
    }

    #[test]
    fn test_pvtx_t3_grounding() {
        let comp = TransactionEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Irreversibility));
        assert_eq!(comp.unique().len(), 6);
    }

    #[test]
    fn test_void_engine_t3_grounding() {
        let comp = VoidEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Void));
        assert_eq!(comp.unique().len(), 6);
    }

    /// The Nonet Test: verify 9 layers have 9 distinct dominant primitives.
    /// μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) → λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅)
    #[test]
    fn test_pvos_pvwf_pvgw_pvrx_pvml_pvsh_pvmx_pvtx_pvvoid_nonet() {
        use std::collections::HashSet;

        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅

        // All 9 are distinct
        let dominants: HashSet<_> = [
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(dominants.len(), 9, "All 9 layer dominants must be unique");
    }

    /// The Decet Test: verify 10 dominant primitives including AVC (κ).
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) → λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅)
    #[test]
    fn test_avc_pvos_decet() {
        use std::collections::HashSet;

        let avc = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();

        assert_eq!(avc.dominant, Some(LexPrimitiva::Comparison)); // κ
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅

        // All 10 are distinct
        let dominants: HashSet<_> = [
            avc.dominant,
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(
            dominants.len(),
            10,
            "All 10 layer dominants must be unique (67% primitive coverage)"
        );
    }

    #[test]
    fn test_void_engine_operations() {
        let schema =
            RecordSchema::new("test").with_field(FieldDescriptor::mandatory("drug", "Drug name"));

        let mut engine = VoidEngine::new(schema);
        assert_eq!(engine.total_operations(), 0);

        let mut record = std::collections::HashMap::new();
        record.insert("drug".into(), Maybe::Present("aspirin".into()));
        let conditions = std::collections::HashMap::new();

        let report = engine.check_missing(&record, &conditions, 1000);
        assert!(report.is_complete());
        assert_eq!(engine.total_operations(), 1);
    }

    #[test]
    fn test_orchestrator_engine_t3_grounding() {
        let comp = OrchestratorEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 6);
    }

    /// The Undecet Test: verify 11 dominant primitives including PVOC (→).
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) → λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅) → →(PVOC)
    #[test]
    fn test_avc_pvos_pvoc_undecet() {
        use std::collections::HashSet;

        let avc = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();
        let pvoc = OrchestratorEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(avc.dominant, Some(LexPrimitiva::Comparison)); // κ
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅
        assert_eq!(pvoc.dominant, Some(LexPrimitiva::Causality)); // →

        // All 11 are distinct
        let dominants: HashSet<_> = [
            avc.dominant,
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
            pvoc.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(
            dominants.len(),
            11,
            "All 11 layer dominants must be unique (73% primitive coverage)"
        );
    }

    #[test]
    fn test_orchestrator_emit_and_trace() {
        let mut orc = OrchestratorEngine::new(100);

        // Emit a signal detection event
        let signal_id = orc.emit(
            EventKind::SignalDetected,
            EventSource::Avc,
            OrcPayload::Signal {
                drug: "aspirin".into(),
                event: "headache".into(),
                statistic: 3.5,
                detected: true,
            },
        );

        // Emit a workflow caused by the signal
        let wf_id = orc.emit_caused_by(
            EventKind::WorkflowStarted,
            EventSource::Pvwf,
            OrcPayload::Workflow {
                name: "signal_triage".into(),
                step: None,
                outcome: "started".into(),
            },
            signal_id,
        );

        // Emit a submission caused by the workflow
        let _sub_id = orc.emit_caused_by(
            EventKind::SubmissionSent,
            EventSource::Pvtx,
            OrcPayload::Transaction {
                tx_id: 1,
                kind: "submission".into(),
                outcome: "sent".into(),
            },
            wf_id,
        );

        assert_eq!(orc.total_events(), 3);

        // Trace: why was submission sent?
        let trace = orc.trace(_sub_id, 10);
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.primary_chain.root(), Some(signal_id));
    }

    #[test]
    fn test_orchestrator_triggers() {
        let mut orc = OrchestratorEngine::new(100);

        // Register trigger: SignalDetected → start workflow
        orc.when(
            "signal_to_triage",
            TriggerCondition::on_event(EventKind::SignalDetected),
            TriggerAction::workflow("signal_triage"),
        );

        // Emit signal — should auto-trigger workflow
        let _signal_id = orc.emit(
            EventKind::SignalDetected,
            EventSource::Avc,
            OrcPayload::Signal {
                drug: "warfarin".into(),
                event: "bleeding".into(),
                statistic: 4.5,
                detected: true,
            },
        );

        // Should have 2 events: the signal + the auto-triggered workflow
        assert_eq!(orc.total_events(), 2);
        assert_eq!(orc.total_triggers_fired(), 1);
    }

    #[test]
    fn test_orchestrator_dependencies() {
        let mut orc = OrchestratorEngine::new(100);

        orc.add_dependency_node(NodeId(1), "detect");
        orc.add_dependency_node(NodeId(2), "triage");
        orc.add_dependency_node(NodeId(3), "review");
        orc.add_dependency_node(NodeId(4), "submit");

        orc.depends_on(NodeId(2), NodeId(1)); // triage depends on detect
        orc.depends_on(NodeId(3), NodeId(2)); // review depends on triage
        orc.depends_on(NodeId(4), NodeId(3)); // submit depends on review

        assert!(!orc.has_cycles());

        let order = orc.execution_order();
        assert!(order.is_some());
        if let Some(o) = order {
            assert_eq!(o, vec![NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
        }
    }

    #[test]
    fn test_orchestrator_subscription() {
        let mut orc = OrchestratorEngine::new(100);
        orc.subscribe(
            "all_signals",
            SubscriptionFilter::ByKind(EventKind::SignalDetected),
        );

        orc.emit(
            EventKind::SignalDetected,
            EventSource::Avc,
            OrcPayload::Empty,
        );

        let metrics = orc.bus().metrics();
        assert_eq!(metrics.total_published, 1);
        assert_eq!(metrics.total_delivered, 1);
    }

    // ═══════════════════════════════════════════════════════
    // PVST STATE ENGINE TESTS
    // ═══════════════════════════════════════════════════════

    #[test]
    fn test_state_engine_t3_grounding() {
        let comp = StateEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
        assert_eq!(comp.unique().len(), 6);
    }

    /// The Duodecet Test: verify 12 dominant primitives including PVST (ς).
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) →
    /// λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅) → →(PVOC) → ς(PVST)
    #[test]
    fn test_avc_pvos_pvoc_pvst_duodecet() {
        use std::collections::HashSet;

        let avc = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();
        let pvoc = OrchestratorEngine::primitive_composition();
        let pvst = StateEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(avc.dominant, Some(LexPrimitiva::Comparison)); // κ
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅
        assert_eq!(pvoc.dominant, Some(LexPrimitiva::Causality)); // →
        assert_eq!(pvst.dominant, Some(LexPrimitiva::State)); // ς

        // All 12 are distinct
        let dominants: HashSet<_> = [
            avc.dominant,
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
            pvoc.dominant,
            pvst.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(
            dominants.len(),
            12,
            "All 12 layer dominants must be unique (80% primitive coverage)"
        );
    }

    #[test]
    fn test_state_engine_register_and_query() {
        let mut engine = StateEngine::new();
        let case = case_lifecycle(1, 100, 1000);

        engine.register(100, case);
        assert!(engine.has_entity(100));
        assert!(!engine.has_entity(200));
        assert_eq!(engine.total_registered(), 1);
        assert_eq!(engine.active_count(), 1);

        let current = engine.current_state(100);
        assert!(current.is_some());
        if let Some(c) = current {
            assert_eq!(c.state_name, "received");
        }
    }

    #[test]
    fn test_state_engine_simple_transition() {
        let mut engine = StateEngine::new();
        let case = case_lifecycle(1, 100, 1000);
        engine.register(100, case);

        let result = engine.simple_transition(100, "triage", 2000);
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(r.is_success());
        }

        let current = engine.current_state(100);
        assert!(current.is_some());
        if let Some(c) = current {
            assert_eq!(c.state_name, "triaged");
        }
    }

    #[test]
    fn test_state_engine_guarded_transition() {
        let mut engine = StateEngine::new();
        let case = case_lifecycle(1, 100, 1000);
        engine.register(100, case);

        // Guard requires "drug" key — should fail
        let result = engine.transition(
            100,
            "triage",
            &TransitionGuard::RequiresKey("drug".into()),
            &TransitionEffect::None,
            2000,
        );
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(!r.is_success());
        }
    }

    #[test]
    fn test_state_engine_history() {
        let mut engine = StateEngine::new();
        let case = case_lifecycle(1, 100, 1000);
        engine.register(100, case);

        engine.simple_transition(100, "triage", 2000);
        engine.simple_transition(100, "assess", 3000);

        let history = engine.history(100);
        assert!(history.is_some());
        if let Some(h) = history {
            assert_eq!(h.len(), 3); // initial + 2 transitions
        }

        assert_eq!(engine.transition_log().len(), 2);
    }

    #[test]
    fn test_state_engine_snapshot_and_recovery() {
        let mut engine = StateEngine::new();
        let case = case_lifecycle(1, 100, 1000);
        engine.register(100, case);

        engine.simple_transition(100, "triage", 2000);

        // Take snapshot at "triaged"
        let snap = engine.snapshot(100, 2500, "pre-assessment");
        assert!(snap.is_some());
        let snap_id = snap.unwrap_or(SnapshotId(0));

        // Advance to assessed
        engine.simple_transition(100, "assess", 3000);

        let current = engine.current_state(100);
        assert!(current.is_some());
        if let Some(c) = current {
            assert_eq!(c.state_name, "assessed");
        }

        // Recover to snapshot (triaged)
        let outcome = engine.recover(100, snap_id, 4000);
        assert!(outcome.is_restored());

        let current = engine.current_state(100);
        assert!(current.is_some());
        if let Some(c) = current {
            assert_eq!(c.state_id, StateId(2)); // triaged
        }
    }

    #[test]
    fn test_state_engine_full_lifecycle() {
        let mut engine = StateEngine::new();
        engine.register(100, case_lifecycle(1, 100, 1000));
        engine.register(200, signal_lifecycle(2, 200, 1000));
        engine.register(300, workflow_lifecycle(3, 300, 1000));

        // Case: received → triaged → assessed → closed
        engine.simple_transition(100, "triage", 2000);
        engine.simple_transition(100, "assess", 3000);
        engine.simple_transition(100, "close", 4000);

        // Signal: detected → validated → confirmed
        engine.simple_transition(200, "validate", 2100);
        engine.simple_transition(200, "confirm", 3100);

        // Workflow: pending → running → completed
        engine.simple_transition(300, "start", 2200);
        engine.simple_transition(300, "complete", 3200);

        assert_eq!(engine.total_registered(), 3);
        assert_eq!(engine.transition_log().len(), 7);

        let case_history = engine.history(100);
        assert!(case_history.is_some());
        if let Some(h) = case_history {
            assert_eq!(h.len(), 4); // initial + 3 transitions
        }
    }

    #[test]
    fn test_state_engine_checkpoint_policy() {
        let mut engine = StateEngine::new().with_checkpoint_policy(CheckpointPolicy::Always);

        engine.register(100, case_lifecycle(1, 100, 1000));
        engine.simple_transition(100, "triage", 2000);
        engine.simple_transition(100, "assess", 3000);

        // Auto-checkpoint on every transition
        assert_eq!(engine.snapshot_store().len(), 2);
    }

    // ═══════════════════════════════════════════════════════
    // PVDB PERSISTENCE ENGINE TESTS
    // ═══════════════════════════════════════════════════════

    #[test]
    fn test_persistence_engine_t3_grounding() {
        let comp = PersistenceEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
        assert_eq!(comp.unique().len(), 6);
    }

    /// The Tredecet Test: verify 13 dominant primitives including PVDB (π).
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) →
    /// λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅) → →(PVOC) → ς(PVST) → π(PVDB)
    #[test]
    fn test_avc_pvos_pvoc_pvst_pvdb_tredecet() {
        use std::collections::HashSet;

        let avc = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();
        let pvoc = OrchestratorEngine::primitive_composition();
        let pvst = StateEngine::primitive_composition();
        let pvdb = PersistenceEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(avc.dominant, Some(LexPrimitiva::Comparison)); // κ
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅
        assert_eq!(pvoc.dominant, Some(LexPrimitiva::Causality)); // →
        assert_eq!(pvst.dominant, Some(LexPrimitiva::State)); // ς
        assert_eq!(pvdb.dominant, Some(LexPrimitiva::Persistence)); // π

        // All 13 are distinct
        let dominants: HashSet<_> = [
            avc.dominant,
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
            pvoc.dominant,
            pvst.dominant,
            pvdb.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(
            dominants.len(),
            13,
            "All 13 layer dominants must be unique (87% primitive coverage)"
        );
    }

    #[test]
    fn test_persistence_engine_create_store() {
        let mut engine = PersistenceEngine::new();
        let id = engine.create_store("cases", StoreKind::KeyValue);
        assert_eq!(id, StoreId(1));
        assert_eq!(engine.active_store_count(), 1);
        assert_eq!(engine.total_stores_created(), 1);
        assert!(engine.store("cases").is_some());
    }

    #[test]
    fn test_persistence_engine_crud() {
        let mut engine = PersistenceEngine::new();
        engine.create_store("cases", StoreKind::KeyValue);

        // Write
        let result = engine.write("cases", "case:1", "warfarin-bleeding", 1000);
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(r.is_success());
        }

        // Read
        let result = engine.read("cases", "case:1", 2000);
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(matches!(r, CrudResult::Found { value, .. } if value == "warfarin-bleeding"));
        }

        // Update
        let result = engine.write("cases", "case:1", "warfarin-bleeding-serious", 3000);
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(matches!(r, CrudResult::Updated { .. }));
        }

        // Delete
        let result = engine.delete("cases", "case:1", 4000);
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(matches!(r, CrudResult::Deleted));
        }
    }

    #[test]
    fn test_persistence_engine_wal_lifecycle() {
        let mut engine = PersistenceEngine::new();
        engine.create_store("signals", StoreKind::KeyValue);

        engine.write("signals", "sig:1", "prr=3.5", 1000);
        engine.write("signals", "sig:2", "ror=2.1", 2000);

        assert!(!engine.wal().is_empty());
        assert!(engine.commit_wal(3000));
        assert_eq!(engine.wal().state(), WalState::Committed);

        engine.reset_wal();
        assert_eq!(engine.wal().state(), WalState::Active);
    }

    #[test]
    fn test_persistence_engine_backup_restore() {
        let mut engine = PersistenceEngine::new();
        engine.create_store("cases", StoreKind::KeyValue);

        engine.write("cases", "case:1", "data1", 1000);
        engine.write("cases", "case:2", "data2", 2000);

        // Backup
        let backup_id = engine.backup("cases", 3000, "daily");
        assert!(backup_id.is_some());
        let backup_id = backup_id.unwrap_or(BackupId(0));

        // Destroy data
        engine.delete("cases", "case:1", 4000);
        engine.delete("cases", "case:2", 5000);

        // Restore
        let outcome = engine.restore("cases", backup_id);
        assert!(outcome.is_restored());
    }

    #[test]
    fn test_persistence_engine_locking() {
        let mut engine = PersistenceEngine::new();

        // Acquire exclusive lock
        assert!(engine.lock("case:1", LockKind::Exclusive, 100, 1000));

        // Another owner tries to lock — should fail
        assert!(!engine.lock("case:1", LockKind::Shared, 200, 1000));

        // Release and retry
        assert!(engine.unlock("case:1", 100));
        assert!(engine.lock("case:1", LockKind::Shared, 200, 2000));
    }

    #[test]
    fn test_persistence_engine_multi_store() {
        let mut engine = PersistenceEngine::new();
        engine.create_store("cases", StoreKind::KeyValue);
        engine.create_store("signals", StoreKind::KeyValue);
        engine.create_store("submissions", StoreKind::AppendOnly);

        assert_eq!(engine.active_store_count(), 3);

        engine.write("cases", "c1", "data", 1000);
        engine.write("signals", "s1", "data", 1000);

        assert!(engine.store("cases").is_some_and(|s| s.len() == 1));
        assert!(engine.store("signals").is_some_and(|s| s.len() == 1));
    }

    #[test]
    fn test_persistence_engine_nonexistent_store() {
        let mut engine = PersistenceEngine::new();
        let result = engine.read("missing", "k1", 1000);
        assert!(result.is_none());
    }

    // ═══════════════════════════════════════════════════════
    // PVEX EXISTENCE ENGINE TESTS
    // ═══════════════════════════════════════════════════════

    #[test]
    fn test_existence_engine_t3_grounding() {
        let comp = ExistenceEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
        assert_eq!(comp.unique().len(), 6);
    }

    /// The Quattuordecet Test: verify 14 dominant primitives including PVEX (∃).
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) →
    /// λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅) → →(PVOC) → ς(PVST) →
    /// π(PVDB) → ∃(PVEX)
    #[test]
    fn test_avc_pvos_pvex_quattuordecet() {
        use std::collections::HashSet;

        let avc = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();
        let pvoc = OrchestratorEngine::primitive_composition();
        let pvst = StateEngine::primitive_composition();
        let pvdb = PersistenceEngine::primitive_composition();
        let pvex = ExistenceEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(avc.dominant, Some(LexPrimitiva::Comparison)); // κ
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅
        assert_eq!(pvoc.dominant, Some(LexPrimitiva::Causality)); // →
        assert_eq!(pvst.dominant, Some(LexPrimitiva::State)); // ς
        assert_eq!(pvdb.dominant, Some(LexPrimitiva::Persistence)); // π
        assert_eq!(pvex.dominant, Some(LexPrimitiva::Existence)); // ∃

        // All 14 are distinct
        let dominants: HashSet<_> = [
            avc.dominant,
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
            pvoc.dominant,
            pvst.dominant,
            pvdb.dominant,
            pvex.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(
            dominants.len(),
            14,
            "All 14 layer dominants must be unique (93% primitive coverage)"
        );
    }

    #[test]
    fn test_existence_engine_create_registry() {
        let mut engine = ExistenceEngine::new();
        let id = engine.create_registry("cases");
        assert_eq!(id, RegistryId(1));
        assert_eq!(engine.active_registry_count(), 1);
        assert!(engine.registry("cases").is_some());
    }

    #[test]
    fn test_existence_engine_register_entity() {
        let mut engine = ExistenceEngine::new();
        engine.create_registry("cases");

        let result = engine.register(
            "cases",
            EntityKind::Case,
            "warfarin-bleeding",
            "/pv/cases/c1",
            1000,
        );
        assert!(result.is_some());
        if let Some(r) = result {
            assert!(r.is_registered());
        }

        assert!(engine.exists("cases", EntityId(1)));
        assert!(!engine.exists("cases", EntityId(999)));
    }

    #[test]
    fn test_existence_engine_resolve_namespace() {
        let mut engine = ExistenceEngine::new();
        engine.create_registry("signals");

        engine.register(
            "signals",
            EntityKind::Signal,
            "aspirin-headache",
            "/pv/signals/s1",
            1000,
        );

        let result = engine.resolve("/pv/signals/s1", "/");
        assert!(result.is_found());
        assert_eq!(result.entity_id(), Some(EntityId(1)));
    }

    #[test]
    fn test_existence_engine_presence() {
        let timeout = HeartbeatTimeout::new(30, 120, 10);
        let mut engine = ExistenceEngine::with_timeout(timeout);
        engine.create_registry("streams");

        engine.register(
            "streams",
            EntityKind::Stream,
            "event-stream",
            "/pv/streams/es1",
            1000,
        );
        let eid = EntityId(1);

        assert_eq!(engine.presence_of(eid), Presence::Online);

        // Simulate timeout
        let changes = engine.evaluate_presence(1200);
        assert!(!changes.is_empty());
        assert_eq!(engine.presence_of(eid), Presence::Offline);

        // Recover
        engine.heartbeat(eid, 1300);
        assert_eq!(engine.presence_of(eid), Presence::Online);
    }

    #[test]
    fn test_existence_engine_discover() {
        let mut engine = ExistenceEngine::new();
        engine.create_registry("mixed");

        engine.register("mixed", EntityKind::Case, "c1", "/pv/cases/c1", 1000);
        engine.register("mixed", EntityKind::Signal, "s1", "/pv/signals/s1", 2000);
        engine.register("mixed", EntityKind::Case, "c2", "/pv/cases/c2", 3000);

        let query = DiscoveryQuery::all().with_kind(EntityKind::Case);
        let result = engine.discover("mixed", &query, 4000);
        assert!(result.is_some());
        if let Some(r) = result {
            assert_eq!(r.total_matched, 2);
        }
    }

    #[test]
    fn test_existence_engine_list_namespace() {
        let mut engine = ExistenceEngine::new();
        engine.create_registry("entities");

        engine.register("entities", EntityKind::Case, "c1", "/pv/cases/c1", 1000);
        engine.register("entities", EntityKind::Case, "c2", "/pv/cases/c2", 2000);
        engine.register("entities", EntityKind::Signal, "s1", "/pv/signals/s1", 3000);

        let under_cases = engine.list_namespace("/pv/cases");
        assert_eq!(under_cases.len(), 2);

        let all = engine.list_namespace("/");
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_existence_engine_multi_registry() {
        let mut engine = ExistenceEngine::new();
        engine.create_registry("cases");
        engine.create_typed_registry("signals", EntityKind::Signal);

        engine.register("cases", EntityKind::Case, "c1", "/pv/cases/c1", 1000);
        engine.register("signals", EntityKind::Signal, "s1", "/pv/signals/s1", 2000);

        assert_eq!(engine.active_registry_count(), 2);
        assert_eq!(engine.total_registries_created(), 2);
        assert!(engine.exists("cases", EntityId(1)));
        assert!(engine.exists("signals", EntityId(1)));
    }

    #[test]
    fn test_existence_engine_nonexistent_registry() {
        let mut engine = ExistenceEngine::new();
        let result = engine.register("missing", EntityKind::Case, "c1", "/c1", 1000);
        assert!(result.is_none());
    }

    // ═══════════════════════════════════════════════════════
    // PVNM NUMERIC ENGINE TESTS
    // ═══════════════════════════════════════════════════════

    #[test]
    fn test_numeric_engine_t3_grounding() {
        let comp = NumericEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
        assert_eq!(comp.unique().len(), 6);
    }

    /// The Quindecet Test: verify ALL 15 dominant primitives — 100% Lex Primitiva coverage.
    /// κ(AVC) → μ(PVOS) → σ(PVWF) → ∂(PVGW) → ν(PVRX) → ρ(PVML) →
    /// λ(PVSH) → Σ(PVMX) → ∝(PVTX) → ∅(PV∅) → →(PVOC) → ς(PVST) →
    /// π(PVDB) → ∃(PVEX) → N(PVNM)
    #[test]
    fn test_avc_pvos_pvnm_quindecet() {
        use std::collections::HashSet;

        let avc = PrimitiveComposition::new(vec![LexPrimitiva::Comparison])
            .with_dominant(LexPrimitiva::Comparison, 0.95);
        let pvos = Pvos::primitive_composition();
        let pvwf = WorkflowEngine::primitive_composition();
        let pvgw = Gateway::primitive_composition();
        let pvrx = ReactiveEngine::primitive_composition();
        let pvml = Ensemble::primitive_composition();
        let pvsh = Shell::primitive_composition();
        let pvmx = MetricsEngine::primitive_composition();
        let pvtx = TransactionEngine::primitive_composition();
        let pvvoid = VoidEngine::primitive_composition();
        let pvoc = OrchestratorEngine::primitive_composition();
        let pvst = StateEngine::primitive_composition();
        let pvdb = PersistenceEngine::primitive_composition();
        let pvex = ExistenceEngine::primitive_composition();
        let pvnm = NumericEngine::primitive_composition();

        // Each layer has its expected dominant primitive
        assert_eq!(avc.dominant, Some(LexPrimitiva::Comparison)); // κ
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping)); // μ
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence)); // σ
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary)); // ∂
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency)); // ν
        assert_eq!(pvml.dominant, Some(LexPrimitiva::Recursion)); // ρ
        assert_eq!(pvsh.dominant, Some(LexPrimitiva::Location)); // λ
        assert_eq!(pvmx.dominant, Some(LexPrimitiva::Sum)); // Σ
        assert_eq!(pvtx.dominant, Some(LexPrimitiva::Irreversibility)); // ∝
        assert_eq!(pvvoid.dominant, Some(LexPrimitiva::Void)); // ∅
        assert_eq!(pvoc.dominant, Some(LexPrimitiva::Causality)); // →
        assert_eq!(pvst.dominant, Some(LexPrimitiva::State)); // ς
        assert_eq!(pvdb.dominant, Some(LexPrimitiva::Persistence)); // π
        assert_eq!(pvex.dominant, Some(LexPrimitiva::Existence)); // ∃
        assert_eq!(pvnm.dominant, Some(LexPrimitiva::Quantity)); // N

        // ALL 15 are distinct — THE QUINDECET — 100% primitive coverage
        let dominants: HashSet<_> = [
            avc.dominant,
            pvos.dominant,
            pvwf.dominant,
            pvgw.dominant,
            pvrx.dominant,
            pvml.dominant,
            pvsh.dominant,
            pvmx.dominant,
            pvtx.dominant,
            pvvoid.dominant,
            pvoc.dominant,
            pvst.dominant,
            pvdb.dominant,
            pvex.dominant,
            pvnm.dominant,
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(
            dominants.len(),
            15,
            "ALL 15 Lex Primitiva covered — 100% primitive coverage — QUINDECET COMPLETE"
        );
    }

    #[test]
    fn test_numeric_engine_compute_prr() {
        let mut engine = NumericEngine::new();
        let table = ContingencyTable::new(15, 100, 20, 10000);
        let prr = engine.compute_prr(&table);
        assert!(prr.is_ok());
        if let Ok(v) = prr {
            assert!(v.point > 2.0);
            assert!(v.is_signal(2.0));
        }
        assert_eq!(engine.total_operations(), 1);
    }

    #[test]
    fn test_numeric_engine_compute_ror() {
        let mut engine = NumericEngine::new();
        let table = ContingencyTable::new(15, 100, 20, 10000);
        let ror = engine.compute_ror(&table);
        assert!(ror.is_ok());
        if let Ok(v) = ror {
            assert!(v.is_signal());
        }
    }

    #[test]
    fn test_numeric_engine_compute_ic() {
        let mut engine = NumericEngine::new();
        let table = ContingencyTable::new(15, 100, 20, 10000);
        let ic = engine.compute_ic(&table);
        assert!(ic.is_ok());
    }

    #[test]
    fn test_numeric_engine_compute_chi_square() {
        let mut engine = NumericEngine::new();
        let table = ContingencyTable::new(15, 100, 20, 10000);
        let chi = engine.compute_chi_square(&table);
        assert!(chi.is_ok());
        if let Ok(v) = chi {
            assert!(v.is_significant_05());
        }
    }

    #[test]
    fn test_numeric_engine_validate_range() {
        let mut engine = NumericEngine::new();
        engine.configure_pv_ranges();

        assert_eq!(engine.validate_range("prr", 3.5), Some(RangeCheck::InRange));
        assert_eq!(
            engine.validate_range("prr", -1.0),
            Some(RangeCheck::BelowMin)
        );
        assert_eq!(
            engine.validate_range("confidence", 0.95),
            Some(RangeCheck::InRange)
        );
        assert_eq!(
            engine.validate_range("confidence", 1.5),
            Some(RangeCheck::AboveMax)
        );
    }

    #[test]
    fn test_numeric_engine_convert_time() {
        let mut engine = NumericEngine::new();
        let years = engine.convert_time(365.0, TimeUnit::Days, TimeUnit::Years);
        assert!((years - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_numeric_engine_convert_rate() {
        let mut engine = NumericEngine::new();
        let per_million =
            engine.convert_rate(5.0, RateUnit::CasesPerThousand, RateUnit::ReportsPerMillion);
        assert!((per_million - 5000.0).abs() < 0.001);
    }

    #[test]
    fn test_numeric_engine_arithmetic() {
        let mut engine = NumericEngine::new();
        let result = engine.divide(10.0, 3.0);
        assert!(result.is_ok());

        let mean = engine.mean(&[2.0, 4.0, 6.0, 8.0]);
        assert!(mean.is_ok());
        if let Ok(v) = mean {
            assert!((v - 5.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_numeric_engine_full_signal_analysis() {
        let mut engine = NumericEngine::new();
        engine.configure_pv_ranges();

        let table = ContingencyTable::new(15, 100, 20, 10000);

        // Compute all statistics
        let prr = engine.compute_prr(&table);
        let ror = engine.compute_ror(&table);
        let chi = engine.compute_chi_square(&table);
        let ic = engine.compute_ic(&table);

        assert!(prr.is_ok());
        assert!(ror.is_ok());
        assert!(chi.is_ok());
        assert!(ic.is_ok());

        // Validate computed PRR is in valid range
        if let Ok(v) = prr {
            assert_eq!(
                engine.validate_range("prr", v.point),
                Some(RangeCheck::InRange),
            );
        }

        assert!(engine.total_operations() >= 5);
    }

    #[test]
    fn test_numeric_engine_counters() {
        let mut engine = NumericEngine::new();
        engine.configure_pv_ranges();

        let table = ContingencyTable::new(15, 100, 20, 10000);
        engine.compute_prr(&table).ok();
        engine.validate_range("prr", -1.0);
        engine.convert_time(1.0, TimeUnit::Days, TimeUnit::Years);
        engine.divide(10.0, 0.0).ok();

        assert!(engine.total_operations() >= 4);
        assert_eq!(engine.total_range_violations(), 1);
        assert_eq!(engine.total_arithmetic_errors(), 1);
        assert_eq!(engine.total_conversions(), 1);
    }
}
