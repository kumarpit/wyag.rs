use core::str;
use std::{
    fs::{self},
    io,
    path::Path,
};

use anyhow::Context;
use indexmap::IndexMap;

use crate::repository::Repository;

// Manages git references

pub struct Ref;

impl Ref {
    pub fn resolve(repository: &Repository, ref_name: &str) -> anyhow::Result<String> {
        let path = repository
            .get_path_to_file(&[ref_name])
            .with_context(|| format!("Not a file: {}", ref_name))?;

        let mut bytes =
            fs::read(&path).with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Remove trailing newline, if any
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }

        let data = str::from_utf8(&bytes).context("Ref file is not valid UTF-8")?;

        if data.starts_with("ref:") {
            // TODO: the &data[5..] slice needs to be parsed and split into a slice of path
            // components
            Ref::resolve(repository, &data[5..])
        } else {
            Ok(data.to_owned())
        }
    }

    pub fn list_at(
        repository: &Repository,
        path: &Path,
    ) -> anyhow::Result<IndexMap<String, String>> {
        let mut result: IndexMap<String, String> = IndexMap::new();
        let mut entries: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("Failed to read dir: {}", path.display()))?
            .collect::<Result<_, io::Error>>()?;

        entries.sort_by_key(|dir_entry| dir_entry.file_name());

        for dir_entry in entries.iter() {
            // TODO: handle dir case
            result.insert(
                dir_entry.file_name().to_string_lossy().into_owned(),
                Self::resolve(repository, &dir_entry.file_name().to_string_lossy())?,
            );
        }

        Ok(result)
    }

    pub fn create_at(repository: &Repository, hash: &str, paths: &[&str]) -> anyhow::Result<()> {
        let path = repository
            .create_file(paths)
            .with_context(|| format!("Couldn't create file"))?;

        fs::write(path, format!("{}\n", hash))?;
        Ok(())
    }
}
