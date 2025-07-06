// gitignore rules processing

use crate::repository::Repository;
use crate::{index::Index, object::GitrsObject};
use core::str;
use std::{collections::HashMap, path::PathBuf};

pub enum MatchKind {
    Include,
    Exclude,
}

pub struct IgnoreRule {
    pub pat: String,
    pub kind: MatchKind,
}

pub struct IgnoreRules {
    // Absolute ignore rules refer to those .gitignore files that live outside the repository
    // index, such as in .config/git/ignore directory, or the repository specific .git/info/exclude
    // directory
    absolute: Vec<IgnoreRule>,
    // The relative git ignore rules refer to those .gitignore files that live in the repository
    // index and only apply to those directories that they reside in
    relative: HashMap<PathBuf, Vec<IgnoreRule>>,
}

impl IgnoreRule {
    pub fn new(pat: &str, kind: MatchKind) -> Self {
        IgnoreRule {
            pat: pat.to_string(),
            kind,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let rule = match trimmed.chars().next()? {
            '!' => Self::new(&trimmed[1..], MatchKind::Include),
            '\\' => Self::new(&trimmed[1..], MatchKind::Exclude),
            _ => Self::new(trimmed, MatchKind::Exclude),
        };

        Some(rule)
    }

    pub fn parse_lines<I>(lines: I) -> Vec<IgnoreRule>
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
    pub fn read(repository: &Repository) -> Option<Self> {
        let index = Index::read(repository)?;

        let relative_ignore_rules: HashMap<_, _> = index
            .entries
            .into_iter()
            .filter_map(|entry| {
                // Only interested in ".gitignore" files
                if entry.path.file_name()? != ".gitignore" {
                    return None;
                }

                // Read the object, ensure it's a blob, and get the data
                let blob_data = match GitrsObject::read(repository, &entry.sha).ok()? {
                    GitrsObject::BlobObject(blob) => blob.get_data(),
                    _ => return None, // or panic! if it's truly an invariant
                };

                // Convert blob data to UTF-8 and parse lines
                let lines = str::from_utf8(&blob_data).ok()?.lines();
                let parent = entry.path.parent()?.to_path_buf();

                Some((parent, IgnoreRule::parse_lines(lines)))
            })
            .collect();

        Some(Self {
            absolute: Vec::new(),
            relative: relative_ignore_rules,
        })
    }
}
