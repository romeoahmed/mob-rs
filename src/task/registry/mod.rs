// mob-rs: `ModOrganizer` Build Tool - Rust Port
//
// SPDX-FileCopyrightText: 2026 Romeo Ahmed
// SPDX-License-Identifier: GPL-3.0-or-later

//! Task registry for resolving task names and aliases.
//!
//! ```text
//! input ["super", "mod*"]
//!   resolve_aliases: "super" --> [usvfs, modorganizer, ...]
//!   match_pattern:   "mod*"  --> [modorganizer, modorganizer-archive, ...]
//!   dedupe + preserve order
//! ```

use std::collections::BTreeSet;

use crate::error::Result;
use anyhow::Context;
use wax::{Glob, Program};

use crate::config::types::Aliases;

/// Registry for looking up tasks by name or pattern.
pub struct TaskRegistry {
    /// Available task names.
    task_names: BTreeSet<String>,

    /// Aliases mapping names to task lists.
    aliases: Aliases,
}

impl TaskRegistry {
    /// Creates a new `TaskRegistry`.
    #[must_use]
    pub const fn new(aliases: Aliases) -> Self {
        Self {
            task_names: BTreeSet::new(),
            aliases,
        }
    }

    /// Registers a task name.
    pub fn register(&mut self, name: impl Into<String>) {
        self.task_names.insert(name.into());
    }

    /// Registers multiple task names.
    pub fn register_all(&mut self, names: impl IntoIterator<Item = impl Into<String>>) {
        for name in names {
            self.task_names.insert(name.into());
        }
    }

    /// Returns all registered task names.
    #[must_use]
    pub const fn all_tasks(&self) -> &BTreeSet<String> {
        &self.task_names
    }

    /// Resolves aliases in a list of task patterns.
    ///
    /// If a pattern matches an alias, it's expanded to the alias targets.
    /// Non-alias patterns are returned as-is.
    #[must_use]
    pub fn resolve_aliases(&self, patterns: &[String]) -> Vec<String> {
        let mut result = Vec::new();

        for pattern in patterns {
            if let Some(targets) = self.aliases.get(pattern) {
                // Alias found - expand it (recursively resolve nested aliases)
                result.extend(self.resolve_aliases(targets));
            } else {
                // Not an alias - keep as-is
                result.push(pattern.clone());
            }
        }

        result
    }

    /// Matches a glob pattern against registered task names.
    ///
    /// Returns all task names that match the pattern.
    ///
    /// # Examples
    ///
    /// - `"*"` matches all tasks
    /// - `"usvfs*"` matches "usvfs", "usvfs-dll", etc.
    /// - `"mod*"` matches "modorganizer", "modorganizer-archive", etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the pattern is not a valid glob.
    pub fn match_pattern(&self, pattern: &str) -> Result<Vec<String>> {
        // If pattern is an exact match, return it directly
        if self.task_names.contains(pattern) {
            return Ok(vec![pattern.to_string()]);
        }

        // Try to parse as glob pattern
        let glob =
            Glob::new(pattern).with_context(|| format!("Invalid glob pattern: {pattern}"))?;

        let matched: Vec<String> = self
            .task_names
            .iter()
            .filter(|name| glob.is_match(name.as_str()))
            .cloned()
            .collect();

        Ok(matched)
    }

    /// Resolves a list of task specifications to concrete task names.
    ///
    /// This method:
    /// 1. Expands aliases
    /// 2. Matches glob patterns
    /// 3. Deduplicates results
    /// 4. Preserves order
    ///
    /// # Errors
    ///
    /// Returns an error if any of the specifications contain an invalid glob pattern.
    pub fn resolve(&self, specs: &[String]) -> Result<Vec<String>> {
        // First expand all aliases
        let expanded = self.resolve_aliases(specs);

        // Then match each pattern
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();

        for pattern in &expanded {
            let matches = self.match_pattern(pattern)?;

            if matches.is_empty() {
                tracing::warn!(pattern = %pattern, "Pattern matched no tasks");
            }

            for name in matches {
                if seen.insert(name.clone()) {
                    result.push(name);
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests;
