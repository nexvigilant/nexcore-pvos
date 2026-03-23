//! # PVWF Process Supervisor
//!
//! Process lifecycle management with supervision trees.
//! Handles state transitions, restart policies, and timeout enforcement.
//!
//! ## Primitives
//! - ς (State) — process state machine
//! - ρ (Recursion) — restart/supervision loops
//! - ∂ (Boundary) — timeout and retry limits

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::PvosError;
use super::workflow::{StepOutput, Workflow, WorkflowId};

/// Unique process identifier.
/// Tier: T2-P (N + σ)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcId(pub u64);

impl GroundsTo for ProcId {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Quantity, LexPrimitiva::Sequence])
    }
}

/// Process state machine.
/// Tier: T1 (ς)
///
/// ```text
/// Pending → Running → Completed
///            ↓  ↑
///         Waiting  → Running (resume)
///            ↓
///          Failed → (restart?) → Pending
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcState {
    /// Waiting to be executed.
    Pending,
    /// Currently executing steps.
    Running,
    /// Paused, waiting for external input (human approval).
    Waiting,
    /// All steps completed successfully.
    Completed,
    /// Execution failed.
    Failed,
}

impl ProcState {
    /// Returns true if the process is terminal (completed or failed).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Returns true if the process can be resumed.
    #[must_use]
    pub fn is_resumable(self) -> bool {
        matches!(self, Self::Waiting | Self::Pending)
    }
}

impl GroundsTo for ProcState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State])
    }
}

/// Restart policy for supervised processes.
/// Tier: T2-P (ρ + ∂)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartPolicy {
    /// Never restart on failure.
    Never,
    /// Restart on failure, up to max_restarts times.
    OnFailure { max_restarts: usize },
    /// Always restart (even on success), up to max_restarts.
    Always { max_restarts: usize },
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::OnFailure { max_restarts: 3 }
    }
}

impl GroundsTo for RestartPolicy {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Recursion, LexPrimitiva::Boundary])
    }
}

/// A running workflow process instance.
/// Tier: T2-C (ς + σ + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WfProcess {
    /// Process ID.
    pub id: ProcId,
    /// Workflow being executed.
    pub workflow: Workflow,
    /// Current step index.
    pub current_step: usize,
    /// Process state.
    pub state: ProcState,
    /// Step outputs collected so far.
    pub outputs: Vec<StepOutput>,
    /// Number of restarts performed.
    pub restarts: usize,
    /// Error message if failed.
    pub error: Option<String>,
    /// Creation time.
    pub created: SystemTime,
    /// Last state change time.
    pub last_updated: SystemTime,
}

impl WfProcess {
    /// Creates a new process for a workflow.
    #[must_use]
    pub fn new(id: ProcId, workflow: Workflow) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            workflow,
            current_step: 0,
            state: ProcState::Pending,
            outputs: Vec::new(),
            restarts: 0,
            error: None,
            created: now,
            last_updated: now,
        }
    }

    /// Transitions to Running state.
    ///
    /// # Errors
    /// Returns `Err` if current state doesn't allow transition.
    pub fn start(&mut self) -> Result<(), PvosError> {
        if !self.state.is_resumable() && self.state != ProcState::Pending {
            return Err(PvosError::InvalidInput(format!(
                "cannot start process in state {:?}",
                self.state
            )));
        }
        self.state = ProcState::Running;
        self.last_updated = SystemTime::now();
        Ok(())
    }

    /// Advances to the next step.
    pub fn advance(&mut self, output: StepOutput) {
        self.outputs.push(output);
        self.current_step += 1;
        self.last_updated = SystemTime::now();

        if self.current_step >= self.workflow.steps.len() {
            self.state = ProcState::Completed;
        }
    }

    /// Marks process as waiting for human input.
    pub fn wait(&mut self) {
        self.state = ProcState::Waiting;
        self.last_updated = SystemTime::now();
    }

    /// Resumes from waiting state.
    ///
    /// # Errors
    /// Returns `Err` if not in Waiting state.
    pub fn resume(&mut self) -> Result<(), PvosError> {
        if self.state != ProcState::Waiting {
            return Err(PvosError::InvalidInput(format!(
                "cannot resume process in state {:?}",
                self.state
            )));
        }
        self.state = ProcState::Running;
        self.last_updated = SystemTime::now();
        Ok(())
    }

    /// Marks process as failed.
    pub fn fail(&mut self, error: &str) {
        self.state = ProcState::Failed;
        self.error = Some(error.to_string());
        self.last_updated = SystemTime::now();
    }

    /// Resets process for restart.
    pub fn restart(&mut self) {
        self.state = ProcState::Pending;
        self.current_step = 0;
        self.outputs.clear();
        self.error = None;
        self.restarts += 1;
        self.last_updated = SystemTime::now();
    }

    /// Skips N steps forward.
    pub fn skip(&mut self, count: usize) {
        for _ in 0..count {
            if self.current_step < self.workflow.steps.len() {
                self.outputs.push(StepOutput::Skipped);
                self.current_step += 1;
            }
        }
        self.last_updated = SystemTime::now();

        if self.current_step >= self.workflow.steps.len() {
            self.state = ProcState::Completed;
        }
    }

    /// Returns the last step output.
    #[must_use]
    pub fn last_output(&self) -> Option<&StepOutput> {
        self.outputs.last()
    }

    /// Returns true if all steps are complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state == ProcState::Completed
    }

    /// Progress as a fraction (0.0 to 1.0).
    #[must_use]
    pub fn progress(&self) -> f64 {
        let total = self.workflow.steps.len();
        if total == 0 {
            1.0
        } else {
            self.current_step as f64 / total as f64
        }
    }
}

impl GroundsTo for WfProcess {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,
            LexPrimitiva::Sequence,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::State, 0.90)
    }
}

/// Process supervisor — watches processes and applies restart policies.
/// Tier: T2-C (ρ + ς + ∂)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supervisor {
    /// Restart policy.
    pub policy: RestartPolicy,
    /// Maximum execution time before timeout.
    pub timeout: Duration,
    /// Supervised processes.
    processes: Vec<WfProcess>,
    /// Next process ID.
    next_id: u64,
    /// Total restarts performed.
    total_restarts: u64,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    /// Creates a new supervisor with default policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policy: RestartPolicy::default(),
            timeout: Duration::from_secs(300),
            processes: Vec::new(),
            next_id: 1,
            total_restarts: 0,
        }
    }

    /// Sets the restart policy.
    #[must_use]
    pub fn with_policy(mut self, policy: RestartPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Sets the timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Spawns a new supervised process.
    pub fn spawn(&mut self, workflow: Workflow) -> ProcId {
        let id = ProcId(self.next_id);
        self.next_id += 1;
        let process = WfProcess::new(id, workflow);
        self.processes.push(process);
        id
    }

    /// Returns a reference to a process.
    #[must_use]
    pub fn get(&self, id: ProcId) -> Option<&WfProcess> {
        self.processes.iter().find(|p| p.id == id)
    }

    /// Returns a mutable reference to a process.
    pub fn get_mut(&mut self, id: ProcId) -> Option<&mut WfProcess> {
        self.processes.iter_mut().find(|p| p.id == id)
    }

    /// Handles a process failure according to restart policy.
    /// Returns `true` if process was restarted.
    pub fn handle_failure(&mut self, id: ProcId) -> bool {
        let should_restart = {
            let process = match self.processes.iter().find(|p| p.id == id) {
                Some(p) => p,
                None => return false,
            };

            match self.policy {
                RestartPolicy::Never => false,
                RestartPolicy::OnFailure { max_restarts } => {
                    process.state == ProcState::Failed && process.restarts < max_restarts
                }
                RestartPolicy::Always { max_restarts } => process.restarts < max_restarts,
            }
        };

        if should_restart {
            if let Some(process) = self.processes.iter_mut().find(|p| p.id == id) {
                process.restart();
                self.total_restarts += 1;
                return true;
            }
        }

        false
    }

    /// Checks all processes for timeout violations.
    /// Returns IDs of timed-out processes.
    pub fn check_timeouts(&mut self) -> Vec<ProcId> {
        let timeout = self.timeout;
        let mut timed_out = Vec::new();

        for process in &mut self.processes {
            if process.state == ProcState::Running {
                let elapsed = process.last_updated.elapsed().unwrap_or(Duration::ZERO);
                if elapsed > timeout {
                    process.fail("timeout exceeded");
                    timed_out.push(process.id);
                }
            }
        }

        timed_out
    }

    /// Number of active (non-terminal) processes.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.processes
            .iter()
            .filter(|p| !p.state.is_terminal())
            .count()
    }

    /// Total processes (all states).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.processes.len()
    }

    /// Total restarts performed.
    #[must_use]
    pub fn total_restarts(&self) -> u64 {
        self.total_restarts
    }

    /// Returns all processes.
    #[must_use]
    pub fn processes(&self) -> &[WfProcess] {
        &self.processes
    }
}

impl GroundsTo for Supervisor {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion,
            LexPrimitiva::State,
            LexPrimitiva::Boundary,
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.90)
    }
}

#[cfg(test)]
mod tests {
    use super::super::workflow::{Step, SyscallKind};
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    fn test_workflow() -> Workflow {
        Workflow::builder("test")
            .syscall("step1", SyscallKind::Detect)
            .syscall("step2", SyscallKind::Store)
            .syscall("step3", SyscallKind::Route)
            .build()
    }

    #[test]
    fn test_proc_state_transitions() {
        let wf = test_workflow();
        let mut proc = WfProcess::new(ProcId(1), wf);
        assert_eq!(proc.state, ProcState::Pending);

        assert!(proc.start().is_ok());
        assert_eq!(proc.state, ProcState::Running);

        proc.advance(StepOutput::Completed);
        assert_eq!(proc.current_step, 1);
        assert!(!proc.is_complete());

        proc.advance(StepOutput::Completed);
        proc.advance(StepOutput::Completed);
        assert!(proc.is_complete());
        assert_eq!(proc.state, ProcState::Completed);
    }

    #[test]
    fn test_proc_wait_resume() {
        let wf = test_workflow();
        let mut proc = WfProcess::new(ProcId(1), wf);
        let _ = proc.start();

        proc.wait();
        assert_eq!(proc.state, ProcState::Waiting);
        assert!(proc.state.is_resumable());

        assert!(proc.resume().is_ok());
        assert_eq!(proc.state, ProcState::Running);
    }

    #[test]
    fn test_proc_failure() {
        let wf = test_workflow();
        let mut proc = WfProcess::new(ProcId(1), wf);
        let _ = proc.start();

        proc.fail("test error");
        assert_eq!(proc.state, ProcState::Failed);
        assert!(proc.state.is_terminal());
        assert_eq!(proc.error, Some("test error".into()));
    }

    #[test]
    fn test_proc_restart() {
        let wf = test_workflow();
        let mut proc = WfProcess::new(ProcId(1), wf);
        let _ = proc.start();
        proc.advance(StepOutput::Completed);
        proc.fail("error");

        proc.restart();
        assert_eq!(proc.state, ProcState::Pending);
        assert_eq!(proc.current_step, 0);
        assert_eq!(proc.restarts, 1);
        assert!(proc.outputs.is_empty());
        assert!(proc.error.is_none());
    }

    #[test]
    fn test_proc_skip() {
        let wf = test_workflow();
        let mut proc = WfProcess::new(ProcId(1), wf);
        let _ = proc.start();

        proc.skip(2);
        assert_eq!(proc.current_step, 2);
        assert_eq!(proc.outputs.len(), 2);
    }

    #[test]
    fn test_proc_progress() {
        let wf = test_workflow();
        let mut proc = WfProcess::new(ProcId(1), wf);
        assert!((proc.progress() - 0.0).abs() < f64::EPSILON);

        let _ = proc.start();
        proc.advance(StepOutput::Completed);
        let expected = 1.0 / 3.0;
        assert!((proc.progress() - expected).abs() < 0.01);
    }

    #[test]
    fn test_supervisor_spawn() {
        let mut sup = Supervisor::new();
        let id = sup.spawn(test_workflow());
        assert_eq!(id.0, 1);
        assert_eq!(sup.total_count(), 1);
        assert_eq!(sup.active_count(), 1);

        let proc = sup.get(id);
        assert!(proc.is_some());
    }

    #[test]
    fn test_supervisor_restart_on_failure() {
        let mut sup = Supervisor::new().with_policy(RestartPolicy::OnFailure { max_restarts: 2 });

        let id = sup.spawn(test_workflow());
        if let Some(proc) = sup.get_mut(id) {
            let _ = proc.start();
            proc.fail("error 1");
        }

        assert!(sup.handle_failure(id)); // Restart 1
        assert_eq!(sup.total_restarts(), 1);

        if let Some(proc) = sup.get_mut(id) {
            let _ = proc.start();
            proc.fail("error 2");
        }

        assert!(sup.handle_failure(id)); // Restart 2
        assert_eq!(sup.total_restarts(), 2);

        if let Some(proc) = sup.get_mut(id) {
            let _ = proc.start();
            proc.fail("error 3");
        }

        assert!(!sup.handle_failure(id)); // Max restarts reached
    }

    #[test]
    fn test_supervisor_never_restart() {
        let mut sup = Supervisor::new().with_policy(RestartPolicy::Never);

        let id = sup.spawn(test_workflow());
        if let Some(proc) = sup.get_mut(id) {
            let _ = proc.start();
            proc.fail("error");
        }

        assert!(!sup.handle_failure(id));
    }

    #[test]
    fn test_proc_state_grounding() {
        let comp = ProcState::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T1Universal);
    }

    #[test]
    fn test_supervisor_grounding() {
        let comp = Supervisor::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Recursion));
    }

    #[test]
    fn test_wf_process_grounding() {
        let comp = WfProcess::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::State));
    }

    #[test]
    fn test_restart_policy_grounding() {
        let comp = RestartPolicy::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
