//! # PVEX Presence — Entity Presence Tracking
//!
//! Monitors the liveness and availability of registered entities
//! through heartbeat mechanisms, timeout detection, and presence
//! state management.
//!
//! ## T1 Grounding (dominant: ∃ Existence)
//!
//! | Symbol | Role | Weight |
//! |--------|------|--------|
//! | ∃ | Existence | 0.35 — is entity alive? |
//! | ν | Frequency | 0.25 — heartbeat rate |
//! | σ | Sequence | 0.20 — heartbeat history |
//! | ∂ | Boundary | 0.20 — timeout thresholds |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::registry::EntityId;
use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ═══════════════════════════════════════════════════════════
// PRESENCE STATE
// ═══════════════════════════════════════════════════════════

/// Presence status of an entity.
///
/// Tier: T2-P (∃ — present/absent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Presence {
    /// Entity is alive and responding.
    Online,
    /// Entity missed recent heartbeats.
    Degraded,
    /// Entity has exceeded timeout.
    Offline,
    /// Entity presence is unknown (no heartbeats yet).
    Unknown,
}

impl Presence {
    /// Whether the entity is reachable.
    #[must_use]
    pub fn is_reachable(&self) -> bool {
        matches!(self, Self::Online | Self::Degraded)
    }

    /// Whether the entity is healthy.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Online)
    }

    /// Display name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Online => "online",
            Self::Degraded => "degraded",
            Self::Offline => "offline",
            Self::Unknown => "unknown",
        }
    }
}

// ═══════════════════════════════════════════════════════════
// HEARTBEAT
// ═══════════════════════════════════════════════════════════

/// Unique heartbeat identifier.
///
/// Tier: T2-P (∃ newtype)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeartbeatId(pub u64);

/// A single heartbeat from an entity.
///
/// Tier: T2-P (∃ + ν — existence pulse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Heartbeat identifier.
    pub id: HeartbeatId,
    /// Entity that sent the heartbeat.
    pub entity_id: EntityId,
    /// When the heartbeat was received.
    pub received_at: u64,
    /// Optional status message.
    pub status: Option<String>,
    /// Sequence number (monotonically increasing per entity).
    pub sequence: u64,
}

impl Heartbeat {
    /// Creates a new heartbeat.
    #[must_use]
    pub fn new(id: HeartbeatId, entity_id: EntityId, received_at: u64, sequence: u64) -> Self {
        Self {
            id,
            entity_id,
            received_at,
            status: None,
            sequence,
        }
    }

    /// Adds a status message.
    #[must_use]
    pub fn with_status(mut self, status: &str) -> Self {
        self.status = Some(status.to_string());
        self
    }
}

// ═══════════════════════════════════════════════════════════
// HEARTBEAT TIMEOUT CONFIGURATION
// ═══════════════════════════════════════════════════════════

/// Timeout thresholds for heartbeat monitoring.
///
/// Tier: T2-P (∂ — timeout boundaries)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HeartbeatTimeout {
    /// Time before entity is considered degraded.
    pub degraded_after: u64,
    /// Time before entity is considered offline.
    pub offline_after: u64,
    /// Expected heartbeat interval.
    pub expected_interval: u64,
}

impl HeartbeatTimeout {
    /// Creates timeout thresholds.
    #[must_use]
    pub fn new(degraded_after: u64, offline_after: u64, expected_interval: u64) -> Self {
        Self {
            degraded_after,
            offline_after,
            expected_interval,
        }
    }

    /// Determines presence based on time since last heartbeat.
    #[must_use]
    pub fn evaluate(&self, time_since_last: u64) -> Presence {
        if time_since_last > self.offline_after {
            Presence::Offline
        } else if time_since_last > self.degraded_after {
            Presence::Degraded
        } else {
            Presence::Online
        }
    }
}

impl Default for HeartbeatTimeout {
    fn default() -> Self {
        Self {
            degraded_after: 30,
            offline_after: 120,
            expected_interval: 10,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// PRESENCE EVENT
// ═══════════════════════════════════════════════════════════

/// A change in entity presence state.
///
/// Tier: T2-P (∃ + ς — existence state change)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceEvent {
    /// Entity whose presence changed.
    pub entity_id: EntityId,
    /// Previous presence state.
    pub from: Presence,
    /// New presence state.
    pub to: Presence,
    /// When the change was detected.
    pub detected_at: u64,
}

impl PresenceEvent {
    /// Whether the entity went offline.
    #[must_use]
    pub fn went_offline(&self) -> bool {
        self.from != Presence::Offline && self.to == Presence::Offline
    }

    /// Whether the entity came online.
    #[must_use]
    pub fn came_online(&self) -> bool {
        self.from != Presence::Online && self.to == Presence::Online
    }
}

// ═══════════════════════════════════════════════════════════
// ENTITY PRESENCE RECORD
// ═══════════════════════════════════════════════════════════

/// Tracks presence state for a single entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityPresence {
    /// Current presence status.
    status: Presence,
    /// Last heartbeat timestamp.
    last_heartbeat_at: Option<u64>,
    /// Next expected heartbeat sequence.
    next_sequence: u64,
    /// Total heartbeats received.
    total_heartbeats: u64,
    /// Total missed heartbeats.
    total_missed: u64,
}

impl EntityPresence {
    fn new() -> Self {
        Self {
            status: Presence::Unknown,
            last_heartbeat_at: None,
            next_sequence: 0,
            total_heartbeats: 0,
            total_missed: 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// PRESENCE MONITOR
// ═══════════════════════════════════════════════════════════

/// Monitors entity presence through heartbeats.
///
/// Tier: T2-C (∃ + ν + σ + ∂ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceMonitor {
    /// Per-entity presence tracking.
    entities: HashMap<u64, EntityPresence>,
    /// Timeout configuration.
    timeout: HeartbeatTimeout,
    /// Next heartbeat ID.
    next_heartbeat_id: u64,
    /// Presence change events log.
    events: Vec<PresenceEvent>,
    /// Total heartbeats processed.
    total_heartbeats: u64,
}

impl PresenceMonitor {
    /// Creates a new presence monitor.
    #[must_use]
    pub fn new(timeout: HeartbeatTimeout) -> Self {
        Self {
            entities: HashMap::new(),
            timeout,
            next_heartbeat_id: 1,
            events: Vec::new(),
            total_heartbeats: 0,
        }
    }

    /// Creates a monitor with default timeout.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(HeartbeatTimeout::default())
    }

    /// Registers an entity for presence monitoring.
    pub fn track(&mut self, entity_id: EntityId) {
        self.entities
            .entry(entity_id.0)
            .or_insert_with(EntityPresence::new);
    }

    /// Records a heartbeat from an entity.
    pub fn heartbeat(&mut self, entity_id: EntityId, timestamp: u64) -> Heartbeat {
        let id = HeartbeatId(self.next_heartbeat_id);
        self.next_heartbeat_id += 1;
        self.total_heartbeats += 1;

        let ep = self
            .entities
            .entry(entity_id.0)
            .or_insert_with(EntityPresence::new);
        let seq = ep.next_sequence;
        ep.next_sequence += 1;
        ep.total_heartbeats += 1;

        let old_status = ep.status;
        ep.last_heartbeat_at = Some(timestamp);
        ep.status = Presence::Online;

        // Record state change
        if old_status != Presence::Online {
            self.events.push(PresenceEvent {
                entity_id,
                from: old_status,
                to: Presence::Online,
                detected_at: timestamp,
            });
        }

        Heartbeat::new(id, entity_id, timestamp, seq)
    }

    /// Evaluates current presence status of all tracked entities.
    /// Call periodically to detect timeouts.
    pub fn evaluate_all(&mut self, now: u64) -> Vec<PresenceEvent> {
        let mut changes = Vec::new();

        for (&entity_raw_id, ep) in &mut self.entities {
            let old_status = ep.status;
            let new_status = match ep.last_heartbeat_at {
                Some(last) => self.timeout.evaluate(now.saturating_sub(last)),
                None => Presence::Unknown,
            };

            if old_status != new_status {
                ep.status = new_status;
                if new_status == Presence::Offline || new_status == Presence::Degraded {
                    ep.total_missed += 1;
                }
                let event = PresenceEvent {
                    entity_id: EntityId(entity_raw_id),
                    from: old_status,
                    to: new_status,
                    detected_at: now,
                };
                changes.push(event.clone());
                self.events.push(event);
            }
        }

        changes
    }

    /// Gets the current presence of an entity.
    #[must_use]
    pub fn status(&self, entity_id: EntityId) -> Presence {
        self.entities
            .get(&entity_id.0)
            .map(|ep| ep.status)
            .unwrap_or(Presence::Unknown)
    }

    /// Returns all entities with a given presence status.
    #[must_use]
    pub fn entities_with_status(&self, status: Presence) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, ep)| ep.status == status)
            .map(|(&id, _)| EntityId(id))
            .collect()
    }

    /// Returns total heartbeats received.
    #[must_use]
    pub fn total_heartbeats(&self) -> u64 {
        self.total_heartbeats
    }

    /// Returns all presence change events.
    #[must_use]
    pub fn events(&self) -> &[PresenceEvent] {
        &self.events
    }

    /// Number of tracked entities.
    #[must_use]
    pub fn tracked_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns the timeout configuration.
    #[must_use]
    pub fn timeout(&self) -> &HeartbeatTimeout {
        &self.timeout
    }
}

impl Default for PresenceMonitor {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl GroundsTo for PresenceMonitor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Existence, // ∃ — entity alive/dead
            LexPrimitiva::Frequency, // ν — heartbeat rate
            LexPrimitiva::Sequence,  // σ — heartbeat ordering
            LexPrimitiva::Boundary,  // ∂ — timeout thresholds
            LexPrimitiva::Mapping,   // μ — entity→presence
        ])
        .with_dominant(LexPrimitiva::Existence, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_presence_monitor_grounding() {
        let comp = PresenceMonitor::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Existence));
        assert_eq!(comp.unique().len(), 5);
    }

    #[test]
    fn test_presence_states() {
        assert!(Presence::Online.is_healthy());
        assert!(Presence::Online.is_reachable());
        assert!(!Presence::Degraded.is_healthy());
        assert!(Presence::Degraded.is_reachable());
        assert!(!Presence::Offline.is_reachable());
        assert!(!Presence::Unknown.is_reachable());
    }

    #[test]
    fn test_heartbeat_timeout_evaluate() {
        let timeout = HeartbeatTimeout::new(30, 120, 10);
        assert_eq!(timeout.evaluate(0), Presence::Online);
        assert_eq!(timeout.evaluate(15), Presence::Online);
        assert_eq!(timeout.evaluate(31), Presence::Degraded);
        assert_eq!(timeout.evaluate(121), Presence::Offline);
    }

    #[test]
    fn test_monitor_track_and_heartbeat() {
        let mut monitor = PresenceMonitor::with_defaults();
        let eid = EntityId(1);

        monitor.track(eid);
        assert_eq!(monitor.status(eid), Presence::Unknown);

        let hb = monitor.heartbeat(eid, 1000);
        assert_eq!(hb.entity_id, eid);
        assert_eq!(hb.sequence, 0);
        assert_eq!(monitor.status(eid), Presence::Online);
    }

    #[test]
    fn test_monitor_evaluate_timeout() {
        let timeout = HeartbeatTimeout::new(30, 120, 10);
        let mut monitor = PresenceMonitor::new(timeout);
        let eid = EntityId(1);

        monitor.heartbeat(eid, 1000);
        assert_eq!(monitor.status(eid), Presence::Online);

        // Evaluate at t=1020 (20 units since last heartbeat) — still online
        let changes = monitor.evaluate_all(1020);
        assert!(changes.is_empty());
        assert_eq!(monitor.status(eid), Presence::Online);

        // Evaluate at t=1050 (50 units since last heartbeat) — degraded
        let changes = monitor.evaluate_all(1050);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].to, Presence::Degraded);
        assert_eq!(monitor.status(eid), Presence::Degraded);

        // Evaluate at t=1200 (200 units since last heartbeat) — offline
        let changes = monitor.evaluate_all(1200);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].to, Presence::Offline);
    }

    #[test]
    fn test_monitor_recover_from_offline() {
        let timeout = HeartbeatTimeout::new(30, 120, 10);
        let mut monitor = PresenceMonitor::new(timeout);
        let eid = EntityId(1);

        monitor.heartbeat(eid, 1000);
        monitor.evaluate_all(1200); // → offline
        assert_eq!(monitor.status(eid), Presence::Offline);

        // New heartbeat recovers
        monitor.heartbeat(eid, 1300);
        assert_eq!(monitor.status(eid), Presence::Online);
    }

    #[test]
    fn test_monitor_entities_with_status() {
        let mut monitor = PresenceMonitor::with_defaults();
        monitor.heartbeat(EntityId(1), 1000);
        monitor.heartbeat(EntityId(2), 1000);
        monitor.track(EntityId(3));

        let online = monitor.entities_with_status(Presence::Online);
        assert_eq!(online.len(), 2);

        let unknown = monitor.entities_with_status(Presence::Unknown);
        assert_eq!(unknown.len(), 1);
    }

    #[test]
    fn test_presence_event_transitions() {
        let event = PresenceEvent {
            entity_id: EntityId(1),
            from: Presence::Online,
            to: Presence::Offline,
            detected_at: 2000,
        };
        assert!(event.went_offline());
        assert!(!event.came_online());

        let recovery = PresenceEvent {
            entity_id: EntityId(1),
            from: Presence::Offline,
            to: Presence::Online,
            detected_at: 3000,
        };
        assert!(recovery.came_online());
        assert!(!recovery.went_offline());
    }

    #[test]
    fn test_heartbeat_with_status() {
        let hb = Heartbeat::new(HeartbeatId(1), EntityId(1), 1000, 0).with_status("healthy");
        assert_eq!(hb.status.as_deref(), Some("healthy"));
    }

    #[test]
    fn test_monitor_counters() {
        let mut monitor = PresenceMonitor::with_defaults();
        monitor.heartbeat(EntityId(1), 1000);
        monitor.heartbeat(EntityId(2), 2000);
        monitor.heartbeat(EntityId(1), 3000);

        assert_eq!(monitor.total_heartbeats(), 3);
        assert_eq!(monitor.tracked_count(), 2);
    }

    #[test]
    fn test_monitor_events_log() {
        let timeout = HeartbeatTimeout::new(10, 30, 5);
        let mut monitor = PresenceMonitor::new(timeout);
        let eid = EntityId(1);

        monitor.heartbeat(eid, 1000);
        monitor.evaluate_all(1050); // → offline

        // Events: Unknown→Online (heartbeat), Online→Offline (evaluate)
        assert_eq!(monitor.events().len(), 2);
    }
}
