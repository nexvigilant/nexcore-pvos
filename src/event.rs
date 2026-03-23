//! # PVOC Core Event Types
//!
//! Orchestration events that track cross-layer causality.
//! Unlike PVRX events (temporal data streams), PVOC events model
//! cause-and-effect relationships between layer operations.
//!
//! ## Primitives
//! - → (Causality) — events ARE causal signals
//! - σ (Sequence) — ordered event chains
//! - π (Persistence) — event metadata for audit trail

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique orchestration event identifier.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrcEventId(pub u64);

impl GroundsTo for OrcEventId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

/// Groups related events within a single causal flow.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(pub u64);

impl GroundsTo for CorrelationId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

/// Links an event directly to the event that caused it.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausationId(pub u64);

impl GroundsTo for CausationId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

// ═══════════════════════════════════════════════════════════
// EVENT SOURCE & KIND
// ═══════════════════════════════════════════════════════════

/// Which PVOS layer emitted the orchestration event.
/// Tier: T2-P (→ + λ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventSource {
    /// PVOS kernel (μ-dominant)
    Pvos,
    /// AVC signal detection (κ-dominant)
    Avc,
    /// PVWF workflow engine (σ-dominant)
    Pvwf,
    /// PVGW gateway (∂-dominant)
    Pvgw,
    /// PVRX reactive streaming (ν-dominant)
    Pvrx,
    /// PVML machine learning (ρ-dominant)
    Pvml,
    /// PVSH shell interface (λ-dominant)
    Pvsh,
    /// PVMX metrics (Σ-dominant)
    Pvmx,
    /// PVTX transactions (∝-dominant)
    Pvtx,
    /// PV∅ void handling (∅-dominant)
    PvVoid,
    /// PVOC orchestrator itself (→-dominant)
    Pvoc,
    /// External system
    External(String),
}

impl GroundsTo for EventSource {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality, LexPrimitiva::Location])
    }
}

/// Classification of orchestration events by layer operation.
/// Tier: T2-C (→ + Σ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    // ── PVOS kernel ─────────────────────────────────────
    /// System boot completed
    SystemBooted,
    /// System shutdown initiated
    SystemShutdown,

    // ── AVC detection ───────────────────────────────────
    /// Signal detected (PRR/ROR/IC/EBGM/Chi²)
    SignalDetected,
    /// Signal dismissed (below threshold)
    SignalDismissed,

    // ── PVWF workflow ───────────────────────────────────
    /// Workflow execution started
    WorkflowStarted,
    /// Workflow completed successfully
    WorkflowCompleted,
    /// Workflow execution failed
    WorkflowFailed,

    // ── PVGW gateway ────────────────────────────────────
    /// Request received at gateway
    RequestReceived,
    /// Request authenticated successfully
    RequestAuthenticated,
    /// Request rejected by gateway
    RequestRejected,

    // ── PVRX streaming ──────────────────────────────────
    /// Event ingested into stream
    StreamIngested,
    /// Monitor alert triggered
    MonitorTriggered,

    // ── PVML learning ───────────────────────────────────
    /// Feedback received for signal
    FeedbackReceived,
    /// Model retrained with new data
    ModelRetrained,
    /// Distribution drift detected
    DriftDetected,

    // ── PVSH shell ──────────────────────────────────────
    /// Shell command executed
    CommandExecuted,

    // ── PVMX metrics ────────────────────────────────────
    /// Metric value updated
    MetricUpdated,
    /// Metric threshold crossed
    ThresholdCrossed,

    // ── PVTX transactions ───────────────────────────────
    /// Transaction committed
    TransactionCommitted,
    /// Regulatory submission sent
    SubmissionSent,

    // ── PV∅ void ────────────────────────────────────────
    /// Missing data detected in record
    MissingDetected,
    /// Underreporting gap identified
    UnderreportingAlert,

    // ── PVOC orchestrator ───────────────────────────────
    /// Trigger fired
    TriggerFired,
    /// Dependency resolved
    DependencyResolved,
    /// Causal trace completed
    TraceCompleted,

    // ── Extensibility ───────────────────────────────────
    /// Custom event kind
    Custom(String),
}

impl GroundsTo for EventKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality, LexPrimitiva::Sum])
    }
}

// ═══════════════════════════════════════════════════════════
// EVENT METADATA
// ═══════════════════════════════════════════════════════════

/// Metadata attached to every orchestration event.
/// Provides causal linking (correlation + causation IDs)
/// and temporal ordering.
///
/// Tier: T2-C (→ + σ + π)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMeta {
    /// Monotonic timestamp (epoch millis or logical clock)
    pub timestamp: u64,
    /// Correlation ID — groups all events in a flow
    pub correlation: Option<CorrelationId>,
    /// Causation ID — the event that directly caused this one
    pub causation: Option<CausationId>,
    /// Which layer/component emitted this event
    pub source: EventSource,
}

impl EventMeta {
    /// Creates metadata with source and timestamp.
    #[must_use]
    pub fn new(source: EventSource, timestamp: u64) -> Self {
        Self {
            timestamp,
            correlation: None,
            causation: None,
            source,
        }
    }

    /// Adds correlation ID.
    #[must_use]
    pub fn with_correlation(mut self, id: CorrelationId) -> Self {
        self.correlation = Some(id);
        self
    }

    /// Adds causation ID linking to the direct cause.
    #[must_use]
    pub fn with_causation(mut self, id: CausationId) -> Self {
        self.causation = Some(id);
        self
    }
}

impl GroundsTo for EventMeta {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,   // → — causal linking
            LexPrimitiva::Sequence,    // σ — temporal ordering
            LexPrimitiva::Persistence, // π — audit-grade metadata
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// CAUSATION CHAIN
// ═══════════════════════════════════════════════════════════

/// Linked sequence of event IDs showing causal lineage.
/// Read left-to-right: A caused B caused C.
///
/// Tier: T2-C (→ + σ + ρ)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausationChain {
    /// Ordered event IDs: `[root_cause, ..., final_effect]`
    links: Vec<OrcEventId>,
}

impl CausationChain {
    /// Creates an empty causation chain.
    #[must_use]
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /// Creates a chain from existing links.
    #[must_use]
    pub fn from_links(links: Vec<OrcEventId>) -> Self {
        Self { links }
    }

    /// Appends an event to the chain.
    pub fn push(&mut self, id: OrcEventId) {
        self.links.push(id);
    }

    /// Returns the root cause (first event in chain).
    #[must_use]
    pub fn root(&self) -> Option<OrcEventId> {
        self.links.first().copied()
    }

    /// Returns the final effect (last event in chain).
    #[must_use]
    pub fn effect(&self) -> Option<OrcEventId> {
        self.links.last().copied()
    }

    /// Returns chain length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.links.len()
    }

    /// Returns true if chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// Returns all links as a slice.
    #[must_use]
    pub fn links(&self) -> &[OrcEventId] {
        &self.links
    }

    /// Checks if an event ID is in this chain.
    #[must_use]
    pub fn contains(&self, id: OrcEventId) -> bool {
        self.links.contains(&id)
    }

    /// Depth of causation (number of hops from root to effect).
    #[must_use]
    pub fn depth(&self) -> usize {
        if self.links.is_empty() {
            0
        } else {
            self.links.len() - 1
        }
    }
}

impl Default for CausationChain {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for CausationChain {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — each link is a causal relationship
            LexPrimitiva::Sequence,  // σ — ordered chain
            LexPrimitiva::Recursion, // ρ — recursive structure (chains of chains)
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// EVENT PAYLOAD
// ═══════════════════════════════════════════════════════════

/// Orchestration event payload, categorized by layer domain.
/// Tier: T2-C (→ + Σ + μ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrcPayload {
    /// Signal detection result
    Signal {
        drug: String,
        event: String,
        statistic: f64,
        detected: bool,
    },
    /// Workflow lifecycle event
    Workflow {
        name: String,
        step: Option<String>,
        outcome: String,
    },
    /// Gateway auth/routing event
    Gateway {
        identity: String,
        action: String,
        allowed: bool,
    },
    /// Metric update
    Metric {
        name: String,
        value: f64,
        threshold: Option<f64>,
    },
    /// Transaction lifecycle
    Transaction {
        tx_id: u64,
        kind: String,
        outcome: String,
    },
    /// Void/absence detection
    Absence {
        field: String,
        reason: String,
        severity: String,
    },
    /// Alert notification
    Alert { severity: String, message: String },
    /// Custom extensible payload
    Custom { kind: String, data: String },
    /// Empty payload (for system events)
    Empty,
}

impl GroundsTo for OrcPayload {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — payload is the causal content
            LexPrimitiva::Sum,       // Σ — enum variants
            LexPrimitiva::Mapping,   // μ — structured key-value data
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// ORC EVENT — THE CORE TYPE
// ═══════════════════════════════════════════════════════════

/// Orchestration event: the fundamental unit of cross-layer causality.
///
/// An `OrcEvent` represents something that happened in one layer
/// that may cause actions in other layers. Unlike PVRX's `Event`
/// (temporal data), `OrcEvent` models the *why* of system behavior.
///
/// Tier: T2-C (→ + σ + μ + π)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrcEvent {
    /// Unique event ID
    pub id: OrcEventId,
    /// What kind of event
    pub kind: EventKind,
    /// Causal metadata (correlation, causation, source)
    pub meta: EventMeta,
    /// Event payload with domain data
    pub payload: OrcPayload,
}

impl OrcEvent {
    /// Creates a new orchestration event.
    #[must_use]
    pub fn new(id: OrcEventId, kind: EventKind, meta: EventMeta, payload: OrcPayload) -> Self {
        Self {
            id,
            kind,
            meta,
            payload,
        }
    }

    /// Creates a system event with empty payload.
    #[must_use]
    pub fn system(id: OrcEventId, kind: EventKind, source: EventSource, timestamp: u64) -> Self {
        Self {
            id,
            kind,
            meta: EventMeta::new(source, timestamp),
            payload: OrcPayload::Empty,
        }
    }

    /// Returns the event source.
    #[must_use]
    pub fn source(&self) -> &EventSource {
        &self.meta.source
    }

    /// Returns the correlation ID if set.
    #[must_use]
    pub fn correlation(&self) -> Option<CorrelationId> {
        self.meta.correlation
    }

    /// Returns the causation ID if set.
    #[must_use]
    pub fn causation(&self) -> Option<CausationId> {
        self.meta.causation
    }

    /// Returns the event timestamp.
    #[must_use]
    pub fn timestamp(&self) -> u64 {
        self.meta.timestamp
    }

    /// Checks if this event was caused by a specific event.
    #[must_use]
    pub fn caused_by(&self, cause_event_id: u64) -> bool {
        self.meta.causation.map_or(false, |c| c.0 == cause_event_id)
    }

    /// Checks if this event belongs to a specific correlation group.
    #[must_use]
    pub fn in_flow(&self, flow_id: u64) -> bool {
        self.meta.correlation.map_or(false, |c| c.0 == flow_id)
    }
}

impl GroundsTo for OrcEvent {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,   // → — the event IS a causal signal
            LexPrimitiva::Sequence,    // σ — temporal ordering
            LexPrimitiva::Mapping,     // μ — structured payload
            LexPrimitiva::Persistence, // π — audit-grade immutability
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

    #[test]
    fn test_orc_event_id_grounding() {
        let comp = OrcEventId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
        assert!(comp.unique().contains(&LexPrimitiva::Causality));
    }

    #[test]
    fn test_correlation_id_grounding() {
        let comp = CorrelationId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
    }

    #[test]
    fn test_causation_id_grounding() {
        let comp = CausationId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
    }

    #[test]
    fn test_event_source_grounding() {
        let comp = EventSource::primitive_composition();
        assert_eq!(comp.unique().len(), 2);
        assert!(comp.unique().contains(&LexPrimitiva::Causality));
        assert!(comp.unique().contains(&LexPrimitiva::Location));
    }

    #[test]
    fn test_event_kind_grounding() {
        let comp = EventKind::primitive_composition();
        assert_eq!(comp.unique().len(), 2);
    }

    #[test]
    fn test_event_meta_builder() {
        let meta = EventMeta::new(EventSource::Avc, 1000)
            .with_correlation(CorrelationId(42))
            .with_causation(CausationId(99));

        assert_eq!(meta.timestamp, 1000);
        assert_eq!(meta.correlation, Some(CorrelationId(42)));
        assert_eq!(meta.causation, Some(CausationId(99)));
        assert_eq!(meta.source, EventSource::Avc);
    }

    #[test]
    fn test_event_meta_grounding() {
        let comp = EventMeta::primitive_composition();
        assert_eq!(comp.unique().len(), 3);
    }

    #[test]
    fn test_causation_chain_operations() {
        let mut chain = CausationChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.depth(), 0);

        chain.push(OrcEventId(1));
        chain.push(OrcEventId(2));
        chain.push(OrcEventId(3));

        assert_eq!(chain.len(), 3);
        assert_eq!(chain.depth(), 2);
        assert_eq!(chain.root(), Some(OrcEventId(1)));
        assert_eq!(chain.effect(), Some(OrcEventId(3)));
        assert!(chain.contains(OrcEventId(2)));
        assert!(!chain.contains(OrcEventId(99)));
    }

    #[test]
    fn test_causation_chain_from_links() {
        let chain =
            CausationChain::from_links(vec![OrcEventId(10), OrcEventId(20), OrcEventId(30)]);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain.root(), Some(OrcEventId(10)));
        assert_eq!(chain.effect(), Some(OrcEventId(30)));
    }

    #[test]
    fn test_causation_chain_grounding() {
        let comp = CausationChain::primitive_composition();
        assert_eq!(comp.unique().len(), 3);
        assert!(comp.unique().contains(&LexPrimitiva::Causality));
        assert!(comp.unique().contains(&LexPrimitiva::Sequence));
        assert!(comp.unique().contains(&LexPrimitiva::Recursion));
    }

    #[test]
    fn test_orc_event_creation() {
        let meta = EventMeta::new(EventSource::Avc, 1000).with_correlation(CorrelationId(1));

        let event = OrcEvent::new(
            OrcEventId(1),
            EventKind::SignalDetected,
            meta,
            OrcPayload::Signal {
                drug: "aspirin".into(),
                event: "headache".into(),
                statistic: 3.5,
                detected: true,
            },
        );

        assert_eq!(event.id, OrcEventId(1));
        assert_eq!(event.kind, EventKind::SignalDetected);
        assert_eq!(*event.source(), EventSource::Avc);
        assert_eq!(event.timestamp(), 1000);
        assert!(event.in_flow(1));
        assert!(!event.in_flow(99));
    }

    #[test]
    fn test_orc_event_system_shortcut() {
        let event = OrcEvent::system(OrcEventId(1), EventKind::SystemBooted, EventSource::Pvos, 0);

        assert_eq!(event.payload, OrcPayload::Empty);
        assert_eq!(*event.source(), EventSource::Pvos);
    }

    #[test]
    fn test_orc_event_caused_by() {
        let meta = EventMeta::new(EventSource::Pvwf, 2000).with_causation(CausationId(42));

        let event = OrcEvent::new(
            OrcEventId(2),
            EventKind::WorkflowStarted,
            meta,
            OrcPayload::Workflow {
                name: "triage".into(),
                step: None,
                outcome: "started".into(),
            },
        );

        assert!(event.caused_by(42));
        assert!(!event.caused_by(99));
    }

    #[test]
    fn test_orc_event_grounding() {
        let comp = OrcEvent::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_orc_payload_variants() {
        let payloads = vec![
            OrcPayload::Signal {
                drug: "warfarin".into(),
                event: "bleeding".into(),
                statistic: 4.5,
                detected: true,
            },
            OrcPayload::Workflow {
                name: "review".into(),
                step: Some("detect".into()),
                outcome: "pending".into(),
            },
            OrcPayload::Gateway {
                identity: "user@org".into(),
                action: "read".into(),
                allowed: true,
            },
            OrcPayload::Metric {
                name: "signals_per_day".into(),
                value: 42.0,
                threshold: Some(100.0),
            },
            OrcPayload::Transaction {
                tx_id: 1,
                kind: "submission".into(),
                outcome: "committed".into(),
            },
            OrcPayload::Absence {
                field: "reporter".into(),
                reason: "not_provided".into(),
                severity: "moderate".into(),
            },
            OrcPayload::Alert {
                severity: "critical".into(),
                message: "threshold breach".into(),
            },
            OrcPayload::Custom {
                kind: "audit".into(),
                data: "{}".into(),
            },
            OrcPayload::Empty,
        ];

        assert_eq!(payloads.len(), 9);
    }

    #[test]
    fn test_event_source_all_layers() {
        let sources = vec![
            EventSource::Pvos,
            EventSource::Avc,
            EventSource::Pvwf,
            EventSource::Pvgw,
            EventSource::Pvrx,
            EventSource::Pvml,
            EventSource::Pvsh,
            EventSource::Pvmx,
            EventSource::Pvtx,
            EventSource::PvVoid,
            EventSource::Pvoc,
            EventSource::External("fda".into()),
        ];

        // All 11 internal sources + external
        assert_eq!(sources.len(), 12);
    }
}
