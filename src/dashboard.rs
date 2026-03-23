//! # PVMX Dashboard Definitions
//!
//! Dashboard structures for visualizing metrics. Each panel queries
//! metrics and renders aggregated views — all Σ-based.
//!
//! ## Primitives
//! - Σ (Sum) — all panels display aggregated metrics
//! - λ (Location) — dashboard navigation
//! - μ (Mapping) — query → visualization mapping

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::aggregator::AggregationFunc;
use super::metric::MetricId;

// ===============================================================
// VISUALIZATION
// ===============================================================

/// How to render a metric panel.
/// Tier: T2-P (Σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visualization {
    /// Line chart (time series).
    Line,
    /// Bar chart.
    Bar,
    /// Single gauge value.
    GaugeDisplay,
    /// Table of values.
    Table,
    /// Heatmap.
    Heatmap,
    /// Single number display.
    Stat,
}

impl GroundsTo for Visualization {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum])
    }
}

// ===============================================================
// QUERY
// ===============================================================

/// Time range for a query.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeRange {
    /// Last N minutes.
    LastMinutes(u64),
    /// Last N hours.
    LastHours(u64),
    /// Last N days.
    LastDays(u64),
    /// Custom range.
    Custom { start: u64, end: u64 },
}

impl TimeRange {
    /// Duration in seconds.
    #[must_use]
    pub fn duration_seconds(&self) -> u64 {
        match self {
            Self::LastMinutes(n) => n * 60,
            Self::LastHours(n) => n * 3600,
            Self::LastDays(n) => n * 86400,
            Self::Custom { start, end } => end.saturating_sub(*start),
        }
    }
}

impl GroundsTo for TimeRange {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

/// A metric query — selects and aggregates data.
/// Tier: T2-C (Σ + σ + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// Target metric.
    pub metric: MetricId,
    /// Aggregation function.
    pub func: AggregationFunc,
    /// Time range.
    pub range: TimeRange,
    /// Step size in seconds (for rollups).
    pub step_seconds: u64,
    /// Optional label filter.
    pub label_filter: Vec<(String, String)>,
}

impl Query {
    /// Creates a simple query with defaults.
    #[must_use]
    pub fn simple(metric: &str, func: AggregationFunc) -> Self {
        Self {
            metric: MetricId::new(metric),
            func,
            range: TimeRange::LastHours(1),
            step_seconds: 60,
            label_filter: Vec::new(),
        }
    }

    /// Sets the time range.
    #[must_use]
    pub fn with_range(mut self, range: TimeRange) -> Self {
        self.range = range;
        self
    }

    /// Sets the step size.
    #[must_use]
    pub fn with_step(mut self, step: u64) -> Self {
        self.step_seconds = step;
        self
    }

    /// Adds a label filter.
    #[must_use]
    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.label_filter.push((key.to_string(), value.to_string()));
        self
    }
}

impl GroundsTo for Query {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Sequence,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Sum, 0.75)
    }
}

// ===============================================================
// PANEL
// ===============================================================

/// A single dashboard panel displaying one metric view.
/// Tier: T2-C (Σ + μ + λ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    /// Panel title.
    pub title: String,
    /// The query driving this panel.
    pub query: Query,
    /// How to render.
    pub visualization: Visualization,
    /// Panel width (grid units, 1-12).
    pub width: u8,
}

impl Panel {
    /// Creates a panel.
    #[must_use]
    pub fn new(title: &str, query: Query, vis: Visualization) -> Self {
        Self {
            title: title.to_string(),
            query,
            visualization: vis,
            width: 6,
        }
    }

    /// Sets the panel width.
    #[must_use]
    pub fn with_width(mut self, width: u8) -> Self {
        self.width = width.clamp(1, 12);
        self
    }
}

impl GroundsTo for Panel {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Mapping,
            LexPrimitiva::Location,
        ])
        .with_dominant(LexPrimitiva::Sum, 0.75)
    }
}

// ===============================================================
// DASHBOARD
// ===============================================================

/// A collection of panels forming a dashboard view.
/// Tier: T2-C (Σ + μ + λ + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    /// Dashboard name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Panels in this dashboard.
    pub panels: Vec<Panel>,
    /// Default time range for all panels.
    pub default_range: TimeRange,
    /// Auto-refresh interval in seconds (0 = no auto-refresh).
    pub refresh_seconds: u64,
}

impl Dashboard {
    /// Creates an empty dashboard.
    #[must_use]
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            panels: Vec::new(),
            default_range: TimeRange::LastHours(1),
            refresh_seconds: 30,
        }
    }

    /// Adds a panel.
    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    /// Number of panels.
    #[must_use]
    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    /// Sets the default time range.
    #[must_use]
    pub fn with_range(mut self, range: TimeRange) -> Self {
        self.default_range = range;
        self
    }

    /// Sets refresh interval.
    #[must_use]
    pub fn with_refresh(mut self, seconds: u64) -> Self {
        self.refresh_seconds = seconds;
        self
    }
}

impl GroundsTo for Dashboard {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Mapping,
            LexPrimitiva::Location,
            LexPrimitiva::Sequence,
        ])
        .with_dominant(LexPrimitiva::Sum, 0.75)
    }
}

// ===============================================================
// PRE-BUILT DASHBOARDS
// ===============================================================

/// Factory for standard PVOS dashboards.
/// Tier: T2-C (Σ + μ + λ + σ)
pub struct DashboardFactory;

impl DashboardFactory {
    /// System health overview dashboard.
    #[must_use]
    pub fn system_health() -> Dashboard {
        let mut d = Dashboard::new("System Health", "Overall PVOS health and performance");

        d.add_panel(Panel::new(
            "Syscall Rate",
            Query::simple("pvos_syscalls_total", AggregationFunc::Rate),
            Visualization::Line,
        ));

        d.add_panel(Panel::new(
            "Active Workflows",
            Query::simple("pvwf_workflows_active", AggregationFunc::Avg),
            Visualization::Stat,
        ));

        d.add_panel(Panel::new(
            "Error Rate",
            Query::simple("pvos_errors_total", AggregationFunc::Rate)
                .with_range(TimeRange::LastMinutes(15)),
            Visualization::Line,
        ));

        d.add_panel(Panel::new(
            "Gateway Latency P95",
            Query::simple("pvgw_request_duration", AggregationFunc::P95),
            Visualization::Line,
        ));

        d
    }

    /// Signal detection dashboard.
    #[must_use]
    pub fn signal_detection() -> Dashboard {
        let mut d = Dashboard::new("Signal Detection", "PV signal detection performance");

        d.add_panel(Panel::new(
            "Signals Detected Today",
            Query::simple("pvos_signals_detected", AggregationFunc::Sum)
                .with_range(TimeRange::LastHours(24)),
            Visualization::Stat,
        ));

        d.add_panel(Panel::new(
            "Detection Rate",
            Query::simple("pvos_syscalls_total", AggregationFunc::Rate)
                .with_label("kind", "detect"),
            Visualization::Line,
        ));

        d.add_panel(Panel::new(
            "False Positive Rate",
            Query::simple("pvml_feedback_total", AggregationFunc::Rate)
                .with_label("outcome", "false_positive"),
            Visualization::Line,
        ));

        d
    }

    /// Workflow status dashboard.
    #[must_use]
    pub fn workflow_status() -> Dashboard {
        let mut d = Dashboard::new("Workflow Status", "PVWF workflow monitoring");

        d.add_panel(Panel::new(
            "Workflows Running",
            Query::simple("pvwf_workflows_active", AggregationFunc::Avg),
            Visualization::GaugeDisplay,
        ));

        d.add_panel(Panel::new(
            "Workflow Duration P95",
            Query::simple("pvwf_workflow_duration", AggregationFunc::P95),
            Visualization::Line,
        ));

        d.add_panel(Panel::new(
            "Completed Today",
            Query::simple("pvwf_workflows_completed", AggregationFunc::Sum)
                .with_range(TimeRange::LastHours(24)),
            Visualization::Stat,
        ));

        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_query_simple() {
        let q = Query::simple("test_metric", AggregationFunc::Sum);
        assert_eq!(q.metric.name(), "test_metric");
        assert_eq!(q.func, AggregationFunc::Sum);
    }

    #[test]
    fn test_query_builder() {
        let q = Query::simple("m", AggregationFunc::Avg)
            .with_range(TimeRange::LastMinutes(30))
            .with_step(15)
            .with_label("env", "prod");

        assert_eq!(q.step_seconds, 15);
        assert_eq!(q.label_filter.len(), 1);
    }

    #[test]
    fn test_time_range_duration() {
        assert_eq!(TimeRange::LastMinutes(5).duration_seconds(), 300);
        assert_eq!(TimeRange::LastHours(2).duration_seconds(), 7200);
        assert_eq!(TimeRange::LastDays(1).duration_seconds(), 86400);
    }

    #[test]
    fn test_panel_creation() {
        let p = Panel::new(
            "Test Panel",
            Query::simple("m", AggregationFunc::Sum),
            Visualization::Line,
        );
        assert_eq!(p.title, "Test Panel");
        assert_eq!(p.width, 6);
    }

    #[test]
    fn test_panel_width_clamp() {
        let p = Panel::new(
            "t",
            Query::simple("m", AggregationFunc::Sum),
            Visualization::Bar,
        )
        .with_width(20);
        assert_eq!(p.width, 12); // Clamped
    }

    #[test]
    fn test_dashboard_creation() {
        let mut d = Dashboard::new("Test", "Test dashboard");
        assert_eq!(d.panel_count(), 0);

        d.add_panel(Panel::new(
            "P1",
            Query::simple("m", AggregationFunc::Sum),
            Visualization::Line,
        ));
        assert_eq!(d.panel_count(), 1);
    }

    #[test]
    fn test_system_health_dashboard() {
        let d = DashboardFactory::system_health();
        assert_eq!(d.name, "System Health");
        assert_eq!(d.panel_count(), 4);
    }

    #[test]
    fn test_signal_detection_dashboard() {
        let d = DashboardFactory::signal_detection();
        assert_eq!(d.name, "Signal Detection");
        assert_eq!(d.panel_count(), 3);
    }

    #[test]
    fn test_workflow_status_dashboard() {
        let d = DashboardFactory::workflow_status();
        assert_eq!(d.name, "Workflow Status");
        assert_eq!(d.panel_count(), 3);
    }

    #[test]
    fn test_dashboard_grounding() {
        let comp = Dashboard::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sum));
    }

    #[test]
    fn test_query_grounding() {
        let comp = Query::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sum));
    }
}
