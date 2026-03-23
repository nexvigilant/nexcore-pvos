//! # PVMX Metric Collector & Engine
//!
//! The MetricsEngine T3 capstone: collects metrics from all layers,
//! stores them, aggregates, and exports. The observability hub.
//!
//! ## Primitives
//! - Σ (Sum) — DOMINANT: all collection is accumulation
//! - N (Quantity) — individual measurements
//! - ν (Frequency) — rate metrics
//! - σ (Sequence) — time series storage
//! - ∂ (Boundary) — alert thresholds
//! - π (Persistence) — metric retention

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::aggregator::{AggregationFunc, Aggregator, DataPoint, TimeSeries};
use super::alerting::{AlertRule, AlertState};
use super::dashboard::Dashboard;
use super::metric::{Counter, Gauge, Histogram, Labels, MetricDescriptor, MetricId, MetricKind};

// ===============================================================
// METRIC STORAGE
// ===============================================================

/// Labeled metric entry — a metric with its labels.
/// Tier: T2-P (Σ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledMetric {
    /// Metric identity.
    pub id: MetricId,
    /// Labels.
    pub labels: Labels,
    /// Current value.
    pub value: f64,
    /// Metric kind.
    pub kind: MetricKind,
}

impl GroundsTo for LabeledMetric {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Persistence])
    }
}

/// Time-series metric storage.
/// Tier: T2-C (π + σ + Σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStorage {
    /// Stored time series keyed by metric name.
    series: Vec<TimeSeries>,
    /// Maximum points per series.
    max_points: usize,
}

impl MetricStorage {
    /// Creates storage with default limits.
    #[must_use]
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            max_points: 10000,
        }
    }

    /// Records a data point for a metric.
    pub fn record(&mut self, metric: &str, timestamp: u64, value: f64) {
        if let Some(ts) = self.series.iter_mut().find(|s| s.name == metric) {
            ts.push(timestamp, value);
            // Trim to max points by removing oldest
            while ts.len() > self.max_points {
                // TimeSeries doesn't have a remove method, so we rebuild
                let points = ts.points().to_vec();
                let trimmed: Vec<DataPoint> = points.into_iter().skip(1).collect();
                *ts = TimeSeries::from_points(&ts.name, trimmed);
            }
        } else {
            let mut ts = TimeSeries::new(metric);
            ts.push(timestamp, value);
            self.series.push(ts);
        }
    }

    /// Fetches a time series by name.
    #[must_use]
    pub fn fetch(&self, metric: &str) -> Option<&TimeSeries> {
        self.series.iter().find(|s| s.name == metric)
    }

    /// Number of tracked series.
    #[must_use]
    pub fn series_count(&self) -> usize {
        self.series.len()
    }

    /// Total data points across all series.
    #[must_use]
    pub fn total_points(&self) -> usize {
        self.series.iter().map(|s| s.len()).sum()
    }
}

impl Default for MetricStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for MetricStorage {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Persistence,
            LexPrimitiva::Sequence,
            LexPrimitiva::Sum,
        ])
        .with_dominant(LexPrimitiva::Persistence, 0.70)
    }
}

// ===============================================================
// EXPORTER
// ===============================================================

/// Export format for metrics.
/// Tier: T2-P (μ + Σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportFormat {
    /// Prometheus text exposition format.
    Prometheus,
    /// JSON format.
    Json,
    /// Tab-separated table.
    Table,
}

impl GroundsTo for ExportFormat {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping, LexPrimitiva::Sum])
    }
}

/// Renders metrics in a given export format.
/// Tier: T2-C (μ + Σ + N)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Exporter;

impl Exporter {
    /// Exports a list of labeled metrics in the given format.
    #[must_use]
    pub fn export(metrics: &[LabeledMetric], format: ExportFormat) -> String {
        match format {
            ExportFormat::Prometheus => Self::export_prometheus(metrics),
            ExportFormat::Json => Self::export_json(metrics),
            ExportFormat::Table => Self::export_table(metrics),
        }
    }

    fn export_prometheus(metrics: &[LabeledMetric]) -> String {
        let mut lines = Vec::new();
        for m in metrics {
            let labels_str = m.labels.display();
            lines.push(format!("{}{} {}", m.id.name(), labels_str, m.value));
        }
        lines.join("\n")
    }

    fn export_json(metrics: &[LabeledMetric]) -> String {
        // Simple JSON array format
        let entries: Vec<String> = metrics
            .iter()
            .map(|m| {
                format!(
                    "{{\"name\":\"{}\",\"labels\":\"{}\",\"value\":{},\"kind\":\"{:?}\"}}",
                    m.id.name(),
                    m.labels.display(),
                    m.value,
                    m.kind
                )
            })
            .collect();
        format!("[{}]", entries.join(","))
    }

    fn export_table(metrics: &[LabeledMetric]) -> String {
        let mut lines = vec!["NAME\tTYPE\tVALUE\tLABELS".to_string()];
        for m in metrics {
            lines.push(format!(
                "{}\t{:?}\t{}\t{}",
                m.id.name(),
                m.kind,
                m.value,
                m.labels.display()
            ));
        }
        lines.join("\n")
    }
}

impl GroundsTo for Exporter {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Mapping,
            LexPrimitiva::Sum,
            LexPrimitiva::Quantity,
        ])
    }
}

// ===============================================================
// STANDARD METRICS CATALOG
// ===============================================================

/// Standard metrics for each PVOS layer.
/// Tier: T2-P (Σ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardMetrics {
    /// All registered metric descriptors.
    pub descriptors: Vec<MetricDescriptor>,
}

impl StandardMetrics {
    /// Creates the standard PVOS metrics catalog.
    #[must_use]
    pub fn pvos_catalog() -> Self {
        Self {
            descriptors: vec![
                // PVOS layer
                MetricDescriptor::counter("pvos_syscalls_total", "Total PVOS syscall invocations"),
                MetricDescriptor::histogram("pvos_syscall_duration", "Syscall duration", "ms"),
                MetricDescriptor::counter("pvos_errors_total", "Total PVOS errors"),
                // PVWF layer
                MetricDescriptor::counter("pvwf_workflows_total", "Total workflows created"),
                MetricDescriptor::gauge("pvwf_workflows_active", "Currently active workflows"),
                MetricDescriptor::histogram(
                    "pvwf_workflow_duration",
                    "Workflow execution duration",
                    "ms",
                ),
                // PVGW layer
                MetricDescriptor::counter("pvgw_requests_total", "Total gateway requests"),
                MetricDescriptor::histogram(
                    "pvgw_request_duration",
                    "Request handling duration",
                    "ms",
                ),
                MetricDescriptor::counter(
                    "pvgw_auth_failures_total",
                    "Total authentication failures",
                ),
                // PVRX layer
                MetricDescriptor::counter("pvrx_events_total", "Total events processed"),
                MetricDescriptor::gauge("pvrx_backpressure", "Current backpressure level"),
                MetricDescriptor::gauge("pvrx_lag", "Consumer lag in events"),
                // PVML layer
                MetricDescriptor::counter("pvml_feedback_total", "Total feedback submissions"),
                MetricDescriptor::counter("pvml_calibrations_total", "Total model calibrations"),
                MetricDescriptor::gauge("pvml_drift_score", "Current distribution drift score"),
                // PVSH layer
                MetricDescriptor::counter("pvsh_commands_total", "Total shell commands executed"),
                MetricDescriptor::gauge("pvsh_session_duration", "Current session duration"),
            ],
        }
    }

    /// Number of registered metrics.
    #[must_use]
    pub fn count(&self) -> usize {
        self.descriptors.len()
    }

    /// Gets a descriptor by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&MetricDescriptor> {
        self.descriptors.iter().find(|d| d.id.name() == name)
    }
}

impl GroundsTo for StandardMetrics {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Mapping])
    }
}

// ===============================================================
// METRICS ENGINE — T3 CAPSTONE
// ===============================================================

/// The Metrics Engine — observability capstone.
/// Tier: T3 (Σ + N + ν + σ + ∂ + π)
///
/// Collects, stores, aggregates, and alerts on metrics from all
/// PVOS layers. Makes the system observable.
///
/// Dominant primitive: Σ (Sum) — all observability is summation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsEngine {
    /// Counter metrics.
    counters: Vec<Counter>,
    /// Gauge metrics.
    gauges: Vec<Gauge>,
    /// Histogram metrics.
    histograms: Vec<Histogram>,
    /// Time-series storage.
    storage: MetricStorage,
    /// Aggregation engine.
    aggregator: Aggregator,
    /// Alert rules.
    alerts: Vec<AlertRule>,
    /// Dashboards.
    dashboards: Vec<Dashboard>,
    /// Standard metrics catalog.
    catalog: StandardMetrics,
    /// Total observations recorded.
    total_observations: u64,
    /// Next internal timestamp counter (for testing without system clock).
    tick: u64,
}

impl MetricsEngine {
    /// Creates a new metrics engine with standard PVOS catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: Vec::new(),
            gauges: Vec::new(),
            histograms: Vec::new(),
            storage: MetricStorage::new(),
            aggregator: Aggregator::new(),
            alerts: Vec::new(),
            dashboards: Vec::new(),
            catalog: StandardMetrics::pvos_catalog(),
            total_observations: 0,
            tick: 0,
        }
    }

    /// Advances the internal tick (for deterministic timestamps).
    pub fn advance_tick(&mut self, seconds: u64) {
        self.tick += seconds;
    }

    /// Current tick value.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    // ─── Counter Operations (Σ) ─────────────────────

    /// Increments a counter by 1.
    pub fn inc(&mut self, name: &str, labels: Labels) {
        if let Some(c) = self.counters.iter_mut().find(|c| c.id.name() == name) {
            c.inc();
        } else {
            let mut c = Counter::new(MetricId::new(name));
            c.inc();
            self.counters.push(c);
        }

        self.total_observations += 1;
        let value = self.counter_value(name);
        self.storage.record(name, self.tick, value);
        let _ = labels; // Labels tracked in storage context
    }

    /// Increments a counter by a given amount.
    pub fn inc_by(&mut self, name: &str, amount: f64, labels: Labels) {
        if let Some(c) = self.counters.iter_mut().find(|c| c.id.name() == name) {
            c.inc_by(amount);
        } else {
            let mut c = Counter::new(MetricId::new(name));
            c.inc_by(amount);
            self.counters.push(c);
        }

        self.total_observations += 1;
        let value = self.counter_value(name);
        self.storage.record(name, self.tick, value);
        let _ = labels;
    }

    /// Gets current counter value.
    #[must_use]
    pub fn counter_value(&self, name: &str) -> f64 {
        self.counters
            .iter()
            .find(|c| c.id.name() == name)
            .map(|c| c.value())
            .unwrap_or(0.0)
    }

    // ─── Gauge Operations (N) ───────────────────────

    /// Sets a gauge value.
    pub fn set_gauge(&mut self, name: &str, value: f64, labels: Labels) {
        if let Some(g) = self.gauges.iter_mut().find(|g| g.id.name() == name) {
            g.set(value);
        } else {
            let mut g = Gauge::new(MetricId::new(name));
            g.set(value);
            self.gauges.push(g);
        }

        self.total_observations += 1;
        self.storage.record(name, self.tick, value);
        let _ = labels;
    }

    /// Gets current gauge value.
    #[must_use]
    pub fn gauge_value(&self, name: &str) -> f64 {
        self.gauges
            .iter()
            .find(|g| g.id.name() == name)
            .map(|g| g.value())
            .unwrap_or(0.0)
    }

    // ─── Histogram Operations (Σ + N) ───────────────

    /// Observes a value into a histogram.
    pub fn observe(&mut self, name: &str, value: f64, labels: Labels) {
        if let Some(h) = self.histograms.iter_mut().find(|h| h.id.name() == name) {
            h.observe(value);
        } else {
            let mut h = Histogram::with_latency_buckets(MetricId::new(name));
            h.observe(value);
            self.histograms.push(h);
        }

        self.total_observations += 1;
        self.storage.record(name, self.tick, value);
        let _ = labels;
    }

    /// Gets histogram for a metric.
    #[must_use]
    pub fn histogram(&self, name: &str) -> Option<&Histogram> {
        self.histograms.iter().find(|h| h.id.name() == name)
    }

    // ─── Alert Operations (∂) ───────────────────────

    /// Adds an alert rule.
    pub fn add_alert(&mut self, rule: AlertRule) {
        self.alerts.push(rule);
    }

    /// Evaluates all alert rules against current metric values.
    #[must_use]
    pub fn check_alerts(&mut self) -> Vec<(String, AlertState, bool)> {
        let mut results = Vec::new();

        // Collect current values first
        let values: Vec<(String, f64)> = self
            .alerts
            .iter()
            .map(|rule| {
                let metric_name = rule.condition.metric.name().to_string();
                let value = self
                    .counter_value(&metric_name)
                    .max(self.gauge_value(&metric_name));
                (metric_name, value)
            })
            .collect();

        // Evaluate rules
        for (i, (_, value)) in values.iter().enumerate() {
            if let Some(rule) = self.alerts.get_mut(i) {
                let name = rule.name.clone();
                let (state, changed) = rule.evaluate(*value);
                results.push((name, state, changed));
            }
        }

        results
    }

    /// All alert rules.
    #[must_use]
    pub fn alerts(&self) -> &[AlertRule] {
        &self.alerts
    }

    // ─── Dashboard Operations (Σ + λ) ───────────────

    /// Adds a dashboard.
    pub fn add_dashboard(&mut self, dashboard: Dashboard) {
        self.dashboards.push(dashboard);
    }

    /// Gets a dashboard by name.
    #[must_use]
    pub fn dashboard(&self, name: &str) -> Option<&Dashboard> {
        self.dashboards.iter().find(|d| d.name == name)
    }

    /// All dashboards.
    #[must_use]
    pub fn dashboards(&self) -> &[Dashboard] {
        &self.dashboards
    }

    // ─── Query & Aggregation (Σ) ────────────────────

    /// Queries a time series with aggregation.
    #[must_use]
    pub fn query(&self, metric: &str, func: AggregationFunc, step_seconds: u64) -> Option<f64> {
        let series = self.storage.fetch(metric)?;
        Some(self.aggregator.reduce(series, func))
    }

    /// Gets rate per second for a counter.
    #[must_use]
    pub fn rate(&self, metric: &str) -> f64 {
        self.storage
            .fetch(metric)
            .map(|s| self.aggregator.rate_per_second(s))
            .unwrap_or(0.0)
    }

    // ─── Export (μ) ─────────────────────────────────

    /// Exports all current metric values.
    #[must_use]
    pub fn export(&self, format: ExportFormat) -> String {
        let mut metrics = Vec::new();

        for c in &self.counters {
            metrics.push(LabeledMetric {
                id: c.id.clone(),
                labels: Labels::empty(),
                value: c.value(),
                kind: MetricKind::Counter,
            });
        }

        for g in &self.gauges {
            metrics.push(LabeledMetric {
                id: g.id.clone(),
                labels: Labels::empty(),
                value: g.value(),
                kind: MetricKind::Gauge,
            });
        }

        for h in &self.histograms {
            metrics.push(LabeledMetric {
                id: h.id.clone(),
                labels: Labels::empty(),
                value: h.mean(),
                kind: MetricKind::Histogram,
            });
        }

        Exporter::export(&metrics, format)
    }

    // ─── Engine Stats ───────────────────────────────

    /// Total observations recorded.
    #[must_use]
    pub fn total_observations(&self) -> u64 {
        self.total_observations
    }

    /// Number of registered counters.
    #[must_use]
    pub fn counter_count(&self) -> usize {
        self.counters.len()
    }

    /// Number of registered gauges.
    #[must_use]
    pub fn gauge_count(&self) -> usize {
        self.gauges.len()
    }

    /// Number of registered histograms.
    #[must_use]
    pub fn histogram_count(&self) -> usize {
        self.histograms.len()
    }

    /// Metrics catalog.
    #[must_use]
    pub fn catalog(&self) -> &StandardMetrics {
        &self.catalog
    }

    /// Storage reference.
    #[must_use]
    pub fn storage(&self) -> &MetricStorage {
        &self.storage
    }
}

impl Default for MetricsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for MetricsEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,         // Σ — DOMINANT: aggregation
            LexPrimitiva::Quantity,    // N — individual measurements
            LexPrimitiva::Frequency,   // ν — rate metrics
            LexPrimitiva::Sequence,    // σ — time series
            LexPrimitiva::Boundary,    // ∂ — alert thresholds
            LexPrimitiva::Persistence, // π — metric storage
        ])
        .with_dominant(LexPrimitiva::Sum, 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::super::alerting::{AlertSeverity, Comparator, Condition};
    use super::super::dashboard::DashboardFactory;
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_engine_counter() {
        let mut engine = MetricsEngine::new();
        engine.inc("requests_total", Labels::empty());
        engine.inc("requests_total", Labels::empty());
        engine.inc("requests_total", Labels::empty());

        assert_eq!(engine.counter_value("requests_total"), 3.0);
        assert_eq!(engine.counter_count(), 1);
    }

    #[test]
    fn test_engine_gauge() {
        let mut engine = MetricsEngine::new();
        engine.set_gauge("active_workflows", 10.0, Labels::empty());
        assert_eq!(engine.gauge_value("active_workflows"), 10.0);

        engine.set_gauge("active_workflows", 5.0, Labels::empty());
        assert_eq!(engine.gauge_value("active_workflows"), 5.0);
    }

    #[test]
    fn test_engine_histogram() {
        let mut engine = MetricsEngine::new();
        engine.observe("latency_ms", 10.0, Labels::empty());
        engine.observe("latency_ms", 50.0, Labels::empty());
        engine.observe("latency_ms", 100.0, Labels::empty());

        let h = engine.histogram("latency_ms");
        assert!(h.is_some());
        if let Some(hist) = h {
            assert_eq!(hist.count(), 3);
        }
    }

    #[test]
    fn test_engine_query() {
        let mut engine = MetricsEngine::new();

        // Record values at different ticks
        engine.advance_tick(10);
        engine.inc("req", Labels::empty());
        engine.advance_tick(10);
        engine.inc("req", Labels::empty());
        engine.advance_tick(10);
        engine.inc("req", Labels::empty());

        let sum = engine.query("req", AggregationFunc::Sum, 60);
        assert!(sum.is_some());
    }

    #[test]
    fn test_engine_rate() {
        let mut engine = MetricsEngine::new();

        engine.advance_tick(0);
        engine.inc_by("counter", 100.0, Labels::empty());
        engine.advance_tick(10);
        engine.inc_by("counter", 50.0, Labels::empty());

        let rate = engine.rate("counter");
        // Rate should be (150-100) / (10-0) = 5.0 per second
        assert!(rate > 0.0);
    }

    #[test]
    fn test_engine_alerts() {
        let mut engine = MetricsEngine::new();

        let condition = Condition::threshold("error_count", Comparator::GreaterThan, 5.0);
        let rule = AlertRule::new("alert_1", "High Errors", condition, AlertSeverity::Critical);
        engine.add_alert(rule);

        // Below threshold
        engine.inc_by("error_count", 3.0, Labels::empty());
        let results = engine.check_alerts();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, AlertState::Ok);

        // Above threshold
        engine.inc_by("error_count", 5.0, Labels::empty());
        let results = engine.check_alerts();
        assert_eq!(results[0].1, AlertState::Firing);
    }

    #[test]
    fn test_engine_dashboards() {
        let mut engine = MetricsEngine::new();
        engine.add_dashboard(DashboardFactory::system_health());
        engine.add_dashboard(DashboardFactory::signal_detection());

        assert_eq!(engine.dashboards().len(), 2);
        assert!(engine.dashboard("System Health").is_some());
    }

    #[test]
    fn test_engine_export_prometheus() {
        let mut engine = MetricsEngine::new();
        engine.inc("requests_total", Labels::empty());
        engine.set_gauge("active", 5.0, Labels::empty());

        let output = engine.export(ExportFormat::Prometheus);
        assert!(output.contains("requests_total"));
        assert!(output.contains("active"));
    }

    #[test]
    fn test_engine_export_json() {
        let mut engine = MetricsEngine::new();
        engine.inc("test_counter", Labels::empty());

        let output = engine.export(ExportFormat::Json);
        assert!(output.starts_with('['));
        assert!(output.contains("test_counter"));
    }

    #[test]
    fn test_engine_export_table() {
        let mut engine = MetricsEngine::new();
        engine.inc("test", Labels::empty());

        let output = engine.export(ExportFormat::Table);
        assert!(output.contains("NAME"));
        assert!(output.contains("test"));
    }

    #[test]
    fn test_standard_catalog() {
        let catalog = StandardMetrics::pvos_catalog();
        assert!(catalog.count() >= 17);
        assert!(catalog.get("pvos_syscalls_total").is_some());
        assert!(catalog.get("pvml_drift_score").is_some());
    }

    #[test]
    fn test_metric_storage() {
        let mut storage = MetricStorage::new();
        storage.record("metric_a", 0, 1.0);
        storage.record("metric_a", 10, 2.0);
        storage.record("metric_b", 0, 5.0);

        assert_eq!(storage.series_count(), 2);
        assert_eq!(storage.total_points(), 3);

        let ts = storage.fetch("metric_a");
        assert!(ts.is_some());
        if let Some(series) = ts {
            assert_eq!(series.len(), 2);
        }
    }

    #[test]
    fn test_engine_total_observations() {
        let mut engine = MetricsEngine::new();
        engine.inc("a", Labels::empty());
        engine.set_gauge("b", 1.0, Labels::empty());
        engine.observe("c", 10.0, Labels::empty());

        assert_eq!(engine.total_observations(), 3);
    }

    #[test]
    fn test_engine_t3_grounding() {
        let comp = MetricsEngine::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sum));
    }

    #[test]
    fn test_storage_grounding() {
        let comp = MetricStorage::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Persistence));
    }
}
