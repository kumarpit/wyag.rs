/// Manages git references (refs), providing utilities
/// to resolve, list, and create references in a repository.
use core::str;
use std::{fs, io, path::Path};

use anyhow::Context;
use indexmap::IndexMap;

use crate::repository::Repository;

pub struct Ref;

impl Ref {
    /// Resolves a git reference to its final SHA-1 hash string.
    ///
    /// If the reference points to another ref (starts with `ref:`),
    /// recursively resolves that reference.
    pub fn resolve(repository: &Repository, ref_path: &[&str]) -> anyhow::Result<String> {
        let path = repository
            .get_path_to_file(ref_path)
            .with_context(|| format!("Not a file: {:?}", ref_path))?;

        let mut bytes =
            fs::read(&path).with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Trim trailing newline, if present
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }

        let data = str::from_utf8(&bytes).context("Ref file is not valid UTF-8")?;

        if data.starts_with("ref:") {
            let parts: Vec<&str> = data[5..].split('/').collect();
            Self::resolve(repository, &parts)
        } else {
            Ok(data.to_owned())
        }
    }

    /// Lists all references at the given directory path inside the repository.
    ///
    /// Returns an ordered map of ref names (relative paths) to their resolved SHA-1 hashes.
    pub fn list_at(
        repository: &Repository,
        path: &Path,
    ) -> anyhow::Result<IndexMap<String, String>> {
        Ok(Self::list_at_dir(repository, path)?.into_iter().collect())
    }

    /// Creates a new reference file at the specified path with the given SHA-1 hash content.
    pub fn create_at(repository: &Repository, hash: &str, paths: &[&str]) -> anyhow::Result<()> {
        let path = repository
            .create_file(paths)
            .with_context(|| format!("Couldn't create file at {:?}", paths))?;

        fs::write(path, format!("{}\n", hash))?;
        Ok(())
    }

    /// Recursively lists references inside a directory, returning a vector of
    /// (ref_path, resolved_hash) tuples.
    fn list_at_dir(repository: &Repository, path: &Path) -> anyhow::Result<Vec<(String, String)>> {
        // Collect path components as strings
        let base_parts: Vec<String> = path
            .components()
            .map(|comp| {
                comp.as_os_str()
                    .to_str()
                    .expect("Could not convert path component to string")
                    .to_string()
            })
            .collect();

        // Read directory entries and sort by filename
        let mut entries: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("Failed to read dir: {}", path.display()))?
            .collect::<Result<_, io::Error>>()?;

        entries.sort_by_key(|entry| entry.file_name());

        entries
            .iter()
            .try_fold(vec![], |mut acc, entry| -> anyhow::Result<_> {
                let file_type = entry.file_type()?;

                if file_type.is_dir() {
                    // Recurse into subdirectory
                    acc.extend(Self::list_at_dir(repository, &entry.path())?);
                } else {
                    // Resolve the ref file to its SHA
                    let file_name = entry.file_name().to_string_lossy().into_owned();
                    let full_path_parts: Vec<&str> = base_parts
                        .iter()
                        .chain(std::iter::once(&file_name))
                        .map(String::as_str)
                        .collect();

                    let resolved = Self::resolve(repository, &full_path_parts)?;
                    acc.push((full_path_parts.join("/"), resolved));
                }

                Ok(acc)
            })
    }
}
