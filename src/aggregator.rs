//! # PVMX Aggregation Engine
//!
//! Computes rollups, aggregations, and time-series transformations.
//! The core Σ-machine: takes sequences of values and reduces them.
//!
//! ## Primitives
//! - Σ (Sum) — all aggregation functions reduce to sums
//! - σ (Sequence) — time series ordering
//! - ν (Frequency) — rate calculations
//! - N (Quantity) — individual data points

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// TIME SERIES
// ===============================================================

/// A single data point in a time series.
/// Tier: T2-P (Σ + σ)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DataPoint {
    /// Timestamp (seconds since epoch).
    pub timestamp: u64,
    /// Value at this point.
    pub value: f64,
}

impl GroundsTo for DataPoint {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Sequence])
    }
}

/// A sequence of timestamped values.
/// Tier: T2-C (σ + Σ + N + ν)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    /// Metric name this series belongs to.
    pub name: String,
    /// Ordered data points.
    points: Vec<DataPoint>,
}

impl TimeSeries {
    /// Creates an empty time series.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            points: Vec::new(),
        }
    }

    /// Creates a time series from existing points.
    #[must_use]
    pub fn from_points(name: &str, points: Vec<DataPoint>) -> Self {
        Self {
            name: name.to_string(),
            points,
        }
    }

    /// Adds a data point. Points are kept sorted by timestamp.
    pub fn push(&mut self, timestamp: u64, value: f64) {
        self.points.push(DataPoint { timestamp, value });
        // Maintain sorted order
        self.points.sort_by_key(|p| p.timestamp);
    }

    /// All data points.
    #[must_use]
    pub fn points(&self) -> &[DataPoint] {
        &self.points
    }

    /// Number of points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Last data point.
    #[must_use]
    pub fn last(&self) -> Option<&DataPoint> {
        self.points.last()
    }

    /// First data point.
    #[must_use]
    pub fn first(&self) -> Option<&DataPoint> {
        self.points.first()
    }

    /// Time range covered (last - first timestamp).
    #[must_use]
    pub fn time_range(&self) -> u64 {
        match (self.first(), self.last()) {
            (Some(f), Some(l)) => l.timestamp.saturating_sub(f.timestamp),
            _ => 0,
        }
    }

    /// All values as a vec.
    #[must_use]
    pub fn values(&self) -> Vec<f64> {
        self.points.iter().map(|p| p.value).collect()
    }
}

impl GroundsTo for TimeSeries {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Sum,
            LexPrimitiva::Quantity,
            LexPrimitiva::Frequency,
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.70)
    }
}

// ===============================================================
// AGGREGATION FUNCTIONS
// ===============================================================

/// Aggregation function to apply over a window of values.
/// Tier: T2-P (Σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregationFunc {
    /// Sum of values (Σ).
    Sum,
    /// Average (Σ/N).
    Avg,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
    /// Count of values.
    Count,
    /// 50th percentile.
    P50,
    /// 95th percentile.
    P95,
    /// 99th percentile.
    P99,
    /// Rate of change per second (Δ/Δt).
    Rate,
}

impl AggregationFunc {
    /// Applies this function to a slice of values.
    #[must_use]
    pub fn apply(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        match self {
            Self::Sum => values.iter().sum(),
            Self::Avg => values.iter().sum::<f64>() / values.len() as f64,
            Self::Min => values.iter().copied().fold(f64::MAX, f64::min),
            Self::Max => values.iter().copied().fold(f64::MIN, f64::max),
            Self::Count => values.len() as f64,
            Self::P50 => percentile_of(values, 0.50),
            Self::P95 => percentile_of(values, 0.95),
            Self::P99 => percentile_of(values, 0.99),
            Self::Rate => {
                // Simple rate: (last - first) / count
                if values.len() < 2 {
                    return 0.0;
                }
                let first = values[0];
                let last = values[values.len() - 1];
                (last - first) / (values.len() - 1) as f64
            }
        }
    }

    /// Display name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Min => "min",
            Self::Max => "max",
            Self::Count => "count",
            Self::P50 => "p50",
            Self::P95 => "p95",
            Self::P99 => "p99",
            Self::Rate => "rate",
        }
    }
}

impl GroundsTo for AggregationFunc {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum])
    }
}

/// Computes the p-th percentile (0.0-1.0) of a value slice.
fn percentile_of(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx]
}

// ===============================================================
// ROLLUP
// ===============================================================

/// An aggregated time series at reduced resolution.
/// Tier: T2-C (Σ + σ + ν)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rollup {
    /// Source metric name.
    pub source: String,
    /// Aggregation function used.
    pub func: AggregationFunc,
    /// Step size in seconds.
    pub step_seconds: u64,
    /// Aggregated data points.
    pub points: Vec<DataPoint>,
}

impl GroundsTo for Rollup {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Sequence,
            LexPrimitiva::Frequency,
        ])
        .with_dominant(LexPrimitiva::Sum, 0.80)
    }
}

// ===============================================================
// AGGREGATOR
// ===============================================================

/// The aggregation engine — reduces time series to summaries.
/// Tier: T2-C (Σ + σ + ν + N)
///
/// Takes raw time series and produces rollups at requested resolution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Aggregator {
    /// Maximum points to retain in a rollup.
    max_rollup_points: usize,
}

impl Aggregator {
    /// Creates an aggregator with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_rollup_points: 1000,
        }
    }

    /// Creates an aggregator with custom max points.
    #[must_use]
    pub fn with_max_points(max: usize) -> Self {
        Self {
            max_rollup_points: max.max(1),
        }
    }

    /// Aggregates a time series into a rollup at the given step.
    #[must_use]
    pub fn aggregate(
        &self,
        series: &TimeSeries,
        func: AggregationFunc,
        step_seconds: u64,
    ) -> Rollup {
        if series.is_empty() || step_seconds == 0 {
            return Rollup {
                source: series.name.clone(),
                func,
                step_seconds,
                points: Vec::new(),
            };
        }

        let start = series.first().map(|p| p.timestamp).unwrap_or(0);
        let end = series.last().map(|p| p.timestamp).unwrap_or(0);

        let mut result_points = Vec::new();
        let mut window_start = start;

        while window_start <= end && result_points.len() < self.max_rollup_points {
            let window_end = window_start + step_seconds;

            let window_values: Vec<f64> = series
                .points()
                .iter()
                .filter(|p| p.timestamp >= window_start && p.timestamp < window_end)
                .map(|p| p.value)
                .collect();

            if !window_values.is_empty() {
                let agg_value = func.apply(&window_values);
                result_points.push(DataPoint {
                    timestamp: window_start,
                    value: agg_value,
                });
            }

            window_start = window_end;
        }

        Rollup {
            source: series.name.clone(),
            func,
            step_seconds,
            points: result_points,
        }
    }

    /// Computes a single aggregate value over the entire series.
    #[must_use]
    pub fn reduce(&self, series: &TimeSeries, func: AggregationFunc) -> f64 {
        func.apply(&series.values())
    }

    /// Computes rate of change per second from a counter time series.
    #[must_use]
    pub fn rate_per_second(&self, series: &TimeSeries) -> f64 {
        if series.len() < 2 {
            return 0.0;
        }

        let first = series.first();
        let last = series.last();

        match (first, last) {
            (Some(f), Some(l)) => {
                let dt = (l.timestamp - f.timestamp) as f64;
                if dt <= 0.0 {
                    return 0.0;
                }
                (l.value - f.value) / dt
            }
            _ => 0.0,
        }
    }
}

impl GroundsTo for Aggregator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Sequence,
            LexPrimitiva::Frequency,
            LexPrimitiva::Quantity,
        ])
        .with_dominant(LexPrimitiva::Sum, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn sample_series() -> TimeSeries {
        TimeSeries::from_points(
            "test_metric",
            vec![
                DataPoint {
                    timestamp: 0,
                    value: 1.0,
                },
                DataPoint {
                    timestamp: 10,
                    value: 3.0,
                },
                DataPoint {
                    timestamp: 20,
                    value: 5.0,
                },
                DataPoint {
                    timestamp: 30,
                    value: 7.0,
                },
                DataPoint {
                    timestamp: 40,
                    value: 9.0,
                },
            ],
        )
    }

    #[test]
    fn test_time_series_basics() {
        let mut ts = TimeSeries::new("test");
        assert!(ts.is_empty());

        ts.push(10, 1.0);
        ts.push(20, 2.0);
        assert_eq!(ts.len(), 2);
        assert_eq!(ts.time_range(), 10);
    }

    #[test]
    fn test_time_series_sorted() {
        let mut ts = TimeSeries::new("test");
        ts.push(30, 3.0);
        ts.push(10, 1.0);
        ts.push(20, 2.0);

        let points = ts.points();
        assert_eq!(points[0].timestamp, 10);
        assert_eq!(points[1].timestamp, 20);
        assert_eq!(points[2].timestamp, 30);
    }

    #[test]
    fn test_aggregation_sum() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((AggregationFunc::Sum.apply(&values) - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregation_avg() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((AggregationFunc::Avg.apply(&values) - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregation_min_max() {
        let values = vec![3.0, 1.0, 4.0, 1.5, 9.0];
        assert!((AggregationFunc::Min.apply(&values) - 1.0).abs() < f64::EPSILON);
        assert!((AggregationFunc::Max.apply(&values) - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregation_percentiles() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let p50 = AggregationFunc::P50.apply(&values);
        assert!((p50 - 50.0).abs() <= 1.0);

        let p95 = AggregationFunc::P95.apply(&values);
        assert!((p95 - 95.0).abs() <= 1.0);
    }

    #[test]
    fn test_aggregation_empty() {
        let empty: Vec<f64> = Vec::new();
        assert_eq!(AggregationFunc::Sum.apply(&empty), 0.0);
        assert_eq!(AggregationFunc::Avg.apply(&empty), 0.0);
    }

    #[test]
    fn test_aggregator_rollup() {
        let series = sample_series();
        let agg = Aggregator::new();

        // Aggregate with 20-second steps
        let rollup = agg.aggregate(&series, AggregationFunc::Avg, 20);
        assert!(!rollup.points.is_empty());
        assert_eq!(rollup.func, AggregationFunc::Avg);
        assert_eq!(rollup.step_seconds, 20);
    }

    #[test]
    fn test_aggregator_reduce() {
        let series = sample_series();
        let agg = Aggregator::new();

        let sum = agg.reduce(&series, AggregationFunc::Sum);
        assert!((sum - 25.0).abs() < f64::EPSILON); // 1+3+5+7+9

        let avg = agg.reduce(&series, AggregationFunc::Avg);
        assert!((avg - 5.0).abs() < f64::EPSILON); // 25/5
    }

    #[test]
    fn test_aggregator_rate() {
        let series = sample_series();
        let agg = Aggregator::new();

        // (9-1) / (40-0) = 8/40 = 0.2 per second
        let rate = agg.rate_per_second(&series);
        assert!((rate - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregator_grounding() {
        let comp = Aggregator::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sum));
    }

    #[test]
    fn test_time_series_grounding() {
        let comp = TimeSeries::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sequence));
    }
}
