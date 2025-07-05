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
    pub fn resolve(repository: &Repository, ref_path: &[&str]) -> anyhow::Result<String> {
        let path = repository
            .get_path_to_file(ref_path)
            .with_context(|| format!("Not a file: {:?}", ref_path))?;

        let mut bytes =
            fs::read(&path).with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Remove trailing newline, if any
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }

        let data = str::from_utf8(&bytes).context("Ref file is not valid UTF-8")?;

        if data.starts_with("ref:") {
            let parts: Vec<&str> = data[5..].split('/').collect();
            Ref::resolve(repository, &parts)
        } else {
            Ok(data.to_owned())
        }
    }

    pub fn list_at(
        repository: &Repository,
        path: &Path,
    ) -> anyhow::Result<IndexMap<String, String>> {
        Ok(Self::list_at_dir(repository, path)?.into_iter().collect())
    }

    pub fn create_at(repository: &Repository, hash: &str, paths: &[&str]) -> anyhow::Result<()> {
        let path = repository
            .create_file(paths)
            .with_context(|| format!("Couldn't create file at {:?}", paths))?;

        fs::write(path, format!("{}\n", hash))?;
        Ok(())
    }

    fn list_at_dir(repository: &Repository, path: &Path) -> anyhow::Result<Vec<(String, String)>> {
        let base_parts: Vec<String> = path
            .components()
            .map(|component| {
                component
                    .as_os_str()
                    .to_str()
                    .expect("Could not convert path component to string")
                    .to_string()
            })
            .collect();

        let mut entries: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("Failed to read dir: {}", path.display()))?
            .collect::<Result<_, io::Error>>()?;

        entries.sort_by_key(|dir_entry| dir_entry.file_name());

        entries
            .iter()
            .try_fold(vec![], |mut acc, dir_entry| -> anyhow::Result<_> {
                let file_type = dir_entry.file_type()?;
                if file_type.is_dir() {
                    acc.extend(Self::list_at_dir(repository, &dir_entry.path())?);
                } else {
                    let file_name = dir_entry.file_name().to_string_lossy().into_owned();
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
