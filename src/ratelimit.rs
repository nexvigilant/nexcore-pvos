//! # PVGW Rate Limiting
//!
//! Token bucket rate limiter with per-identity quotas.
//! Enforces fair access across all callers at the boundary.
//!
//! ## Primitives
//! - N (Quantity) — token counts, quotas
//! - ∂ (Boundary) — enforce limits
//! - ς (State) — bucket state tracking

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

/// Service tier determining default quotas.
/// Tier: T2-P (N)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    /// Free tier: minimal quotas.
    Free,
    /// Standard tier: moderate quotas.
    Standard,
    /// Enterprise tier: high quotas.
    Enterprise,
    /// Internal tier: unlimited.
    Internal,
}

impl Tier {
    /// Returns the default quota for this tier.
    #[must_use]
    pub fn default_quota(self) -> Quota {
        match self {
            Self::Free => Quota::per_minute(10),
            Self::Standard => Quota::per_minute(100),
            Self::Enterprise => Quota::per_minute(1000),
            Self::Internal => Quota::unlimited(),
        }
    }
}

impl GroundsTo for Tier {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
    }
}

/// Rate limit quota specification.
/// Tier: T2-P (N + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quota {
    /// Maximum requests per window.
    pub max_requests: u64,
    /// Window duration in seconds.
    pub window_secs: u64,
    /// Burst allowance (above max_requests, temporarily).
    pub burst: u64,
}

impl Quota {
    /// Creates a per-minute quota.
    #[must_use]
    pub fn per_minute(max: u64) -> Self {
        Self {
            max_requests: max,
            window_secs: 60,
            burst: max / 5 + 1, // 20% burst
        }
    }

    /// Creates a per-second quota.
    #[must_use]
    pub fn per_second(max: u64) -> Self {
        Self {
            max_requests: max,
            window_secs: 1,
            burst: max + 1,
        }
    }

    /// Creates an unlimited quota (internal use).
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            max_requests: u64::MAX,
            window_secs: 1,
            burst: u64::MAX,
        }
    }

    /// Tokens refilled per second.
    #[must_use]
    pub fn refill_rate(&self) -> f64 {
        if self.window_secs == 0 {
            return 0.0;
        }
        self.max_requests as f64 / self.window_secs as f64
    }
}

impl GroundsTo for Quota {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Boundary])
    }
}

/// Token bucket for a single identity.
/// Tier: T2-C (N + ∂ + ς)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    /// Current available tokens.
    tokens: f64,
    /// Maximum capacity (burst limit).
    capacity: f64,
    /// Tokens added per second.
    refill_rate: f64,
    /// Last refill timestamp.
    last_refill: SystemTime,
    /// Total requests served.
    total_served: u64,
    /// Total requests rejected.
    total_rejected: u64,
}

impl TokenBucket {
    /// Creates a new bucket from a quota.
    #[must_use]
    fn from_quota(quota: &Quota) -> Self {
        let capacity = (quota.max_requests + quota.burst) as f64;
        Self {
            tokens: capacity, // Start full
            capacity,
            refill_rate: quota.refill_rate(),
            last_refill: SystemTime::now(),
            total_served: 0,
            total_rejected: 0,
        }
    }

    /// Attempts to consume one token.
    /// Returns `true` if allowed, `false` if rate-limited.
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            self.total_served += 1;
            true
        } else {
            self.total_rejected += 1;
            false
        }
    }

    /// Refills tokens based on elapsed time.
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().unwrap_or(Duration::ZERO);

        let new_tokens = elapsed.as_secs_f64() * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = SystemTime::now();
    }

    /// Current token count.
    #[must_use]
    pub fn available(&self) -> f64 {
        self.tokens
    }

    /// Total requests served by this bucket.
    #[must_use]
    pub fn served(&self) -> u64 {
        self.total_served
    }

    /// Total requests rejected by this bucket.
    #[must_use]
    pub fn rejected(&self) -> u64 {
        self.total_rejected
    }
}

impl GroundsTo for TokenBucket {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,
            LexPrimitiva::Boundary,
            LexPrimitiva::State,
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.85)
    }
}

/// Rate limit error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitError {
    /// Identity that was rate-limited.
    pub identity: String,
    /// Retry after this many seconds.
    pub retry_after_secs: u64,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rate limited: retry after {}s", self.retry_after_secs)
    }
}

impl std::error::Error for RateLimitError {}

/// Per-identity rate limiter.
/// Tier: T2-C (N + ∂ + ς + μ)
///
/// Maintains a token bucket per identity name.
/// Fair: each identity gets its own bucket.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimiter {
    /// Default quota for new identities.
    default_quota: Option<Quota>,
    /// Per-identity token buckets.
    buckets: HashMap<String, TokenBucket>,
    /// Per-identity quota overrides.
    overrides: HashMap<String, Quota>,
}

impl RateLimiter {
    /// Creates a new rate limiter with default quota.
    #[must_use]
    pub fn new(default_quota: Quota) -> Self {
        Self {
            default_quota: Some(default_quota),
            buckets: HashMap::new(),
            overrides: HashMap::new(),
        }
    }

    /// Creates a disabled rate limiter (everything passes).
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            default_quota: None,
            buckets: HashMap::new(),
            overrides: HashMap::new(),
        }
    }

    /// Sets a quota override for a specific identity.
    pub fn set_override(&mut self, identity: &str, quota: Quota) {
        self.overrides.insert(identity.to_string(), quota);
    }

    /// Checks if a request from the given identity is allowed.
    ///
    /// # Errors
    /// Returns `RateLimitError` if the identity has exceeded its quota.
    pub fn check(&mut self, identity: &str) -> Result<(), RateLimitError> {
        let quota = match self.overrides.get(identity) {
            Some(q) => q,
            None => match &self.default_quota {
                Some(q) => q,
                None => return Ok(()), // Disabled
            },
        };

        let bucket = self
            .buckets
            .entry(identity.to_string())
            .or_insert_with(|| TokenBucket::from_quota(quota));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(RateLimitError {
                identity: identity.to_string(),
                retry_after_secs: quota.window_secs,
            })
        }
    }

    /// Returns usage stats for an identity.
    #[must_use]
    pub fn stats(&self, identity: &str) -> Option<(u64, u64)> {
        self.buckets
            .get(identity)
            .map(|b| (b.served(), b.rejected()))
    }

    /// Total identities tracked.
    #[must_use]
    pub fn tracked_count(&self) -> usize {
        self.buckets.len()
    }
}

impl GroundsTo for RateLimiter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity,
            LexPrimitiva::Boundary,
            LexPrimitiva::State,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_tier_default_quotas() {
        let free = Tier::Free.default_quota();
        assert_eq!(free.max_requests, 10);

        let standard = Tier::Standard.default_quota();
        assert_eq!(standard.max_requests, 100);

        let enterprise = Tier::Enterprise.default_quota();
        assert_eq!(enterprise.max_requests, 1000);

        let internal = Tier::Internal.default_quota();
        assert_eq!(internal.max_requests, u64::MAX);
    }

    #[test]
    fn test_quota_refill_rate() {
        let q = Quota::per_minute(60);
        assert!((q.refill_rate() - 1.0).abs() < f64::EPSILON); // 1 token/sec
    }

    #[test]
    fn test_token_bucket_consume() {
        let quota = Quota::per_second(5);
        let mut bucket = TokenBucket::from_quota(&quota);

        // Should allow initial burst
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        assert_eq!(bucket.served(), 5);
    }

    #[test]
    fn test_rate_limiter_allows_within_quota() {
        let mut limiter = RateLimiter::new(Quota::per_second(100));

        // Should allow a few requests immediately
        for _ in 0..10 {
            assert!(limiter.check("user1").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_disabled() {
        let mut limiter = RateLimiter::disabled();

        // Should always allow
        for _ in 0..1000 {
            assert!(limiter.check("any_user").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_per_identity() {
        let mut limiter = RateLimiter::new(Quota::per_second(5));

        // Each identity gets its own bucket
        assert!(limiter.check("user_a").is_ok());
        assert!(limiter.check("user_b").is_ok());
        assert_eq!(limiter.tracked_count(), 2);
    }

    #[test]
    fn test_rate_limiter_override() {
        let mut limiter = RateLimiter::new(Quota::per_second(1));
        limiter.set_override("vip", Quota::per_second(1000));

        // VIP should have more capacity
        for _ in 0..100 {
            assert!(limiter.check("vip").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_stats() {
        let mut limiter = RateLimiter::new(Quota::per_second(100));
        let _ = limiter.check("tracked");

        let stats = limiter.stats("tracked");
        assert!(stats.is_some());
        if let Some((served, rejected)) = stats {
            assert_eq!(served, 1);
            assert_eq!(rejected, 0);
        }

        let missing = limiter.stats("unknown");
        assert!(missing.is_none());
    }

    #[test]
    fn test_token_bucket_grounding() {
        let comp = TokenBucket::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }

    #[test]
    fn test_rate_limiter_grounding() {
        let comp = RateLimiter::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Quantity));
    }
}
