//! # PVRX Backpressure & Flow Control
//!
//! Mechanisms for managing event flow when consumers are slower than producers.
//! Prevents unbounded memory growth and ensures system stability.
//!
//! ## Primitives
//! - ν (Frequency) — rate monitoring
//! - ∂ (Boundary) — capacity limits
//! - ς (State) — pressure state tracking
//! - N (Quantity) — buffer depth measurement

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// STRATEGY & POLICY
// ===============================================================

/// Strategy for handling overflow when buffer is full.
/// Tier: T2-P (∂ + ν)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackpressureStrategy {
    /// Drop newest events when buffer is full.
    DropNewest,
    /// Drop oldest events when buffer is full.
    DropOldest,
    /// Buffer with bounded capacity (reject on overflow).
    BoundedBuffer,
    /// Sample: accept every Nth event.
    Sample,
}

impl GroundsTo for BackpressureStrategy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary, LexPrimitiva::Frequency])
    }
}

/// Buffer policy configuration.
/// Tier: T2-P (N + ∂)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BufferPolicy {
    /// Maximum buffer capacity.
    pub capacity: usize,
    /// High watermark (trigger backpressure at this %).
    pub high_watermark: f64,
    /// Low watermark (release backpressure at this %).
    pub low_watermark: f64,
    /// Sample rate (1 = every event, N = every Nth event).
    pub sample_rate: usize,
}

impl BufferPolicy {
    /// Creates a default buffer policy.
    #[must_use]
    pub fn default_policy() -> Self {
        Self {
            capacity: 10_000,
            high_watermark: 0.80,
            low_watermark: 0.50,
            sample_rate: 1,
        }
    }

    /// Creates a tight policy for resource-constrained systems.
    #[must_use]
    pub fn tight() -> Self {
        Self {
            capacity: 1_000,
            high_watermark: 0.70,
            low_watermark: 0.30,
            sample_rate: 1,
        }
    }

    /// Creates a policy for high-throughput systems.
    #[must_use]
    pub fn high_throughput() -> Self {
        Self {
            capacity: 100_000,
            high_watermark: 0.90,
            low_watermark: 0.60,
            sample_rate: 1,
        }
    }

    /// Sets sample rate (for Sample strategy).
    #[must_use]
    pub fn with_sample_rate(mut self, rate: usize) -> Self {
        self.sample_rate = rate.max(1);
        self
    }
}

impl GroundsTo for BufferPolicy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Boundary])
    }
}

// ===============================================================
// PRESSURE STATE
// ===============================================================

/// Current pressure state of the flow controller.
/// Tier: T2-P (ς + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PressureState {
    /// Normal operation, accepting all events.
    Normal,
    /// Warning: approaching capacity.
    Elevated,
    /// Critical: at or above high watermark.
    Critical,
    /// Backpressure active: dropping/sampling events.
    Shedding,
}

impl GroundsTo for PressureState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State, LexPrimitiva::Boundary])
    }
}

/// Result of attempting to admit an event.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmitResult {
    /// Event accepted.
    Accepted,
    /// Event dropped (backpressure active).
    Dropped,
    /// Event sampled out (not selected for processing).
    Sampled,
    /// Event rejected (buffer at capacity).
    Rejected,
}

impl GroundsTo for AdmitResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

// ===============================================================
// THROTTLE
// ===============================================================

/// Rate throttle that limits throughput to a target events/second.
/// Tier: T2-P (ν + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Throttle {
    /// Target events per second.
    target_rate: f64,
    /// Minimum interval between events.
    min_interval: Duration,
    /// Last event accepted.
    last_accepted: Option<SystemTime>,
    /// Total events checked.
    total_checked: u64,
    /// Total events throttled.
    total_throttled: u64,
}

impl Throttle {
    /// Creates a new throttle with target rate.
    #[must_use]
    pub fn new(events_per_second: f64) -> Self {
        let min_interval = if events_per_second > 0.0 {
            Duration::from_secs_f64(1.0 / events_per_second)
        } else {
            Duration::from_secs(u64::MAX)
        };

        Self {
            target_rate: events_per_second,
            min_interval,
            last_accepted: None,
            total_checked: 0,
            total_throttled: 0,
        }
    }

    /// Creates a disabled throttle (unlimited rate).
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            target_rate: f64::MAX,
            min_interval: Duration::ZERO,
            last_accepted: None,
            total_checked: 0,
            total_throttled: 0,
        }
    }

    /// Checks if an event at the given timestamp should be allowed.
    pub fn check(&mut self, now: SystemTime) -> bool {
        self.total_checked += 1;

        match self.last_accepted {
            None => {
                self.last_accepted = Some(now);
                true
            }
            Some(last) => {
                let elapsed = now.duration_since(last).unwrap_or(Duration::ZERO);
                if elapsed >= self.min_interval {
                    self.last_accepted = Some(now);
                    true
                } else {
                    self.total_throttled += 1;
                    false
                }
            }
        }
    }

    /// Target rate.
    #[must_use]
    pub fn target_rate(&self) -> f64 {
        self.target_rate
    }

    /// Total events throttled.
    #[must_use]
    pub fn throttled(&self) -> u64 {
        self.total_throttled
    }
}

impl GroundsTo for Throttle {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Frequency, LexPrimitiva::Quantity])
    }
}

// ===============================================================
// FLOW CONTROLLER
// ===============================================================

/// Flow controller managing backpressure for event streams.
/// Tier: T2-C (ν + ∂ + ς + N)
///
/// Monitors buffer depth and applies backpressure strategies
/// when consumers can't keep up with producers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowController {
    /// Backpressure strategy.
    strategy: BackpressureStrategy,
    /// Buffer policy.
    policy: BufferPolicy,
    /// Current buffer depth.
    current_depth: usize,
    /// Current pressure state.
    state: PressureState,
    /// Rate throttle.
    throttle: Throttle,
    /// Sample counter (for Sample strategy).
    sample_counter: u64,
    /// Total events admitted.
    total_admitted: u64,
    /// Total events dropped.
    total_dropped: u64,
    /// Total events sampled out.
    total_sampled: u64,
}

impl FlowController {
    /// Creates a new flow controller.
    #[must_use]
    pub fn new(strategy: BackpressureStrategy, policy: BufferPolicy) -> Self {
        Self {
            strategy,
            policy,
            current_depth: 0,
            state: PressureState::Normal,
            throttle: Throttle::disabled(),
            sample_counter: 0,
            total_admitted: 0,
            total_dropped: 0,
            total_sampled: 0,
        }
    }

    /// Creates a flow controller with rate throttling.
    #[must_use]
    pub fn with_throttle(mut self, events_per_second: f64) -> Self {
        self.throttle = Throttle::new(events_per_second);
        self
    }

    /// Attempts to admit an event. Call this before processing.
    pub fn admit(&mut self, now: SystemTime) -> AdmitResult {
        // Check throttle first
        if !self.throttle.check(now) {
            self.total_dropped += 1;
            return AdmitResult::Dropped;
        }

        // Check sampling
        if self.strategy == BackpressureStrategy::Sample {
            self.sample_counter += 1;
            if self.sample_counter % (self.policy.sample_rate as u64) != 0 {
                self.total_sampled += 1;
                return AdmitResult::Sampled;
            }
        }

        // Strategy-based capacity check
        let result = match self.strategy {
            BackpressureStrategy::DropNewest => {
                if self.current_depth >= self.policy.capacity {
                    self.total_dropped += 1;
                    AdmitResult::Dropped
                } else {
                    self.current_depth += 1;
                    self.total_admitted += 1;
                    AdmitResult::Accepted
                }
            }
            BackpressureStrategy::DropOldest => {
                if self.current_depth >= self.policy.capacity {
                    // Logically evict oldest, depth stays the same
                    self.total_dropped += 1;
                } else {
                    self.current_depth += 1;
                }
                self.total_admitted += 1;
                AdmitResult::Accepted
            }
            BackpressureStrategy::BoundedBuffer => {
                if self.current_depth >= self.policy.capacity {
                    self.total_dropped += 1;
                    AdmitResult::Rejected
                } else {
                    self.current_depth += 1;
                    self.total_admitted += 1;
                    AdmitResult::Accepted
                }
            }
            BackpressureStrategy::Sample => {
                // Sampling already handled above; accept the event
                self.current_depth += 1;
                self.total_admitted += 1;
                AdmitResult::Accepted
            }
        };

        // Update pressure state after depth change
        self.update_pressure();
        result
    }

    /// Signals that an event has been consumed (decreases depth).
    pub fn consume(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
        self.update_pressure();
    }

    /// Signals N events consumed.
    pub fn consume_n(&mut self, n: usize) {
        self.current_depth = self.current_depth.saturating_sub(n);
        self.update_pressure();
    }

    fn update_pressure(&mut self) {
        if self.policy.capacity == 0 {
            self.state = PressureState::Normal;
            return;
        }

        let utilization = self.current_depth as f64 / self.policy.capacity as f64;

        self.state = if utilization >= 1.0 {
            PressureState::Shedding
        } else if utilization >= self.policy.high_watermark {
            PressureState::Critical
        } else if utilization >= self.policy.low_watermark {
            PressureState::Elevated
        } else {
            PressureState::Normal
        };
    }

    /// Current pressure state.
    #[must_use]
    pub fn pressure(&self) -> PressureState {
        self.state
    }

    /// Current buffer depth.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.current_depth
    }

    /// Buffer utilization (0.0 to 1.0+).
    #[must_use]
    pub fn utilization(&self) -> f64 {
        if self.policy.capacity == 0 {
            return 0.0;
        }
        self.current_depth as f64 / self.policy.capacity as f64
    }

    /// Total events admitted.
    #[must_use]
    pub fn admitted(&self) -> u64 {
        self.total_admitted
    }

    /// Total events dropped.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.total_dropped
    }

    /// Total events sampled out.
    #[must_use]
    pub fn sampled(&self) -> u64 {
        self.total_sampled
    }
}

impl GroundsTo for FlowController {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Boundary,
            LexPrimitiva::State,
            LexPrimitiva::Quantity,
        ])
        .with_dominant(LexPrimitiva::Frequency, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_flow_controller_normal() {
        let policy = BufferPolicy {
            capacity: 100,
            high_watermark: 0.80,
            low_watermark: 0.50,
            sample_rate: 1,
        };
        let mut fc = FlowController::new(BackpressureStrategy::DropNewest, policy);

        let now = SystemTime::now();
        let result = fc.admit(now);
        assert_eq!(result, AdmitResult::Accepted);
        assert_eq!(fc.pressure(), PressureState::Normal);
    }

    #[test]
    fn test_flow_controller_pressure_transitions() {
        let policy = BufferPolicy {
            capacity: 10,
            high_watermark: 0.80,
            low_watermark: 0.50,
            sample_rate: 1,
        };
        let mut fc = FlowController::new(BackpressureStrategy::BoundedBuffer, policy);

        let now = SystemTime::now();

        // Fill to 50% -> Normal still (boundary is at 50%)
        for _ in 0..4 {
            fc.admit(now);
        }
        assert_eq!(fc.pressure(), PressureState::Normal);

        // Fill to 60% -> Elevated
        for _ in 0..2 {
            fc.admit(now);
        }
        assert_eq!(fc.pressure(), PressureState::Elevated);

        // Fill to 80% -> Critical
        for _ in 0..2 {
            fc.admit(now);
        }
        assert_eq!(fc.pressure(), PressureState::Critical);
    }

    #[test]
    fn test_bounded_buffer_rejection() {
        let policy = BufferPolicy {
            capacity: 5,
            high_watermark: 0.80,
            low_watermark: 0.50,
            sample_rate: 1,
        };
        let mut fc = FlowController::new(BackpressureStrategy::BoundedBuffer, policy);

        let now = SystemTime::now();

        // Fill to capacity
        for _ in 0..5 {
            fc.admit(now);
        }

        // Next should be rejected
        let result = fc.admit(now);
        assert_eq!(result, AdmitResult::Rejected);
    }

    #[test]
    fn test_drop_newest() {
        let policy = BufferPolicy {
            capacity: 3,
            high_watermark: 0.60,
            low_watermark: 0.30,
            sample_rate: 1,
        };
        let mut fc = FlowController::new(BackpressureStrategy::DropNewest, policy);

        let now = SystemTime::now();

        // Fill past high watermark
        for _ in 0..3 {
            fc.admit(now);
        }

        // Next should be dropped
        let result = fc.admit(now);
        assert_eq!(result, AdmitResult::Dropped);
        assert_eq!(fc.dropped(), 1);
    }

    #[test]
    fn test_sampling() {
        let policy = BufferPolicy::default_policy().with_sample_rate(3);
        let mut fc = FlowController::new(BackpressureStrategy::Sample, policy);

        let now = SystemTime::now();

        // Only every 3rd event should be accepted
        let r1 = fc.admit(now); // counter=1, 1%3!=0 -> Sampled
        let r2 = fc.admit(now); // counter=2, 2%3!=0 -> Sampled
        let r3 = fc.admit(now); // counter=3, 3%3==0 -> Accepted

        assert_eq!(r1, AdmitResult::Sampled);
        assert_eq!(r2, AdmitResult::Sampled);
        assert_eq!(r3, AdmitResult::Accepted);
        assert_eq!(fc.sampled(), 2);
    }

    #[test]
    fn test_consume_reduces_depth() {
        let policy = BufferPolicy::default_policy();
        let mut fc = FlowController::new(BackpressureStrategy::BoundedBuffer, policy);

        let now = SystemTime::now();

        fc.admit(now);
        fc.admit(now);
        fc.admit(now);
        assert_eq!(fc.depth(), 3);

        fc.consume();
        assert_eq!(fc.depth(), 2);

        fc.consume_n(2);
        assert_eq!(fc.depth(), 0);
    }

    #[test]
    fn test_throttle() {
        let mut throttle = Throttle::new(10.0); // 10 events/sec = 100ms interval

        let base = SystemTime::now();

        // First always accepted
        assert!(throttle.check(base));

        // Too soon (50ms < 100ms) -> throttled
        assert!(!throttle.check(base + Duration::from_millis(50)));

        // After 100ms -> accepted
        assert!(throttle.check(base + Duration::from_millis(100)));

        assert_eq!(throttle.throttled(), 1);
    }

    #[test]
    fn test_throttle_disabled() {
        let mut throttle = Throttle::disabled();
        let now = SystemTime::now();

        for _ in 0..100 {
            assert!(throttle.check(now));
        }
        assert_eq!(throttle.throttled(), 0);
    }

    #[test]
    fn test_utilization() {
        let policy = BufferPolicy {
            capacity: 100,
            high_watermark: 0.80,
            low_watermark: 0.50,
            sample_rate: 1,
        };
        let mut fc = FlowController::new(BackpressureStrategy::BoundedBuffer, policy);

        let now = SystemTime::now();
        for _ in 0..50 {
            fc.admit(now);
        }

        assert!((fc.utilization() - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn test_flow_controller_grounding() {
        let comp = FlowController::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Frequency));
    }

    #[test]
    fn test_throttle_grounding() {
        let comp = Throttle::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
