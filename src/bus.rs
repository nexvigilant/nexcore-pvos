//! # PVOC Event Bus
//!
//! Central pub/sub for orchestration events across PVOS layers.
//! Unlike PVRX's PubSub (data stream routing), the EventBus
//! routes causal orchestration events with backpressure and replay.
//!
//! ## Primitives
//! - → (Causality) — bus routes causal signals between layers
//! - μ (Mapping) — event-kind → subscribers routing
//! - σ (Sequence) — ordered event history for replay
//! - ∂ (Boundary) — backpressure, capacity limits

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::event::{EventKind, EventSource, OrcEvent, OrcEventId};
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// T2-P NEWTYPES
// ═══════════════════════════════════════════════════════════

/// Unique subscription identifier.
/// Tier: T2-P (→)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BusSubscriptionId(pub u64);

impl GroundsTo for BusSubscriptionId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Causality])
    }
}

// ═══════════════════════════════════════════════════════════
// SUBSCRIPTION FILTER
// ═══════════════════════════════════════════════════════════

/// What events a subscriber is interested in.
/// Tier: T2-P (→ + κ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubscriptionFilter {
    /// Subscribe to a specific event kind
    ByKind(EventKind),
    /// Subscribe to events from a specific source
    BySource(EventSource),
    /// Subscribe to a kind from a specific source
    ByKindAndSource {
        kind: EventKind,
        source: EventSource,
    },
    /// Subscribe to all events
    All,
}

impl SubscriptionFilter {
    /// Checks if an event matches this filter.
    #[must_use]
    pub fn matches(&self, event: &OrcEvent) -> bool {
        match self {
            Self::ByKind(kind) => &event.kind == kind,
            Self::BySource(source) => event.source() == source,
            Self::ByKindAndSource { kind, source } => {
                &event.kind == kind && event.source() == source
            }
            Self::All => true,
        }
    }
}

impl GroundsTo for SubscriptionFilter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,  // → — filtering causal signals
            LexPrimitiva::Comparison, // κ — matching logic
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// SUBSCRIPTION
// ═══════════════════════════════════════════════════════════

/// A registered interest in receiving orchestration events.
/// Tier: T2-C (→ + μ + κ)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusSubscription {
    /// Subscription ID
    pub id: BusSubscriptionId,
    /// Human-readable name
    pub name: String,
    /// Event filter
    pub filter: SubscriptionFilter,
    /// Whether this subscription is active
    pub active: bool,
    /// Total events delivered to this subscription
    pub delivered: u64,
}

impl BusSubscription {
    /// Creates a new subscription.
    #[must_use]
    pub fn new(id: BusSubscriptionId, name: &str, filter: SubscriptionFilter) -> Self {
        Self {
            id,
            name: name.into(),
            filter,
            active: true,
            delivered: 0,
        }
    }

    /// Checks if this subscription matches the event.
    #[must_use]
    pub fn matches(&self, event: &OrcEvent) -> bool {
        self.active && self.filter.matches(event)
    }
}

impl GroundsTo for BusSubscription {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,  // → — subscription is a causal binding
            LexPrimitiva::Mapping,    // μ — filter → handler mapping
            LexPrimitiva::Comparison, // κ — event matching
        ])
    }
}

// ═══════════════════════════════════════════════════════════
// DELIVERY RESULT
// ═══════════════════════════════════════════════════════════

/// Result of delivering an event through the bus.
/// Tier: T2-P (→ + Σ)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryResult {
    /// Event that was published
    pub event_id: OrcEventId,
    /// Subscriptions that matched and received the event
    pub delivered_to: Vec<BusSubscriptionId>,
    /// Whether the event was stored in history for replay
    pub stored: bool,
}

impl DeliveryResult {
    /// Returns the number of subscribers that received the event.
    #[must_use]
    pub fn delivery_count(&self) -> usize {
        self.delivered_to.len()
    }
}

// ═══════════════════════════════════════════════════════════
// BUS BACKPRESSURE
// ═══════════════════════════════════════════════════════════

/// Backpressure strategy when the bus is overwhelmed.
/// Tier: T2-P (∂ + ν)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusBackpressure {
    /// Drop oldest events when capacity reached
    DropOldest,
    /// Drop newest events (reject new publishes)
    DropNewest,
    /// No limit (unbounded, use with caution)
    Unbounded,
}

impl Default for BusBackpressure {
    fn default() -> Self {
        Self::DropOldest
    }
}

// ═══════════════════════════════════════════════════════════
// BUS METRICS
// ═══════════════════════════════════════════════════════════

/// Operational metrics for the event bus.
/// Tier: T2-P (Σ + N)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusMetrics {
    /// Total events published
    pub total_published: u64,
    /// Total events delivered (across all subscriptions)
    pub total_delivered: u64,
    /// Total events dropped due to backpressure
    pub total_dropped: u64,
    /// Total events replayed
    pub total_replayed: u64,
    /// Current history size
    pub history_size: usize,
    /// Active subscription count
    pub active_subscriptions: usize,
}

// ═══════════════════════════════════════════════════════════
// EVENT BUS — THE COMPOSED TYPE
// ═══════════════════════════════════════════════════════════

/// Central event bus for cross-layer orchestration events.
///
/// Routes `OrcEvent`s from publishers to matching subscribers.
/// Maintains event history for replay and audit. Applies
/// backpressure when capacity is reached.
///
/// Tier: T2-C (→ + μ + σ + ∂ + ν)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBus {
    /// Registered subscriptions
    subscriptions: Vec<BusSubscription>,
    /// Event history for replay (bounded)
    history: Vec<OrcEvent>,
    /// Maximum history size
    history_capacity: usize,
    /// Backpressure strategy
    backpressure: BusBackpressure,
    /// Next subscription ID
    next_sub_id: u64,
    /// Metrics
    total_published: u64,
    total_delivered: u64,
    total_dropped: u64,
    total_replayed: u64,
}

impl EventBus {
    /// Creates a new event bus with given history capacity.
    #[must_use]
    pub fn new(history_capacity: usize) -> Self {
        Self {
            subscriptions: Vec::new(),
            history: Vec::with_capacity(history_capacity),
            history_capacity,
            backpressure: BusBackpressure::default(),
            next_sub_id: 1,
            total_published: 0,
            total_delivered: 0,
            total_dropped: 0,
            total_replayed: 0,
        }
    }

    /// Creates a bus with specified backpressure strategy.
    #[must_use]
    pub fn with_backpressure(mut self, strategy: BusBackpressure) -> Self {
        self.backpressure = strategy;
        self
    }

    /// Registers a new subscription. Returns the subscription ID.
    pub fn subscribe(&mut self, name: &str, filter: SubscriptionFilter) -> BusSubscriptionId {
        let id = BusSubscriptionId(self.next_sub_id);
        self.next_sub_id += 1;

        let sub = BusSubscription::new(id, name, filter);
        self.subscriptions.push(sub);
        id
    }

    /// Removes a subscription by ID.
    pub fn unsubscribe(&mut self, id: BusSubscriptionId) {
        self.subscriptions.retain(|s| s.id != id);
    }

    /// Deactivates a subscription without removing it.
    pub fn pause_subscription(&mut self, id: BusSubscriptionId) {
        for sub in &mut self.subscriptions {
            if sub.id == id {
                sub.active = false;
            }
        }
    }

    /// Reactivates a paused subscription.
    pub fn resume_subscription(&mut self, id: BusSubscriptionId) {
        for sub in &mut self.subscriptions {
            if sub.id == id {
                sub.active = true;
            }
        }
    }

    /// Publishes an event to all matching subscribers.
    /// Returns delivery result showing which subscriptions received it.
    pub fn publish(&mut self, event: OrcEvent) -> DeliveryResult {
        self.total_published += 1;

        // Find matching subscriptions
        let mut delivered_to = Vec::new();
        for sub in &mut self.subscriptions {
            if sub.matches(&event) {
                delivered_to.push(sub.id);
                sub.delivered += 1;
                self.total_delivered += 1;
            }
        }

        // Store in history
        let stored = self.store_event(event.clone());

        DeliveryResult {
            event_id: event.id,
            delivered_to,
            stored,
        }
    }

    /// Stores event in history, applying backpressure if needed.
    fn store_event(&mut self, event: OrcEvent) -> bool {
        match self.backpressure {
            BusBackpressure::Unbounded => {
                self.history.push(event);
                true
            }
            BusBackpressure::DropOldest => {
                if self.history.len() >= self.history_capacity {
                    self.history.remove(0);
                    self.total_dropped += 1;
                }
                self.history.push(event);
                true
            }
            BusBackpressure::DropNewest => {
                if self.history.len() >= self.history_capacity {
                    self.total_dropped += 1;
                    false
                } else {
                    self.history.push(event);
                    true
                }
            }
        }
    }

    /// Replays all events matching a filter to a specific subscription.
    /// Returns the count of replayed events.
    pub fn replay(&mut self, filter: &SubscriptionFilter, target: BusSubscriptionId) -> u64 {
        let mut count = 0u64;
        for sub in &mut self.subscriptions {
            if sub.id == target {
                for event in &self.history {
                    if filter.matches(event) {
                        sub.delivered += 1;
                        count += 1;
                    }
                }
                break;
            }
        }
        self.total_replayed += count;
        count
    }

    /// Returns all events in history matching a filter.
    #[must_use]
    pub fn query_history(&self, filter: &SubscriptionFilter) -> Vec<&OrcEvent> {
        self.history.iter().filter(|e| filter.matches(e)).collect()
    }

    /// Returns the last N events from history.
    #[must_use]
    pub fn recent(&self, count: usize) -> &[OrcEvent] {
        let start = self.history.len().saturating_sub(count);
        &self.history[start..]
    }

    /// Returns an event from history by ID.
    #[must_use]
    pub fn get_event(&self, id: OrcEventId) -> Option<&OrcEvent> {
        self.history.iter().find(|e| e.id == id)
    }

    /// Returns current bus metrics.
    #[must_use]
    pub fn metrics(&self) -> BusMetrics {
        BusMetrics {
            total_published: self.total_published,
            total_delivered: self.total_delivered,
            total_dropped: self.total_dropped,
            total_replayed: self.total_replayed,
            history_size: self.history.len(),
            active_subscriptions: self.subscriptions.iter().filter(|s| s.active).count(),
        }
    }

    /// Returns the number of active subscriptions.
    #[must_use]
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns current history size.
    #[must_use]
    pub fn history_size(&self) -> usize {
        self.history.len()
    }

    /// Clears all event history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl GroundsTo for EventBus {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → — routes causal signals
            LexPrimitiva::Mapping,   // μ — event → subscriber routing
            LexPrimitiva::Sequence,  // σ — ordered event history
            LexPrimitiva::Boundary,  // ∂ — backpressure, capacity
            LexPrimitiva::Frequency, // ν — delivery rate tracking
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::super::event::{EventMeta, OrcPayload};
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn make_event(id: u64, kind: EventKind, source: EventSource) -> OrcEvent {
        OrcEvent::system(OrcEventId(id), kind, source, id * 100)
    }

    fn make_signal_event(id: u64) -> OrcEvent {
        OrcEvent::new(
            OrcEventId(id),
            EventKind::SignalDetected,
            EventMeta::new(EventSource::Avc, id * 100),
            OrcPayload::Signal {
                drug: "aspirin".into(),
                event: "headache".into(),
                statistic: 3.5,
                detected: true,
            },
        )
    }

    #[test]
    fn test_subscription_id_grounding() {
        let comp = BusSubscriptionId::primitive_composition();
        assert_eq!(comp.unique().len(), 1);
    }

    #[test]
    fn test_subscription_filter_by_kind() {
        let filter = SubscriptionFilter::ByKind(EventKind::SignalDetected);
        let matching = make_event(1, EventKind::SignalDetected, EventSource::Avc);
        let non_matching = make_event(2, EventKind::WorkflowStarted, EventSource::Pvwf);

        assert!(filter.matches(&matching));
        assert!(!filter.matches(&non_matching));
    }

    #[test]
    fn test_subscription_filter_by_source() {
        let filter = SubscriptionFilter::BySource(EventSource::Pvml);
        let matching = make_event(1, EventKind::ModelRetrained, EventSource::Pvml);
        let non_matching = make_event(2, EventKind::SignalDetected, EventSource::Avc);

        assert!(filter.matches(&matching));
        assert!(!filter.matches(&non_matching));
    }

    #[test]
    fn test_subscription_filter_all() {
        let filter = SubscriptionFilter::All;
        assert!(filter.matches(&make_event(1, EventKind::SystemBooted, EventSource::Pvos)));
        assert!(filter.matches(&make_event(2, EventKind::SignalDetected, EventSource::Avc)));
    }

    #[test]
    fn test_bus_subscribe_and_publish() {
        let mut bus = EventBus::new(100);

        let sub_id = bus.subscribe(
            "signal_watcher",
            SubscriptionFilter::ByKind(EventKind::SignalDetected),
        );
        let event = make_signal_event(1);
        let result = bus.publish(event);

        assert_eq!(result.delivery_count(), 1);
        assert!(result.delivered_to.contains(&sub_id));
        assert!(result.stored);
    }

    #[test]
    fn test_bus_multiple_subscribers() {
        let mut bus = EventBus::new(100);

        let sub1 = bus.subscribe("all_events", SubscriptionFilter::All);
        let sub2 = bus.subscribe(
            "signals_only",
            SubscriptionFilter::ByKind(EventKind::SignalDetected),
        );
        let _sub3 = bus.subscribe(
            "workflows_only",
            SubscriptionFilter::ByKind(EventKind::WorkflowStarted),
        );

        let result = bus.publish(make_signal_event(1));

        // All + signals_only match, workflows_only does not
        assert_eq!(result.delivery_count(), 2);
        assert!(result.delivered_to.contains(&sub1));
        assert!(result.delivered_to.contains(&sub2));
    }

    #[test]
    fn test_bus_unsubscribe() {
        let mut bus = EventBus::new(100);
        let sub_id = bus.subscribe("temp", SubscriptionFilter::All);
        assert_eq!(bus.subscription_count(), 1);

        bus.unsubscribe(sub_id);
        assert_eq!(bus.subscription_count(), 0);

        let result = bus.publish(make_signal_event(1));
        assert_eq!(result.delivery_count(), 0);
    }

    #[test]
    fn test_bus_pause_resume() {
        let mut bus = EventBus::new(100);
        let sub_id = bus.subscribe("pausable", SubscriptionFilter::All);

        bus.pause_subscription(sub_id);
        let result = bus.publish(make_signal_event(1));
        assert_eq!(result.delivery_count(), 0);

        bus.resume_subscription(sub_id);
        let result = bus.publish(make_signal_event(2));
        assert_eq!(result.delivery_count(), 1);
    }

    #[test]
    fn test_bus_history_and_query() {
        let mut bus = EventBus::new(100);
        bus.publish(make_event(1, EventKind::SignalDetected, EventSource::Avc));
        bus.publish(make_event(2, EventKind::WorkflowStarted, EventSource::Pvwf));
        bus.publish(make_event(3, EventKind::SignalDetected, EventSource::Avc));

        assert_eq!(bus.history_size(), 3);

        let signals = bus.query_history(&SubscriptionFilter::ByKind(EventKind::SignalDetected));
        assert_eq!(signals.len(), 2);

        let recent = bus.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, OrcEventId(2));
        assert_eq!(recent[1].id, OrcEventId(3));
    }

    #[test]
    fn test_bus_get_event() {
        let mut bus = EventBus::new(100);
        bus.publish(make_signal_event(42));

        let found = bus.get_event(OrcEventId(42));
        assert!(found.is_some());

        let not_found = bus.get_event(OrcEventId(999));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_bus_backpressure_drop_oldest() {
        let mut bus = EventBus::new(3).with_backpressure(BusBackpressure::DropOldest);

        bus.publish(make_event(1, EventKind::SystemBooted, EventSource::Pvos));
        bus.publish(make_event(2, EventKind::SignalDetected, EventSource::Avc));
        bus.publish(make_event(3, EventKind::WorkflowStarted, EventSource::Pvwf));
        assert_eq!(bus.history_size(), 3);

        // Adding 4th should drop event 1
        bus.publish(make_event(4, EventKind::MetricUpdated, EventSource::Pvmx));
        assert_eq!(bus.history_size(), 3);

        let metrics = bus.metrics();
        assert_eq!(metrics.total_dropped, 1);

        // Event 1 should be gone
        assert!(bus.get_event(OrcEventId(1)).is_none());
        // Event 4 should be present
        assert!(bus.get_event(OrcEventId(4)).is_some());
    }

    #[test]
    fn test_bus_backpressure_drop_newest() {
        let mut bus = EventBus::new(2).with_backpressure(BusBackpressure::DropNewest);

        bus.publish(make_event(1, EventKind::SystemBooted, EventSource::Pvos));
        bus.publish(make_event(2, EventKind::SignalDetected, EventSource::Avc));
        let result = bus.publish(make_event(3, EventKind::WorkflowStarted, EventSource::Pvwf));

        assert_eq!(bus.history_size(), 2);
        assert!(!result.stored); // Event 3 not stored
        assert_eq!(bus.metrics().total_dropped, 1);
    }

    #[test]
    fn test_bus_replay() {
        let mut bus = EventBus::new(100);
        bus.publish(make_event(1, EventKind::SignalDetected, EventSource::Avc));
        bus.publish(make_event(2, EventKind::WorkflowStarted, EventSource::Pvwf));
        bus.publish(make_event(3, EventKind::SignalDetected, EventSource::Avc));

        let sub_id = bus.subscribe(
            "late_joiner",
            SubscriptionFilter::ByKind(EventKind::SignalDetected),
        );
        let replayed = bus.replay(
            &SubscriptionFilter::ByKind(EventKind::SignalDetected),
            sub_id,
        );

        assert_eq!(replayed, 2);
        assert_eq!(bus.metrics().total_replayed, 2);
    }

    #[test]
    fn test_bus_metrics() {
        let mut bus = EventBus::new(100);
        bus.subscribe("all", SubscriptionFilter::All);
        bus.subscribe(
            "signals",
            SubscriptionFilter::ByKind(EventKind::SignalDetected),
        );

        bus.publish(make_signal_event(1));
        bus.publish(make_event(2, EventKind::WorkflowStarted, EventSource::Pvwf));

        let metrics = bus.metrics();
        assert_eq!(metrics.total_published, 2);
        assert_eq!(metrics.total_delivered, 3); // 2 from "all" + 1 from "signals"
        assert_eq!(metrics.history_size, 2);
        assert_eq!(metrics.active_subscriptions, 2);
    }

    #[test]
    fn test_bus_clear_history() {
        let mut bus = EventBus::new(100);
        bus.publish(make_signal_event(1));
        bus.publish(make_signal_event(2));
        assert_eq!(bus.history_size(), 2);

        bus.clear_history();
        assert_eq!(bus.history_size(), 0);
    }

    #[test]
    fn test_event_bus_grounding() {
        let comp = EventBus::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Causality));
        assert_eq!(comp.unique().len(), 5);
    }
}
