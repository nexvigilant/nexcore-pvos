//! # PVSH Read-Eval-Print Loop
//!
//! The REPL is the σ-loop that drives the shell: read input, evaluate
//! as command, print output, repeat. Manages session state, history,
//! and prompt rendering.
//!
//! ## Primitives
//! - σ (Sequence) — the REPL loop itself
//! - λ (Location) — prompt shows current path
//! - π (Persistence) — command history
//! - ς (State) — session state

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// HISTORY
// ===============================================================

/// A single history entry.
/// Tier: T2-P (σ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Command that was entered.
    pub command: String,
    /// When it was entered.
    pub timestamp: SystemTime,
    /// Sequence number.
    pub seq: u64,
}

impl GroundsTo for HistoryEntry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence, LexPrimitiva::Persistence])
    }
}

/// Command history buffer.
/// Tier: T2-C (σ + π + λ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    /// History entries.
    entries: Vec<HistoryEntry>,
    /// Maximum entries to keep.
    max_entries: usize,
    /// Next sequence number.
    next_seq: u64,
}

impl History {
    /// Creates a new history with given max size.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max_entries.max(1),
            next_seq: 1,
        }
    }

    /// Pushes a command to history.
    pub fn push(&mut self, command: &str) {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return;
        }

        // Skip duplicates of the last entry
        if let Some(last) = self.entries.last() {
            if last.command == trimmed {
                return;
            }
        }

        self.entries.push(HistoryEntry {
            command: trimmed.to_string(),
            timestamp: SystemTime::now(),
            seq: self.next_seq,
        });
        self.next_seq += 1;

        // Trim to max size
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// All entries.
    #[must_use]
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Last N entries.
    #[must_use]
    pub fn last_n(&self, n: usize) -> &[HistoryEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    /// Search history for commands matching a prefix.
    #[must_use]
    pub fn search(&self, prefix: &str) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.command.starts_with(prefix))
            .collect()
    }

    /// Gets entry by sequence number.
    #[must_use]
    pub fn get_by_seq(&self, seq: u64) -> Option<&HistoryEntry> {
        self.entries.iter().find(|e| e.seq == seq)
    }

    /// Clears history.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl GroundsTo for History {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Persistence,
            LexPrimitiva::Location,
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.70)
    }
}

// ===============================================================
// PROMPT
// ===============================================================

/// Prompt configuration.
/// Tier: T2-P (λ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Prefix before path (e.g., "pvos").
    pub prefix: String,
    /// Suffix after path (e.g., "> ").
    pub suffix: String,
    /// Whether to show full path or just basename.
    pub show_full_path: bool,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            prefix: "pvos".to_string(),
            suffix: "> ".to_string(),
            show_full_path: true,
        }
    }
}

impl PromptConfig {
    /// Renders a prompt string for the given path.
    #[must_use]
    pub fn render(&self, path_display: &str) -> String {
        format!("{}:{}{}", self.prefix, path_display, self.suffix)
    }
}

impl GroundsTo for PromptConfig {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Location])
    }
}

// ===============================================================
// REPL STATE
// ===============================================================

/// Session state for the REPL.
/// Tier: T2-P (ς)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplState {
    /// Ready for input.
    Ready,
    /// Executing a command.
    Executing,
    /// Waiting for confirmation.
    Confirming,
    /// Terminated.
    Exited,
}

impl GroundsTo for ReplState {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::State])
    }
}

// ===============================================================
// REPL
// ===============================================================

/// The Read-Eval-Print Loop state machine.
/// Tier: T2-C (σ + λ + π + ς)
///
/// This is a pure state machine — no I/O. Input is fed via `input()`,
/// output is returned from `step()`. This makes it fully testable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repl {
    /// Session state.
    state: ReplState,
    /// Command history.
    history: History,
    /// Prompt configuration.
    prompt_config: PromptConfig,
    /// Total commands executed.
    total_commands: u64,
    /// Total errors encountered.
    total_errors: u64,
    /// Whether the REPL should exit.
    exit_requested: bool,
}

impl Repl {
    /// Creates a new REPL session.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ReplState::Ready,
            history: History::new(1000),
            prompt_config: PromptConfig::default(),
            total_commands: 0,
            total_errors: 0,
            exit_requested: false,
        }
    }

    /// Creates a REPL with custom history size.
    #[must_use]
    pub fn with_history_size(max: usize) -> Self {
        Self {
            history: History::new(max),
            ..Self::new()
        }
    }

    /// Creates a REPL with custom prompt.
    #[must_use]
    pub fn with_prompt(mut self, config: PromptConfig) -> Self {
        self.prompt_config = config;
        self
    }

    /// Current state.
    #[must_use]
    pub fn state(&self) -> ReplState {
        self.state
    }

    /// Renders the prompt for the given path.
    #[must_use]
    pub fn prompt(&self, path_display: &str) -> String {
        self.prompt_config.render(path_display)
    }

    /// Records a command in history and increments counters.
    pub fn record_command(&mut self, input: &str, was_error: bool) {
        self.history.push(input);
        self.total_commands += 1;
        if was_error {
            self.total_errors += 1;
        }
    }

    /// Requests exit.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
        self.state = ReplState::Exited;
    }

    /// Whether exit has been requested.
    #[must_use]
    pub fn should_exit(&self) -> bool {
        self.exit_requested
    }

    /// History reference.
    #[must_use]
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Mutable history reference.
    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    /// Total commands executed.
    #[must_use]
    pub fn total_commands(&self) -> u64 {
        self.total_commands
    }

    /// Total errors.
    #[must_use]
    pub fn total_errors(&self) -> u64 {
        self.total_errors
    }

    /// Session uptime as a display string.
    #[must_use]
    pub fn session_stats(&self) -> String {
        format!(
            "commands={}, errors={}, history={}",
            self.total_commands,
            self.total_errors,
            self.history.len(),
        )
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for Repl {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,
            LexPrimitiva::Location,
            LexPrimitiva::Persistence,
            LexPrimitiva::State,
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.70)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_history_push() {
        let mut h = History::new(100);
        h.push("cd /signals");
        h.push("ls");
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_history_skip_empty() {
        let mut h = History::new(100);
        h.push("");
        h.push("  ");
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn test_history_skip_duplicates() {
        let mut h = History::new(100);
        h.push("ls");
        h.push("ls");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn test_history_max_size() {
        let mut h = History::new(3);
        h.push("a");
        h.push("b");
        h.push("c");
        h.push("d");
        assert_eq!(h.len(), 3);
        assert_eq!(h.entries()[0].command, "b"); // "a" evicted
    }

    #[test]
    fn test_history_search() {
        let mut h = History::new(100);
        h.push("cd /signals");
        h.push("cd /cases");
        h.push("ls");

        let results = h.search("cd");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_history_last_n() {
        let mut h = History::new(100);
        h.push("a");
        h.push("b");
        h.push("c");

        let last = h.last_n(2);
        assert_eq!(last.len(), 2);
        assert_eq!(last[0].command, "b");
        assert_eq!(last[1].command, "c");
    }

    #[test]
    fn test_prompt_render() {
        let cfg = PromptConfig::default();
        assert_eq!(cfg.render("/signals"), "pvos:/signals> ");
    }

    #[test]
    fn test_repl_lifecycle() {
        let mut repl = Repl::new();
        assert_eq!(repl.state(), ReplState::Ready);
        assert!(!repl.should_exit());

        repl.record_command("ls", false);
        assert_eq!(repl.total_commands(), 1);
        assert_eq!(repl.total_errors(), 0);

        repl.record_command("bad_cmd", true);
        assert_eq!(repl.total_errors(), 1);

        repl.request_exit();
        assert!(repl.should_exit());
        assert_eq!(repl.state(), ReplState::Exited);
    }

    #[test]
    fn test_repl_session_stats() {
        let mut repl = Repl::new();
        repl.record_command("ls", false);
        repl.record_command("cd /signals", false);

        let stats = repl.session_stats();
        assert!(stats.contains("commands=2"));
        assert!(stats.contains("history=2"));
    }

    #[test]
    fn test_repl_grounding() {
        let comp = Repl::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sequence));
    }

    #[test]
    fn test_history_grounding() {
        let comp = History::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Sequence));
    }
}
