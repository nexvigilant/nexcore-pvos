//! # PVRX Publish/Subscribe
//!
//! Topic-based event distribution with fanout and load-balancing delivery modes.
//! Decouples event producers from consumers for reactive PV pipelines.
//!
//! ## Primitives
//! - ν (Frequency) — event delivery rate
//! - μ (Mapping) — topic → subscriber routing
//! - σ (Sequence) — ordered delivery
//! - ∂ (Boundary) — topic isolation

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::stream::{Event, EventPayload, StreamId};

// ===============================================================
// TOPIC & SUBSCRIPTION
// ===============================================================

/// Unique topic identifier.
/// Tier: T2-P (μ + ∂)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic(pub String);

impl Topic {
    /// Creates a new topic.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Topic name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl GroundsTo for Topic {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping, LexPrimitiva::Boundary])
    }
}

/// Unique subscriber identifier.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriberId(pub String);

impl SubscriberId {
    /// Creates a new subscriber ID.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl GroundsTo for SubscriberId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

/// Topic filter for subscription matching.
/// Tier: T2-P (μ + ∂)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TopicFilter {
    /// Exact topic match.
    Exact(String),
    /// Prefix match (e.g., "pv.signals.*").
    Prefix(String),
    /// Match all topics.
    All,
}

impl TopicFilter {
    /// Tests whether a topic matches this filter.
    #[must_use]
    pub fn matches(&self, topic: &Topic) -> bool {
        match self {
            Self::Exact(name) => topic.name() == name,
            Self::Prefix(prefix) => topic.name().starts_with(prefix),
            Self::All => true,
        }
    }
}

impl GroundsTo for TopicFilter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping, LexPrimitiva::Boundary])
    }
}

/// How events are delivered to subscribers of a topic.
/// Tier: T2-P (ν)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeliveryMode {
    /// Every subscriber receives every event.
    Fanout,
    /// Events are distributed round-robin across subscribers.
    LoadBalance,
}

impl GroundsTo for DeliveryMode {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Frequency])
    }
}

// ===============================================================
// SUBSCRIBER & SUBSCRIPTION
// ===============================================================

/// A subscription binding a subscriber to a topic filter.
/// Tier: T2-C (μ + ν + ∂ + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Subscriber identity.
    pub subscriber: SubscriberId,
    /// Topic filter.
    pub filter: TopicFilter,
    /// Delivery mode.
    pub delivery_mode: DeliveryMode,
    /// Events delivered to this subscriber.
    pub delivered: u64,
}

impl Subscription {
    /// Creates a new subscription.
    #[must_use]
    pub fn new(subscriber: SubscriberId, filter: TopicFilter, delivery_mode: DeliveryMode) -> Self {
        Self {
            subscriber,
            filter,
            delivery_mode,
            delivered: 0,
        }
    }
}

impl GroundsTo for Subscription {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Mapping,
            LexPrimitiva::Frequency,
            LexPrimitiva::Boundary,
            LexPrimitiva::Sequence,
        ])
        .with_dominant(LexPrimitiva::Mapping, 0.75)
    }
}

// ===============================================================
// DELIVERY RECORD
// ===============================================================

/// Record of an event delivery to a subscriber.
/// Tier: T2-P (σ + ν)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    /// Topic the event was published to.
    pub topic: Topic,
    /// Subscriber that received the event.
    pub subscriber: SubscriberId,
    /// Payload that was delivered.
    pub payload: EventPayload,
}

impl GroundsTo for DeliveryRecord {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Frequency])
    }
}

// ===============================================================
// PUB/SUB ENGINE
// ===============================================================

/// Topic-based pub/sub event router.
/// Tier: T2-C (ν + μ + σ + ∂)
///
/// Publishers send events to topics.
/// Subscribers receive events matching their filters.
/// Delivery modes control fan-out vs load-balancing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PubSub {
    /// Registered subscriptions.
    subscriptions: Vec<Subscription>,
    /// Delivery log for replay/audit.
    deliveries: Vec<DeliveryRecord>,
    /// Round-robin counters per topic (for LoadBalance mode).
    round_robin: HashMap<String, usize>,
    /// Total events published.
    total_published: u64,
    /// Total deliveries made.
    total_delivered: u64,
}

impl PubSub {
    /// Creates a new pub/sub engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes to topics matching a filter.
    pub fn subscribe(&mut self, subscriber: SubscriberId, filter: TopicFilter, mode: DeliveryMode) {
        self.subscriptions
            .push(Subscription::new(subscriber, filter, mode));
    }

    /// Unsubscribes a subscriber from all topics.
    pub fn unsubscribe(&mut self, subscriber: &SubscriberId) {
        self.subscriptions.retain(|s| &s.subscriber != subscriber);
    }

    /// Publishes an event to a topic.
    /// Returns the list of delivery records for this publish.
    pub fn publish(&mut self, topic: &Topic, payload: EventPayload) -> Vec<DeliveryRecord> {
        self.total_published += 1;
        let mut records = Vec::new();

        // Find matching subscriptions
        let matching_indices: Vec<usize> = self
            .subscriptions
            .iter()
            .enumerate()
            .filter(|(_, s)| s.filter.matches(topic))
            .map(|(i, _)| i)
            .collect();

        if matching_indices.is_empty() {
            return records;
        }

        // Group by delivery mode
        let fanout_indices: Vec<usize> = matching_indices
            .iter()
            .filter(|&&i| self.subscriptions[i].delivery_mode == DeliveryMode::Fanout)
            .copied()
            .collect();

        let lb_indices: Vec<usize> = matching_indices
            .iter()
            .filter(|&&i| self.subscriptions[i].delivery_mode == DeliveryMode::LoadBalance)
            .copied()
            .collect();

        // Fanout: deliver to all
        for &idx in &fanout_indices {
            let record = DeliveryRecord {
                topic: topic.clone(),
                subscriber: self.subscriptions[idx].subscriber.clone(),
                payload: payload.clone(),
            };
            records.push(record);
            self.subscriptions[idx].delivered += 1;
            self.total_delivered += 1;
        }

        // LoadBalance: deliver to one (round-robin)
        if !lb_indices.is_empty() {
            let counter = self
                .round_robin
                .entry(topic.name().to_string())
                .or_insert(0);
            let selected = lb_indices[*counter % lb_indices.len()];
            *counter += 1;

            let record = DeliveryRecord {
                topic: topic.clone(),
                subscriber: self.subscriptions[selected].subscriber.clone(),
                payload: payload.clone(),
            };
            records.push(record);
            self.subscriptions[selected].delivered += 1;
            self.total_delivered += 1;
        }

        self.deliveries.extend(records.clone());
        records
    }

    /// Returns total events published.
    #[must_use]
    pub fn total_published(&self) -> u64 {
        self.total_published
    }

    /// Returns total deliveries made.
    #[must_use]
    pub fn total_delivered(&self) -> u64 {
        self.total_delivered
    }

    /// Returns the number of active subscriptions.
    #[must_use]
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Returns all delivery records.
    #[must_use]
    pub fn deliveries(&self) -> &[DeliveryRecord] {
        &self.deliveries
    }

    /// Returns subscriptions for a given subscriber.
    #[must_use]
    pub fn subscriptions_for(&self, subscriber: &SubscriberId) -> Vec<&Subscription> {
        self.subscriptions
            .iter()
            .filter(|s| &s.subscriber == subscriber)
            .collect()
    }
}

impl GroundsTo for PubSub {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Mapping,
            LexPrimitiva::Sequence,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.75)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_topic_filter_exact() {
        let filter = TopicFilter::Exact("pv.signals".into());
        assert!(filter.matches(&Topic::new("pv.signals")));
        assert!(!filter.matches(&Topic::new("pv.cases")));
    }

    #[test]
    fn test_topic_filter_prefix() {
        let filter = TopicFilter::Prefix("pv.".into());
        assert!(filter.matches(&Topic::new("pv.signals")));
        assert!(filter.matches(&Topic::new("pv.cases")));
        assert!(!filter.matches(&Topic::new("admin.metrics")));
    }

    #[test]
    fn test_topic_filter_all() {
        let filter = TopicFilter::All;
        assert!(filter.matches(&Topic::new("anything")));
        assert!(filter.matches(&Topic::new("pv.signals")));
    }

    #[test]
    fn test_fanout_delivery() {
        let mut pubsub = PubSub::new();

        pubsub.subscribe(
            SubscriberId::new("sub_a"),
            TopicFilter::Exact("pv.signals".into()),
            DeliveryMode::Fanout,
        );
        pubsub.subscribe(
            SubscriberId::new("sub_b"),
            TopicFilter::Exact("pv.signals".into()),
            DeliveryMode::Fanout,
        );

        let topic = Topic::new("pv.signals");
        let payload = EventPayload::Metric {
            name: "prr".into(),
            value: 3.5,
        };
        let records = pubsub.publish(&topic, payload);

        // Fanout: both subscribers receive the event
        assert_eq!(records.len(), 2);
        assert_eq!(pubsub.total_delivered(), 2);
    }

    #[test]
    fn test_load_balance_delivery() {
        let mut pubsub = PubSub::new();

        pubsub.subscribe(
            SubscriberId::new("worker_1"),
            TopicFilter::Exact("pv.cases".into()),
            DeliveryMode::LoadBalance,
        );
        pubsub.subscribe(
            SubscriberId::new("worker_2"),
            TopicFilter::Exact("pv.cases".into()),
            DeliveryMode::LoadBalance,
        );

        let topic = Topic::new("pv.cases");

        // First publish goes to worker_1
        let r1 = pubsub.publish(
            &topic,
            EventPayload::Metric {
                name: "a".into(),
                value: 1.0,
            },
        );
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].subscriber.0, "worker_1");

        // Second publish goes to worker_2
        let r2 = pubsub.publish(
            &topic,
            EventPayload::Metric {
                name: "b".into(),
                value: 2.0,
            },
        );
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].subscriber.0, "worker_2");

        // Third wraps back to worker_1
        let r3 = pubsub.publish(
            &topic,
            EventPayload::Metric {
                name: "c".into(),
                value: 3.0,
            },
        );
        assert_eq!(r3.len(), 1);
        assert_eq!(r3[0].subscriber.0, "worker_1");
    }

    #[test]
    fn test_unsubscribe() {
        let mut pubsub = PubSub::new();
        let sub = SubscriberId::new("ephemeral");

        pubsub.subscribe(sub.clone(), TopicFilter::All, DeliveryMode::Fanout);
        assert_eq!(pubsub.subscription_count(), 1);

        pubsub.unsubscribe(&sub);
        assert_eq!(pubsub.subscription_count(), 0);
    }

    #[test]
    fn test_no_matching_subscribers() {
        let mut pubsub = PubSub::new();
        pubsub.subscribe(
            SubscriberId::new("sub"),
            TopicFilter::Exact("other.topic".into()),
            DeliveryMode::Fanout,
        );

        let topic = Topic::new("pv.signals");
        let records = pubsub.publish(
            &topic,
            EventPayload::Metric {
                name: "x".into(),
                value: 1.0,
            },
        );
        assert!(records.is_empty());
        assert_eq!(pubsub.total_published(), 1);
        assert_eq!(pubsub.total_delivered(), 0);
    }

    #[test]
    fn test_subscriptions_for() {
        let mut pubsub = PubSub::new();
        let sub = SubscriberId::new("multi");

        pubsub.subscribe(
            sub.clone(),
            TopicFilter::Exact("a".into()),
            DeliveryMode::Fanout,
        );
        pubsub.subscribe(
            sub.clone(),
            TopicFilter::Exact("b".into()),
            DeliveryMode::Fanout,
        );
        pubsub.subscribe(
            SubscriberId::new("other"),
            TopicFilter::All,
            DeliveryMode::Fanout,
        );

        let subs = pubsub.subscriptions_for(&sub);
        assert_eq!(subs.len(), 2);
    }

    #[test]
    fn test_pubsub_grounding() {
        let comp = PubSub::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }

    #[test]
    fn test_topic_grounding() {
        let comp = Topic::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
