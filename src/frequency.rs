//! # PVTF Frequency Primitives
//!
//! Temporal frequency types for adaptive polling, retry, and periodic monitoring.
//! Frequency (ν) is the dominant primitive — all types answer "how often?"
//!
//! ## Primitives
//! - ν (Frequency) — DOMINANT: rate, cadence, temporal pattern
//! - κ (Comparison) — threshold evaluation for rate adaptation
//! - ∂ (Boundary) — min/max rate limits, backoff ceilings
//! - N (Quantity) — interval durations, retry counts
//! - ∝ (Irreversibility) — retry exhaustion (once max retries hit, it's final)
//! - ∃ (Existence) — liveness/presence detection via periodic probes
//! - → (Causality) — monitor triggers downstream actions
//!
//! ## Key Insight
//!
//! In PV systems, frequency determines signal sensitivity. Too low: missed
//! emerging safety signals. Too high: alert fatigue. Adaptive frequency is
//! the immune system's "fever response" — ramp up vigilance when anomalies
//! appear, throttle back during homeostasis.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// ADAPTIVE POLLER
// ===============================================================

/// Self-adjusting polling interval that adapts to observed event rates.
/// Tier: T2-C (ν κ ∂ N)
///
/// Highest frequency confidence in the corpus (0.932).
/// κ compares event rate against thresholds to trigger rate changes.
/// ∂ enforces min/max interval bounds. N measures the intervals.
#[derive(Debug, Clone)]
pub struct AdaptivePoller {
    /// Current polling interval.
    interval: Duration,
    /// Minimum interval (fastest polling).
    min_interval: Duration,
    /// Maximum interval (slowest polling).
    max_interval: Duration,
    /// Speedup factor when events detected (< 1.0).
    speedup: f64,
    /// Slowdown factor when idle (> 1.0).
    slowdown: f64,
    /// Number of polls executed.
    poll_count: u64,
    /// Last poll time.
    last_poll: Option<Instant>,
}

impl AdaptivePoller {
    /// Creates a new adaptive poller.
    #[must_use]
    pub fn new(initial: Duration, min: Duration, max: Duration) -> Self {
        Self {
            interval: initial,
            min_interval: min,
            max_interval: max,
            speedup: 0.5,
            slowdown: 1.5,
            poll_count: 0,
            last_poll: None,
        }
    }

    /// Creates with default bounds (100ms min, 60s max, 1s initial).
    #[must_use]
    pub fn default_bounds() -> Self {
        Self::new(
            Duration::from_secs(1),
            Duration::from_millis(100),
            Duration::from_secs(60),
        )
    }

    /// Returns the current interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Returns the total number of polls.
    #[must_use]
    pub fn poll_count(&self) -> u64 {
        self.poll_count
    }

    /// Signals that an event was detected — speed up polling.
    pub fn on_event(&mut self) {
        let nanos = (self.interval.as_nanos() as f64 * self.speedup) as u64;
        let new_interval = Duration::from_nanos(nanos);
        self.interval = new_interval.max(self.min_interval);
    }

    /// Signals an idle cycle — slow down polling.
    pub fn on_idle(&mut self) {
        let nanos = (self.interval.as_nanos() as f64 * self.slowdown) as u64;
        let new_interval = Duration::from_nanos(nanos);
        self.interval = new_interval.min(self.max_interval);
    }

    /// Records a poll tick.
    pub fn tick(&mut self) {
        self.poll_count += 1;
        self.last_poll = Some(Instant::now());
    }

    /// Returns true if enough time has elapsed since the last poll.
    #[must_use]
    pub fn should_poll(&self) -> bool {
        match self.last_poll {
            None => true,
            Some(last) => last.elapsed() >= self.interval,
        }
    }

    /// Returns the ratio of current interval to max interval (0.0 = fastest, 1.0 = slowest).
    #[must_use]
    pub fn load_factor(&self) -> f64 {
        if self.max_interval.is_zero() {
            return 0.0;
        }
        self.interval.as_nanos() as f64 / self.max_interval.as_nanos() as f64
    }
}

impl GroundsTo for AdaptivePoller {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Comparison,
            LexPrimitiva::Boundary,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// RETRY STRATEGY
// ===============================================================

/// Configurable retry strategy with exponential backoff and jitter.
/// Tier: T2-C (ν ∝ ∂ N)
///
/// ∝ (Irreversibility) is key: each retry attempt consumes one of a finite
/// budget. Once exhausted, the failure is permanent. ∂ caps the maximum
/// delay. N counts remaining attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// Maximum number of retry attempts.
    max_retries: u32,
    /// Base delay between retries.
    base_delay_ms: u64,
    /// Maximum delay cap.
    max_delay_ms: u64,
    /// Backoff multiplier per attempt.
    multiplier: f64,
    /// Current attempt number (0-indexed).
    current_attempt: u32,
}

impl RetryStrategy {
    /// Creates a new retry strategy.
    #[must_use]
    pub fn new(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
            multiplier: 2.0,
            current_attempt: 0,
        }
    }

    /// Creates a strategy with exponential backoff defaults (3 retries, 100ms base, 10s max).
    #[must_use]
    pub fn exponential() -> Self {
        Self::new(3, 100, 10_000)
    }

    /// Creates a fixed-interval strategy (no backoff).
    #[must_use]
    pub fn fixed(max_retries: u32, delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms: delay_ms,
            max_delay_ms: delay_ms,
            multiplier: 1.0,
            current_attempt: 0,
        }
    }

    /// Returns true if retries remain.
    #[must_use]
    pub fn has_remaining(&self) -> bool {
        self.current_attempt < self.max_retries
    }

    /// Returns the number of remaining retries.
    #[must_use]
    pub fn remaining(&self) -> u32 {
        self.max_retries.saturating_sub(self.current_attempt)
    }

    /// Returns the current attempt number.
    #[must_use]
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }

    /// Computes the delay for the current attempt and advances the counter.
    /// Returns `None` if no retries remain.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if !self.has_remaining() {
            return None;
        }
        let delay =
            (self.base_delay_ms as f64 * self.multiplier.powi(self.current_attempt as i32)) as u64;
        let capped = delay.min(self.max_delay_ms);
        self.current_attempt += 1;
        Some(Duration::from_millis(capped))
    }

    /// Resets the retry counter.
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }

    /// Returns the total maximum possible wait time if all retries are exhausted.
    #[must_use]
    pub fn max_total_wait(&self) -> Duration {
        let mut total_ms = 0u64;
        for i in 0..self.max_retries {
            let delay = (self.base_delay_ms as f64 * self.multiplier.powi(i as i32)) as u64;
            total_ms += delay.min(self.max_delay_ms);
        }
        Duration::from_millis(total_ms)
    }
}

impl GroundsTo for RetryStrategy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Irreversibility,
            LexPrimitiva::Boundary,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// PERIODIC MONITOR
// ===============================================================

/// Periodic health monitor that tracks liveness and triggers on state changes.
/// Tier: T2-C (ν ∃ ∂ →)
///
/// ν defines the check cadence, ∃ validates target liveness, ∂ sets
/// healthy/unhealthy thresholds, → triggers downstream actions on transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicMonitor {
    /// Name of the monitored target.
    target: String,
    /// Check interval in milliseconds.
    interval_ms: u64,
    /// Number of consecutive failures before marking unhealthy.
    failure_threshold: u32,
    /// Number of consecutive successes before marking healthy.
    recovery_threshold: u32,
    /// Current consecutive failure count.
    consecutive_failures: u32,
    /// Current consecutive success count.
    consecutive_successes: u32,
    /// Current health status.
    healthy: bool,
    /// Total checks performed.
    total_checks: u64,
    /// Total failures observed.
    total_failures: u64,
}

impl PeriodicMonitor {
    /// Creates a new monitor for the given target.
    #[must_use]
    pub fn new(target: &str, interval_ms: u64) -> Self {
        Self {
            target: target.to_string(),
            interval_ms,
            failure_threshold: 3,
            recovery_threshold: 2,
            consecutive_failures: 0,
            consecutive_successes: 0,
            healthy: true,
            total_checks: 0,
            total_failures: 0,
        }
    }

    /// Sets the failure and recovery thresholds.
    #[must_use]
    pub fn with_thresholds(mut self, failure: u32, recovery: u32) -> Self {
        self.failure_threshold = failure;
        self.recovery_threshold = recovery;
        self
    }

    /// Returns the target name.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Returns whether the target is currently healthy.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    /// Returns the check interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        Duration::from_millis(self.interval_ms)
    }

    /// Records a successful check. Returns `true` if state changed to healthy.
    pub fn record_success(&mut self) -> bool {
        self.total_checks += 1;
        self.consecutive_failures = 0;
        self.consecutive_successes += 1;
        if !self.healthy && self.consecutive_successes >= self.recovery_threshold {
            self.healthy = true;
            return true; // State transition: unhealthy → healthy
        }
        false
    }

    /// Records a failed check. Returns `true` if state changed to unhealthy.
    pub fn record_failure(&mut self) -> bool {
        self.total_checks += 1;
        self.total_failures += 1;
        self.consecutive_successes = 0;
        self.consecutive_failures += 1;
        if self.healthy && self.consecutive_failures >= self.failure_threshold {
            self.healthy = false;
            return true; // State transition: healthy → unhealthy
        }
        false
    }

    /// Returns the failure rate (0.0 to 1.0).
    #[must_use]
    pub fn failure_rate(&self) -> f64 {
        if self.total_checks == 0 {
            return 0.0;
        }
        self.total_failures as f64 / self.total_checks as f64
    }

    /// Returns the total number of checks.
    #[must_use]
    pub fn total_checks(&self) -> u64 {
        self.total_checks
    }

    /// Returns uptime ratio (1.0 - failure_rate).
    #[must_use]
    pub fn uptime(&self) -> f64 {
        1.0 - self.failure_rate()
    }
}

impl GroundsTo for PeriodicMonitor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Frequency,
            LexPrimitiva::Existence,
            LexPrimitiva::Boundary,
            LexPrimitiva::Causality,
        ])
    }
}

// ===============================================================
// TESTS
// ===============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- AdaptivePoller ---

    #[test]
    fn adaptive_poller_default_bounds() {
        let ap = AdaptivePoller::default_bounds();
        assert_eq!(ap.interval(), Duration::from_secs(1));
        assert_eq!(ap.poll_count(), 0);
    }

    #[test]
    fn adaptive_poller_speedup_on_event() {
        let mut ap = AdaptivePoller::default_bounds();
        let before = ap.interval();
        ap.on_event();
        assert!(ap.interval() < before);
    }

    #[test]
    fn adaptive_poller_slowdown_on_idle() {
        let mut ap = AdaptivePoller::default_bounds();
        let before = ap.interval();
        ap.on_idle();
        assert!(ap.interval() > before);
    }

    #[test]
    fn adaptive_poller_respects_min_bound() {
        let mut ap = AdaptivePoller::new(
            Duration::from_millis(200),
            Duration::from_millis(100),
            Duration::from_secs(60),
        );
        for _ in 0..100 {
            ap.on_event();
        }
        assert!(ap.interval() >= Duration::from_millis(100));
    }

    #[test]
    fn adaptive_poller_respects_max_bound() {
        let mut ap = AdaptivePoller::new(
            Duration::from_secs(30),
            Duration::from_millis(100),
            Duration::from_secs(60),
        );
        for _ in 0..100 {
            ap.on_idle();
        }
        assert!(ap.interval() <= Duration::from_secs(60));
    }

    #[test]
    fn adaptive_poller_tick_increments() {
        let mut ap = AdaptivePoller::default_bounds();
        ap.tick();
        ap.tick();
        assert_eq!(ap.poll_count(), 2);
    }

    #[test]
    fn adaptive_poller_should_poll_initial() {
        let ap = AdaptivePoller::default_bounds();
        assert!(ap.should_poll());
    }

    #[test]
    fn adaptive_poller_load_factor() {
        let ap = AdaptivePoller::new(
            Duration::from_secs(30),
            Duration::from_millis(100),
            Duration::from_secs(60),
        );
        let lf = ap.load_factor();
        assert!((lf - 0.5).abs() < 0.01);
    }

    #[test]
    fn adaptive_poller_grounding() {
        let comp = AdaptivePoller::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Frequency
        );
        assert_eq!(comp.primitives.len(), 4);
    }

    // --- RetryStrategy ---

    #[test]
    fn retry_strategy_exponential_delays() {
        let mut rs = RetryStrategy::exponential();
        let d1 = rs.next_delay();
        let d2 = rs.next_delay();
        assert!(d1.is_some());
        assert!(d2.is_some());
        assert!(d2.expect("tested") > d1.expect("tested"));
    }

    #[test]
    fn retry_strategy_exhaustion() {
        let mut rs = RetryStrategy::new(2, 100, 1000);
        assert!(rs.has_remaining());
        let _ = rs.next_delay();
        let _ = rs.next_delay();
        assert!(!rs.has_remaining());
        assert!(rs.next_delay().is_none());
    }

    #[test]
    fn retry_strategy_fixed_interval() {
        let mut rs = RetryStrategy::fixed(3, 500);
        let d1 = rs.next_delay();
        let d2 = rs.next_delay();
        assert_eq!(d1, d2); // Fixed = same delay
    }

    #[test]
    fn retry_strategy_reset() {
        let mut rs = RetryStrategy::new(2, 100, 1000);
        let _ = rs.next_delay();
        let _ = rs.next_delay();
        assert!(!rs.has_remaining());
        rs.reset();
        assert!(rs.has_remaining());
        assert_eq!(rs.remaining(), 2);
    }

    #[test]
    fn retry_strategy_max_total_wait() {
        let rs = RetryStrategy::fixed(3, 100);
        let total = rs.max_total_wait();
        assert_eq!(total, Duration::from_millis(300));
    }

    #[test]
    fn retry_strategy_grounding() {
        let comp = RetryStrategy::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Frequency
        );
    }

    // --- PeriodicMonitor ---

    #[test]
    fn periodic_monitor_starts_healthy() {
        let pm = PeriodicMonitor::new("db", 5000);
        assert!(pm.is_healthy());
        assert_eq!(pm.target(), "db");
    }

    #[test]
    fn periodic_monitor_transitions_to_unhealthy() {
        let mut pm = PeriodicMonitor::new("api", 1000);
        assert!(!pm.record_failure()); // 1 of 3
        assert!(!pm.record_failure()); // 2 of 3
        let changed = pm.record_failure(); // 3 of 3 → transition
        assert!(changed);
        assert!(!pm.is_healthy());
    }

    #[test]
    fn periodic_monitor_recovery() {
        let mut pm = PeriodicMonitor::new("svc", 1000);
        for _ in 0..3 {
            pm.record_failure();
        }
        assert!(!pm.is_healthy());
        assert!(!pm.record_success()); // 1 of 2
        let recovered = pm.record_success(); // 2 of 2 → transition
        assert!(recovered);
        assert!(pm.is_healthy());
    }

    #[test]
    fn periodic_monitor_failure_rate() {
        let mut pm = PeriodicMonitor::new("test", 1000);
        pm.record_success();
        pm.record_failure();
        pm.record_success();
        pm.record_failure();
        assert!((pm.failure_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn periodic_monitor_uptime() {
        let mut pm = PeriodicMonitor::new("test", 1000);
        for _ in 0..8 {
            pm.record_success();
        }
        for _ in 0..2 {
            pm.record_failure();
        }
        assert!((pm.uptime() - 0.8).abs() < 0.001);
    }

    #[test]
    fn periodic_monitor_custom_thresholds() {
        let mut pm = PeriodicMonitor::new("strict", 500).with_thresholds(1, 1);
        let changed = pm.record_failure(); // 1 of 1 → immediate transition
        assert!(changed);
        assert!(!pm.is_healthy());
        let recovered = pm.record_success(); // 1 of 1 → immediate recovery
        assert!(recovered);
    }

    #[test]
    fn periodic_monitor_grounding() {
        let comp = PeriodicMonitor::primitive_composition();
        assert_eq!(
            comp.dominant.expect("has dominant"),
            LexPrimitiva::Frequency
        );
        assert_eq!(comp.primitives.len(), 4);
    }
}
