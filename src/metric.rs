//! # PVMX Core Metric Types
//!
//! Foundational metric types: counters, gauges, histograms, and labels.
//! Every metric is a Σ-operation: counters sum events, gauges sum to averages,
//! histograms sum into buckets.
//!
//! ## Primitives
//! - Σ (Sum) — counters increment, histograms accumulate
//! - N (Quantity) — individual gauge measurements
//! - π (Persistence) — metric storage identity

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// METRIC IDENTITY
// ===============================================================

/// Unique metric identifier.
/// Tier: T2-P (Σ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricId(pub String);

impl MetricId {
    /// Creates a metric ID.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// The metric name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl GroundsTo for MetricId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum])
    }
}

/// Key-value labels attached to a metric.
/// Tier: T2-P (Σ + N)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Labels {
    /// Label pairs.
    pairs: Vec<(String, String)>,
}

impl Labels {
    /// Creates empty labels.
    #[must_use]
    pub fn empty() -> Self {
        Self { pairs: Vec::new() }
    }

    /// Creates labels from key-value pairs.
    #[must_use]
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            pairs: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    /// Adds a label pair.
    pub fn add(&mut self, key: &str, value: &str) {
        self.pairs.push((key.to_string(), value.to_string()));
    }

    /// Gets a label value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Number of labels.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// All label pairs.
    #[must_use]
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    /// Formats as `{key=value,...}`.
    #[must_use]
    pub fn display(&self) -> String {
        if self.pairs.is_empty() {
            return "{}".to_string();
        }
        let inner: Vec<String> = self.pairs.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("{{{}}}", inner.join(","))
    }
}

impl GroundsTo for Labels {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Quantity])
    }
}

// ===============================================================
// METRIC KINDS
// ===============================================================

/// What kind of metric this is.
/// Tier: T2-P (Σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricKind {
    /// Monotonically increasing counter.
    Counter,
    /// Point-in-time gauge.
    Gauge,
    /// Distribution histogram.
    Histogram,
    /// Summary statistics.
    Summary,
}

impl GroundsTo for MetricKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum])
    }
}

// ===============================================================
// COUNTER
// ===============================================================

/// Monotonically increasing counter — the purest Σ.
/// Tier: T2-P (Σ)
///
/// Counters only go up. They represent cumulative totals:
/// total requests, total errors, total signals detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    /// Metric identity.
    pub id: MetricId,
    /// Current value (monotonically increasing).
    value: f64,
    /// Total increments performed.
    increments: u64,
}

impl Counter {
    /// Creates a new counter at zero.
    #[must_use]
    pub fn new(id: MetricId) -> Self {
        Self {
            id,
            value: 0.0,
            increments: 0,
        }
    }

    /// Increments by 1.
    pub fn inc(&mut self) {
        self.value += 1.0;
        self.increments += 1;
    }

    /// Increments by a positive amount. Negative values are ignored.
    pub fn inc_by(&mut self, amount: f64) {
        if amount > 0.0 {
            self.value += amount;
            self.increments += 1;
        }
    }

    /// Current value.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Total increment operations.
    #[must_use]
    pub fn increments(&self) -> u64 {
        self.increments
    }
}

impl GroundsTo for Counter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum]).with_dominant(LexPrimitiva::Sum, 0.95)
    }
}

// ===============================================================
// GAUGE
// ===============================================================

/// Point-in-time measurement — the N primitive.
/// Tier: T2-P (N + Σ)
///
/// Gauges can go up and down. They represent current state:
/// active workflows, queue depth, memory usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge {
    /// Metric identity.
    pub id: MetricId,
    /// Current value.
    value: f64,
    /// Total observations.
    observations: u64,
    /// Running sum for average calculation (Σ).
    running_sum: f64,
}

impl Gauge {
    /// Creates a new gauge at zero.
    #[must_use]
    pub fn new(id: MetricId) -> Self {
        Self {
            id,
            value: 0.0,
            observations: 0,
            running_sum: 0.0,
        }
    }

    /// Sets the gauge value.
    pub fn set(&mut self, value: f64) {
        self.value = value;
        self.observations += 1;
        self.running_sum += value;
    }

    /// Increments the gauge by 1.
    pub fn inc(&mut self) {
        self.set(self.value + 1.0);
    }

    /// Decrements the gauge by 1.
    pub fn dec(&mut self) {
        self.set(self.value - 1.0);
    }

    /// Current value.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Average over all observations (Σ/N).
    #[must_use]
    pub fn average(&self) -> f64 {
        if self.observations == 0 {
            return 0.0;
        }
        self.running_sum / self.observations as f64
    }

    /// Total observations.
    #[must_use]
    pub fn observations(&self) -> u64 {
        self.observations
    }
}

impl GroundsTo for Gauge {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Sum])
            .with_dominant(LexPrimitiva::Quantity, 0.80)
    }
}

// ===============================================================
// HISTOGRAM
// ===============================================================

/// Bucket boundary for histogram distribution.
/// Tier: T1 (N)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BucketBound(pub f64);

impl GroundsTo for BucketBound {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity])
    }
}

/// A single histogram bucket.
/// Tier: T2-P (Σ + N)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    /// Upper bound (exclusive).
    pub upper_bound: f64,
    /// Count of observations in this bucket.
    pub count: u64,
}

/// Distribution histogram — Σ across buckets.
/// Tier: T2-C (Σ + N + ∂)
///
/// Histograms observe values into predefined buckets.
/// Each bucket is a partial Σ; total sum is a global Σ.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    /// Metric identity.
    pub id: MetricId,
    /// Buckets with upper bounds.
    buckets: Vec<Bucket>,
    /// Total count of observations.
    total_count: u64,
    /// Sum of all observed values (Σ).
    total_sum: f64,
    /// Minimum observed value.
    min: f64,
    /// Maximum observed value.
    max: f64,
}

impl Histogram {
    /// Creates a histogram with given bucket boundaries.
    /// Bounds are sorted automatically.
    #[must_use]
    pub fn new(id: MetricId, mut bounds: Vec<f64>) -> Self {
        bounds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        bounds.dedup();

        let buckets = bounds
            .iter()
            .map(|&b| Bucket {
                upper_bound: b,
                count: 0,
            })
            .collect();

        Self {
            id,
            buckets,
            total_count: 0,
            total_sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    /// Creates a histogram with default latency buckets (ms).
    #[must_use]
    pub fn with_latency_buckets(id: MetricId) -> Self {
        Self::new(
            id,
            vec![
                1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 5000.0,
            ],
        )
    }

    /// Observes a value into the histogram.
    pub fn observe(&mut self, value: f64) {
        self.total_count += 1;
        self.total_sum += value;

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        // Increment all buckets whose upper bound >= value
        for bucket in &mut self.buckets {
            if value <= bucket.upper_bound {
                bucket.count += 1;
            }
        }
    }

    /// Total observations.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.total_count
    }

    /// Sum of all observations (Σ).
    #[must_use]
    pub fn sum(&self) -> f64 {
        self.total_sum
    }

    /// Mean value (Σ/N).
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.total_sum / self.total_count as f64
    }

    /// Minimum observed value.
    #[must_use]
    pub fn min(&self) -> f64 {
        if self.total_count == 0 { 0.0 } else { self.min }
    }

    /// Maximum observed value.
    #[must_use]
    pub fn max(&self) -> f64 {
        if self.total_count == 0 { 0.0 } else { self.max }
    }

    /// Estimates a percentile (0.0-1.0) from bucket boundaries.
    #[must_use]
    pub fn percentile(&self, p: f64) -> f64 {
        if self.total_count == 0 || self.buckets.is_empty() {
            return 0.0;
        }

        let target = (p * self.total_count as f64).ceil() as u64;
        for bucket in &self.buckets {
            if bucket.count >= target {
                return bucket.upper_bound;
            }
        }

        // Beyond all buckets — return max
        self.max
    }

    /// P50 estimate.
    #[must_use]
    pub fn p50(&self) -> f64 {
        self.percentile(0.50)
    }

    /// P95 estimate.
    #[must_use]
    pub fn p95(&self) -> f64 {
        self.percentile(0.95)
    }

    /// P99 estimate.
    #[must_use]
    pub fn p99(&self) -> f64 {
        self.percentile(0.99)
    }

    /// All buckets.
    #[must_use]
    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }
}

impl GroundsTo for Histogram {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Quantity,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Sum, 0.80)
    }
}

// ===============================================================
// METRIC DESCRIPTOR
// ===============================================================

/// Full metric descriptor — identity + kind + help text.
/// Tier: T2-P (Σ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDescriptor {
    /// Metric identity.
    pub id: MetricId,
    /// Metric kind.
    pub kind: MetricKind,
    /// Human-readable help text.
    pub help: String,
    /// Unit (e.g., "seconds", "bytes", "requests").
    pub unit: String,
}

impl MetricDescriptor {
    /// Creates a counter descriptor.
    #[must_use]
    pub fn counter(name: &str, help: &str) -> Self {
        Self {
            id: MetricId::new(name),
            kind: MetricKind::Counter,
            help: help.to_string(),
            unit: String::new(),
        }
    }

    /// Creates a gauge descriptor.
    #[must_use]
    pub fn gauge(name: &str, help: &str) -> Self {
        Self {
            id: MetricId::new(name),
            kind: MetricKind::Gauge,
            help: help.to_string(),
            unit: String::new(),
        }
    }

    /// Creates a histogram descriptor.
    #[must_use]
    pub fn histogram(name: &str, help: &str, unit: &str) -> Self {
        Self {
            id: MetricId::new(name),
            kind: MetricKind::Histogram,
            help: help.to_string(),
            unit: unit.to_string(),
        }
    }
}

impl GroundsTo for MetricDescriptor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Persistence])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_counter_monotonic() {
        let mut c = Counter::new(MetricId::new("test_total"));
        assert_eq!(c.value(), 0.0);

        c.inc();
        assert_eq!(c.value(), 1.0);

        c.inc_by(5.0);
        assert_eq!(c.value(), 6.0);

        // Negative values ignored — counters only go up
        c.inc_by(-3.0);
        assert_eq!(c.value(), 6.0);

        assert_eq!(c.increments(), 2); // Only 2 successful increments
    }

    #[test]
    fn test_gauge_up_and_down() {
        let mut g = Gauge::new(MetricId::new("active_workflows"));
        g.set(10.0);
        assert_eq!(g.value(), 10.0);

        g.inc();
        assert_eq!(g.value(), 11.0);

        g.dec();
        assert_eq!(g.value(), 10.0);

        g.set(5.0);
        assert_eq!(g.value(), 5.0);
    }

    #[test]
    fn test_gauge_average() {
        let mut g = Gauge::new(MetricId::new("latency"));
        g.set(10.0);
        g.set(20.0);
        g.set(30.0);
        assert!((g.average() - 20.0).abs() < f64::EPSILON);
        assert_eq!(g.observations(), 3);
    }

    #[test]
    fn test_histogram_observe() {
        let mut h = Histogram::new(
            MetricId::new("request_duration"),
            vec![10.0, 50.0, 100.0, 500.0],
        );

        h.observe(5.0);
        h.observe(25.0);
        h.observe(75.0);
        h.observe(200.0);

        assert_eq!(h.count(), 4);
        assert!((h.sum() - 305.0).abs() < f64::EPSILON);
        assert!((h.mean() - 76.25).abs() < f64::EPSILON);
        assert!((h.min() - 5.0).abs() < f64::EPSILON);
        assert!((h.max() - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_histogram_buckets() {
        let mut h = Histogram::new(MetricId::new("test"), vec![10.0, 50.0, 100.0]);

        // 3 values <= 10, 1 value in (10,50], 1 value in (50,100]
        h.observe(1.0);
        h.observe(5.0);
        h.observe(10.0);
        h.observe(30.0);
        h.observe(80.0);

        let buckets = h.buckets();
        assert_eq!(buckets[0].count, 3); // <= 10
        assert_eq!(buckets[1].count, 4); // <= 50
        assert_eq!(buckets[2].count, 5); // <= 100
    }

    #[test]
    fn test_histogram_percentiles() {
        let mut h = Histogram::with_latency_buckets(MetricId::new("latency"));

        // Observe 100 values evenly distributed
        for i in 1..=100 {
            h.observe(i as f64);
        }

        // P50 should be around 50ms bucket
        let p50 = h.p50();
        assert!(p50 >= 50.0 && p50 <= 100.0);

        // P99 should be in a high bucket
        let p99 = h.p99();
        assert!(p99 >= 100.0);
    }

    #[test]
    fn test_histogram_empty() {
        let h = Histogram::new(MetricId::new("empty"), vec![10.0]);
        assert_eq!(h.count(), 0);
        assert_eq!(h.mean(), 0.0);
        assert_eq!(h.p50(), 0.0);
        assert_eq!(h.min(), 0.0);
        assert_eq!(h.max(), 0.0);
    }

    #[test]
    fn test_labels() {
        let labels = Labels::from_pairs(&[("method", "POST"), ("endpoint", "/api/signals")]);
        assert_eq!(labels.len(), 2);
        assert_eq!(labels.get("method"), Some("POST"));
        assert_eq!(labels.get("endpoint"), Some("/api/signals"));
        assert_eq!(labels.get("missing"), None);
    }

    #[test]
    fn test_labels_display() {
        let empty = Labels::empty();
        assert_eq!(empty.display(), "{}");

        let labels = Labels::from_pairs(&[("k", "v")]);
        assert_eq!(labels.display(), "{k=v}");
    }

    #[test]
    fn test_metric_descriptor() {
        let d = MetricDescriptor::counter("requests_total", "Total requests served");
        assert_eq!(d.kind, MetricKind::Counter);
        assert_eq!(d.id.name(), "requests_total");
    }

    #[test]
    fn test_counter_grounding() {
        let comp = Counter::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sum));
    }

    #[test]
    fn test_histogram_grounding() {
        let comp = Histogram::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sum));
    }
}
