//! # PVSH Tab Completion
//!
//! Context-aware completion for paths, commands, and arguments.
//! Completion is the ∃-operation on λ-space: check what exists
//! at partial path locations.
//!
//! ## Primitives
//! - λ (Location) — partial path resolution
//! - ∃ (Existence) — what exists at this prefix?
//! - μ (Mapping) — command name completion

use serde::{Deserialize, Serialize};

use nexcore_lex_primitiva::{GroundsTo, LexPrimitiva, PrimitiveComposition};

use super::command::CommandRegistry;
use super::path::{PathResolver, PathSegment, PvPath};

// ===============================================================
// COMPLETION TYPES
// ===============================================================

/// What kind of completion is being requested.
/// Tier: T2-P (λ + ∃)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionKind {
    /// Completing a command name.
    Command,
    /// Completing a path.
    Path,
    /// Completing a flag name.
    Flag,
    /// Completing a flag value.
    Value,
}

impl GroundsTo for CompletionKind {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Location, LexPrimitiva::Existence])
    }
}

/// A single completion candidate.
/// Tier: T2-P (λ)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// The completed text.
    pub text: String,
    /// Brief description.
    pub description: String,
    /// What kind of completion this is.
    pub kind: CompletionKind,
    /// Whether this is a directory/container (append `/`).
    pub is_directory: bool,
}

impl Candidate {
    /// Creates a command completion candidate.
    #[must_use]
    pub fn command(name: &str, desc: &str) -> Self {
        Self {
            text: name.to_string(),
            description: desc.to_string(),
            kind: CompletionKind::Command,
            is_directory: false,
        }
    }

    /// Creates a path completion candidate.
    #[must_use]
    pub fn path(name: &str, is_dir: bool) -> Self {
        Self {
            text: name.to_string(),
            description: if is_dir { "directory" } else { "resource" }.to_string(),
            kind: CompletionKind::Path,
            is_directory: is_dir,
        }
    }

    /// Creates a flag completion candidate.
    #[must_use]
    pub fn flag(name: &str, desc: &str) -> Self {
        Self {
            text: format!("--{name}"),
            description: desc.to_string(),
            kind: CompletionKind::Flag,
            is_directory: false,
        }
    }

    /// The display text (with trailing `/` for directories).
    #[must_use]
    pub fn display(&self) -> String {
        if self.is_directory {
            format!("{}/", self.text)
        } else {
            self.text.clone()
        }
    }
}

impl GroundsTo for Candidate {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![LexPrimitiva::Location])
    }
}

// ===============================================================
// COMPLETER
// ===============================================================

/// Completion engine — generates candidates based on context.
/// Tier: T2-C (λ + ∃ + μ)
///
/// Uses the path resolver and command registry to suggest completions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Completer {
    /// Path resolver for path completions.
    resolver: PathResolver,
    /// Cached command flags per command kind.
    command_flags: Vec<(String, Vec<String>)>,
}

impl Completer {
    /// Creates a completer with default PVOS namespace.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut command_flags = Vec::new();

        command_flags.push((
            "detect".to_string(),
            vec![
                "drug".into(),
                "event".into(),
                "algorithm".into(),
                "threshold".into(),
            ],
        ));
        command_flags.push((
            "workflow".to_string(),
            vec!["name".into(), "signal".into(), "priority".into()],
        ));
        command_flags.push((
            "stream".to_string(),
            vec!["topic".into(), "filter".into(), "limit".into()],
        ));
        command_flags.push((
            "learn".to_string(),
            vec!["signal".into(), "outcome".into(), "source".into()],
        ));

        Self {
            resolver: PathResolver::with_defaults(),
            command_flags,
        }
    }

    /// Generates completions for the given input at cursor position.
    #[must_use]
    pub fn complete(&self, input: &str, current_path: &PvPath) -> Vec<Candidate> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return self.complete_commands("");
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        // First word → command completion
        if parts.len() <= 1 && !trimmed.ends_with(' ') {
            return self.complete_commands(parts.first().copied().unwrap_or(""));
        }

        // After command → argument completion
        let cmd = parts.first().copied().unwrap_or("");
        let last = parts.last().copied().unwrap_or("");

        // If last token starts with `--`, complete flags
        if last.starts_with("--") {
            return self.complete_flags(cmd, last.trim_start_matches('-'));
        }

        // If command is cd/ls or last looks like a path, complete paths
        if cmd == "cd" || cmd == "ls" || last.contains('/') {
            return self.complete_paths(last, current_path);
        }

        // Default: try path completion
        self.complete_paths(last, current_path)
    }

    fn complete_commands(&self, prefix: &str) -> Vec<Candidate> {
        CommandRegistry::all_commands()
            .into_iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| {
                let kind = super::command::CommandKind::parse(c);
                Candidate::command(c, kind.help())
            })
            .collect()
    }

    fn complete_paths(&self, partial: &str, current: &PvPath) -> Vec<Candidate> {
        let (parent_path, prefix) = if partial.contains('/') {
            let last_slash = partial.rfind('/').unwrap_or(0);
            let parent_str = &partial[..=last_slash];
            let prefix = &partial[last_slash + 1..];
            (current.join(parent_str), prefix.to_string())
        } else {
            (current.clone(), partial.to_string())
        };

        let children = self.resolver.children_of(&parent_path);
        children
            .into_iter()
            .filter(|c| c.starts_with(&prefix))
            .map(|c| {
                let seg = PathSegment::parse(&c);
                Candidate::path(&c, seg.is_namespace())
            })
            .collect()
    }

    fn complete_flags(&self, command: &str, prefix: &str) -> Vec<Candidate> {
        self.command_flags
            .iter()
            .filter(|(cmd, _)| cmd == command)
            .flat_map(|(_, flags)| flags.iter())
            .filter(|f| f.starts_with(prefix))
            .map(|f| Candidate::flag(f, ""))
            .collect()
    }

    /// Resolver reference.
    #[must_use]
    pub fn resolver(&self) -> &PathResolver {
        &self.resolver
    }

    /// Mutable resolver reference for dynamic registration.
    pub fn resolver_mut(&mut self) -> &mut PathResolver {
        &mut self.resolver
    }
}

impl GroundsTo for Completer {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Location,
            LexPrimitiva::Existence,
            LexPrimitiva::Mapping,
        ])
        .with_dominant(LexPrimitiva::Location, 0.75)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::GroundingTier;

    #[test]
    fn test_complete_empty_input() {
        let completer = Completer::with_defaults();
        let candidates = completer.complete("", &PvPath::root());
        // All commands should appear
        assert!(candidates.len() >= 10);
    }

    #[test]
    fn test_complete_command_prefix() {
        let completer = Completer::with_defaults();
        let candidates = completer.complete("st", &PvPath::root());
        let names: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
        assert!(names.contains(&"status"));
        assert!(names.contains(&"stream"));
    }

    #[test]
    fn test_complete_path_at_root() {
        let completer = Completer::with_defaults();
        let candidates = completer.complete("cd s", &PvPath::root());
        let names: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
        assert!(names.contains(&"signals"));
        assert!(names.contains(&"streams"));
        assert!(names.contains(&"system"));
    }

    #[test]
    fn test_complete_nested_path() {
        let completer = Completer::with_defaults();
        let candidates = completer.complete("cd /cases/p", &PvPath::root());
        let names: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
        assert!(names.contains(&"pending"));
        assert!(names.contains(&"processing"));
    }

    #[test]
    fn test_complete_flags() {
        let completer = Completer::with_defaults();
        let candidates = completer.complete("detect --dr", &PvPath::root());
        assert!(!candidates.is_empty());
        assert!(candidates[0].text.contains("drug"));
    }

    #[test]
    fn test_candidate_display_directory() {
        let c = Candidate::path("signals", true);
        assert_eq!(c.display(), "signals/");
    }

    #[test]
    fn test_candidate_display_resource() {
        let c = Candidate::path("aspirin", false);
        assert_eq!(c.display(), "aspirin");
    }

    #[test]
    fn test_completer_grounding() {
        let comp = Completer::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
        assert_eq!(comp.dominant, Some(LexPrimitiva::Location));
    }

    #[test]
    fn test_completion_kind_grounding() {
        let comp = CompletionKind::primitive_composition();
        assert_eq!(GroundingTier::classify(&comp), GroundingTier::T2Primitive);
    }
}
