use anyhow::anyhow;
use log::error;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use typed_builder::TypedBuilder;

use crate::{
    object::{GitrsObject, ObjectType},
    repository::Repository,
};

/// Four‑byte file signature (“DIRC”) + binary version number.
const INDEX_SIGNATURE: &[u8; 4] = b"DIRC";
const INDEX_VERSION: u32 = 2;

const SHA_BYTES: usize = 20; // raw SHA‑1 (or any 160‑bit hash)

#[derive(Default)]
pub struct Index {
    pub version: u32,
    pub entries: Vec<IndexEntry>,
}

/// **Greatly simplified** index entry
#[derive(TypedBuilder)]
pub struct IndexEntry {
    pub mtime: SystemTime,
    #[builder(setter(into))]
    pub sha: String, // 40‑char hex string on the Rust side
    pub size_in_bytes: u64,
    pub path: PathBuf,
}

/////////////////////////////////////
// IndexEntry ⇄ byte‑vector
/////////////////////////////////////

// TODO: these could be TryFrom trait implementations
impl IndexEntry {
    /// Serialise one entry into raw bytes.
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + SHA_BYTES + 8 + 2 + self.path.as_os_str().len());

        // 1. mtime (u64 big‑endian)
        buf.extend_from_slice(&Self::system_time_to_secs(self.mtime).to_be_bytes());

        // 2. 20‑byte raw hash
        let raw_sha =
            hex::decode(&self.sha).expect("`sha` field must contain a valid 40‑char hex string");
        assert_eq!(raw_sha.len(), SHA_BYTES, "hash must be 20 bytes/160 bits");
        buf.extend_from_slice(&raw_sha);

        // 3. file size (u64 big‑endian)
        buf.extend_from_slice(&self.size_in_bytes.to_be_bytes());

        // 4. path: u16 length + UTF‑8 bytes
        let path_bytes = self.path.to_string_lossy().as_bytes().to_owned();
        let len = u16::try_from(path_bytes.len())
            .expect("Paths longer than 65 535 bytes are unsupported");

        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(&path_bytes);

        buf
    }

    /// Consume a slice, returning one entry and advancing the slice.
    fn take_from(buf: &mut &[u8]) -> Option<Self> {
        // Need at least the fixed fields first
        if buf.len() < 8 + SHA_BYTES + 8 + 2 {
            return None;
        }

        // 1. mtime
        let secs = u64::from_be_bytes(buf[..8].try_into().unwrap());
        *buf = &buf[8..];

        // 2. SHA
        let sha_raw = &buf[..SHA_BYTES];
        *buf = &buf[SHA_BYTES..];
        let sha_hex = hex::encode(sha_raw);

        // 3. size
        let size_in_bytes = u64::from_be_bytes(buf[..8].try_into().unwrap());
        *buf = &buf[8..];

        // 4. path (length‑prefixed)
        let len = u16::from_be_bytes(buf[..2].try_into().unwrap()) as usize;
        *buf = &buf[2..];
        if buf.len() < len {
            return None;
        }
        let path = PathBuf::from(std::str::from_utf8(&buf[..len]).unwrap());
        *buf = &buf[len..];

        Some(Self {
            mtime: Self::secs_to_system_time(secs),
            sha: sha_hex,
            size_in_bytes,
            path,
        })
    }

    /// Convert `SystemTime` to seconds since the Unix epoch (never panics).
    fn system_time_to_secs(t: SystemTime) -> u64 {
        t.duration_since(UNIX_EPOCH)
            .unwrap_or_else(|e| Duration::from_secs(0) + e.duration())
            .as_secs()
    }

    /// Reverse of `system_time_to_secs`.
    fn secs_to_system_time(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }
}

/////////////////////////////////////
// Index I/O
/////////////////////////////////////

impl Index {
    /// Read `repo/.gitrs/index`.  
    /// Returns `None` if the file is missing or corrupt.
    pub fn read(repository: &Repository) -> Option<Self> {
        if let Some(index_file) = repository.get_path_to_file_if_exists(&["index"]) {
            let data = fs::read(index_file).ok()?;
            let mut cursor: &[u8] = &data;

            // ── header ──────────────────────────────────────────────────────────
            if cursor.len() < 12 || &cursor[..4] != INDEX_SIGNATURE {
                error!("Index file length < header length OR mismatched signature");
                return None;
            }
            cursor = &cursor[4..];

            let version = u32::from_be_bytes(cursor[..4].try_into().unwrap());
            cursor = &cursor[4..];

            if version != INDEX_VERSION {
                error!("Only index version 2 is supported");
                return None; // unsupported version
            }

            let count = u32::from_be_bytes(cursor[..4].try_into().unwrap());
            cursor = &cursor[4..];

            // ── entries ─────────────────────────────────────────────────────────
            let mut entries = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let entry = IndexEntry::take_from(&mut cursor)?;
                entries.push(entry);
            }

            Some(Self { version, entries })
        } else {
            Some(Index::default())
        }
    }

    /// Serialise this `Index` back to disk (overwrites if present).
    pub fn write(&self, repository: &Repository) -> anyhow::Result<()> {
        let index_file = repository
            .create_file(&["index"])
            .ok_or_else(|| anyhow!("Could not create index file"))?;

        let mut f = File::create(index_file)?;

        // ── header ──────────────────────────────────────────────────────────
        f.write_all(INDEX_SIGNATURE)?;
        f.write_all(&INDEX_VERSION.to_be_bytes())?;
        f.write_all(&(self.entries.len() as u32).to_be_bytes())?;

        // ── entries ─────────────────────────────────────────────────────────
        for e in &self.entries {
            f.write_all(&e.to_bytes())?;
        }

        f.flush()?;

        Ok(())
    }

    // Given a list of paths, stages them in the repository (i.e adds them to the index file -- or
    // creates an index if there is not exisiting index file)
    pub fn add(&mut self, repository: &Repository, paths: Vec<PathBuf>) -> anyhow::Result<()> {
        // TODO: remove the given paths from the index if they exist

        for path in paths {
            if !repository.contains(&path) {
                return Err(anyhow!("Path {} outside worktree", path.display()));
            }

            let data = fs::read(&path)?;
            let mut blob = GitrsObject::deserialize(&data, ObjectType::Blob);
            let sha = blob.write(&repository);

            let metadata = fs::metadata(&path)?;
            let mtime = metadata.modified()?;
            let size = metadata.len();
            let canonical_path = fs::canonicalize(&path)?;

            self.entries.push(
                IndexEntry::builder()
                    .mtime(mtime)
                    .sha(sha)
                    .size_in_bytes(size)
                    .path(canonical_path)
                    .build(),
            );
        }

        self.write(repository)?;

        Ok(())
    }
}
