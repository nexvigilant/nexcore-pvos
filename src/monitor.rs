//! # PVRX Continuous Monitoring & ReactiveEngine
//!
//! Condition-based monitors that observe event streams and fire alerts.
//! The `ReactiveEngine` (T3, ν-dominant) is the capstone type that ties
//! stream, window, pubsub, backpressure, and monitoring together.
//!
//! ## Primitives
//! - ν (Frequency) — DOMINANT: continuous observation
//! - σ (Sequence) — ordered alert history
//! - ∂ (Boundary) — threshold conditions
//! - ς (State) — monitor state tracking
//! - → (Causality) — condition → alert chains
//! - Σ (Sum) — aggregate conditions

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::backpressure::{BackpressureStrategy, BufferPolicy, FlowController};
use super::pubsub::PubSub;
use super::stream::{EventPayload, EventStream, StreamId, StreamSource};
use super::window::{WindowConfig, WindowEngine};

// ===============================================================
// MONITOR TYPES
// ===============================================================

/// Unique monitor identifier.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonitorId(pub u64);

impl GroundsTo for MonitorId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

/// Condition that triggers a monitor alert.
/// Tier: T2-P (∂ + κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Value exceeds threshold.
    ThresholdAbove(f64),
    /// Value falls below threshold.
    ThresholdBelow(f64),
    /// Rate of change exceeds limit (events/sec delta).
    RateChange(f64),
    /// No events received within duration (seconds).
    Absence(f64),
    /// Count in window exceeds limit.
    CountExceeds(usize),
    /// Custom condition with name and threshold.
    Custom { name: String, threshold: f64 },
}

impl Condition {
    /// Evaluates the condition against a numeric value.
    #[must_use]
    pub fn evaluate(&self, value: f64) -> bool {
        match self {
            Self::ThresholdAbove(t) => value > *t,
            Self::ThresholdBelow(t) => value < *t,
            Self::RateChange(limit) => value.abs() > *limit,
            Self::Absence(secs) => value > *secs,
            Self::CountExceeds(limit) => value as usize > *limit,
            Self::Custom { threshold, .. } => value > *threshold,
        }
    }
}

impl GroundsTo for Condition {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Comparison])
    }
}

/// Alert severity levels.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational.
    Info,
    /// Warning: attention needed.
    Warning,
    /// Critical: immediate action required.
    Critical,
    /// Emergency: system-level concern.
    Emergency,
}

impl GroundsTo for AlertSeverity {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

/// An alert fired by a monitor.
/// Tier: T2-C (σ + ∂ + → + ν)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Monitor that fired.
    pub monitor_id: MonitorId,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Condition that triggered.
    pub condition_desc: String,
    /// Value that triggered the condition.
    pub trigger_value: f64,
    /// When the alert fired.
    pub fired_at: SystemTime,
    /// Monitor name.
    pub monitor_name: String,
}

impl GroundsTo for Alert {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Boundary,
            LexPrimitiva::Causality,
            LexPrimitiva::Frequency,
        ])
        .with_dominant(LexPrimitiva::Causality, 0.75)
    }
}

/// State of a monitor.
/// Tier: T2-P (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonitorState {
    /// Monitor is active and checking conditions.
    Active,
    /// Monitor is paused (not checking).
    Paused,
    /// Monitor has been triggered and is in cooldown.
    Cooldown,
    /// Monitor is disabled.
    Disabled,
}

impl GroundsTo for MonitorState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State])
    }
}

// ===============================================================
// MONITOR
// ===============================================================

/// A continuous monitor observing a stream for conditions.
/// Tier: T2-C (ν + ∂ + ς + →)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitor {
    /// Monitor identity.
    pub id: MonitorId,
    /// Human-readable name.
    pub name: String,
    /// Condition to check.
    pub condition: Condition,
    /// Alert severity when triggered.
    pub severity: AlertSeverity,
    /// Current state.
    pub state: MonitorState,
    /// Stream to observe.
    pub stream_id: StreamId,
    /// Total times triggered.
    pub trigger_count: u64,
    /// Last trigger time.
    pub last_triggered: Option<SystemTime>,
}

impl Monitor {
    /// Creates a new active monitor.
    #[must_use]
    pub fn new(
        id: MonitorId,
        name: &str,
        condition: Condition,
        severity: AlertSeverity,
        stream_id: StreamId,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            condition,
            severity,
            state: MonitorState::Active,
            stream_id,
            trigger_count: 0,
            last_triggered: None,
        }
    }

    /// Checks a value against this monitor's condition.
    /// Returns an Alert if triggered.
    pub fn check(&mut self, value: f64) -> Option<Alert> {
        if self.state != MonitorState::Active {
            return None;
        }

        if self.condition.evaluate(value) {
            self.trigger_count += 1;
            let now = SystemTime::now();
            self.last_triggered = Some(now);

            Some(Alert {
                monitor_id: self.id,
                severity: self.severity,
                condition_desc: format!("{:?}", self.condition),
                trigger_value: value,
                fired_at: now,
                monitor_name: self.name.clone(),
            })
        } else {
            None
        }
    }

    /// Pauses the monitor.
    pub fn pause(&mut self) {
        self.state = MonitorState::Paused;
    }

    /// Resumes the monitor.
    pub fn resume(&mut self) {
        self.state = MonitorState::Active;
    }

    /// Disables the monitor.
    pub fn disable(&mut self) {
        self.state = MonitorState::Disabled;
    }
}

impl GroundsTo for Monitor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Boundary,
            LexPrimitiva::State,
            LexPrimitiva::Causality,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.80)
    }
}

// ===============================================================
// REACTIVE ENGINE — T3 CAPSTONE
// ===============================================================

/// The PVRX Reactive Engine.
///
/// T3 capstone type integrating streams, windows, pub/sub,
/// backpressure, and continuous monitoring into a unified
/// reactive processing platform.
///
/// **Dominant primitive: ν (Frequency)** — continuous observation
/// and frequency-based event processing define this layer.
///
/// Grounding: ν + σ + ∂ + ς + → + Σ (6 T1 primitives)
///
/// Tier: T3 Domain-Specific
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveEngine {
    /// Event streams.
    streams: Vec<EventStream>,
    /// Window engines (one per stream that needs windowing).
    windows: Vec<(StreamId, WindowEngine)>,
    /// Pub/sub event router.
    pubsub: PubSub,
    /// Flow controller.
    flow: FlowController,
    /// Active monitors.
    monitors: Vec<Monitor>,
    /// Alert history.
    alerts: Vec<Alert>,
    /// Next monitor ID.
    next_monitor_id: u64,
    /// Next stream ID.
    next_stream_id: u64,
    /// Total events processed.
    total_events: u64,
}

impl ReactiveEngine {
    /// Creates a new reactive engine with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            windows: Vec::new(),
            pubsub: PubSub::new(),
            flow: FlowController::new(
                BackpressureStrategy::DropOldest,
                BufferPolicy::default_policy(),
            ),
            monitors: Vec::new(),
            alerts: Vec::new(),
            next_monitor_id: 1,
            next_stream_id: 1,
            total_events: 0,
        }
    }

    /// Creates a reactive engine with custom flow control.
    #[must_use]
    pub fn with_flow(strategy: BackpressureStrategy, policy: BufferPolicy) -> Self {
        Self {
            flow: FlowController::new(strategy, policy),
            ..Self::new()
        }
    }

    /// Creates a new event stream.
    pub fn create_stream(&mut self, source: StreamSource, capacity: usize) -> StreamId {
        let id = StreamId(self.next_stream_id);
        self.next_stream_id += 1;
        self.streams.push(EventStream::new(id, source, capacity));
        id
    }

    /// Attaches a window engine to a stream.
    pub fn attach_window(&mut self, stream_id: StreamId, config: WindowConfig) {
        self.windows.push((stream_id, WindowEngine::new(config)));
    }

    /// Adds a monitor.
    pub fn add_monitor(
        &mut self,
        name: &str,
        condition: Condition,
        severity: AlertSeverity,
        stream_id: StreamId,
    ) -> MonitorId {
        let id = MonitorId(self.next_monitor_id);
        self.next_monitor_id += 1;
        self.monitors
            .push(Monitor::new(id, name, condition, severity, stream_id));
        id
    }

    /// Ingests an event into a stream, running it through monitors and windows.
    /// Returns any alerts triggered.
    pub fn ingest(&mut self, stream_id: StreamId, payload: EventPayload) -> Vec<Alert> {
        let now = SystemTime::now();

        // Backpressure check
        let admit = self.flow.admit(now);
        if admit != super::backpressure::AdmitResult::Accepted {
            return Vec::new();
        }

        self.total_events += 1;
        let mut triggered_alerts = Vec::new();

        // Extract numeric value for monitoring
        let value = payload.value().unwrap_or(0.0);

        // Push to stream
        if let Some(stream) = self.streams.iter_mut().find(|s| s.id() == stream_id) {
            stream.push(payload);
        }

        // Run through window engines
        for (wid, engine) in &mut self.windows {
            if *wid == stream_id {
                engine.process_value(value, now);
            }
        }

        // Check monitors
        for monitor in &mut self.monitors {
            if monitor.stream_id == stream_id {
                if let Some(alert) = monitor.check(value) {
                    triggered_alerts.push(alert);
                }
            }
        }

        self.alerts.extend(triggered_alerts.clone());
        triggered_alerts
    }

    /// Returns the pub/sub engine.
    #[must_use]
    pub fn pubsub(&self) -> &PubSub {
        &self.pubsub
    }

    /// Returns mutable pub/sub engine.
    pub fn pubsub_mut(&mut self) -> &mut PubSub {
        &mut self.pubsub
    }

    /// Returns all alerts.
    #[must_use]
    pub fn alerts(&self) -> &[Alert] {
        &self.alerts
    }

    /// Returns alerts for a specific monitor.
    #[must_use]
    pub fn alerts_for(&self, monitor_id: MonitorId) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| a.monitor_id == monitor_id)
            .collect()
    }

    /// Returns all monitors.
    #[must_use]
    pub fn monitors(&self) -> &[Monitor] {
        &self.monitors
    }

    /// Total events processed.
    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.total_events
    }

    /// Number of active streams.
    #[must_use]
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Number of active monitors.
    #[must_use]
    pub fn monitor_count(&self) -> usize {
        self.monitors.len()
    }

    /// Flow controller reference.
    #[must_use]
    pub fn flow(&self) -> &FlowController {
        &self.flow
    }
}

impl Default for ReactiveEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for ReactiveEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency, // ν - DOMINANT: continuous reactive observation
            LexPrimitiva::Sequence,  // σ - ordered event streams
            LexPrimitiva::Boundary,  // ∂ - threshold conditions
            LexPrimitiva::State,     // ς - monitor state tracking
            LexPrimitiva::Causality, // → - condition → alert chains
            LexPrimitiva::Sum,       // Σ - window aggregations
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::EventPayload;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_condition_threshold_above() {
        let cond = Condition::ThresholdAbove(3.0);
        assert!(cond.evaluate(4.0));
        assert!(!cond.evaluate(2.0));
        assert!(!cond.evaluate(3.0)); // Not strictly above
    }

    #[test]
    fn test_condition_threshold_below() {
        let cond = Condition::ThresholdBelow(2.0);
        assert!(cond.evaluate(1.0));
        assert!(!cond.evaluate(3.0));
    }

    #[test]
    fn test_condition_count_exceeds() {
        let cond = Condition::CountExceeds(10);
        assert!(cond.evaluate(11.0));
        assert!(!cond.evaluate(10.0));
    }

    #[test]
    fn test_monitor_check() {
        let mut monitor = Monitor::new(
            MonitorId(1),
            "prr_spike",
            Condition::ThresholdAbove(3.0),
            AlertSeverity::Critical,
            StreamId(1),
        );

        // Below threshold: no alert
        let alert = monitor.check(2.0);
        assert!(alert.is_none());

        // Above threshold: alert fires
        let alert = monitor.check(5.0);
        assert!(alert.is_some());
        if let Some(a) = alert {
            assert_eq!(a.severity, AlertSeverity::Critical);
            assert!((a.trigger_value - 5.0).abs() < f64::EPSILON);
        }

        assert_eq!(monitor.trigger_count, 1);
    }

    #[test]
    fn test_monitor_pause_resume() {
        let mut monitor = Monitor::new(
            MonitorId(1),
            "test",
            Condition::ThresholdAbove(1.0),
            AlertSeverity::Warning,
            StreamId(1),
        );

        monitor.pause();
        assert!(monitor.check(100.0).is_none()); // Paused: no alert

        monitor.resume();
        assert!(monitor.check(100.0).is_some()); // Active: fires
    }

    #[test]
    fn test_reactive_engine_ingest_and_alert() {
        let mut engine = ReactiveEngine::new();

        let stream = engine.create_stream(StreamSource::Generator("test".into()), 100);
        engine.add_monitor(
            "high_prr",
            Condition::ThresholdAbove(3.0),
            AlertSeverity::Critical,
            stream,
        );

        // Below threshold: no alerts
        let alerts = engine.ingest(
            stream,
            EventPayload::Metric {
                name: "prr".into(),
                value: 2.0,
            },
        );
        assert!(alerts.is_empty());

        // Above threshold: alert fires
        let alerts = engine.ingest(
            stream,
            EventPayload::Metric {
                name: "prr".into(),
                value: 5.0,
            },
        );
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_reactive_engine_multiple_monitors() {
        let mut engine = ReactiveEngine::new();

        let stream = engine.create_stream(StreamSource::Generator("test".into()), 100);
        engine.add_monitor(
            "high",
            Condition::ThresholdAbove(5.0),
            AlertSeverity::Critical,
            stream,
        );
        engine.add_monitor(
            "very_high",
            Condition::ThresholdAbove(10.0),
            AlertSeverity::Emergency,
            stream,
        );

        let alerts = engine.ingest(
            stream,
            EventPayload::Metric {
                name: "x".into(),
                value: 15.0,
            },
        );

        // Both monitors fire
        assert_eq!(alerts.len(), 2);
    }

    #[test]
    fn test_reactive_engine_windowing() {
        let mut engine = ReactiveEngine::new();

        let stream = engine.create_stream(StreamSource::Generator("test".into()), 100);
        engine.attach_window(stream, WindowConfig::global());

        for i in 0..10 {
            engine.ingest(
                stream,
                EventPayload::Metric {
                    name: format!("m{i}"),
                    value: i as f64,
                },
            );
        }

        assert_eq!(engine.total_events(), 10);
    }

    #[test]
    fn test_reactive_engine_stream_count() {
        let mut engine = ReactiveEngine::new();
        engine.create_stream(StreamSource::Gateway, 100);
        engine.create_stream(StreamSource::Generator("a".into()), 100);
        assert_eq!(engine.stream_count(), 2);
    }

    #[test]
    fn test_reactive_engine_alerts_for() {
        let mut engine = ReactiveEngine::new();
        let stream = engine.create_stream(StreamSource::Generator("test".into()), 100);

        let m1 = engine.add_monitor(
            "a",
            Condition::ThresholdAbove(0.0),
            AlertSeverity::Info,
            stream,
        );
        let m2 = engine.add_monitor(
            "b",
            Condition::ThresholdAbove(100.0),
            AlertSeverity::Warning,
            stream,
        );

        engine.ingest(
            stream,
            EventPayload::Metric {
                name: "x".into(),
                value: 50.0,
            },
        );

        assert_eq!(engine.alerts_for(m1).len(), 1);
        assert_eq!(engine.alerts_for(m2).len(), 0);
    }

    #[test]
    fn test_reactive_engine_t3_grounding() {
        let comp = ReactiveEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.unique().len(), 6);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }

    // ═══════════════════════════════════════════════════════════
    // THE QUARTET TEST
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_pvos_pvwf_pvgw_pvrx_quartet() {
        use crate::Pvos;
        use crate::gateway::Gateway;
        use crate::workflow::Workflow;

        // PVOS: μ (Mapping) dominant
        let pvos = Pvos::primitive_composition();
        assert_eq!(pvos.dominant, Some(LexPrimitiva::Mapping));

        // PVWF: σ (Sequence) dominant
        let pvwf = Workflow::primitive_composition();
        assert_eq!(pvwf.dominant, Some(LexPrimitiva::Sequence));

        // PVGW: ∂ (Boundary) dominant
        let pvgw = Gateway::primitive_composition();
        assert_eq!(pvgw.dominant, Some(LexPrimitiva::Boundary));

        // PVRX: ν (Frequency) dominant
        let pvrx = ReactiveEngine::primitive_composition();
        assert_eq!(pvrx.dominant, Some(LexPrimitiva::Frequency));

        // All four are distinct dominant primitives
        let dominants = [pvos.dominant, pvwf.dominant, pvgw.dominant, pvrx.dominant];
        for i in 0..dominants.len() {
            for j in (i + 1)..dominants.len() {
                assert_ne!(
                    dominants[i], dominants[j],
                    "Layers must have distinct dominant primitives"
                );
            }
        }
    }
}
