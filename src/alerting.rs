//! # PVMX Threshold Alerting
//!
//! Alert rules evaluate metric values against thresholds and transition
//! through a state machine with hysteresis to prevent flapping.
//!
//! ## Primitives
//! - Σ (Sum) — metric values being evaluated
//! - ∂ (Boundary) — threshold conditions
//! - ν (Frequency) — rate-based alerts
//! - ς (State) — alert state machine

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::metric::MetricId;

// ===============================================================
// ALERT IDENTITY
// ===============================================================

/// Unique alert rule identifier.
/// Tier: T2-P (Σ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlertRuleId(pub String);

impl AlertRuleId {
    /// Creates an alert rule ID.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl GroundsTo for AlertRuleId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum])
    }
}

// ===============================================================
// COMPARATOR
// ===============================================================

/// How to compare a metric value against a threshold.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Comparator {
    /// Value > threshold.
    GreaterThan,
    /// Value >= threshold.
    GreaterOrEqual,
    /// Value < threshold.
    LessThan,
    /// Value <= threshold.
    LessOrEqual,
    /// Value == threshold (within epsilon).
    Equal,
}

impl Comparator {
    /// Evaluates the comparison.
    #[must_use]
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThan => value > threshold,
            Self::GreaterOrEqual => value >= threshold,
            Self::LessThan => value < threshold,
            Self::LessOrEqual => value <= threshold,
            Self::Equal => (value - threshold).abs() < f64::EPSILON,
        }
    }

    /// Display symbol.
    #[must_use]
    pub fn symbol(&self) -> &str {
        match self {
            Self::GreaterThan => ">",
            Self::GreaterOrEqual => ">=",
            Self::LessThan => "<",
            Self::LessOrEqual => "<=",
            Self::Equal => "==",
        }
    }
}

impl GroundsTo for Comparator {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

// ===============================================================
// CONDITION
// ===============================================================

/// A threshold condition — when to trigger an alert.
/// Tier: T2-P (Σ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Which metric to evaluate.
    pub metric: MetricId,
    /// How to compare.
    pub comparator: Comparator,
    /// Threshold value.
    pub threshold: f64,
    /// Number of consecutive evaluations that must fail before firing.
    /// This provides hysteresis — prevents flapping.
    pub for_count: u32,
}

impl Condition {
    /// Creates a simple threshold condition.
    #[must_use]
    pub fn threshold(metric: &str, cmp: Comparator, threshold: f64) -> Self {
        Self {
            metric: MetricId::new(metric),
            comparator: cmp,
            threshold,
            for_count: 1,
        }
    }

    /// Sets the hysteresis count (consecutive failures before firing).
    #[must_use]
    pub fn with_for_count(mut self, count: u32) -> Self {
        self.for_count = count.max(1);
        self
    }

    /// Evaluates this condition against a value.
    #[must_use]
    pub fn evaluate(&self, value: f64) -> bool {
        self.comparator.evaluate(value, self.threshold)
    }

    /// Human-readable description.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "{} {} {}",
            self.metric.name(),
            self.comparator.symbol(),
            self.threshold
        )
    }
}

impl GroundsTo for Condition {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sum, LexPrimitiva::Boundary])
    }
}

// ===============================================================
// ALERT SEVERITY
// ===============================================================

/// How severe an alert is.
/// Tier: T2-P (∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational — worth knowing.
    Info,
    /// Warning — needs attention soon.
    Warning,
    /// Critical — immediate action required.
    Critical,
}

impl GroundsTo for AlertSeverity {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Boundary])
    }
}

// ===============================================================
// ALERT STATE MACHINE
// ===============================================================

/// Alert lifecycle state with hysteresis.
/// Tier: T2-P (ς + ∂)
///
/// State machine: OK → Pending → Firing → Resolved → OK
/// Hysteresis: must stay in violation for `for_count` consecutive
/// evaluations before transitioning from Pending to Firing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertState {
    /// Normal — condition not met.
    Ok,
    /// Condition met but not yet for `for_count` — hysteresis.
    Pending,
    /// Alert is actively firing.
    Firing,
    /// Was firing, now resolved.
    Resolved,
}

impl GroundsTo for AlertState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State, LexPrimitiva::Boundary])
    }
}

// ===============================================================
// NOTIFICATION
// ===============================================================

/// Where to send alert notifications.
/// Tier: T2-P (μ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationTarget {
    /// Log to audit trail.
    Log,
    /// Send to a named channel.
    Channel(String),
    /// Publish to a PVRX topic.
    Stream(String),
}

impl GroundsTo for NotificationTarget {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping])
    }
}

// ===============================================================
// ALERT RULE
// ===============================================================

/// A complete alert rule — condition + severity + targets.
/// Tier: T2-C (Σ + ∂ + ς + μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule identity.
    pub id: AlertRuleId,
    /// Name for display.
    pub name: String,
    /// Condition to evaluate.
    pub condition: Condition,
    /// Severity when firing.
    pub severity: AlertSeverity,
    /// Current state.
    state: AlertState,
    /// Consecutive violation count (for hysteresis).
    consecutive_violations: u32,
    /// Where to notify.
    pub targets: Vec<NotificationTarget>,
    /// Total times this rule has fired.
    total_fires: u64,
}

impl AlertRule {
    /// Creates a new alert rule.
    #[must_use]
    pub fn new(id: &str, name: &str, condition: Condition, severity: AlertSeverity) -> Self {
        Self {
            id: AlertRuleId::new(id),
            name: name.to_string(),
            condition,
            severity,
            state: AlertState::Ok,
            consecutive_violations: 0,
            targets: vec![NotificationTarget::Log],
            total_fires: 0,
        }
    }

    /// Adds a notification target.
    #[must_use]
    pub fn with_target(mut self, target: NotificationTarget) -> Self {
        self.targets.push(target);
        self
    }

    /// Current alert state.
    #[must_use]
    pub fn state(&self) -> AlertState {
        self.state
    }

    /// Total times fired.
    #[must_use]
    pub fn total_fires(&self) -> u64 {
        self.total_fires
    }

    /// Evaluates the rule with a new metric value.
    /// Returns the new state and whether a transition occurred.
    pub fn evaluate(&mut self, value: f64) -> (AlertState, bool) {
        let condition_met = self.condition.evaluate(value);
        let old_state = self.state;

        match (self.state, condition_met) {
            // OK + condition met → start hysteresis
            (AlertState::Ok, true) => {
                self.consecutive_violations = 1;
                if self.condition.for_count <= 1 {
                    self.state = AlertState::Firing;
                    self.total_fires += 1;
                } else {
                    self.state = AlertState::Pending;
                }
            }
            // Pending + condition still met → increment counter
            (AlertState::Pending, true) => {
                self.consecutive_violations += 1;
                if self.consecutive_violations >= self.condition.for_count {
                    self.state = AlertState::Firing;
                    self.total_fires += 1;
                }
            }
            // Pending + condition no longer met → back to OK
            (AlertState::Pending, false) => {
                self.consecutive_violations = 0;
                self.state = AlertState::Ok;
            }
            // Firing + condition still met → stay firing
            (AlertState::Firing, true) => {
                // Stay firing
            }
            // Firing + condition resolved → transition to resolved
            (AlertState::Firing, false) => {
                self.consecutive_violations = 0;
                self.state = AlertState::Resolved;
            }
            // Resolved + condition met → back to firing
            (AlertState::Resolved, true) => {
                self.consecutive_violations = 1;
                self.state = AlertState::Firing;
                self.total_fires += 1;
            }
            // Resolved + still clear → back to OK
            (AlertState::Resolved, false) => {
                self.state = AlertState::Ok;
            }
            // OK + still clear → nothing
            (AlertState::Ok, false) => {}
        }

        let transitioned = self.state != old_state;
        (self.state, transitioned)
    }

    /// Resets the alert to OK state.
    pub fn reset(&mut self) {
        self.state = AlertState::Ok;
        self.consecutive_violations = 0;
    }

    /// Human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "[{:?}] {} — {} (fires={})",
            self.state,
            self.name,
            self.condition.describe(),
            self.total_fires
        )
    }
}

impl GroundsTo for AlertRule {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sum,
            LexPrimitiva::Boundary,
            LexPrimitiva::State,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Boundary, 0.75)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_comparator_evaluate() {
        assert!(Comparator::GreaterThan.evaluate(5.0, 3.0));
        assert!(!Comparator::GreaterThan.evaluate(3.0, 5.0));
        assert!(Comparator::LessThan.evaluate(1.0, 5.0));
        assert!(Comparator::GreaterOrEqual.evaluate(5.0, 5.0));
        assert!(Comparator::LessOrEqual.evaluate(5.0, 5.0));
    }

    #[test]
    fn test_condition_threshold() {
        let c = Condition::threshold("error_rate", Comparator::GreaterThan, 0.05);
        assert!(c.evaluate(0.10));
        assert!(!c.evaluate(0.01));
    }

    #[test]
    fn test_condition_describe() {
        let c = Condition::threshold("error_rate", Comparator::GreaterThan, 0.05);
        assert_eq!(c.describe(), "error_rate > 0.05");
    }

    #[test]
    fn test_alert_immediate_fire() {
        let condition = Condition::threshold("error_rate", Comparator::GreaterThan, 0.05);
        let mut rule = AlertRule::new(
            "alert_1",
            "High Error Rate",
            condition,
            AlertSeverity::Critical,
        );

        assert_eq!(rule.state(), AlertState::Ok);

        // Condition met with for_count=1 → immediately fire
        let (state, changed) = rule.evaluate(0.10);
        assert_eq!(state, AlertState::Firing);
        assert!(changed);
        assert_eq!(rule.total_fires(), 1);
    }

    #[test]
    fn test_alert_hysteresis() {
        let condition =
            Condition::threshold("error_rate", Comparator::GreaterThan, 0.05).with_for_count(3);
        let mut rule = AlertRule::new(
            "alert_2",
            "Sustained Error",
            condition,
            AlertSeverity::Warning,
        );

        // First violation → Pending
        let (state, _) = rule.evaluate(0.10);
        assert_eq!(state, AlertState::Pending);

        // Second violation → still Pending
        let (state, _) = rule.evaluate(0.08);
        assert_eq!(state, AlertState::Pending);

        // Third violation → Firing
        let (state, changed) = rule.evaluate(0.12);
        assert_eq!(state, AlertState::Firing);
        assert!(changed);
        assert_eq!(rule.total_fires(), 1);
    }

    #[test]
    fn test_alert_hysteresis_reset() {
        let condition =
            Condition::threshold("error_rate", Comparator::GreaterThan, 0.05).with_for_count(3);
        let mut rule = AlertRule::new("alert_3", "Flap Guard", condition, AlertSeverity::Info);

        // Two violations
        rule.evaluate(0.10);
        rule.evaluate(0.10);
        assert_eq!(rule.state(), AlertState::Pending);

        // Recovery before for_count → back to OK
        let (state, _) = rule.evaluate(0.01);
        assert_eq!(state, AlertState::Ok);
        assert_eq!(rule.total_fires(), 0);
    }

    #[test]
    fn test_alert_resolve_cycle() {
        let condition = Condition::threshold("error_rate", Comparator::GreaterThan, 0.05);
        let mut rule = AlertRule::new(
            "alert_4",
            "Resolve Test",
            condition,
            AlertSeverity::Critical,
        );

        // Fire
        rule.evaluate(0.10);
        assert_eq!(rule.state(), AlertState::Firing);

        // Resolve
        let (state, _) = rule.evaluate(0.01);
        assert_eq!(state, AlertState::Resolved);

        // Fully clear
        let (state, _) = rule.evaluate(0.01);
        assert_eq!(state, AlertState::Ok);
    }

    #[test]
    fn test_alert_re_fire_from_resolved() {
        let condition = Condition::threshold("error_rate", Comparator::GreaterThan, 0.05);
        let mut rule = AlertRule::new("alert_5", "Re-fire Test", condition, AlertSeverity::Warning);

        // Fire → Resolve → Re-fire
        rule.evaluate(0.10);
        rule.evaluate(0.01); // Resolved
        rule.evaluate(0.10); // Re-fire

        assert_eq!(rule.state(), AlertState::Firing);
        assert_eq!(rule.total_fires(), 2);
    }

    #[test]
    fn test_alert_summary() {
        let condition = Condition::threshold("latency", Comparator::GreaterThan, 100.0);
        let rule = AlertRule::new("alert_6", "High Latency", condition, AlertSeverity::Warning);
        let summary = rule.summary();
        assert!(summary.contains("High Latency"));
        assert!(summary.contains("latency"));
    }

    #[test]
    fn test_alert_reset() {
        let condition = Condition::threshold("m", Comparator::GreaterThan, 1.0);
        let mut rule = AlertRule::new("r", "R", condition, AlertSeverity::Info);

        rule.evaluate(5.0); // Fire
        assert_eq!(rule.state(), AlertState::Firing);

        rule.reset();
        assert_eq!(rule.state(), AlertState::Ok);
    }

    #[test]
    fn test_alert_rule_grounding() {
        let comp = AlertRule::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Boundary));
    }

    #[test]
    fn test_condition_grounding() {
        let comp = Condition::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
