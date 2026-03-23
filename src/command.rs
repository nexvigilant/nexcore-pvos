//! # PVSH Command System
//!
//! Command parsing, registration, and dispatch for the PV shell.
//! Commands are the μ-bridge: they map user intent to system operations.
//!
//! ## Primitives
//! - μ (Mapping) — command name → handler dispatch
//! - λ (Location) — commands operate on paths
//! - σ (Sequence) — command pipelines, argument lists

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

// ===============================================================
// COMMAND TYPES
// ===============================================================

/// Named command identifier.
/// Tier: T2-P (μ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandName(pub String);

impl CommandName {
    /// Creates a command name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_lowercase())
    }
}

impl GroundsTo for CommandName {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping])
    }
}

/// Parsed command arguments.
/// Tier: T2-P (σ)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Args {
    /// Positional arguments.
    pub positional: Vec<String>,
    /// Named flags (--name value).
    pub flags: Vec<(String, String)>,
    /// Boolean switches (--verbose).
    pub switches: Vec<String>,
}

impl Args {
    /// Parses a slice of argument tokens.
    #[must_use]
    pub fn parse(tokens: &[&str]) -> Self {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut switches = Vec::new();

        let mut i = 0;
        while i < tokens.len() {
            let token = tokens[i];
            if token.starts_with("--") {
                let name = token.trim_start_matches('-').to_string();
                if i + 1 < tokens.len() && !tokens[i + 1].starts_with("--") {
                    flags.push((name, tokens[i + 1].to_string()));
                    i += 2;
                } else {
                    switches.push(name);
                    i += 1;
                }
            } else {
                positional.push(token.to_string());
                i += 1;
            }
        }

        Self {
            positional,
            flags,
            switches,
        }
    }

    /// Gets a flag value by name.
    #[must_use]
    pub fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Checks if a switch is present.
    #[must_use]
    pub fn has_switch(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }

    /// Gets the first positional argument.
    #[must_use]
    pub fn first(&self) -> Option<&str> {
        self.positional.first().map(|s| s.as_str())
    }
}

impl GroundsTo for Args {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Sequence])
    }
}

// ===============================================================
// COMMAND OUTPUT
// ===============================================================

/// Output format from a command execution.
/// Tier: T2-P (μ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Output {
    /// Plain text output.
    Text(String),
    /// Tabular data (headers + rows).
    Table {
        /// Column headers.
        headers: Vec<String>,
        /// Row data.
        rows: Vec<Vec<String>>,
    },
    /// Structured JSON output.
    Json(String),
    /// Error message.
    Error(String),
    /// Empty / no output.
    Empty,
}

impl Output {
    /// Creates text output.
    #[must_use]
    pub fn text(msg: &str) -> Self {
        Self::Text(msg.to_string())
    }

    /// Creates error output.
    #[must_use]
    pub fn error(msg: &str) -> Self {
        Self::Error(msg.to_string())
    }

    /// Creates a table.
    #[must_use]
    pub fn table(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> Self {
        Self::Table {
            headers: headers.into_iter().map(|h| h.to_string()).collect(),
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(|c| c.to_string()).collect())
                .collect(),
        }
    }

    /// Whether this output is an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Whether this output is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Formats output as a display string.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Text(t) => t.clone(),
            Self::Table { headers, rows } => {
                let mut lines = Vec::new();
                lines.push(headers.join("\t"));
                for row in rows {
                    lines.push(row.join("\t"));
                }
                lines.join("\n")
            }
            Self::Json(j) => j.clone(),
            Self::Error(e) => format!("error: {e}"),
            Self::Empty => String::new(),
        }
    }
}

impl GroundsTo for Output {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping])
    }
}

// ===============================================================
// COMMAND KIND
// ===============================================================

/// Built-in command types.
/// Tier: T2-P (μ + λ)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandKind {
    /// Print working directory.
    Pwd,
    /// Change directory.
    Cd,
    /// List resources at path.
    Ls,
    /// Show help.
    Help,
    /// Show command history.
    History,
    /// Define/show aliases.
    Alias,
    /// Run signal detection.
    Detect,
    /// Trigger/manage workflows.
    Workflow,
    /// Subscribe to reactive streams.
    Stream,
    /// Record feedback / trigger learning.
    Learn,
    /// Show system status and health.
    Status,
    /// Exit the shell.
    Exit,
    /// Unknown command.
    Unknown(String),
}

impl CommandKind {
    /// Parses a command name string.
    #[must_use]
    pub fn parse(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "pwd" => Self::Pwd,
            "cd" => Self::Cd,
            "ls" | "list" => Self::Ls,
            "help" | "?" => Self::Help,
            "history" => Self::History,
            "alias" => Self::Alias,
            "detect" | "signal" => Self::Detect,
            "workflow" | "wf" => Self::Workflow,
            "stream" | "subscribe" => Self::Stream,
            "learn" | "feedback" => Self::Learn,
            "status" | "health" => Self::Status,
            "exit" | "quit" | "q" => Self::Exit,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Whether this is a known builtin.
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    /// Help text for this command.
    #[must_use]
    pub fn help(&self) -> &str {
        match self {
            Self::Pwd => "Print current working path",
            Self::Cd => "Change directory: cd <path>",
            Self::Ls => "List resources at current or given path",
            Self::Help => "Show help for commands",
            Self::History => "Show command history",
            Self::Alias => "Define or list aliases: alias <name> <command>",
            Self::Detect => "Run signal detection: detect --drug <d> --event <e>",
            Self::Workflow => "Manage workflows: workflow <action> [args]",
            Self::Stream => "Subscribe to streams: stream <topic>",
            Self::Learn => "Record feedback: learn --signal <id> --outcome <o>",
            Self::Status => "Show system health and metrics",
            Self::Exit => "Exit the shell",
            Self::Unknown(_) => "Unknown command",
        }
    }
}

impl GroundsTo for CommandKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping, LexPrimitiva::Location])
    }
}

// ===============================================================
// PARSED COMMAND
// ===============================================================

/// A fully parsed command ready for execution.
/// Tier: T2-C (μ + λ + σ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// The command kind.
    pub kind: CommandKind,
    /// Parsed arguments.
    pub args: Args,
    /// Raw input string.
    pub raw: String,
}

impl GroundsTo for ParsedCommand {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Mapping,
            LexPrimitiva::Location,
            LexPrimitiva::Sequence,
        ])
    }
}

// ===============================================================
// COMMAND REGISTRY
// ===============================================================

/// Alias mapping for command shortcuts.
/// Tier: T2-P (μ + π)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alias {
    /// Short name.
    pub name: String,
    /// Expansion.
    pub expansion: String,
}

impl GroundsTo for Alias {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Mapping, LexPrimitiva::Persistence])
    }
}

/// Command registry — maps names to command kinds and aliases.
/// Tier: T2-C (μ + λ + σ + π)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandRegistry {
    /// User-defined aliases.
    aliases: Vec<Alias>,
}

impl CommandRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses an input line into a command.
    #[must_use]
    pub fn parse(&self, input: &str) -> ParsedCommand {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return ParsedCommand {
                kind: CommandKind::Unknown(String::new()),
                args: Args::default(),
                raw: String::new(),
            };
        }

        // Expand aliases
        let expanded = self.expand_alias(trimmed);

        let parts: Vec<&str> = expanded.split_whitespace().collect();
        let (cmd_name, arg_tokens) = parts.split_first().map_or(("", &[][..]), |(c, a)| (*c, a));

        ParsedCommand {
            kind: CommandKind::parse(cmd_name),
            args: Args::parse(arg_tokens),
            raw: trimmed.to_string(),
        }
    }

    /// Registers an alias.
    pub fn add_alias(&mut self, name: &str, expansion: &str) {
        // Remove existing alias with same name
        self.aliases.retain(|a| a.name != name);
        self.aliases.push(Alias {
            name: name.to_string(),
            expansion: expansion.to_string(),
        });
    }

    /// Gets an alias expansion.
    #[must_use]
    pub fn get_alias(&self, name: &str) -> Option<&str> {
        self.aliases
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.expansion.as_str())
    }

    /// All aliases.
    #[must_use]
    pub fn aliases(&self) -> &[Alias] {
        &self.aliases
    }

    /// Returns all known command names.
    #[must_use]
    pub fn all_commands() -> Vec<&'static str> {
        vec![
            "pwd", "cd", "ls", "help", "history", "alias", "detect", "workflow", "stream", "learn",
            "status", "exit",
        ]
    }

    fn expand_alias(&self, input: &str) -> String {
        let first_word = input.split_whitespace().next().unwrap_or("");
        if let Some(expansion) = self.get_alias(first_word) {
            let rest = input[first_word.len()..].trim_start();
            if rest.is_empty() {
                expansion.to_string()
            } else {
                format!("{expansion} {rest}")
            }
        } else {
            input.to_string()
        }
    }
}

impl GroundsTo for CommandRegistry {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Mapping,
            LexPrimitiva::Location,
            LexPrimitiva::Sequence,
            LexPrimitiva::Persistence,
        ])
        .with_dominant(LexPrimitiva::Mapping, 0.80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_parse_simple_command() {
        let reg = CommandRegistry::new();
        let cmd = reg.parse("pwd");
        assert_eq!(cmd.kind, CommandKind::Pwd);
        assert!(cmd.args.positional.is_empty());
    }

    #[test]
    fn test_parse_command_with_args() {
        let reg = CommandRegistry::new();
        let cmd = reg.parse("cd /signals/2024");
        assert_eq!(cmd.kind, CommandKind::Cd);
        assert_eq!(cmd.args.first(), Some("/signals/2024"));
    }

    #[test]
    fn test_parse_command_with_flags() {
        let reg = CommandRegistry::new();
        let cmd = reg.parse("detect --drug aspirin --event headache");
        assert_eq!(cmd.kind, CommandKind::Detect);
        assert_eq!(cmd.args.flag("drug"), Some("aspirin"));
        assert_eq!(cmd.args.flag("event"), Some("headache"));
    }

    #[test]
    fn test_parse_command_with_switches() {
        let reg = CommandRegistry::new();
        let cmd = reg.parse("ls --verbose");
        assert_eq!(cmd.kind, CommandKind::Ls);
        assert!(cmd.args.has_switch("verbose"));
    }

    #[test]
    fn test_parse_unknown_command() {
        let reg = CommandRegistry::new();
        let cmd = reg.parse("foobar");
        assert!(!cmd.kind.is_builtin());
    }

    #[test]
    fn test_alias_expansion() {
        let mut reg = CommandRegistry::new();
        reg.add_alias("d", "detect --drug");

        let cmd = reg.parse("d aspirin --event headache");
        assert_eq!(cmd.kind, CommandKind::Detect);
        assert_eq!(cmd.args.flag("drug"), Some("aspirin"));
    }

    #[test]
    fn test_alias_overwrite() {
        let mut reg = CommandRegistry::new();
        reg.add_alias("x", "ls");
        reg.add_alias("x", "pwd");

        assert_eq!(reg.get_alias("x"), Some("pwd"));
    }

    #[test]
    fn test_command_kind_parse_aliases() {
        assert_eq!(CommandKind::parse("list"), CommandKind::Ls);
        assert_eq!(CommandKind::parse("wf"), CommandKind::Workflow);
        assert_eq!(CommandKind::parse("?"), CommandKind::Help);
        assert_eq!(CommandKind::parse("quit"), CommandKind::Exit);
        assert_eq!(CommandKind::parse("q"), CommandKind::Exit);
    }

    #[test]
    fn test_command_help_text() {
        assert!(!CommandKind::Pwd.help().is_empty());
        assert!(!CommandKind::Detect.help().is_empty());
        assert!(!CommandKind::Exit.help().is_empty());
    }

    #[test]
    fn test_output_text() {
        let out = Output::text("hello");
        assert_eq!(out.render(), "hello");
        assert!(!out.is_error());
        assert!(!out.is_empty());
    }

    #[test]
    fn test_output_table() {
        let out = Output::table(
            vec!["NAME", "TYPE"],
            vec![vec!["aspirin", "drug"], vec!["headache", "event"]],
        );
        let rendered = out.render();
        assert!(rendered.contains("NAME"));
        assert!(rendered.contains("aspirin"));
    }

    #[test]
    fn test_output_error() {
        let out = Output::error("not found");
        assert!(out.is_error());
        assert!(out.render().contains("not found"));
    }

    #[test]
    fn test_command_registry_grounding() {
        let comp = CommandRegistry::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Composite);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Mapping));
    }

    #[test]
    fn test_parsed_command_grounding() {
        let comp = ParsedCommand::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
