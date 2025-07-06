// Definitions and methods for the gitrs "repository"

use core::panic;
use std::{
    env,
    fs::{self, File, canonicalize},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, ensure};
use flate2::{Compression, write::ZlibEncoder};
use log::error;

pub struct Repository {
    pub worktree: PathBuf, // canonicalized
    pub gitdir: PathBuf,
}

impl Repository {
    const REQUIRED_DIRS: [&'static [&'static str]; 4] = [
        &["branches"],
        &["objects"],
        &["refs", "tags"],
        &["refs", "heads"],
    ];

    const REQUIRED_FILES: [&'static [&'static str; 2]; 3] = [
        &[
            "description",
            "Unamed repository; edit this file 'description' to name the repository",
        ],
        &["HEAD", "ref: refs/heads/master"],
        &["config", ""],
    ];

    /////////////////////////////////////
    /// Repository Initialization
    /////////////////////////////////////

    /// Constructs an in-memory handle to an existing repository
    pub fn new(worktree: &Path) -> Self {
        Self {
            worktree: fs::canonicalize(worktree).expect("Failed to canonicalize worktree path"),
            gitdir: worktree.join(".gitrs"),
        }
    }

    /// Initializes a new gitrs repository
    pub fn init(worktree: &Path) -> Result<Self> {
        let gitdir = worktree.join(".gitrs");

        ensure!(
            worktree.exists(),
            "Invalid worktree: {}",
            worktree.display()
        );

        ensure!(
            !gitdir.exists() || is_empty_dir(&gitdir),
            "Expected empty directory at: {}",
            gitdir.display()
        );

        fs::create_dir_all(&gitdir)
            .with_context(|| format!("Failed to create directory {}", gitdir.display()))?;

        let repo = Self::new(worktree);

        for segments in Self::REQUIRED_DIRS {
            repo.compute_or_create_repo_dir(segments, true)
                .ok_or_else(|| anyhow!("Could not create directory: {:?}", segments))?;
        }

        for [file, content] in Self::REQUIRED_FILES {
            let (_, path) = repo
                .compute_or_create_repo_file(&[file], true)
                .ok_or_else(|| anyhow!("Could not create file: {}", file))?;
            repo.write_to_repo_file(&path, content.as_bytes())?;
        }

        Ok(repo)
    }

    /// Recursively searches for a repository starting from the given path
    pub fn find_repository_at(current_path: &Path) -> Option<Self> {
        let path = canonicalize(current_path).ok()?;
        if path.join(".gitrs").exists() {
            Some(Self::new(current_path))
        } else {
            path.parent().and_then(Self::find_repository_at)
        }
    }

    /// Finds the closest repository to the current working directory
    pub fn find_repository() -> Self {
        Self::find_repository_at(&env::current_dir().unwrap())
            .expect("Expected a repository at current dir")
    }

    /////////////////////////////////////
    /// Repository File Management
    /////////////////////////////////////

    pub fn get_path_to_file_if_exists(&self, paths: &[&str]) -> Option<PathBuf> {
        self.compute_or_create_repo_file(paths, false)
            .and_then(|(_, path)| path.exists().then_some(path))
    }

    pub fn get_path_to_dir_if_exists(&self, paths: &[&str]) -> Option<PathBuf> {
        self.compute_or_create_repo_dir(paths, false)
    }

    pub fn create_file(&self, paths: &[&str]) -> Option<PathBuf> {
        self.compute_or_create_repo_file(paths, true)
            .and_then(|(_, path)| path.exists().then_some(path))
    }

    pub fn contains(&self, path: &Path) -> bool {
        let canonicalized_path = fs::canonicalize(path).expect("Failed to canonicalize path");
        canonicalized_path.starts_with(&self.worktree)
    }

    /// Compresses and writes to a file (upserts if exists)
    pub fn upsert_file(&self, paths: &[&str], data: &Vec<u8>) -> Option<PathBuf> {
        let (file, path) = self.compute_or_create_repo_file(paths, true)?;
        ZlibEncoder::new(file, Compression::default())
            .write_all(data)
            .map_err(|e| error!("Could not compress file at {}: {}", path.display(), e))
            .ok()?;
        Some(path)
    }

    /// Computes a full path under `.gitrs` directory
    fn compute_repo_path(&self, paths: &[&str]) -> PathBuf {
        paths.iter().fold(self.gitdir.clone(), |mut acc, p| {
            acc.push(p);
            acc
        })
    }

    /// Computes or creates a file path under the repository
    fn compute_or_create_repo_file(
        &self,
        paths: &[&str],
        create_if_missing: bool,
    ) -> Option<(File, PathBuf)> {
        // Ensure parent dir exists or is created if create_if_missing is true
        let _dir = self.compute_or_create_repo_dir(&paths[..paths.len() - 1], create_if_missing)?;
        let path = self.compute_repo_path(paths);

        let file = if create_if_missing {
            File::create(&path)
                .map_err(|e| {
                    error!("Error creating file {}: {}", path.display(), e);
                })
                .ok()
        } else {
            File::open(&path).ok()
        }?;

        Some((file, path))
    }

    /// Computes or creates a directory path under the repository
    fn compute_or_create_repo_dir(
        &self,
        paths: &[&str],
        create_if_missing: bool,
    ) -> Option<PathBuf> {
        let path = self.compute_repo_path(paths);
        if path.exists() {
            assert!(path.is_dir(), "Expected a directory at {}", path.display());
            Some(path)
        } else if create_if_missing {
            fs::create_dir_all(&path)
                .unwrap_or_else(|e| panic!("Failed to create directory {}: {}", path.display(), e));
            Some(path)
        } else {
            None
        }
    }

    /// Writes content to a file (truncates first)
    fn write_to_repo_file(&self, path: &PathBuf, content: &[u8]) -> Result<()> {
        File::create(path)
            .with_context(|| format!("Could not create file: {}", path.display()))?
            .write_all(content)
            .with_context(|| format!("Could not write to file: {}", path.display()))
    }
}

/// Returns true if a directory exists and is empty
pub fn is_empty_dir(path: &Path) -> bool {
    path.is_dir() && fs::read_dir(path).map_or(false, |mut entries| entries.next().is_none())
}
