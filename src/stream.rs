//! # PVRX Core Stream Abstractions
//!
//! Ordered, bounded event buffers for reactive processing.
//! Events are timestamped payloads pushed through processing pipelines.
//!
//! ## Primitives
//! - σ (Sequence) — ordered event buffers
//! - ν (Frequency) — event rate tracking

use std::collections::VecDeque;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique stream identifier.
/// Tier: T2-P (σ + ν)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub u64);

impl GroundsTo for StreamId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Frequency])
    }
}

/// Unique event identifier.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub u64);

impl GroundsTo for EventId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

/// Computed event rate (events per second).
/// Tier: T2-P (ν + N)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rate(pub f64);

impl GroundsTo for Rate {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Frequency, LexPrimitiva::Quantity])
    }
}

// ═══════════════════════════════════════════════════════════
// EVENT TYPES
// ═══════════════════════════════════════════════════════════

/// Where events originate.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamSource {
    /// Database polling.
    Database(String),
    /// Gateway incoming requests.
    Gateway,
    /// File-based import.
    File(String),
    /// Programmatic generator (testing).
    Generator(String),
}

/// Where processed events are routed.
/// Tier: T2-P (→)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamSink {
    /// Trigger a PVWF workflow.
    Workflow(String),
    /// Fire an alert.
    Alert,
    /// Feed into aggregation.
    Aggregate,
    /// Write to audit log.
    Log,
}

/// Event payload — the data carried by each event.
/// Tier: T2-C (σ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPayload {
    /// Signal detection result.
    Signal {
        drug: String,
        event: String,
        statistic: f64,
        detected: bool,
    },
    /// Case report.
    Case {
        case_id: u64,
        serious: bool,
        source: String,
    },
    /// Numeric metric observation.
    Metric { name: String, value: f64 },
    /// Alert notification.
    Alert { severity: String, message: String },
    /// Freeform data.
    Custom { kind: String, data: String },
}

impl EventPayload {
    /// Extracts a numeric value from the payload, if available.
    #[must_use]
    pub fn value(&self) -> Option<f64> {
        match self {
            Self::Signal { statistic, .. } => Some(*statistic),
            Self::Metric { value, .. } => Some(*value),
            Self::Case { case_id, .. } => Some(*case_id as f64),
            _ => None,
        }
    }

    /// Returns true if this is a signal event.
    #[must_use]
    pub fn is_signal(&self) -> bool {
        matches!(self, Self::Signal { .. })
    }

    /// Returns true if this is a serious case.
    #[must_use]
    pub fn is_serious(&self) -> bool {
        matches!(self, Self::Case { serious: true, .. })
    }
}

/// A timestamped event in the stream.
/// Tier: T2-C (σ + ν)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier.
    pub id: EventId,
    /// When the event occurred.
    pub timestamp: SystemTime,
    /// Event payload data.
    pub payload: EventPayload,
    /// Source stream.
    pub source: StreamId,
}

impl Event {
    /// Creates a new event with current timestamp.
    #[must_use]
    pub fn new(id: EventId, payload: EventPayload, source: StreamId) -> Self {
        Self {
            id,
            timestamp: SystemTime::now(),
            payload,
            source,
        }
    }

    /// Creates an event with explicit timestamp (for testing/replay).
    #[must_use]
    pub fn with_timestamp(
        id: EventId,
        payload: EventPayload,
        source: StreamId,
        timestamp: SystemTime,
    ) -> Self {
        Self {
            id,
            timestamp,
            payload,
            source,
        }
    }
}

impl GroundsTo for Event {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Frequency])
            .with_dominant(LexPrimitiva::Sequence, 0.85)
    }
}

// ═══════════════════════════════════════════════════════════
// EVENT STREAM — BOUNDED PUSH BUFFER
// ═══════════════════════════════════════════════════════════

/// Bounded, ordered event buffer.
/// Tier: T2-C (σ + ν + ∂)
///
/// Push-based: events are appended; consumers drain.
/// Bounded: excess events are dropped (oldest first) when capacity reached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStream {
    /// Stream identifier.
    id: StreamId,
    /// Event source.
    source: StreamSource,
    /// Buffered events.
    events: VecDeque<Event>,
    /// Maximum buffer capacity.
    capacity: usize,
    /// Next event ID.
    next_event_id: u64,
    /// Total events received.
    total_received: u64,
    /// Total events dropped (overflow).
    total_dropped: u64,
}

impl EventStream {
    /// Creates a new event stream.
    #[must_use]
    pub fn new(id: StreamId, source: StreamSource, capacity: usize) -> Self {
        Self {
            id,
            source,
            events: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            next_event_id: 1,
            total_received: 0,
            total_dropped: 0,
        }
    }

    /// Pushes an event into the stream.
    /// Returns the assigned EventId, or None if dropped.
    pub fn push(&mut self, payload: EventPayload) -> Option<EventId> {
        self.total_received += 1;

        if self.events.len() >= self.capacity {
            self.events.pop_front(); // Drop oldest
            self.total_dropped += 1;
        }

        let event_id = EventId(self.next_event_id);
        self.next_event_id += 1;

        let event = Event::new(event_id, payload, self.id);
        self.events.push_back(event);
        Some(event_id)
    }

    /// Pushes an event with explicit timestamp.
    pub fn push_with_timestamp(
        &mut self,
        payload: EventPayload,
        timestamp: SystemTime,
    ) -> Option<EventId> {
        self.total_received += 1;

        if self.events.len() >= self.capacity {
            self.events.pop_front();
            self.total_dropped += 1;
        }

        let event_id = EventId(self.next_event_id);
        self.next_event_id += 1;

        let event = Event::with_timestamp(event_id, payload, self.id, timestamp);
        self.events.push_back(event);
        Some(event_id)
    }

    /// Drains all buffered events.
    pub fn drain(&mut self) -> Vec<Event> {
        self.events.drain(..).collect()
    }

    /// Peeks at all buffered events without consuming.
    #[must_use]
    pub fn peek(&self) -> &VecDeque<Event> {
        &self.events
    }

    /// Number of buffered events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Stream identifier.
    #[must_use]
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Total events received (including dropped).
    #[must_use]
    pub fn total_received(&self) -> u64 {
        self.total_received
    }

    /// Total events dropped due to overflow.
    #[must_use]
    pub fn total_dropped(&self) -> u64 {
        self.total_dropped
    }

    /// Approximate event rate (events per second) over recent history.
    #[must_use]
    pub fn rate(&self) -> Rate {
        if self.events.len() < 2 {
            return Rate(0.0);
        }
        let first = &self.events[0];
        let last = &self.events[self.events.len() - 1];

        let duration = last
            .timestamp
            .duration_since(first.timestamp)
            .unwrap_or(std::time::Duration::from_secs(1));

        let secs = duration.as_secs_f64().max(0.001);
        Rate(self.events.len() as f64 / secs)
    }
}

impl GroundsTo for EventStream {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Frequency,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;
    use std::time::Duration;

    #[test]
    fn test_push_and_drain() {
        let mut stream = EventStream::new(StreamId(1), StreamSource::Generator("test".into()), 100);

        stream.push(EventPayload::Metric {
            name: "prr".into(),
            value: 3.5,
        });
        stream.push(EventPayload::Metric {
            name: "ror".into(),
            value: 2.1,
        });

        assert_eq!(stream.len(), 2);
        assert_eq!(stream.total_received(), 2);

        let events = stream.drain();
        assert_eq!(events.len(), 2);
        assert!(stream.is_empty());
    }

    #[test]
    fn test_bounded_overflow() {
        let mut stream = EventStream::new(StreamId(1), StreamSource::Generator("test".into()), 3);

        for i in 0..5 {
            stream.push(EventPayload::Metric {
                name: format!("m{i}"),
                value: i as f64,
            });
        }

        assert_eq!(stream.len(), 3);
        assert_eq!(stream.total_received(), 5);
        assert_eq!(stream.total_dropped(), 2);
    }

    #[test]
    fn test_event_payload_value() {
        let signal = EventPayload::Signal {
            drug: "x".into(),
            event: "y".into(),
            statistic: 4.2,
            detected: true,
        };
        assert_eq!(signal.value(), Some(4.2));

        let metric = EventPayload::Metric {
            name: "prr".into(),
            value: 3.0,
        };
        assert_eq!(metric.value(), Some(3.0));

        let custom = EventPayload::Custom {
            kind: "x".into(),
            data: "y".into(),
        };
        assert!(custom.value().is_none());
    }

    #[test]
    fn test_event_payload_predicates() {
        let signal = EventPayload::Signal {
            drug: "x".into(),
            event: "y".into(),
            statistic: 1.0,
            detected: true,
        };
        assert!(signal.is_signal());
        assert!(!signal.is_serious());

        let case = EventPayload::Case {
            case_id: 1,
            serious: true,
            source: "faers".into(),
        };
        assert!(case.is_serious());
        assert!(!case.is_signal());
    }

    #[test]
    fn test_event_with_timestamp() {
        let ts = SystemTime::now();
        let event = Event::with_timestamp(
            EventId(1),
            EventPayload::Metric {
                name: "test".into(),
                value: 1.0,
            },
            StreamId(1),
            ts,
        );
        assert_eq!(event.timestamp, ts);
    }

    #[test]
    fn test_stream_rate() {
        let mut stream = EventStream::new(StreamId(1), StreamSource::Generator("test".into()), 100);

        let base = SystemTime::now();
        for i in 0..10 {
            let ts = base + Duration::from_millis(i * 100); // 100ms apart
            stream.push_with_timestamp(
                EventPayload::Metric {
                    name: "m".into(),
                    value: i as f64,
                },
                ts,
            );
        }

        let rate = stream.rate();
        // 10 events over ~900ms ≈ 11.1 events/sec
        assert!(rate.0 > 5.0);
    }

    #[test]
    fn test_stream_peek() {
        let mut stream = EventStream::new(StreamId(1), StreamSource::Gateway, 100);
        stream.push(EventPayload::Metric {
            name: "a".into(),
            value: 1.0,
        });

        assert_eq!(stream.peek().len(), 1);
        assert_eq!(stream.len(), 1); // Peek doesn't consume
    }

    #[test]
    fn test_stream_empty_rate() {
        let stream = EventStream::new(StreamId(1), StreamSource::Gateway, 100);
        assert!((stream.rate().0 - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_event_stream_grounding() {
        let comp = EventStream::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }

    #[test]
    fn test_event_grounding() {
        let comp = Event::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
