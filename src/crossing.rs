//! # PVGW Crossing Audit
//!
//! Append-only log of every boundary crossing event.
//! Every request/response through the gateway is recorded for
//! regulatory compliance (GxP, GDPR, HIPAA, 21 CFR Part 11).
//!
//! ## Primitive: π (Persistence)
//!
//! The crossing log is write-once, read-many. Once recorded,
//! an event cannot be modified — only queried.
//!
//! ## Compliance Integration
//!
//! Each crossing event carries compliance tags indicating which
//! regulations apply, enabling automated audit trail generation.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

/// Level of detail captured in the crossing audit.
/// Tier: T2-P (π)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuditLevel {
    /// Record identity and path only.
    Minimal,
    /// Record identity, path, method, status.
    Standard,
    /// Record everything including request/response bodies.
    Full,
    /// Regulatory-grade: full + compliance tags + digital signature.
    Regulatory,
}

impl GroundsTo for AuditLevel {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Persistence])
    }
}

/// Regulatory compliance tag attached to crossing events.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceTag {
    /// EU General Data Protection Regulation.
    Gdpr,
    /// US Health Insurance Portability and Accountability Act.
    Hipaa,
    /// Good Practice guidelines (GxP family).
    Gxp,
    /// FDA 21 CFR Part 11 (electronic records/signatures).
    Cfr11,
    /// ISO 27001 Information Security.
    Iso27001,
}

impl GroundsTo for ComplianceTag {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

/// Outcome of a boundary crossing attempt.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossingOutcome {
    /// Request allowed and completed.
    Allowed,
    /// Request denied (auth failure).
    Denied,
    /// Request rate-limited.
    RateLimited,
    /// Request failed during processing.
    Failed,
}

impl GroundsTo for CrossingOutcome {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

/// A single boundary crossing event.
/// Tier: T2-C (π + ∂ + σ)
///
/// Immutable once created. The crossing log is append-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossingEvent {
    /// Sequential event ID.
    pub id: u64,
    /// When the crossing occurred.
    pub timestamp: SystemTime,
    /// Identity that attempted the crossing.
    pub identity: String,
    /// Path requested.
    pub path: String,
    /// Method used.
    pub method: String,
    /// Outcome of the crossing.
    pub outcome: CrossingOutcome,
    /// Audit level applied.
    pub level: AuditLevel,
    /// Compliance tags.
    pub tags: Vec<ComplianceTag>,
    /// Request body (only at Full/Regulatory level).
    pub request_body: Option<String>,
    /// Response summary (only at Full/Regulatory level).
    pub response_summary: Option<String>,
    /// FNV-1a integrity hash of event fields.
    pub integrity: u64,
}

impl GroundsTo for CrossingEvent {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence,
            LexPrimitiva::Boundary,
            LexPrimitiva::Sequence,
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.85)
    }
}

/// Append-only crossing audit log.
/// Tier: T2-C (π + σ)
///
/// All boundary crossings are recorded here.
/// Events cannot be modified or deleted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrossingLog {
    /// Recorded events (append-only).
    events: Vec<CrossingEvent>,
    /// Next event ID.
    next_id: u64,
    /// Default audit level.
    default_level: Option<AuditLevel>,
}

impl CrossingLog {
    /// Creates a new crossing log.
    #[must_use]
    pub fn new(default_level: AuditLevel) -> Self {
        Self {
            events: Vec::new(),
            next_id: 1,
            default_level: Some(default_level),
        }
    }

    /// Records a boundary crossing event.
    pub fn record(
        &mut self,
        identity: &str,
        path: &str,
        method: &str,
        outcome: CrossingOutcome,
        level: Option<AuditLevel>,
        tags: Vec<ComplianceTag>,
        request_body: Option<&str>,
        response_summary: Option<&str>,
    ) -> u64 {
        let effective_level = level.or(self.default_level).unwrap_or(AuditLevel::Standard);

        // Only capture bodies at Full/Regulatory level
        let body = if effective_level >= AuditLevel::Full {
            request_body.map(String::from)
        } else {
            None
        };

        let response = if effective_level >= AuditLevel::Full {
            response_summary.map(String::from)
        } else {
            None
        };

        let id = self.next_id;
        self.next_id += 1;

        // Compute integrity hash
        let integrity = fnv1a_crossing(id, identity, path, method);

        let event = CrossingEvent {
            id,
            timestamp: SystemTime::now(),
            identity: identity.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            outcome,
            level: effective_level,
            tags,
            request_body: body,
            response_summary: response,
            integrity,
        };

        self.events.push(event);
        id
    }

    /// Verifies event integrity.
    #[must_use]
    pub fn verify(&self, event_id: u64) -> bool {
        self.events
            .iter()
            .find(|e| e.id == event_id)
            .map(|e| {
                let expected = fnv1a_crossing(e.id, &e.identity, &e.path, &e.method);
                e.integrity == expected
            })
            .unwrap_or(false)
    }

    /// Returns all events for a given identity.
    #[must_use]
    pub fn events_for(&self, identity: &str) -> Vec<&CrossingEvent> {
        self.events
            .iter()
            .filter(|e| e.identity == identity)
            .collect()
    }

    /// Returns events matching a compliance tag.
    #[must_use]
    pub fn events_with_tag(&self, tag: ComplianceTag) -> Vec<&CrossingEvent> {
        self.events
            .iter()
            .filter(|e| e.tags.contains(&tag))
            .collect()
    }

    /// Returns events with a specific outcome.
    #[must_use]
    pub fn events_by_outcome(&self, outcome: CrossingOutcome) -> Vec<&CrossingEvent> {
        self.events
            .iter()
            .filter(|e| e.outcome == outcome)
            .collect()
    }

    /// Total events recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// All events (read-only).
    #[must_use]
    pub fn events(&self) -> &[CrossingEvent] {
        &self.events
    }
}

impl GroundsTo for CrossingLog {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Persistence, LexPrimitiva::Sequence])
            .with_dominant(LexPrimitiva::Persistence, 0.90)
    }
}

/// FNV-1a hash for crossing event integrity.
fn fnv1a_crossing(id: u64, identity: &str, path: &str, method: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = OFFSET;
    for byte in id
        .to_le_bytes()
        .iter()
        .chain(identity.as_bytes())
        .chain(b"|")
        .chain(path.as_bytes())
        .chain(b"|")
        .chain(method.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_record_and_verify() {
        let mut log = CrossingLog::new(AuditLevel::Standard);

        let id = log.record(
            "admin",
            "/api/v1/signals",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![ComplianceTag::Gxp],
            None,
            None,
        );

        assert_eq!(id, 1);
        assert_eq!(log.len(), 1);
        assert!(log.verify(id));
    }

    #[test]
    fn test_audit_level_body_capture() {
        let mut log = CrossingLog::new(AuditLevel::Full);

        log.record(
            "user",
            "/api/v1/cases",
            "POST",
            CrossingOutcome::Allowed,
            None,
            vec![],
            Some("{\"drug\": \"aspirin\"}"),
            Some("201 Created"),
        );

        let events = log.events();
        assert_eq!(events.len(), 1);
        assert!(events[0].request_body.is_some());
        assert!(events[0].response_summary.is_some());
    }

    #[test]
    fn test_minimal_level_no_body() {
        let mut log = CrossingLog::new(AuditLevel::Minimal);

        log.record(
            "user",
            "/path",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![],
            Some("should not be captured"),
            Some("should not be captured"),
        );

        let events = log.events();
        assert!(events[0].request_body.is_none());
        assert!(events[0].response_summary.is_none());
    }

    #[test]
    fn test_events_for_identity() {
        let mut log = CrossingLog::new(AuditLevel::Standard);

        log.record(
            "alice",
            "/a",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![],
            None,
            None,
        );
        log.record(
            "bob",
            "/b",
            "POST",
            CrossingOutcome::Denied,
            None,
            vec![],
            None,
            None,
        );
        log.record(
            "alice",
            "/c",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![],
            None,
            None,
        );

        let alice_events = log.events_for("alice");
        assert_eq!(alice_events.len(), 2);
    }

    #[test]
    fn test_events_by_compliance_tag() {
        let mut log = CrossingLog::new(AuditLevel::Standard);

        log.record(
            "u",
            "/a",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![ComplianceTag::Gdpr],
            None,
            None,
        );
        log.record(
            "u",
            "/b",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![ComplianceTag::Hipaa],
            None,
            None,
        );
        log.record(
            "u",
            "/c",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![ComplianceTag::Gdpr, ComplianceTag::Hipaa],
            None,
            None,
        );

        assert_eq!(log.events_with_tag(ComplianceTag::Gdpr).len(), 2);
        assert_eq!(log.events_with_tag(ComplianceTag::Hipaa).len(), 2);
        assert_eq!(log.events_with_tag(ComplianceTag::Cfr11).len(), 0);
    }

    #[test]
    fn test_events_by_outcome() {
        let mut log = CrossingLog::new(AuditLevel::Standard);

        log.record(
            "u",
            "/a",
            "GET",
            CrossingOutcome::Allowed,
            None,
            vec![],
            None,
            None,
        );
        log.record(
            "u",
            "/b",
            "POST",
            CrossingOutcome::Denied,
            None,
            vec![],
            None,
            None,
        );
        log.record(
            "u",
            "/c",
            "GET",
            CrossingOutcome::RateLimited,
            None,
            vec![],
            None,
            None,
        );

        assert_eq!(log.events_by_outcome(CrossingOutcome::Allowed).len(), 1);
        assert_eq!(log.events_by_outcome(CrossingOutcome::Denied).len(), 1);
        assert_eq!(log.events_by_outcome(CrossingOutcome::RateLimited).len(), 1);
    }

    #[test]
    fn test_crossing_log_grounding() {
        let comp = CrossingLog::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }

    #[test]
    fn test_crossing_event_grounding() {
        let comp = CrossingEvent::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }
}
