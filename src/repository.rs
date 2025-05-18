use core::panic;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Repository {
    pub worktree: PathBuf,
    pub gitdir: PathBuf,
}

impl Repository {
    pub fn new(worktree: &Path) -> Self {
        let gitdir = worktree.join(".gitrs");
        if worktree.exists() {
            if !worktree.is_dir() {
                panic!("Expected a directory at: {}", worktree.display());
            }

            if gitdir.exists() && !is_empty_dir(gitdir.as_path()) {
                panic!("Expected empty directory at: {}", gitdir.display());
            }
        } else {
            fs::create_dir_all(gitdir.as_path()).unwrap_or_else(|e| {
                panic!("Failed to create the path {}: {}", gitdir.display(), e)
            });
        }

        let repository = Self {
            worktree: worktree.to_path_buf(),
            gitdir,
        };

        let did_create_dirs = [
            repository.repo_dir(&["branches"], true),
            repository.repo_dir(&["objects"], true),
            repository.repo_dir(&["refs", "tags"], true),
            repository.repo_dir(&["refs", "heads"], true),
        ]
        .iter()
        .all(|opt| opt.is_some());

        if !did_create_dirs {
            panic!("An error occurred when initializing the gitrs repository");
        }

        repository.write_to_repo_file(
            &repository
                .repo_file(&["description"], false)
                .expect("Could not make descrption file"),
            b"Unamed repository; edit this file 'description' to name the repository.\n",
        );

        repository.write_to_repo_file(
            &repository
                .repo_file(&["HEAD"], false)
                .expect("Could not make HEAD file"),
            b"ref: refs/heads/master\n",
        );

        // TODO: Figure out config file management

        repository
    }

    // Computes the path under a repository's gitrs directory
    fn repo_path(&self, paths: &[&str]) -> PathBuf {
        paths.iter().fold(self.gitdir.clone(), |mut acc, path| {
            acc.push(path);
            acc
        })
    }

    // Same as repo_path, but creates the trailing directories if they don't exist if the
    // should_mkdir flag is set
    fn repo_file(&self, paths: &[&str], should_mkdir: bool) -> Option<PathBuf> {
        match self.repo_dir(&paths[..paths.len() - 1], should_mkdir) {
            Some(_) => Some(self.repo_path(paths)),
            None => None,
        }
    }

    // Same as repo_path, but creates the path if the should_mkdir flag is true
    fn repo_dir(&self, paths: &[&str], should_mkdir: bool) -> Option<PathBuf> {
        let path = self.repo_path(paths);
        if path.exists() {
            if !path.is_dir() {
                panic!("Expected a directory at {}", path.display());
            }
            Some(path)
        } else if should_mkdir {
            fs::create_dir_all(&path)
                .unwrap_or_else(|e| panic!("Failed to create the path {}: {}", path.display(), e));
            Some(path)
        } else {
            None
        }
    }

    fn write_to_repo_file(&self, path: &PathBuf, content: &[u8]) {
        File::create(path)
            .unwrap_or_else(|e| panic!("Could not create file {}: {}", path.display(), e))
            .write_all(content)
            .unwrap_or_else(|e| panic!("Could not write to file {}: {}", path.display(), e));
    }
}

fn is_empty_dir(path: &Path) -> bool {
    path.is_dir() && fs::read_dir(path).map_or(false, |mut entries| entries.next().is_none())
}
