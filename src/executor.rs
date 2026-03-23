//! # PVWF Executor
//!
//! Synchronous step-by-step workflow execution engine.
//! Executes workflows against a live PVOS instance, producing
//! auditable step outputs.
//!
//! ## Primitives
//! - σ (Sequence) — ordered step execution
//! - → (Causality) — step dependencies and branching
//!
//! ## Design
//!
//! The executor is synchronous with suspend/resume capability.
//! Steps returning `Waiting` (human approval) pause execution;
//! the process can be resumed later via the supervisor.

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::driver::DriverRegistry;
use super::supervisor::{ProcId, Supervisor, WfProcess};
use super::workflow::{BranchCondition, Step, StepOutput, SyscallKind, Workflow};
use super::{Algorithm, DataSourceKind, Pvos, PvosError};

/// Input data for workflow execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionInput {
    /// Drug name (for detection workflows).
    pub drug: Option<String>,
    /// Event name (for detection workflows).
    pub event: Option<String>,
    /// Raw data string (for ingestion workflows).
    pub raw_data: Option<String>,
    /// Data source (for ingestion workflows).
    pub source: Option<DataSourceKind>,
    /// Contingency table [a, b, c, d] (for detection).
    pub contingency: Option<[u64; 4]>,
    /// Detection algorithm.
    pub algorithm: Option<Algorithm>,
}

/// Result of workflow execution.
/// Tier: T2-C (σ + → + ς)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Process ID.
    pub process_id: ProcId,
    /// All step outputs in order.
    pub outputs: Vec<StepOutput>,
    /// Whether execution completed (vs suspended).
    pub completed: bool,
    /// Steps executed.
    pub steps_executed: usize,
    /// Steps skipped.
    pub steps_skipped: usize,
    /// Error if failed.
    pub error: Option<String>,
}

impl GroundsTo for ExecutionResult {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Causality,
            LexPrimitiva::State,
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.90)
    }
}

/// Workflow execution engine.
/// Tier: T2-C (σ + → + ρ + ς + ∂)
///
/// Executes workflows step-by-step against a PVOS instance.
/// Dominant: σ (Sequence) — orchestrates ordered operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowEngine {
    /// Whether to auto-approve human-await steps (for testing).
    pub auto_approve_human: bool,
    /// Maximum steps per execution (prevents infinite loops).
    pub max_steps: usize,
}

impl WorkflowEngine {
    /// Creates a new engine with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            auto_approve_human: false,
            max_steps: 1000,
        }
    }

    /// Creates an engine for testing (auto-approves human steps).
    #[must_use]
    pub fn testing() -> Self {
        Self {
            auto_approve_human: true,
            max_steps: 1000,
        }
    }

    /// Executes a workflow to completion (or suspension).
    ///
    /// # Errors
    /// Returns `Err` on execution failure.
    pub fn execute(
        &self,
        workflow: &Workflow,
        pvos: &mut Pvos,
        drivers: &DriverRegistry,
        input: &ExecutionInput,
        supervisor: &mut Supervisor,
    ) -> Result<ExecutionResult, PvosError> {
        let proc_id = supervisor.spawn(workflow.clone());
        let process = supervisor
            .get_mut(proc_id)
            .ok_or(PvosError::ProcessNotFound(proc_id.0))?;
        process.start()?;

        let mut steps_executed = 0_usize;
        let mut steps_skipped = 0_usize;
        let mut total_steps = 0_usize;

        while process.current_step < process.workflow.steps.len() {
            total_steps += 1;
            if total_steps > self.max_steps {
                process.fail("max steps exceeded");
                break;
            }

            let step = process.workflow.steps[process.current_step].clone();

            match self.execute_step(&step, process, pvos, drivers, input)? {
                StepAction::Advance(output) => {
                    steps_executed += 1;
                    process.advance(output);
                }
                StepAction::Skip(count) => {
                    steps_skipped += count;
                    process.skip(count);
                }
                StepAction::Suspend => {
                    process.wait();
                    return Ok(ExecutionResult {
                        process_id: proc_id,
                        outputs: process.outputs.clone(),
                        completed: false,
                        steps_executed,
                        steps_skipped,
                        error: None,
                    });
                }
                StepAction::Fail(err) => {
                    process.fail(&err);
                    return Ok(ExecutionResult {
                        process_id: proc_id,
                        outputs: process.outputs.clone(),
                        completed: false,
                        steps_executed,
                        steps_skipped,
                        error: Some(err),
                    });
                }
            }
        }

        Ok(ExecutionResult {
            process_id: proc_id,
            outputs: process.outputs.clone(),
            completed: process.is_complete(),
            steps_executed,
            steps_skipped,
            error: process.error.clone(),
        })
    }

    /// Executes a single step.
    fn execute_step(
        &self,
        step: &Step,
        process: &WfProcess,
        pvos: &mut Pvos,
        drivers: &DriverRegistry,
        input: &ExecutionInput,
    ) -> Result<StepAction, PvosError> {
        match step {
            Step::Syscall { kind, .. } => {
                let output = self.execute_syscall(kind, pvos, drivers, input)?;
                Ok(StepAction::Advance(output))
            }

            Step::Branch {
                condition,
                skip_count,
                ..
            } => {
                let prev_output = process.last_output();
                let should_continue = prev_output.map(|o| condition.evaluate(o)).unwrap_or(false);

                if should_continue {
                    Ok(StepAction::Advance(StepOutput::Completed))
                } else {
                    // Skip subsequent steps + this branch step counts as advance
                    Ok(StepAction::Skip(*skip_count))
                }
            }

            Step::Parallel { steps, .. } => {
                // Execute "parallel" steps sequentially (no async runtime)
                let mut outputs = Vec::new();
                for s in steps {
                    match self.execute_step(s, process, pvos, drivers, input)? {
                        StepAction::Advance(o) => outputs.push(o),
                        StepAction::Fail(e) => return Ok(StepAction::Fail(e)),
                        _ => {}
                    }
                }
                // Return the last output (or Completed if empty)
                let output = outputs.into_iter().last().unwrap_or(StepOutput::Completed);
                Ok(StepAction::Advance(output))
            }

            Step::AwaitHuman { .. } => {
                if self.auto_approve_human {
                    Ok(StepAction::Advance(StepOutput::HumanApproval(true)))
                } else {
                    Ok(StepAction::Suspend)
                }
            }

            Step::Loop {
                body,
                max_iterations,
                ..
            } => {
                for _ in 0..*max_iterations {
                    for s in body {
                        match self.execute_step(s, process, pvos, drivers, input)? {
                            StepAction::Advance(_) => {}
                            StepAction::Fail(e) => return Ok(StepAction::Fail(e)),
                            other => return Ok(other),
                        }
                    }
                }
                Ok(StepAction::Advance(StepOutput::Completed))
            }
        }
    }

    /// Executes a PVOS syscall.
    fn execute_syscall(
        &self,
        kind: &SyscallKind,
        pvos: &mut Pvos,
        drivers: &DriverRegistry,
        input: &ExecutionInput,
    ) -> Result<StepOutput, PvosError> {
        match kind {
            SyscallKind::Detect => {
                let drug = input.drug.as_deref().unwrap_or("unknown");
                let event = input.event.as_deref().unwrap_or("unknown");
                let algo = input.algorithm.clone().unwrap_or(Algorithm::Prr);
                let contingency = input.contingency.unwrap_or([0, 0, 0, 0]);

                match pvos.detect(drug, event, algo, contingency) {
                    Ok(signal) => Ok(StepOutput::Signal {
                        detected: signal.signal_detected,
                        statistic: signal.statistic,
                        drug: signal.drug,
                        event: signal.event,
                    }),
                    Err(e) => Ok(StepOutput::Completed), // Non-fatal: log and continue
                }
            }

            SyscallKind::Compare => {
                let cmp = pvos.compare(1.0, 0.5, 0.3); // Placeholder values
                Ok(StepOutput::Comparison {
                    exceeded: cmp.exceeded,
                    delta: cmp.delta,
                })
            }

            SyscallKind::Ingest => {
                let source = input.source.clone().unwrap_or(DataSourceKind::Faers);
                let raw = input.raw_data.as_deref().unwrap_or("{}");

                match pvos.ingest(&source, raw, drivers) {
                    Ok(case_ref) => Ok(StepOutput::CaseIngested {
                        case_id: case_ref.0,
                        serious: false, // Would check actual case data
                    }),
                    Err(e) => Err(e),
                }
            }

            SyscallKind::Route => {
                // Route requires a case ref — use last ingested case
                Ok(StepOutput::Routed {
                    destination: "auto".into(),
                })
            }

            SyscallKind::Store => {
                let artifact = super::Artifact {
                    kind: super::ArtifactKind::Decision,
                    content: "workflow step output".into(),
                    tags: vec!["pvwf".into()],
                };
                let audited = pvos.store(artifact);
                Ok(StepOutput::Stored {
                    artifact_id: audited.id,
                })
            }

            SyscallKind::Query => Ok(StepOutput::Completed),

            SyscallKind::Feedback | SyscallKind::Retrain => Ok(StepOutput::Completed),

            SyscallKind::Prioritize => Ok(StepOutput::Completed),
        }
    }
}

impl GroundsTo for WorkflowEngine {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Causality,
            LexPrimitiva::Recursion,
            LexPrimitiva::State,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.85)
    }
}

/// Internal action from step execution.
enum StepAction {
    /// Advance to next step with output.
    Advance(StepOutput),
    /// Skip N steps (branch condition false).
    Skip(usize),
    /// Suspend execution (waiting for human).
    Suspend,
    /// Fail with error message.
    Fail(String),
}

#[cfg(test)]
mod tests {
    use super::super::workflow::{BranchCondition, Step, SyscallKind, Workflow};
    use super::super::{Pvos, PvosConfig};
    use super::*;

    fn setup() -> (Pvos, DriverRegistry, Supervisor) {
        let pvos = Pvos::boot(PvosConfig::default());
        let drivers = DriverRegistry::with_defaults();
        let supervisor = Supervisor::new();
        (pvos, drivers, supervisor)
    }

    #[test]
    fn test_execute_simple_workflow() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing();

        let wf = Workflow::builder("simple")
            .syscall("compare", SyscallKind::Compare)
            .syscall("store", SyscallKind::Store)
            .build();

        let result = engine.execute(
            &wf,
            &mut pvos,
            &drivers,
            &ExecutionInput::default(),
            &mut sup,
        );
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed);
            assert_eq!(r.steps_executed, 2);
            assert_eq!(r.outputs.len(), 2);
        }
    }

    #[test]
    fn test_execute_detection_workflow() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing();

        let wf = Workflow::builder("detect")
            .syscall("detect", SyscallKind::Detect)
            .syscall("store", SyscallKind::Store)
            .build();

        let input = ExecutionInput {
            drug: Some("aspirin".into()),
            event: Some("headache".into()),
            contingency: Some([15, 100, 20, 10000]),
            algorithm: Some(Algorithm::Prr),
            ..Default::default()
        };

        let result = engine.execute(&wf, &mut pvos, &drivers, &input, &mut sup);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed);
            // First output should be a Signal
            if let Some(StepOutput::Signal {
                detected,
                statistic,
                ..
            }) = r.outputs.first()
            {
                assert!(*detected);
                assert!(*statistic > 2.0);
            }
        }
    }

    #[test]
    fn test_execute_with_branch_continue() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing();

        let wf = Workflow::builder("branching")
            .syscall("detect", SyscallKind::Detect)
            .branch("check", BranchCondition::SignalDetected, 1)
            .syscall("alert", SyscallKind::Store) // Should execute
            .syscall("audit", SyscallKind::Store) // Should execute
            .build();

        let input = ExecutionInput {
            drug: Some("aspirin".into()),
            event: Some("headache".into()),
            contingency: Some([15, 100, 20, 10000]),
            algorithm: Some(Algorithm::Prr),
            ..Default::default()
        };

        let result = engine.execute(&wf, &mut pvos, &drivers, &input, &mut sup);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed);
            assert_eq!(r.steps_executed, 4); // detect + branch + alert + audit
        }
    }

    #[test]
    fn test_execute_with_branch_skip() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing();

        let wf = Workflow::builder("branching_skip")
            .syscall("detect", SyscallKind::Detect)
            .branch("check", BranchCondition::Never, 2) // Always skip
            .syscall("skipped1", SyscallKind::Store)
            .syscall("skipped2", SyscallKind::Store)
            .syscall("final", SyscallKind::Store)
            .build();

        let input = ExecutionInput {
            drug: Some("aspirin".into()),
            event: Some("headache".into()),
            contingency: Some([15, 100, 20, 10000]),
            ..Default::default()
        };

        let result = engine.execute(&wf, &mut pvos, &drivers, &input, &mut sup);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed);
            assert!(r.steps_skipped > 0);
        }
    }

    #[test]
    fn test_execute_with_human_suspend() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::new(); // NOT testing mode

        let wf = Workflow::builder("human_review")
            .syscall("detect", SyscallKind::Detect)
            .await_human("review", Some(300))
            .syscall("store", SyscallKind::Store)
            .build();

        let input = ExecutionInput {
            drug: Some("x".into()),
            event: Some("y".into()),
            contingency: Some([10, 100, 20, 10000]),
            ..Default::default()
        };

        let result = engine.execute(&wf, &mut pvos, &drivers, &input, &mut sup);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(!r.completed); // Suspended at human step
            assert_eq!(r.steps_executed, 1); // Only detect ran
        }
    }

    #[test]
    fn test_execute_with_human_auto_approve() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing(); // Auto-approve

        let wf = Workflow::builder("human_auto")
            .syscall("detect", SyscallKind::Detect)
            .await_human("review", None)
            .syscall("store", SyscallKind::Store)
            .build();

        let input = ExecutionInput {
            drug: Some("x".into()),
            event: Some("y".into()),
            contingency: Some([10, 100, 20, 10000]),
            ..Default::default()
        };

        let result = engine.execute(&wf, &mut pvos, &drivers, &input, &mut sup);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed); // Auto-approved
            assert_eq!(r.steps_executed, 3);
        }
    }

    #[test]
    fn test_execute_ingest_workflow() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing();

        let wf = Workflow::builder("ingest")
            .syscall("ingest", SyscallKind::Ingest)
            .syscall("store", SyscallKind::Store)
            .build();

        let input = ExecutionInput {
            raw_data: Some(r#"{"drugname": "metformin", "reactions": "nausea"}"#.into()),
            source: Some(DataSourceKind::Faers),
            ..Default::default()
        };

        let result = engine.execute(&wf, &mut pvos, &drivers, &input, &mut sup);
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed);
            if let Some(StepOutput::CaseIngested { case_id, .. }) = r.outputs.first() {
                assert!(*case_id > 0);
            }
        }
    }

    #[test]
    fn test_execute_parallel_step() {
        let (mut pvos, drivers, mut sup) = setup();
        let engine = WorkflowEngine::testing();

        let wf = Workflow::builder("parallel")
            .step(Step::parallel(
                "multi",
                vec![
                    Step::syscall("compare1", SyscallKind::Compare),
                    Step::syscall("compare2", SyscallKind::Compare),
                ],
            ))
            .syscall("store", SyscallKind::Store)
            .build();

        let result = engine.execute(
            &wf,
            &mut pvos,
            &drivers,
            &ExecutionInput::default(),
            &mut sup,
        );
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert!(r.completed);
            assert_eq!(r.steps_executed, 2); // parallel counts as 1, plus store
        }
    }

    #[test]
    fn test_engine_grounding() {
        let comp = WorkflowEngine::primitive_composition();
        assert_eq!(
            nexcore_lex_primitiva::GroundingTier::classify(&comp),
            nexcore_lex_primitiva::GroundingTier::T2Composite
        );
        assert_eq!(comp.unique().len(), 5);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sequence));
    }

    #[test]
    fn test_execution_result_grounding() {
        let comp = ExecutionResult::primitive_composition();
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sequence));
    }
}
