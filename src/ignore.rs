use crate::repository::Repository;
use crate::{index::Index, object::GitrsObject};
use core::str;
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
    pub pat: String,
    pub kind: MatchKind,
}

impl From<(&str, MatchKind)> for IgnoreRule {
    fn from((pat, kind): (&str, MatchKind)) -> Self {
        Self {
            pat: pat.to_string(),
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

/// Collection of `.gitignore` rules, both absolute and relative.
#[derive(Debug)]
pub struct IgnoreRules {
    /// Ignore rules from outside the repo (e.g. `~/.config/git/ignore`)
    absolute: Vec<IgnoreRule>,
    /// `.gitignore` rules tracked in the repo, keyed by their parent directory.
    relative: HashMap<PathBuf, Vec<IgnoreRule>>,
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
                if file_name != ".gitignore" {
                    return None;
                }

                let blob_data = match GitrsObject::read(repository, &entry.sha).ok()? {
                    GitrsObject::BlobObject(blob) => blob.get_data(),
                    _ => return None, // this should never happen...
                };

                let lines = str::from_utf8(&blob_data).ok()?.lines();
                let parent = entry.path.parent()?.to_path_buf();

                Some((parent, IgnoreRule::parse_lines(lines)))
            })
            .collect();

        Some(Self {
            absolute: Vec::new(), // TODO: read absolute ignore rules
            relative,
        })
    }
}
