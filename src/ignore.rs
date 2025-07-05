// gitignore rules processing

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
    absolute: Vec<IgnoreRule>,
    relative: HashMap<PathBuf, IgnoreRule>,
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
    pub fn read() {
        todo!();
    }
}
