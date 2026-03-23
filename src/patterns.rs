//! # PVWF Standard Patterns
//!
//! Pre-built workflow patterns for common pharmacovigilance operations.
//! These are declarative compositions of PVOS syscalls — pure data structures
//! that can be executed by the WorkflowEngine.
//!
//! ## Primitive: σ (Sequence)
//!
//! Patterns are named sequences of steps. Each pattern encodes
//! a standard operating procedure as a serializable workflow.
//!
//! ## Available Patterns
//!
//! | Pattern               | Steps | Description                            |
//! |-----------------------|-------|----------------------------------------|
//! | Signal Detection      | 5     | ingest → detect → branch → alert → store |
//! | Case Processing       | 5     | ingest → branch → route → await → store  |
//! | PSUR Generation       | 6     | detect×3 → compare → review → store     |
//! | Continuous Monitoring  | 4     | loop(detect → feedback) → retrain → store |
//! | Expedited Reporting   | 4     | ingest → branch(serious?) → alert → store |

use super::workflow::{BranchCondition, Step, SyscallKind, Workflow};

/// Creates a signal detection workflow.
///
/// Flow: ingest → detect → branch(signal?) → alert → store
///
/// If a signal is detected, routes to alert; otherwise skips to store.
/// This is the bread-and-butter PV workflow.
///
/// ```rust
/// use nexcore_pvos::patterns::signal_detection_workflow;
///
/// let wf = signal_detection_workflow();
/// assert_eq!(wf.name, "signal_detection");
/// assert_eq!(wf.step_count(), 5);
/// ```
#[must_use]
pub fn signal_detection_workflow() -> Workflow {
    Workflow::builder("signal_detection")
        .description("Standard signal detection: ingest → detect → branch → alert → store")
        .syscall("ingest_case", SyscallKind::Ingest)
        .syscall("run_detection", SyscallKind::Detect)
        .branch(
            "check_signal",
            BranchCondition::SignalDetected,
            1, // Skip alert if no signal
        )
        .syscall("alert_safety_team", SyscallKind::Route)
        .syscall("store_results", SyscallKind::Store)
        .build()
}

/// Creates a case processing workflow.
///
/// Flow: ingest → branch(serious?) → route → await_human → store
///
/// Serious cases go through human review; non-serious are auto-routed.
#[must_use]
pub fn case_processing_workflow() -> Workflow {
    Workflow::builder("case_processing")
        .description("Case processing: ingest → triage → route → review → store")
        .syscall("receive_case", SyscallKind::Ingest)
        .branch(
            "triage_seriousness",
            BranchCondition::IsSerious,
            1, // Skip expedited routing if not serious
        )
        .syscall("expedited_route", SyscallKind::Route)
        .await_human("medical_review", Some(86400)) // 24h timeout
        .syscall("archive_case", SyscallKind::Store)
        .build()
}

/// Creates a PSUR (Periodic Safety Update Report) workflow.
///
/// Flow: detect(PRR) ∥ detect(ROR) ∥ detect(Chi²) → compare → review → store
///
/// Runs multiple detection algorithms in parallel, compares results,
/// requires human review before final storage.
#[must_use]
pub fn psur_workflow() -> Workflow {
    Workflow::builder("psur_generation")
        .description("PSUR: multi-algorithm detection → comparison → review → store")
        .step(Step::parallel(
            "multi_algorithm_detection",
            vec![
                Step::syscall("prr_detect", SyscallKind::Detect),
                Step::syscall("ror_detect", SyscallKind::Detect),
                Step::syscall("chi2_detect", SyscallKind::Detect),
            ],
        ))
        .syscall("compare_results", SyscallKind::Compare)
        .branch(
            "significant_findings",
            BranchCondition::StatisticAbove(2.0),
            0, // Don't skip — just marks the branch point
        )
        .await_human("safety_board_review", Some(604800)) // 7-day timeout
        .syscall("archive_psur", SyscallKind::Store)
        .syscall("submit_to_authority", SyscallKind::Store)
        .build()
}

/// Creates a continuous monitoring workflow.
///
/// Flow: loop(detect → feedback) × N → retrain → store
///
/// Iteratively detects and collects feedback, then retrains
/// the detection model based on accumulated outcomes.
#[must_use]
pub fn continuous_monitoring_workflow(iterations: usize) -> Workflow {
    Workflow::builder("continuous_monitoring")
        .description("Continuous monitoring: loop(detect → feedback) → retrain → store")
        .step(Step::loop_step(
            "detection_feedback_loop",
            vec![
                Step::syscall("detect_signal", SyscallKind::Detect),
                Step::syscall("record_feedback", SyscallKind::Feedback),
            ],
            iterations,
        ))
        .syscall("retrain_model", SyscallKind::Retrain)
        .syscall("store_model_state", SyscallKind::Store)
        .build()
}

/// Creates an expedited reporting workflow.
///
/// Flow: ingest → branch(serious?) → alert → store
///
/// Expedited path for serious/fatal cases requiring immediate
/// regulatory notification (15-day rule).
#[must_use]
pub fn expedited_reporting_workflow() -> Workflow {
    Workflow::builder("expedited_reporting")
        .description("Expedited reporting: ingest → triage → alert → store (15-day rule)")
        .syscall("receive_report", SyscallKind::Ingest)
        .branch(
            "is_serious_or_fatal",
            BranchCondition::IsSerious,
            1, // Skip immediate alert if not serious
        )
        .syscall("notify_authority", SyscallKind::Route)
        .syscall("archive_expedited", SyscallKind::Store)
        .build()
}

/// Creates a signal refinement workflow.
///
/// Flow: detect → branch(above threshold?) → query(historical) → compare → store
///
/// Refines a detected signal by comparing against historical data.
#[must_use]
pub fn signal_refinement_workflow(threshold: f64) -> Workflow {
    Workflow::builder("signal_refinement")
        .description("Signal refinement: detect → threshold check → historical comparison → store")
        .syscall("initial_detection", SyscallKind::Detect)
        .branch(
            "above_threshold",
            BranchCondition::StatisticAbove(threshold),
            2, // Skip query + compare if below threshold
        )
        .syscall("query_historical", SyscallKind::Query)
        .syscall("compare_with_baseline", SyscallKind::Compare)
        .syscall("store_refined_signal", SyscallKind::Store)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_detection_pattern() {
        let wf = signal_detection_workflow();
        assert_eq!(wf.name, "signal_detection");
        assert_eq!(wf.step_count(), 5);

        let names = wf.step_names();
        assert_eq!(names[0], "ingest_case");
        assert_eq!(names[1], "run_detection");
        assert_eq!(names[2], "check_signal");
        assert_eq!(names[3], "alert_safety_team");
        assert_eq!(names[4], "store_results");
    }

    #[test]
    fn test_case_processing_pattern() {
        let wf = case_processing_workflow();
        assert_eq!(wf.name, "case_processing");
        assert_eq!(wf.step_count(), 5);

        // Must have a human review step
        let has_human = wf
            .steps
            .iter()
            .any(|s| matches!(s, Step::AwaitHuman { .. }));
        assert!(has_human);
    }

    #[test]
    fn test_psur_pattern() {
        let wf = psur_workflow();
        assert_eq!(wf.name, "psur_generation");
        assert_eq!(wf.step_count(), 6);

        // First step should be parallel (3 detection algorithms)
        if let Step::Parallel { steps, .. } = &wf.steps[0] {
            assert_eq!(steps.len(), 3);
        } else {
            // Force failure with descriptive message
            assert!(false, "first step should be Parallel");
        }
    }

    #[test]
    fn test_continuous_monitoring_pattern() {
        let wf = continuous_monitoring_workflow(10);
        assert_eq!(wf.name, "continuous_monitoring");
        assert_eq!(wf.step_count(), 3); // loop + retrain + store

        // First step should be a loop
        if let Step::Loop {
            body,
            max_iterations,
            ..
        } = &wf.steps[0]
        {
            assert_eq!(*max_iterations, 10);
            assert_eq!(body.len(), 2); // detect + feedback
        } else {
            assert!(false, "first step should be Loop");
        }
    }

    #[test]
    fn test_expedited_reporting_pattern() {
        let wf = expedited_reporting_workflow();
        assert_eq!(wf.name, "expedited_reporting");
        assert_eq!(wf.step_count(), 4);

        // Should have a seriousness branch
        let has_serious_branch = wf.steps.iter().any(|s| {
            matches!(
                s,
                Step::Branch {
                    condition: BranchCondition::IsSerious,
                    ..
                }
            )
        });
        assert!(has_serious_branch);
    }

    #[test]
    fn test_signal_refinement_pattern() {
        let wf = signal_refinement_workflow(3.0);
        assert_eq!(wf.name, "signal_refinement");
        assert_eq!(wf.step_count(), 5);

        // Should have a threshold branch
        let has_threshold = wf.steps.iter().any(|s| {
            matches!(s, Step::Branch {
                condition: BranchCondition::StatisticAbove(t),
                ..
            } if (*t - 3.0).abs() < f64::EPSILON)
        });
        assert!(has_threshold);
    }

    #[test]
    fn test_all_patterns_serializable() {
        let patterns: Vec<Workflow> = vec![
            signal_detection_workflow(),
            case_processing_workflow(),
            psur_workflow(),
            continuous_monitoring_workflow(5),
            expedited_reporting_workflow(),
            signal_refinement_workflow(2.0),
        ];

        for wf in &patterns {
            let json = serde_json::to_string(wf);
            assert!(json.is_ok(), "failed to serialize: {}", wf.name);

            if let Ok(j) = json {
                let deser: Result<Workflow, _> = serde_json::from_str(&j);
                assert!(deser.is_ok(), "failed to deserialize: {}", wf.name);
                if let Ok(d) = deser {
                    assert_eq!(d.name, wf.name);
                    assert_eq!(d.step_count(), wf.step_count());
                }
            }
        }
    }

    #[test]
    fn test_pattern_descriptions_non_empty() {
        let patterns = vec![
            signal_detection_workflow(),
            case_processing_workflow(),
            psur_workflow(),
            continuous_monitoring_workflow(5),
            expedited_reporting_workflow(),
            signal_refinement_workflow(2.0),
        ];

        for wf in &patterns {
            assert!(
                !wf.description.is_empty(),
                "workflow {} has no description",
                wf.name
            );
        }
    }
}
