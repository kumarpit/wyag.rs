// Definitions and methods for the gitrs "repository"
use core::panic;
use std::fs::{self, File, canonicalize};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Ok, Result, anyhow, bail, ensure};
use flate2::Compression;
use flate2::write::ZlibEncoder;

pub struct Repository {
    pub worktree: PathBuf,
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
        &["HEAD", "refs/head/master"],
        &["config", ""],
    ];

    /////////////////////////////////////
    /// Repository Initialization
    /////////////////////////////////////

    // Construct a Repository object
    // WARN: Use this to create an in-memory representation of an existing repository, not to
    // initialize a new repository
    pub fn new(worktree: &Path) -> Self {
        Self {
            worktree: worktree.to_path_buf(),
            gitdir: worktree.join(".gitrs"),
        }
    }

    // Initialize a gitrs repository within the given worktree path
    // TODO: Perform clean up of resources if this bails
    pub fn init(worktree: &Path) -> Result<Self> {
        let gitdir = worktree.join(".gitrs");
        ensure!(
            worktree.exists(),
            "Invalid worktree: {}",
            worktree.display()
        );

        if gitdir.exists() && !is_empty_dir(gitdir.as_path()) {
            bail!("Expected empty directory at: {}", gitdir.display());
        } else {
            fs::create_dir_all(gitdir.as_path())
                .with_context(|| format!("Failed to create the path {}", gitdir.display()))?;
        }

        let repository = Self::new(worktree);

        Self::REQUIRED_DIRS.iter().try_for_each(|segments| {
            repository
                .compute_or_create_repo_dir(segments, true)
                .ok_or(anyhow!(
                    "Could not create paths for segments: {:?}",
                    segments
                ))?;
            Ok(())
        })?;

        Self::REQUIRED_FILES
            .iter()
            .try_for_each(|[file, content]| {
                repository.write_to_repo_file(
                    &repository
                        .compute_or_create_repo_file(&[file], true)
                        .ok_or(anyhow!("Could not make file: {}", file))?
                        .1,
                    content.as_bytes(),
                )?;
                Ok(())
            })?;

        Ok(repository)
    }

    /// Finds the root directory of the nearest gitrs repository by traversing parents of the
    /// `current_path`
    pub fn find_repository(current_path: &Path) -> Option<Repository> {
        let canonical_current_path = canonicalize(current_path).ok()?;
        canonical_current_path
            .join(".gitrs")
            .exists()
            .then(|| Repository::new(current_path))
            .or_else(|| match canonical_current_path.parent() {
                None => None,
                Some(parent_dir) => Repository::find_repository(parent_dir),
            })
    }

    /////////////////////////////////////
    /// Repository File Management
    /////////////////////////////////////

    pub fn get_path_to_file(&self, paths: &[&str]) -> Option<PathBuf> {
        self.compute_or_create_repo_file(paths, false)
            .and_then(|(_, path)| path.exists().then(|| path))
    }

    pub fn get_path_to_dir(&self, paths: &[&str]) -> Option<PathBuf> {
        self.compute_or_create_repo_dir(paths, false)
    }

    // Creates the file if it does not exists or truncates it if it does and appends compressed
    // data
    pub fn upsert_file(&self, paths: &[&str], data: &Vec<u8>) -> Option<PathBuf> {
        let (file, path) = self.compute_or_create_repo_file(paths, true)?;
        ZlibEncoder::new(file, Compression::default())
            .write_all(&data)
            .map_err(|e| eprintln!("Could not compress file at: {} {}", path.display(), e))
            .ok()?;

        Some(path)
    }

    // Computes the path under a repository's gitrs directory
    fn compute_repo_path(&self, paths: &[&str]) -> PathBuf {
        paths.iter().fold(self.gitdir.clone(), |mut acc, path| {
            acc.push(path);
            acc
        })
    }

    // Creates the trailing directories and file if the should_create flag is set
    fn compute_or_create_repo_file(
        &self,
        paths: &[&str],
        should_create: bool,
    ) -> Option<(File, PathBuf)> {
        match self.compute_or_create_repo_dir(&paths[..paths.len() - 1], should_create) {
            Some(_) => {
                let file_path = self.compute_repo_path(paths);
                let file = should_create
                    .then(|| {
                        File::create(&file_path)
                            .map_err(|e| {
                                eprintln!(
                                    "An error occurred creating file at: {} {}",
                                    file_path.display(),
                                    e
                                )
                            })
                            .ok()
                    })
                    .unwrap_or_else(|| File::open(&file_path).ok());
                Some((file?, file_path))
            }
            None => None,
        }
    }

    // Same as compute_repo_path, but creates the path if the should_create flag is true
    fn compute_or_create_repo_dir(&self, paths: &[&str], should_create: bool) -> Option<PathBuf> {
        let path = self.compute_repo_path(paths);
        if path.exists() {
            assert!(path.is_dir(), "Expected a directory at {}", path.display());
            return Some(path);
        }
        should_create.then(|| {
            fs::create_dir_all(&path)
                .unwrap_or_else(|e| panic!("Failed to create the path {}: {}", path.display(), e));
            path
        })
    }

    // WARN: Truncates before writing!
    fn write_to_repo_file(&self, path: &PathBuf, content: &[u8]) -> Result<()> {
        File::create(path)
            .map_err(|e| anyhow!("Could not create file: {} {}", path.display(), e))?
            .write_all(content)
            .map_err(|e| anyhow!("Could not write data to file: {} {}", path.display(), e))?;
        Ok(())
    }
}

// Returns true if the an empty directory exists at the given path
fn is_empty_dir(path: &Path) -> bool {
    path.is_dir() && fs::read_dir(path).map_or(false, |mut entries| entries.next().is_none())
}
