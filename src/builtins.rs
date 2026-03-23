//! # PVSH Built-in Commands
//!
//! Concrete implementations of shell commands. Each builtin operates
//! at a path location (λ) and interacts with a specific PVOS layer.
//!
//! ## Primitives
//! - λ (Location) — all builtins operate on paths
//! - μ (Mapping) — command dispatch
//! - σ (Sequence) — history, pipelines
//! - Each builtin also touches its target layer's dominant primitive

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::command::{Args, CommandKind, CommandRegistry, Output, ParsedCommand};
use super::path::{Navigator, PvPath};
use super::repl::Repl;

// ===============================================================
// SHELL CONTEXT
// ===============================================================

/// Minimal context available to builtins during execution.
/// Tier: T2-C (λ + μ + σ + ∃ + π)
///
/// This struct bundles the shell state that builtins need to read/modify.
/// It avoids requiring references to the full PVOS stack for basic ops.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellContext {
    /// Current navigator state.
    pub navigator: Navigator,
    /// Command registry.
    pub commands: CommandRegistry,
    /// REPL session state.
    pub repl: Repl,
}

impl ShellContext {
    /// Creates a new shell context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            navigator: Navigator::new(),
            commands: CommandRegistry::new(),
            repl: Repl::new(),
        }
    }
}

impl Default for ShellContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for ShellContext {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Mapping,
            LexPrimitiva::Sequence,
            LexPrimitiva::Existence,
            LexPrimitiva::Persistence,
        ])
        .with_dominant(LexPrimitiva::Location, 0.75)
    }
}

// ===============================================================
// BUILTIN EXECUTION
// ===============================================================

/// Executes a parsed command against the shell context.
/// Returns the output and whether the context was mutated.
///
/// This is the central dispatch function — the μ-bridge from
/// command names to implementations.
pub fn execute(cmd: &ParsedCommand, ctx: &mut ShellContext) -> Output {
    match &cmd.kind {
        CommandKind::Pwd => exec_pwd(ctx),
        CommandKind::Cd => exec_cd(&cmd.args, ctx),
        CommandKind::Ls => exec_ls(&cmd.args, ctx),
        CommandKind::Help => exec_help(&cmd.args),
        CommandKind::History => exec_history(&cmd.args, ctx),
        CommandKind::Alias => exec_alias(&cmd.args, ctx),
        CommandKind::Detect => exec_detect(&cmd.args, ctx),
        CommandKind::Workflow => exec_workflow(&cmd.args, ctx),
        CommandKind::Stream => exec_stream(&cmd.args, ctx),
        CommandKind::Learn => exec_learn(&cmd.args, ctx),
        CommandKind::Status => exec_status(ctx),
        CommandKind::Exit => exec_exit(ctx),
        CommandKind::Unknown(name) => Output::error(&format!("unknown command: {name}")),
    }
}

// ===============================================================
// COMMAND IMPLEMENTATIONS
// ===============================================================

fn exec_pwd(ctx: &ShellContext) -> Output {
    Output::text(&ctx.navigator.pwd().display())
}

fn exec_cd(args: &Args, ctx: &mut ShellContext) -> Output {
    let target = args.first().unwrap_or("/");
    ctx.navigator.cd(target);
    Output::Empty
}

fn exec_ls(args: &Args, ctx: &ShellContext) -> Output {
    let target = if let Some(path_str) = args.first() {
        ctx.navigator.pwd().join(path_str)
    } else {
        ctx.navigator.pwd().clone()
    };

    // Show path info
    let basename = target.basename().name();
    let depth = target.depth();

    // Build a table showing namespace contents
    let children = match target.namespace() {
        Some(seg) => {
            let ns = seg.name();
            match ns {
                "signals" => vec!["detected", "pending", "reviewed"],
                "cases" => vec!["pending", "processing", "closed"],
                "workflows" => vec!["running", "completed", "patterns"],
                "models" => vec!["current", "versions", "experiments"],
                "streams" => vec!["topics", "monitors"],
                "system" => vec!["health", "metrics", "config"],
                _ => vec![],
            }
        }
        None => {
            // Root listing
            vec![
                "signals",
                "cases",
                "workflows",
                "models",
                "streams",
                "system",
            ]
        }
    };

    let rows: Vec<Vec<&str>> = children.iter().map(|c| vec![*c, "dir", &"—"]).collect();

    if rows.is_empty() {
        Output::text(&format!("{basename} (depth={depth}): empty"))
    } else {
        Output::table(vec!["NAME", "TYPE", "SIZE"], rows)
    }
}

fn exec_help(args: &Args) -> Output {
    if let Some(cmd_name) = args.first() {
        let kind = CommandKind::parse(cmd_name);
        Output::text(&format!("{cmd_name}: {}", kind.help()))
    } else {
        let commands = CommandRegistry::all_commands();
        // Collect owned strings to avoid lifetime issues with temporary CommandKind
        let rows_owned: Vec<(String, String)> = commands
            .iter()
            .map(|c| {
                let kind = CommandKind::parse(c);
                ((*c).to_string(), kind.help().to_string())
            })
            .collect();

        let rows: Vec<Vec<&str>> = rows_owned
            .iter()
            .map(|(c, h)| vec![c.as_str(), h.as_str()])
            .collect();

        Output::table(vec!["COMMAND", "DESCRIPTION"], rows)
    }
}

fn exec_history(args: &Args, ctx: &ShellContext) -> Output {
    let n: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(20);

    let entries = ctx.repl.history().last_n(n);

    if entries.is_empty() {
        return Output::text("No history");
    }

    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|e| vec![format!("{}", e.seq), e.command.clone()])
        .collect();

    let row_refs: Vec<Vec<&str>> = rows
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();

    Output::table(vec!["#", "COMMAND"], row_refs)
}

fn exec_alias(args: &Args, ctx: &mut ShellContext) -> Output {
    if args.positional.len() >= 2 {
        let name = &args.positional[0];
        let expansion = args.positional[1..].join(" ");
        ctx.commands.add_alias(name, &expansion);
        Output::text(&format!("alias {name}='{expansion}'"))
    } else if let Some(name) = args.first() {
        match ctx.commands.get_alias(name) {
            Some(exp) => Output::text(&format!("{name}='{exp}'")),
            None => Output::error(&format!("no alias: {name}")),
        }
    } else {
        let aliases = ctx.commands.aliases();
        if aliases.is_empty() {
            return Output::text("No aliases defined");
        }
        let lines: Vec<String> = aliases
            .iter()
            .map(|a| format!("{}='{}'", a.name, a.expansion))
            .collect();
        Output::text(&lines.join("\n"))
    }
}

fn exec_detect(args: &Args, ctx: &ShellContext) -> Output {
    let drug = args.flag("drug").unwrap_or("unknown");
    let event = args.flag("event").unwrap_or("unknown");
    let algo = args.flag("algorithm").unwrap_or("PRR");
    let path = ctx.navigator.pwd().display();

    // In the real system this would call PVOS.detect()
    // For now, return formatted output showing what would execute
    Output::text(&format!(
        "Signal detection at {path}:\n  drug={drug}\n  event={event}\n  algorithm={algo}\n  status=ready (connect PVOS for live detection)"
    ))
}

fn exec_workflow(args: &Args, ctx: &ShellContext) -> Output {
    let action = args.first().unwrap_or("list");
    let path = ctx.navigator.pwd().display();

    match action {
        "list" => Output::table(
            vec!["ID", "NAME", "STATE"],
            vec![
                vec!["wf_1", "signal_triage", "idle"],
                vec!["wf_2", "case_processing", "idle"],
            ],
        ),
        "start" => {
            let name = args
                .positional
                .get(1)
                .map(|s| s.as_str())
                .unwrap_or("unnamed");
            Output::text(&format!("Workflow '{name}' queued at {path}"))
        }
        _ => Output::error(&format!("unknown workflow action: {action}")),
    }
}

fn exec_stream(args: &Args, ctx: &ShellContext) -> Output {
    let topic = args.first().unwrap_or("signals.detected");
    let path = ctx.navigator.pwd().display();

    Output::text(&format!(
        "Subscribed to topic '{topic}' at {path}\n  (connect PVRX for live streaming)"
    ))
}

fn exec_learn(args: &Args, ctx: &ShellContext) -> Output {
    let signal = args.flag("signal").unwrap_or("unknown");
    let outcome = args.flag("outcome").unwrap_or("unknown");
    let path = ctx.navigator.pwd().display();

    Output::text(&format!(
        "Feedback recorded at {path}:\n  signal={signal}\n  outcome={outcome}\n  (connect PVML for live learning)"
    ))
}

fn exec_status(ctx: &ShellContext) -> Output {
    let path = ctx.navigator.pwd().display();
    let stats = ctx.repl.session_stats();

    Output::table(
        vec!["METRIC", "VALUE"],
        vec![
            vec!["current_path", &path],
            vec!["session", &stats],
            vec!["stack_depth", &ctx.navigator.stack_depth().to_string()],
            vec!["aliases", &ctx.commands.aliases().len().to_string()],
        ],
    )
}

fn exec_exit(ctx: &mut ShellContext) -> Output {
    ctx.repl.request_exit();
    Output::text("Goodbye.")
}

// ===============================================================
// SHELL — THE T3 CAPSTONE
// ===============================================================

/// The PV Shell — interactive interface to the PVOS stack.
/// Tier: T3 (λ + μ + σ + ∃ + π + ς)
///
/// Provides a location-aware command interpreter. The 7th primitive
/// layer, adding λ-dominant navigation on top of the autonomous core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shell {
    /// Shell context (navigator + commands + repl).
    context: ShellContext,
    /// Completer for tab completion.
    completer: super::completion::Completer,
    /// Total inputs processed.
    total_inputs: u64,
}

impl Shell {
    /// Creates a new shell.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: ShellContext::new(),
            completer: super::completion::Completer::with_defaults(),
            total_inputs: 0,
        }
    }

    /// Returns the current prompt string.
    #[must_use]
    pub fn prompt(&self) -> String {
        self.context
            .repl
            .prompt(&self.context.navigator.pwd().display())
    }

    /// Processes a single input line. Returns the output.
    ///
    /// This is the core REPL step: parse → execute → record.
    pub fn input(&mut self, line: &str) -> Output {
        self.total_inputs += 1;

        let cmd = self.context.commands.parse(line);
        let output = execute(&cmd, &mut self.context);
        self.context.repl.record_command(line, output.is_error());

        output
    }

    /// Tab-completes the given partial input.
    #[must_use]
    pub fn complete(&self, partial: &str) -> Vec<super::completion::Candidate> {
        self.completer
            .complete(partial, self.context.navigator.pwd())
    }

    /// Whether the shell should exit.
    #[must_use]
    pub fn should_exit(&self) -> bool {
        self.context.repl.should_exit()
    }

    /// Current path.
    #[must_use]
    pub fn pwd(&self) -> &PvPath {
        self.context.navigator.pwd()
    }

    /// Shell context reference.
    #[must_use]
    pub fn context(&self) -> &ShellContext {
        &self.context
    }

    /// Total inputs processed.
    #[must_use]
    pub fn total_inputs(&self) -> u64 {
        self.total_inputs
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl GroundsTo for Shell {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Mapping,
            LexPrimitiva::Sequence,
            LexPrimitiva::Existence,
            LexPrimitiva::Persistence,
            LexPrimitiva::State,
        ])
        .with_dominant(LexPrimitiva::Location, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_exec_pwd() {
        let mut ctx = ShellContext::new();
        ctx.navigator.cd("/signals");
        let out = exec_pwd(&ctx);
        assert_eq!(out.render(), "/signals");
    }

    #[test]
    fn test_exec_cd() {
        let mut ctx = ShellContext::new();
        let args = Args::parse(&["/signals/2024"]);
        exec_cd(&args, &mut ctx);
        assert_eq!(ctx.navigator.pwd().display(), "/signals/2024");
    }

    #[test]
    fn test_exec_ls_root() {
        let ctx = ShellContext::new();
        let args = Args::default();
        let out = exec_ls(&args, &ctx);
        let rendered = out.render();
        assert!(rendered.contains("signals"));
        assert!(rendered.contains("cases"));
    }

    #[test]
    fn test_exec_help_all() {
        let out = exec_help(&Args::default());
        let rendered = out.render();
        assert!(rendered.contains("pwd"));
        assert!(rendered.contains("detect"));
    }

    #[test]
    fn test_exec_help_specific() {
        let args = Args::parse(&["detect"]);
        let out = exec_help(&args);
        assert!(!out.is_error());
    }

    #[test]
    fn test_exec_detect() {
        let ctx = ShellContext::new();
        let args = Args::parse(&["--drug", "aspirin", "--event", "headache"]);
        let out = exec_detect(&args, &ctx);
        let rendered = out.render();
        assert!(rendered.contains("aspirin"));
        assert!(rendered.contains("headache"));
    }

    #[test]
    fn test_exec_alias() {
        let mut ctx = ShellContext::new();
        // Note: Args::parse separates --flags from positional args,
        // so alias expansion only captures positional tokens after name.
        let args = Args::parse(&["d", "detect"]);
        let out = exec_alias(&args, &mut ctx);
        assert!(!out.is_error());
        assert_eq!(ctx.commands.get_alias("d"), Some("detect"));
    }

    #[test]
    fn test_exec_exit() {
        let mut ctx = ShellContext::new();
        let out = exec_exit(&mut ctx);
        assert!(ctx.repl.should_exit());
        assert!(out.render().contains("Goodbye"));
    }

    #[test]
    fn test_exec_status() {
        let ctx = ShellContext::new();
        let out = exec_status(&ctx);
        let rendered = out.render();
        assert!(rendered.contains("current_path"));
    }

    #[test]
    fn test_exec_unknown_command() {
        let mut ctx = ShellContext::new();
        let cmd = ctx.commands.parse("foobar");
        let out = execute(&cmd, &mut ctx);
        assert!(out.is_error());
    }

    #[test]
    fn test_shell_lifecycle() {
        let mut shell = Shell::new();
        assert_eq!(shell.pwd().display(), "/");
        assert!(!shell.should_exit());

        let out = shell.input("pwd");
        assert_eq!(out.render(), "/");

        shell.input("cd /signals");
        assert_eq!(shell.pwd().display(), "/signals");

        shell.input("exit");
        assert!(shell.should_exit());
        assert_eq!(shell.total_inputs(), 3);
    }

    #[test]
    fn test_shell_completion() {
        let shell = Shell::new();
        let candidates = shell.complete("st");
        let names: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
        assert!(names.contains(&"status"));
    }

    #[test]
    fn test_shell_t3_grounding() {
        let comp = Shell::primitive_composition();
        assert_eq!(
            GroundingTier::classify(&comp),
            GroundingTier::T3DomainSpecific
        );
        assert_eq!(comp.dominant, Some(LexPrimitiva::Location));
    }

    #[test]
    fn test_shell_context_grounding() {
        let comp = ShellContext::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Location));
    }

    #[test]
    fn test_shell_prompt_reflects_path() {
        let mut shell = Shell::new();
        assert!(shell.prompt().contains("/"));

        shell.input("cd /models/versions");
        assert!(shell.prompt().contains("/models/versions"));
    }

    #[test]
    fn test_shell_history_records() {
        let mut shell = Shell::new();
        shell.input("ls");
        shell.input("cd /signals");
        shell.input("pwd");

        assert_eq!(shell.context().repl.history().len(), 3);
    }

    #[test]
    fn test_exec_workflow_list() {
        let ctx = ShellContext::new();
        let args = Args::parse(&["list"]);
        let out = exec_workflow(&args, &ctx);
        assert!(!out.is_error());
    }

    #[test]
    fn test_exec_learn() {
        let ctx = ShellContext::new();
        let args = Args::parse(&["--signal", "sig_123", "--outcome", "confirmed"]);
        let out = exec_learn(&args, &ctx);
        let rendered = out.render();
        assert!(rendered.contains("sig_123"));
        assert!(rendered.contains("confirmed"));
    }
}
