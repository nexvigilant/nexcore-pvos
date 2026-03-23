//! # PVOC Causal Tracing
//!
//! Reconstructs the chain of events that led to an outcome.
//! Answers the regulatory question: "Why did X happen?"
//! by walking the causal graph backward from effect to root cause.
//!
//! ## Primitives
//! - → (Causality) — trace IS the causal history
//! - π (Persistence) — durable trace log for regulatory audit
//! - ρ (Recursion) — recursive graph traversal
//! - σ (Sequence) — ordered trace reconstruction

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::event::{CausationChain, EventKind, EventSource, OrcEvent, OrcEventId};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique trace identifier.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub u64);

impl GroundsTo for TraceId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

// ═══════════════════════════════════════════════════════════
// TRACE NODE
// ═══════════════════════════════════════════════════════════

/// A single step in a causal trace.
/// Records what happened, when, and what caused it.
///
/// Tier: T2-C (→ + σ + μ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceNode {
    /// The event at this step
    pub event_id: OrcEventId,
    /// What kind of event
    pub kind: EventKind,
    /// Which layer emitted it
    pub source: EventSource,
    /// Timestamp
    pub timestamp: u64,
    /// Depth from the queried event (0 = the queried event itself)
    pub depth: usize,
}

impl TraceNode {
    /// Creates a trace node from an event.
    #[must_use]
    pub fn from_event(event: &OrcEvent, depth: usize) -> Self {
        Self {
            event_id: event.id,
            kind: event.kind.clone(),
            source: event.source().clone(),
            timestamp: event.timestamp(),
            depth,
        }
    }
}

impl GroundsTo for TraceNode {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — node in causal chain
            LexPrimitiva::Sequence,  // σ — temporal position
            LexPrimitiva::Mapping,   // μ — event data mapping
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// CAUSAL TRACE
// ═══════════════════════════════════════════════════════════

/// A complete causal trace from effect back to root cause(s).
/// Contains all paths through the causal graph that led
/// to the queried event.
///
/// Tier: T2-C (→ + σ + ρ + π)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalTrace {
    /// Unique trace ID
    pub id: TraceId,
    /// The event being investigated
    pub target: OrcEventId,
    /// All nodes in the trace (ordered by depth, then timestamp)
    pub nodes: Vec<TraceNode>,
    /// Root causes (events with no known cause)
    pub root_causes: Vec<OrcEventId>,
    /// Maximum depth reached
    pub max_depth: usize,
    /// The primary causal chain (shortest path from root to target)
    pub primary_chain: CausationChain,
}

impl CausalTrace {
    /// Creates a new trace for a target event.
    #[must_use]
    pub fn new(id: TraceId, target: OrcEventId) -> Self {
        Self {
            id,
            target,
            nodes: Vec::new(),
            root_causes: Vec::new(),
            max_depth: 0,
            primary_chain: CausationChain::new(),
        }
    }

    /// Returns the number of events in the trace.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the trace is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the number of root causes found.
    #[must_use]
    pub fn root_cause_count(&self) -> usize {
        self.root_causes.len()
    }

    /// Returns the primary root cause (first root in the primary chain).
    #[must_use]
    pub fn primary_root(&self) -> Option<OrcEventId> {
        self.primary_chain.root()
    }

    /// Returns all unique sources involved in the trace.
    #[must_use]
    pub fn involved_sources(&self) -> Vec<EventSource> {
        let mut sources: Vec<EventSource> = self
            .nodes
            .iter()
            .map(|n| n.source.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        sources.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        sources
    }

    /// Returns trace nodes at a specific depth.
    #[must_use]
    pub fn at_depth(&self, depth: usize) -> Vec<&TraceNode> {
        self.nodes.iter().filter(|n| n.depth == depth).collect()
    }
}

impl GroundsTo for CausalTrace {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,   // → — the trace IS causality history
            LexPrimitiva::Sequence,    // σ — ordered reconstruction
            LexPrimitiva::Recursion,   // ρ — recursive graph traversal
            LexPrimitiva::Persistence, // π — persisted for regulatory audit
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// TRACE QUERY
// ═══════════════════════════════════════════════════════════

/// Parameters for a trace query.
/// Tier: T2-P (→ + ∂)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceQuery {
    /// The event to investigate
    pub target: OrcEventId,
    /// Maximum depth to search (prevents infinite traversal)
    pub max_depth: usize,
    /// Optional: only include events from these sources
    pub source_filter: Option<Vec<EventSource>>,
}

impl TraceQuery {
    /// Creates a trace query for a target event.
    #[must_use]
    pub fn new(target: OrcEventId, max_depth: usize) -> Self {
        Self {
            target,
            max_depth,
            source_filter: None,
        }
    }

    /// Restricts the trace to events from specific sources.
    #[must_use]
    pub fn with_sources(mut self, sources: Vec<EventSource>) -> Self {
        self.source_filter = Some(sources);
        self
    }

    /// Checks if an event source is allowed by the filter.
    #[must_use]
    pub fn allows_source(&self, source: &EventSource) -> bool {
        self.source_filter
            .as_ref()
            .map_or(true, |allowed| allowed.contains(source))
    }
}

impl GroundsTo for TraceQuery {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — querying causal history
            LexPrimitiva::Boundary,  // ∂ — depth limit, source filter
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// ROOT CAUSE ANALYSIS
// ═══════════════════════════════════════════════════════════

/// Result of root cause analysis.
/// Tier: T2-C (→ + κ + Σ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootCause {
    /// The root cause event ID
    pub event_id: OrcEventId,
    /// What kind of event started the chain
    pub kind: EventKind,
    /// Which layer originated it
    pub source: EventSource,
    /// How many downstream effects it caused (impact)
    pub downstream_count: usize,
    /// Depth from the queried event to this root
    pub depth: usize,
}

impl RootCause {
    /// Creates a root cause entry.
    #[must_use]
    pub fn new(
        event_id: OrcEventId,
        kind: EventKind,
        source: EventSource,
        downstream_count: usize,
        depth: usize,
    ) -> Self {
        Self {
            event_id,
            kind,
            source,
            downstream_count,
            depth,
        }
    }
}

impl GroundsTo for RootCause {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,  // → — identifies the cause
            LexPrimitiva::Comparison, // κ — ranking root causes
            LexPrimitiva::Sum,        // Σ — counting downstream impact
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// TRACE LOG — PERSISTENT CAUSAL HISTORY
// ═══════════════════════════════════════════════════════════

/// Persistent log of causal relationships between events.
/// Maps each event to the event that directly caused it.
/// Supports backward traversal for trace reconstruction.
///
/// Tier: T2-C (→ + π + μ + ρ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLog {
    /// Causal links: effect → cause
    cause_map: HashMap<OrcEventId, OrcEventId>,
    /// Event metadata: id → (kind, source, timestamp)
    event_info: HashMap<OrcEventId, (EventKind, EventSource, u64)>,
    /// Next trace ID
    next_trace_id: u64,
    /// Total causal links recorded
    total_links: u64,
    /// Total traces performed
    total_traces: u64,
}

impl TraceLog {
    /// Creates an empty trace log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cause_map: HashMap::new(),
            event_info: HashMap::new(),
            next_trace_id: 1,
            total_links: 0,
            total_traces: 0,
        }
    }

    /// Records a causal relationship: `cause` caused `effect`.
    pub fn record_link(&mut self, cause: OrcEventId, effect: OrcEventId) {
        self.cause_map.insert(effect, cause);
        self.total_links += 1;
    }

    /// Records event metadata for trace reconstruction.
    pub fn record_event(&mut self, event: &OrcEvent) {
        self.event_info.insert(
            event.id,
            (
                event.kind.clone(),
                event.source().clone(),
                event.timestamp(),
            ),
        );

        // If the event has a causation ID, record the link
        if let Some(causation) = event.meta.causation {
            self.cause_map.insert(event.id, OrcEventId(causation.0));
            self.total_links += 1;
        }
    }

    /// Traces backward from an effect to find all causes.
    /// Uses BFS with depth limiting.
    pub fn trace_back(&mut self, query: &TraceQuery) -> CausalTrace {
        let trace_id = TraceId(self.next_trace_id);
        self.next_trace_id += 1;
        self.total_traces += 1;

        let mut trace = CausalTrace::new(trace_id, query.target);
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(OrcEventId, usize)> = VecDeque::new();

        queue.push_back((query.target, 0));

        while let Some((event_id, depth)) = queue.pop_front() {
            if visited.contains(&event_id) || depth > query.max_depth {
                continue;
            }
            visited.insert(event_id);

            // Get event info
            if let Some((kind, source, timestamp)) = self.event_info.get(&event_id) {
                // Check source filter
                if !query.allows_source(source) {
                    continue;
                }

                let node = TraceNode {
                    event_id,
                    kind: kind.clone(),
                    source: source.clone(),
                    timestamp: *timestamp,
                    depth,
                };
                trace.nodes.push(node);

                if depth > trace.max_depth {
                    trace.max_depth = depth;
                }
            }

            // Follow the causal link backward
            if let Some(&cause_id) = self.cause_map.get(&event_id) {
                queue.push_back((cause_id, depth + 1));
            } else if depth > 0 {
                // No cause found — this is a root cause
                trace.root_causes.push(event_id);
            }
        }

        // Build primary chain: walk from target back to first root
        let mut chain_ids = Vec::new();
        let mut current = query.target;
        let mut chain_visited = HashSet::new();
        while let Some(&cause) = self.cause_map.get(&current) {
            if chain_visited.contains(&cause) {
                break; // Prevent infinite loop
            }
            chain_visited.insert(cause);
            chain_ids.push(cause);
            current = cause;
        }
        chain_ids.reverse();
        chain_ids.push(query.target);
        trace.primary_chain = CausationChain::from_links(chain_ids);

        // Sort nodes by depth then timestamp
        trace
            .nodes
            .sort_by(|a, b| a.depth.cmp(&b.depth).then(a.timestamp.cmp(&b.timestamp)));

        trace
    }

    /// Finds the direct cause of an event.
    #[must_use]
    pub fn direct_cause(&self, effect: OrcEventId) -> Option<OrcEventId> {
        self.cause_map.get(&effect).copied()
    }

    /// Returns all events directly caused by a given event.
    #[must_use]
    pub fn direct_effects(&self, cause: OrcEventId) -> Vec<OrcEventId> {
        self.cause_map
            .iter()
            .filter(|&(_, &c)| c == cause)
            .map(|(&effect, _)| effect)
            .collect()
    }

    /// Returns the total number of causal links recorded.
    #[must_use]
    pub fn total_links(&self) -> u64 {
        self.total_links
    }

    /// Returns the total number of traces performed.
    #[must_use]
    pub fn total_traces(&self) -> u64 {
        self.total_traces
    }

    /// Returns the number of events with metadata.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.event_info.len()
    }
}

impl Default for TraceLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for TraceLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,   // → — stores causal relationships
            LexPrimitiva::Persistence, // π — durable audit trail
            LexPrimitiva::Mapping,     // μ — effect → cause mapping
            LexPrimitiva::Recursion,   // ρ — recursive traversal
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::super::event::{CausationId, EventMeta, OrcPayload};
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_event_with_cause(
        id: u64,
        kind: EventKind,
        source: EventSource,
        cause: Option<u64>,
    ) -> OrcEvent {
        let mut meta = EventMeta::new(source, id * 100);
        if let Some(c) = cause {
            meta = meta.with_causation(CausationId(c));
        }
        OrcEvent::new(OrcEventId(id), kind, meta, OrcPayload::Empty)
    }

    #[test]
    fn test_trace_id_grounding() {
        let comp = TraceId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
    }

    #[test]
    fn test_trace_node_from_event() {
        let event = OrcEvent::system(
            OrcEventId(1),
            EventKind::SignalDetected,
            EventSource::Avc,
            1000,
        );
        let node = TraceNode::from_event(&event, 0);
        assert_eq!(node.event_id, OrcEventId(1));
        assert_eq!(node.kind, EventKind::SignalDetected);
        assert_eq!(node.source, EventSource::Avc);
        assert_eq!(node.depth, 0);
    }

    #[test]
    fn test_causal_trace_empty() {
        let trace = CausalTrace::new(TraceId(1), OrcEventId(1));
        assert!(trace.is_empty());
        assert_eq!(trace.len(), 0);
        assert_eq!(trace.root_cause_count(), 0);
    }

    #[test]
    fn test_trace_query_source_filter() {
        let query = TraceQuery::new(OrcEventId(1), 10)
            .with_sources(vec![EventSource::Avc, EventSource::Pvwf]);

        assert!(query.allows_source(&EventSource::Avc));
        assert!(query.allows_source(&EventSource::Pvwf));
        assert!(!query.allows_source(&EventSource::Pvml));
    }

    #[test]
    fn test_trace_query_no_filter() {
        let query = TraceQuery::new(OrcEventId(1), 10);
        assert!(query.allows_source(&EventSource::Avc));
        assert!(query.allows_source(&EventSource::Pvml));
    }

    #[test]
    fn test_trace_log_record_and_query() {
        let mut log = TraceLog::new();

        // Chain: signal(1) → workflow(2) → metric(3)
        let e1 = make_event_with_cause(1, EventKind::SignalDetected, EventSource::Avc, None);
        let e2 = make_event_with_cause(2, EventKind::WorkflowStarted, EventSource::Pvwf, Some(1));
        let e3 = make_event_with_cause(3, EventKind::MetricUpdated, EventSource::Pvmx, Some(2));

        log.record_event(&e1);
        log.record_event(&e2);
        log.record_event(&e3);

        assert_eq!(log.total_links(), 2);
        assert_eq!(log.event_count(), 3);

        // Direct cause queries
        assert_eq!(log.direct_cause(OrcEventId(3)), Some(OrcEventId(2)));
        assert_eq!(log.direct_cause(OrcEventId(2)), Some(OrcEventId(1)));
        assert_eq!(log.direct_cause(OrcEventId(1)), None);
    }

    #[test]
    fn test_trace_log_trace_back() {
        let mut log = TraceLog::new();

        // Chain: signal(1) → workflow(2) → submission(3)
        let e1 = make_event_with_cause(1, EventKind::SignalDetected, EventSource::Avc, None);
        let e2 = make_event_with_cause(2, EventKind::WorkflowCompleted, EventSource::Pvwf, Some(1));
        let e3 = make_event_with_cause(3, EventKind::SubmissionSent, EventSource::Pvtx, Some(2));

        log.record_event(&e1);
        log.record_event(&e2);
        log.record_event(&e3);

        let query = TraceQuery::new(OrcEventId(3), 10);
        let trace = log.trace_back(&query);

        assert_eq!(trace.target, OrcEventId(3));
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.max_depth, 2);
        assert_eq!(trace.root_causes, vec![OrcEventId(1)]);

        // Primary chain: 1 → 2 → 3
        assert_eq!(trace.primary_chain.root(), Some(OrcEventId(1)));
        assert_eq!(trace.primary_chain.effect(), Some(OrcEventId(3)));
        assert_eq!(trace.primary_chain.len(), 3);
    }

    #[test]
    fn test_trace_log_depth_limit() {
        let mut log = TraceLog::new();

        // Long chain: 1 → 2 → 3 → 4 → 5
        let e1 = make_event_with_cause(1, EventKind::SignalDetected, EventSource::Avc, None);
        let e2 = make_event_with_cause(2, EventKind::WorkflowStarted, EventSource::Pvwf, Some(1));
        let e3 = make_event_with_cause(3, EventKind::MetricUpdated, EventSource::Pvmx, Some(2));
        let e4 = make_event_with_cause(4, EventKind::ThresholdCrossed, EventSource::Pvmx, Some(3));
        let e5 = make_event_with_cause(5, EventKind::SubmissionSent, EventSource::Pvtx, Some(4));

        log.record_event(&e1);
        log.record_event(&e2);
        log.record_event(&e3);
        log.record_event(&e4);
        log.record_event(&e5);

        // Trace with depth limit of 2 from event 5
        let query = TraceQuery::new(OrcEventId(5), 2);
        let trace = log.trace_back(&query);

        // Should only reach depth 2: events 5, 4, 3
        assert_eq!(trace.max_depth, 2);
        assert!(trace.len() <= 3);
    }

    #[test]
    fn test_trace_log_direct_effects() {
        let mut log = TraceLog::new();

        // Fork: signal(1) → workflow(2), signal(1) → alert(3)
        let e1 = make_event_with_cause(1, EventKind::SignalDetected, EventSource::Avc, None);
        let e2 = make_event_with_cause(2, EventKind::WorkflowStarted, EventSource::Pvwf, Some(1));
        let e3 = make_event_with_cause(3, EventKind::TriggerFired, EventSource::Pvoc, Some(1));

        log.record_event(&e1);
        log.record_event(&e2);
        log.record_event(&e3);

        let effects = log.direct_effects(OrcEventId(1));
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_trace_involved_sources() {
        let mut log = TraceLog::new();

        let e1 = make_event_with_cause(1, EventKind::SignalDetected, EventSource::Avc, None);
        let e2 = make_event_with_cause(2, EventKind::WorkflowStarted, EventSource::Pvwf, Some(1));
        let e3 = make_event_with_cause(3, EventKind::SubmissionSent, EventSource::Pvtx, Some(2));

        log.record_event(&e1);
        log.record_event(&e2);
        log.record_event(&e3);

        let query = TraceQuery::new(OrcEventId(3), 10);
        let trace = log.trace_back(&query);

        let sources = trace.involved_sources();
        assert_eq!(sources.len(), 3);
    }

    #[test]
    fn test_causal_trace_grounding() {
        let comp = CausalTrace::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_trace_log_grounding() {
        let comp = TraceLog::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 4);
    }

    #[test]
    fn test_root_cause_creation() {
        let root = RootCause::new(
            OrcEventId(1),
            EventKind::SignalDetected,
            EventSource::Avc,
            5,
            3,
        );
        assert_eq!(root.event_id, OrcEventId(1));
        assert_eq!(root.downstream_count, 5);
        assert_eq!(root.depth, 3);
    }

    #[test]
    fn test_trace_at_depth() {
        let mut trace = CausalTrace::new(TraceId(1), OrcEventId(3));
        trace.nodes.push(TraceNode {
            event_id: OrcEventId(3),
            kind: EventKind::SubmissionSent,
            source: EventSource::Pvtx,
            timestamp: 300,
            depth: 0,
        });
        trace.nodes.push(TraceNode {
            event_id: OrcEventId(2),
            kind: EventKind::WorkflowCompleted,
            source: EventSource::Pvwf,
            timestamp: 200,
            depth: 1,
        });
        trace.nodes.push(TraceNode {
            event_id: OrcEventId(1),
            kind: EventKind::SignalDetected,
            source: EventSource::Avc,
            timestamp: 100,
            depth: 2,
        });

        let depth_0 = trace.at_depth(0);
        assert_eq!(depth_0.len(), 1);
        assert_eq!(depth_0[0].event_id, OrcEventId(3));

        let depth_2 = trace.at_depth(2);
        assert_eq!(depth_2.len(), 1);
        assert_eq!(depth_2[0].event_id, OrcEventId(1));
    }
}
