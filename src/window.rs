//! # PVRX Temporal Windowing
//!
//! Time-based event aggregation over sliding, tumbling, session, and global windows.
//! Windows partition the continuous event stream into finite chunks for analysis.
//!
//! ## Primitives
//! - ν (Frequency) — time-based partitioning
//! - σ (Sequence) — ordered event accumulation
//! - Σ (Sum) — aggregate computations within windows
//! - N (Quantity) — count-based triggers

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::stream::{Event, EventPayload};

// ===============================================================
// WINDOW TYPES
// ===============================================================

/// Window strategy defining how events are partitioned.
/// Tier: T2-P (ν + σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowKind {
    /// Fixed-size, non-overlapping time windows.
    Tumbling,
    /// Overlapping windows with configurable slide interval.
    Sliding,
    /// Gap-based: window closes after inactivity timeout.
    Session,
    /// Single unbounded window (all events).
    Global,
}

impl GroundsTo for WindowKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Frequency, LexPrimitiva::Sequence])
    }
}

/// Configuration for a window.
/// Tier: T2-C (ν + σ + N + Σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window strategy.
    pub kind: WindowKind,
    /// Window duration (for Tumbling/Sliding).
    pub duration: Duration,
    /// Slide interval (for Sliding only; ignored otherwise).
    pub slide: Duration,
    /// Session gap timeout (for Session only; ignored otherwise).
    pub gap_timeout: Duration,
    /// Maximum events per window (0 = unlimited).
    pub max_events: usize,
}

impl WindowConfig {
    /// Creates a tumbling window configuration.
    #[must_use]
    pub fn tumbling(duration: Duration) -> Self {
        Self {
            kind: WindowKind::Tumbling,
            duration,
            slide: Duration::ZERO,
            gap_timeout: Duration::ZERO,
            max_events: 0,
        }
    }

    /// Creates a sliding window configuration.
    #[must_use]
    pub fn sliding(duration: Duration, slide: Duration) -> Self {
        Self {
            kind: WindowKind::Sliding,
            duration,
            slide,
            gap_timeout: Duration::ZERO,
            max_events: 0,
        }
    }

    /// Creates a session window configuration.
    #[must_use]
    pub fn session(gap_timeout: Duration) -> Self {
        Self {
            kind: WindowKind::Session,
            duration: Duration::ZERO,
            slide: Duration::ZERO,
            gap_timeout,
            max_events: 0,
        }
    }

    /// Creates a global (unbounded) window configuration.
    #[must_use]
    pub fn global() -> Self {
        Self {
            kind: WindowKind::Global,
            duration: Duration::ZERO,
            slide: Duration::ZERO,
            gap_timeout: Duration::ZERO,
            max_events: 0,
        }
    }

    /// Sets maximum events per window.
    #[must_use]
    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }
}

impl GroundsTo for WindowConfig {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Sequence,
            LexPrimitiva::Quantity,
            LexPrimitiva::Sum,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.80)
    }
}

// ===============================================================
// WINDOW PANE — A SINGLE WINDOW INSTANCE
// ===============================================================

/// A single window pane holding accumulated events.
/// Tier: T2-C (ν + σ + Σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPane {
    /// Pane sequence number.
    pub seq: u64,
    /// When this pane opened.
    pub opened_at: SystemTime,
    /// When this pane closes (None = not yet determined).
    pub closes_at: Option<SystemTime>,
    /// Events accumulated in this pane.
    events: Vec<f64>,
    /// Event count.
    count: usize,
    /// Running sum of numeric values.
    sum: f64,
    /// Minimum value seen.
    min: f64,
    /// Maximum value seen.
    max: f64,
}

impl WindowPane {
    /// Creates a new pane.
    #[must_use]
    pub fn new(seq: u64, opened_at: SystemTime, closes_at: Option<SystemTime>) -> Self {
        Self {
            seq,
            opened_at,
            closes_at,
            events: Vec::new(),
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    /// Adds a numeric value to this pane.
    pub fn add(&mut self, value: f64) {
        self.events.push(value);
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    /// Returns the aggregate result for this pane.
    #[must_use]
    pub fn result(&self) -> WindowResult {
        let mean = if self.count > 0 {
            self.sum / self.count as f64
        } else {
            0.0
        };

        WindowResult {
            pane_seq: self.seq,
            count: self.count,
            sum: self.sum,
            mean,
            min: if self.count > 0 { self.min } else { 0.0 },
            max: if self.count > 0 { self.max } else { 0.0 },
            opened_at: self.opened_at,
            closed_at: self.closes_at,
        }
    }

    /// Number of events in this pane.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether the pane is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Whether this pane should close based on time.
    #[must_use]
    pub fn should_close(&self, now: SystemTime) -> bool {
        match self.closes_at {
            Some(close_time) => now >= close_time,
            None => false,
        }
    }
}

impl GroundsTo for WindowPane {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Sequence,
            LexPrimitiva::Sum,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.75)
    }
}

// ===============================================================
// WINDOW RESULT — AGGREGATE OUTPUT
// ===============================================================

/// Aggregate result from a closed window pane.
/// Tier: T2-P (Σ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowResult {
    /// Pane sequence number.
    pub pane_seq: u64,
    /// Total events in window.
    pub count: usize,
    /// Sum of values.
    pub sum: f64,
    /// Mean of values.
    pub mean: f64,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// When the window opened.
    pub opened_at: SystemTime,
    /// When the window closed (None if still open).
    pub closed_at: Option<SystemTime>,
}

impl GroundsTo for WindowResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Quantity])
    }
}

// ===============================================================
// WINDOW ENGINE
// ===============================================================

/// Engine managing windowed aggregation over event streams.
/// Tier: T2-C (ν + σ + Σ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowEngine {
    /// Configuration.
    config: WindowConfig,
    /// Current active pane.
    current_pane: Option<WindowPane>,
    /// Completed pane results.
    completed: Vec<WindowResult>,
    /// Next pane sequence.
    next_seq: u64,
    /// Last event timestamp (for session gap detection).
    last_event_time: Option<SystemTime>,
    /// Total events processed.
    total_processed: u64,
}

impl WindowEngine {
    /// Creates a new window engine with given configuration.
    #[must_use]
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            current_pane: None,
            completed: Vec::new(),
            next_seq: 1,
            last_event_time: None,
            total_processed: 0,
        }
    }

    /// Processes an event, extracting its numeric value for windowing.
    /// Returns completed pane results if any windows closed.
    pub fn process(&mut self, event: &Event) -> Vec<WindowResult> {
        let value = event.payload.value().unwrap_or(0.0);
        self.process_value(value, event.timestamp)
    }

    /// Processes a raw numeric value at a given timestamp.
    /// Returns completed pane results if any windows closed.
    pub fn process_value(&mut self, value: f64, timestamp: SystemTime) -> Vec<WindowResult> {
        self.total_processed += 1;
        let mut results = Vec::new();

        match self.config.kind {
            WindowKind::Tumbling => {
                self.process_tumbling(value, timestamp, &mut results);
            }
            WindowKind::Sliding => {
                self.process_sliding(value, timestamp, &mut results);
            }
            WindowKind::Session => {
                self.process_session(value, timestamp, &mut results);
            }
            WindowKind::Global => {
                self.process_global(value, timestamp);
            }
        }

        self.last_event_time = Some(timestamp);
        results
    }

    fn process_tumbling(
        &mut self,
        value: f64,
        timestamp: SystemTime,
        results: &mut Vec<WindowResult>,
    ) {
        // Check if current pane should close
        if let Some(ref pane) = self.current_pane {
            if pane.should_close(timestamp) || self.pane_full(pane) {
                let result = pane.result();
                results.push(result.clone());
                self.completed.push(result);
                self.current_pane = None;
            }
        }

        // Open new pane if needed
        if self.current_pane.is_none() {
            let seq = self.next_seq;
            self.next_seq += 1;
            let closes_at = timestamp + self.config.duration;
            self.current_pane = Some(WindowPane::new(seq, timestamp, Some(closes_at)));
        }

        // Add value to current pane
        if let Some(ref mut pane) = self.current_pane {
            pane.add(value);
        }
    }

    fn process_sliding(
        &mut self,
        value: f64,
        timestamp: SystemTime,
        results: &mut Vec<WindowResult>,
    ) {
        // Check if current pane should close and slide
        if let Some(ref pane) = self.current_pane {
            if pane.should_close(timestamp) || self.pane_full(pane) {
                let result = pane.result();
                results.push(result.clone());
                self.completed.push(result);

                // Slide: open new pane offset by slide interval
                let seq = self.next_seq;
                self.next_seq += 1;
                let new_start = timestamp;
                let closes_at = new_start + self.config.duration;
                self.current_pane = Some(WindowPane::new(seq, new_start, Some(closes_at)));
            }
        }

        if self.current_pane.is_none() {
            let seq = self.next_seq;
            self.next_seq += 1;
            let closes_at = timestamp + self.config.duration;
            self.current_pane = Some(WindowPane::new(seq, timestamp, Some(closes_at)));
        }

        if let Some(ref mut pane) = self.current_pane {
            pane.add(value);
        }
    }

    fn process_session(
        &mut self,
        value: f64,
        timestamp: SystemTime,
        results: &mut Vec<WindowResult>,
    ) {
        // Check if session timed out (gap exceeded)
        let session_expired = self.last_event_time.map_or(false, |last| {
            timestamp.duration_since(last).unwrap_or(Duration::ZERO) > self.config.gap_timeout
        });

        if session_expired {
            if let Some(ref pane) = self.current_pane {
                let result = pane.result();
                results.push(result.clone());
                self.completed.push(result);
            }
            self.current_pane = None;
        }

        // Open new session if needed
        if self.current_pane.is_none() {
            let seq = self.next_seq;
            self.next_seq += 1;
            self.current_pane = Some(WindowPane::new(seq, timestamp, None));
        }

        if let Some(ref mut pane) = self.current_pane {
            pane.add(value);
        }
    }

    fn process_global(&mut self, value: f64, timestamp: SystemTime) {
        if self.current_pane.is_none() {
            let seq = self.next_seq;
            self.next_seq += 1;
            self.current_pane = Some(WindowPane::new(seq, timestamp, None));
        }

        if let Some(ref mut pane) = self.current_pane {
            pane.add(value);
        }
    }

    fn pane_full(&self, pane: &WindowPane) -> bool {
        self.config.max_events > 0 && pane.len() >= self.config.max_events
    }

    /// Forces the current pane to close and returns its result.
    pub fn flush(&mut self) -> Option<WindowResult> {
        self.current_pane.take().map(|pane| {
            let result = pane.result();
            self.completed.push(result.clone());
            result
        })
    }

    /// Returns all completed results.
    #[must_use]
    pub fn completed(&self) -> &[WindowResult] {
        &self.completed
    }

    /// Returns the current (open) pane, if any.
    #[must_use]
    pub fn current_pane(&self) -> Option<&WindowPane> {
        self.current_pane.as_ref()
    }

    /// Total events processed by this engine.
    #[must_use]
    pub fn total_processed(&self) -> u64 {
        self.total_processed
    }

    /// Number of completed windows.
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }
}

impl GroundsTo for WindowEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Sequence,
            LexPrimitiva::Sum,
            LexPrimitiva::Quantity,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{EventId, StreamId};
    use nexcore_lex_primitiva::GroundingTier;

    fn make_event(id: u64, value: f64, timestamp: SystemTime) -> Event {
        Event::with_timestamp(
            EventId(id),
            EventPayload::Metric {
                name: "test".into(),
                value,
            },
            StreamId(1),
            timestamp,
        )
    }

    #[test]
    fn test_tumbling_window() {
        let config = WindowConfig::tumbling(Duration::from_secs(10));
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();

        // First window: events at 0s, 3s, 7s
        let results = engine.process_value(1.0, base);
        assert!(results.is_empty());
        engine.process_value(2.0, base + Duration::from_secs(3));
        engine.process_value(3.0, base + Duration::from_secs(7));

        // Event at 11s should close first window
        let results = engine.process_value(4.0, base + Duration::from_secs(11));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 3);
        assert!((results[0].sum - 6.0).abs() < f64::EPSILON);
        assert!((results[0].mean - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sliding_window() {
        let config = WindowConfig::sliding(Duration::from_secs(5), Duration::from_secs(2));
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();

        engine.process_value(1.0, base);
        engine.process_value(2.0, base + Duration::from_secs(2));
        engine.process_value(3.0, base + Duration::from_secs(4));

        // At 6s, the 5s window should close
        let results = engine.process_value(4.0, base + Duration::from_secs(6));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 3);
    }

    #[test]
    fn test_session_window() {
        let config = WindowConfig::session(Duration::from_secs(5));
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();

        // Session 1: events close together
        engine.process_value(1.0, base);
        engine.process_value(2.0, base + Duration::from_secs(2));

        // Gap of 10s > timeout of 5s: closes session 1
        let results = engine.process_value(3.0, base + Duration::from_secs(12));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 2);
        assert!((results[0].sum - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_global_window() {
        let config = WindowConfig::global();
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();
        for i in 0..100 {
            engine.process_value(i as f64, base + Duration::from_secs(i));
        }

        // Global never auto-closes
        assert_eq!(engine.completed_count(), 0);
        assert!(engine.current_pane().is_some());
        assert_eq!(engine.current_pane().map(|p| p.len()), Some(100));
    }

    #[test]
    fn test_flush() {
        let config = WindowConfig::global();
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();
        engine.process_value(10.0, base);
        engine.process_value(20.0, base + Duration::from_secs(1));

        let result = engine.flush();
        assert!(result.is_some());
        if let Some(r) = result {
            assert_eq!(r.count, 2);
            assert!((r.sum - 30.0).abs() < f64::EPSILON);
        }

        // After flush, current pane is gone
        assert!(engine.current_pane().is_none());
    }

    #[test]
    fn test_max_events_trigger() {
        let config = WindowConfig::tumbling(Duration::from_secs(3600)).with_max_events(3);
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();
        engine.process_value(1.0, base);
        engine.process_value(2.0, base + Duration::from_secs(1));
        engine.process_value(3.0, base + Duration::from_secs(2));

        // 4th event triggers close (max_events=3)
        let results = engine.process_value(4.0, base + Duration::from_secs(3));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 3);
    }

    #[test]
    fn test_process_event() {
        let config = WindowConfig::global();
        let mut engine = WindowEngine::new(config);

        let base = SystemTime::now();
        let event = make_event(1, 42.0, base);
        let results = engine.process(&event);
        assert!(results.is_empty());
        assert_eq!(engine.total_processed(), 1);
    }

    #[test]
    fn test_window_pane_empty() {
        let pane = WindowPane::new(1, SystemTime::now(), None);
        assert!(pane.is_empty());
        assert_eq!(pane.len(), 0);
        let result = pane.result();
        assert_eq!(result.count, 0);
        assert!((result.sum - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_window_pane_aggregates() {
        let mut pane = WindowPane::new(1, SystemTime::now(), None);
        pane.add(10.0);
        pane.add(20.0);
        pane.add(30.0);

        let result = pane.result();
        assert_eq!(result.count, 3);
        assert!((result.sum - 60.0).abs() < f64::EPSILON);
        assert!((result.mean - 20.0).abs() < f64::EPSILON);
        assert!((result.min - 10.0).abs() < f64::EPSILON);
        assert!((result.max - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_window_engine_grounding() {
        let comp = WindowEngine::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }

    #[test]
    fn test_window_config_grounding() {
        let comp = WindowConfig::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }
}
