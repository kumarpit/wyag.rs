use std::{fs, path::PathBuf, time::SystemTime};

use crate::repository::Repository;

pub struct Index {
    version: usize,
    entries: Vec<IndexEntry>,
}

// NOTE: This is simplified and stores a lot less information than actual Git index entries.
pub struct IndexEntry {
    // Last time when the file's data was modified
    mtime: SystemTime,
    sha: String,
    // Size of the file
    size_in_bytes: u64,
    // Absolute path
    path: PathBuf,
}

impl Index {
    pub fn read(repository: &Repository) -> Option<Self> {
        let index_file_path = repository.get_path_to_file(&["index"])?;
        let data = fs::read(index_file_path).expect("Couldn't read index file");
        todo!();
    }
}
