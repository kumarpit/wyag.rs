use std::{fs, path::PathBuf, time::SystemTime};

use crate::repository::Repository;

pub struct Index {
    pub version: u32,
    pub entries: Vec<IndexEntry>,
}

// NOTE: This is simplified and stores a lot less information than actual Git index entries.
pub struct IndexEntry {
    // Last time when the file's data was modified
    pub mtime: SystemTime,
    pub sha: String,
    // Size of the file
    pub size_in_bytes: u64,
    // Absolute path
    pub path: PathBuf,
}

impl Index {
    pub fn read(repository: &Repository) -> Option<Self> {
        // index may not exist (in the case of a fresh repository)
        let index_file_path = repository.get_path_to_file(&["index"])?;
        let data = fs::read(index_file_path).expect("Couldn't read index file");

        if &data[..4] != b"DIRC" {
            panic!("Invalid index signature");
        }

        let version = u32::from_be_bytes(data[4..8].try_into().unwrap());
        if version != 2 {
            panic!("Only index version 2 is supported");
        }

        let count = u32::from_be_bytes(data[8..12].try_into().unwrap());
        let content = &data[8..];

        for _ in 0..count {
            // TODO: parse git index
        }

        None
    }
}
