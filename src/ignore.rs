use log::{debug, info};

use crate::repository::Repository;
use crate::{index::Index, object::GitrsObject};
use core::str;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

/// Describes how an ignore rule should behave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchKind {
    Include,
    Exclude,
}

/// A single `.gitignore`-style rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreRule {
    pub pat: glob::Pattern,
    pub kind: MatchKind,
}

/// Collection of `.gitignore` rules, both absolute and relative.
#[derive(Debug)]
pub struct IgnoreRules {
    /// Ignore rules from outside the repo (e.g. `~/.config/git/ignore`)
    absolute: Vec<IgnoreRule>,
    /// `.gitignore` rules tracked in the repo, keyed by their parent directory.
    relative: HashMap<PathBuf, Vec<IgnoreRule>>,
}

impl From<(&str, MatchKind)> for IgnoreRule {
    fn from((pat, kind): (&str, MatchKind)) -> Self {
        Self {
            pat: glob::Pattern::new(pat)
                .expect(&format!("Couldn't create glob pattern from {}", pat)),
            kind,
        }
    }
}

impl IgnoreRule {
    /// Parses a single line into an ignore rule.
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let (kind, pattern) = match trimmed.chars().next()? {
            '!' => (MatchKind::Include, &trimmed[1..]),
            '\\' => (MatchKind::Exclude, &trimmed[1..]),
            _ => (MatchKind::Exclude, trimmed),
        };

        Some(IgnoreRule::from((pattern, kind)))
    }

    /// Parses multiple lines into ignore rules.
    pub fn parse_lines<I>(lines: I) -> Vec<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        lines
            .into_iter()
            .filter_map(|line| Self::parse(line.as_ref()))
            .collect()
    }
}

impl IgnoreRules {
    /// Reads all `.gitignore` rules from the repository's index.
    pub fn read(repository: &Repository) -> Option<Self> {
        let index = Index::read(repository)?;

        let relative = index
            .entries
            .into_iter()
            .filter_map(|entry| {
                let file_name = entry.path.file_name()?;
                if file_name != ".gitrsignore" {
                    return None;
                }

                let blob_data = match GitrsObject::read(repository, &entry.sha).ok()? {
                    GitrsObject::BlobObject(blob) => blob.get_data(),
                    _ => panic!("Malformed .gitignore entry, not a blob object"),
                };

                debug!("Dumping .gitrsignore data");
                GitrsObject::dump(&blob_data);

                let lines = str::from_utf8(&blob_data).ok()?.lines();
                let parent = entry.path.parent()?.to_path_buf();

                Some((parent, IgnoreRule::parse_lines(lines)))
            })
            .collect();

        debug!("Relative ignore rules: {:?}", relative);

        Some(Self {
            absolute: Vec::new(), // TODO: read absolute ignore rules
            relative,
        })
    }

    /// Checks if the given path matches any ignore rules. Checks for the nearest gitrsignore file,
    /// eventually checking the absolute ignore rules if none of the repository-specific ignore
    /// rules match
    pub fn check(&self, path: &Path) -> Option<MatchKind> {
        debug!("Called check on: {:?}", path.display());
        std::iter::successors(path.parent(), |p| p.parent()).find_map(|parent| {
            // TODO: if nothing matches then look up absolute rules
            debug!("Trying to lookup path: {:?}", parent.display());
            self.relative
                .get(parent)
                .and_then(|rule_set| Self::matches_rules(rule_set, path))
        })
    }

    // If the path matches some rule, returns whether to include or exclude the file
    fn matches_rules(rules: &Vec<IgnoreRule>, path: &Path) -> Option<MatchKind> {
        for IgnoreRule { pat, kind } in rules {
            info!("Matching pattern: {:?} for {}", pat, path.display());
            if pat.matches_path(path) {
                return Some(kind.to_owned());
            }
        }
        None
    }
}
