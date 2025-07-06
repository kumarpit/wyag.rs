// Definitions and methods for the various object types tracked by gitrs
pub mod blob;
pub mod commit;
pub mod error;
pub mod tag;
pub mod tree;

use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::str::{FromStr, from_utf8};

use anyhow::{Result, anyhow};
use flate2::bufread::ZlibDecoder;
use log::info;
use sha1::{Digest, Sha1};

use crate::refs::Ref;
use crate::repository::Repository;
use blob::Blob;
use commit::Commit;
use error::ObjectError;
use tag::Tag;
use tree::Tree;

/////////////////////////////////////
/// Object Representation
/////////////////////////////////////

/// Trait representing a gitrs object that can be serialized and deserialized.
pub trait Object {
    /// Serialize the object into a vector of bytes.
    fn serialize(&mut self) -> Vec<u8>;

    /// Deserialize the object from a slice of bytes.
    fn deserialize(data: &[u8]) -> Self;
}

/// Enum of all supported gitrs object types.
pub enum GitrsObject {
    BlobObject(Blob),
    CommitObject(Commit),
    TagObject(Tag),
    TreeObject(Tree),
}

/// Enum representing the type of git object.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectType {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl std::fmt::Display for ObjectType {
    /// Formats the object type as a string (e.g. "blob", "commit").
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ObjectType::*;
        let name = match self {
            Blob => "blob",
            Commit => "commit",
            Tag => "tag",
            Tree => "tree",
        };
        write!(f, "{name}")
    }
}

impl TryFrom<&str> for ObjectType {
    type Error = ObjectError;

    /// Tries to convert a string into a valid ObjectType.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "blob" => Ok(ObjectType::Blob),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            "tree" => Ok(ObjectType::Tree),
            other => Err(ObjectError::UnrecognizedObjectType(other.to_string())),
        }
    }
}

impl FromStr for ObjectType {
    type Err = String;

    /// Parses an ObjectType from a string, returning an error string on failure.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ObjectType::try_from(s).map_err(|e| e.to_string())
    }
}

/// Options to control how object resolution behaves.
pub struct ObjectFindOptions {
    /// The expected type of the object.
    pub object_type: ObjectType,
    /// Whether to follow tags/commits to resolve to the final object type.
    pub should_follow: bool,
}

impl GitrsObject {
    /// Returns the `ObjectType` corresponding to this GitrsObject.objec
    pub fn get_type(&self) -> ObjectType {
        match self {
            GitrsObject::BlobObject(_) => ObjectType::Blob,
            GitrsObject::CommitObject(_) => ObjectType::Commit,
            GitrsObject::TagObject(_) => ObjectType::Tag,
            GitrsObject::TreeObject(_) => ObjectType::Tree,
        }
    }

    /// Serializes the git object including its type header and content.
    pub fn serialize(&mut self) -> Vec<u8> {
        match self {
            GitrsObject::BlobObject(blob) => blob.serialize(),
            GitrsObject::CommitObject(commit) => commit.serialize(),
            GitrsObject::TagObject(tag) => tag.serialize(),
            GitrsObject::TreeObject(tree) => tree.serialize(),
        }
    }

    /// Deserializes data into the appropriate GitrsObject variant based on the type string.
    pub fn deserialize(data: &[u8], object_type: ObjectType) -> Self {
        match object_type {
            ObjectType::Blob => Self::BlobObject(Blob::deserialize(data)),
            ObjectType::Commit => Self::CommitObject(Commit::deserialize(data)),
            ObjectType::Tag => Self::TagObject(Tag::deserialize(data)),
            ObjectType::Tree => Self::TreeObject(Tree::deserialize(data)),
        }
    }

    /// Deserialize data and write the object to the repository, returning the SHA-1 hash.
    pub fn deserialize_and_write(
        repository: &Repository,
        data: &[u8],
        object_type: ObjectType,
    ) -> String {
        Self::deserialize(data, object_type).write(repository)
    }

    /// Reads and decompresses an object by its SHA from the repository.
    ///
    /// Validates header and size, then returns the parsed object.
    pub fn read(repository: &Repository, sha: &str) -> Result<Self> {
        let path = repository
            .get_path_to_file_if_exists(&["objects", &sha[..2], &sha[2..]])
            .ok_or_else(|| anyhow!("Object file does not exist"))?;

        let file = File::open(path).expect("Could not open file");
        let buf_reader = BufReader::new(file);
        let mut decoder = ZlibDecoder::new(buf_reader);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        info!("Dumping raw object");
        Self::dump(&decompressed);

        let type_end = decompressed
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| anyhow!("Malformed object: missing space in header"))?;

        let size_end = decompressed[type_end..]
            .iter()
            .position(|&b| b == 0)
            .map(|i| i + type_end)
            .ok_or_else(|| anyhow!("Malformed object: missing null byte in header"))?;

        let object_type_str = from_utf8(&decompressed[..type_end])?;
        let object_type = ObjectType::try_from(object_type_str)?;
        let object_size: usize = from_utf8(&decompressed[type_end + 1..size_end])?.parse()?;

        let content = &decompressed[size_end + 1..];
        if object_size != content.len() {
            return Err(anyhow!(
                "Malformed object {}: size mismatch (expected {}, got {})",
                sha,
                object_size,
                content.len()
            ));
        }

        Ok(Self::deserialize(content, object_type))
    }

    /// Serializes and writes the object into the repository, returning its SHA-1 hash.
    pub fn write(&mut self, repository: &Repository) -> String {
        let data = self.serialize();
        let header = format!("{} {}\x00", self.get_type(), data.len());

        let mut payload = header.into_bytes();
        payload.extend(data);

        let sha = Self::hash(&mut payload);

        repository
            .upsert_file(&["objects", &sha[..2], &sha[2..]], &payload)
            .expect("Could not write object file");

        sha
    }

    // Returns the SHA-1 hash of the given byte vector
    fn hash(data: &mut Vec<u8>) -> String {
        let mut hasher = Sha1::new();
        hasher.update(&data);
        hex::encode(hasher.finalize())
    }

    /// Finds the SHA-1 hash for a given object name with optional resolution options.
    pub fn find(
        repository: &Repository,
        name: &str,
        options_opt: Option<ObjectFindOptions>,
    ) -> Result<String> {
        let shas = Self::resolve(repository, name)?;

        match shas.len() {
            0 => Err(anyhow!("Couldn't find object with name: {}", name)),
            1 => {
                let sha = &shas[0];
                match options_opt {
                    Some(options) => Self::find_with_options(repository, sha, options),
                    None => Ok(sha.clone()),
                }
            }
            _ => Err(anyhow!(
                "Ambiguous reference '{}'. Candidates: {:?}",
                name,
                shas
            )),
        }
    }

    /// Helper to find an object by SHA and check/follow its type if requested.
    fn find_with_options(
        repository: &Repository,
        sha: &str,
        options: ObjectFindOptions,
    ) -> Result<String> {
        let object = Self::read(repository, sha)?;

        if object.get_type() == options.object_type {
            return Ok(sha.to_owned());
        }

        if !options.should_follow {
            return Err(anyhow!("Couldn't resolve object type"));
        }

        match object {
            GitrsObject::CommitObject(commit) if options.object_type == ObjectType::Tree => {
                Self::find_with_options(repository, commit.get_tree_hash(), options)
            }
            GitrsObject::TagObject(tag) => {
                Self::find_with_options(repository, tag.get_object_hash(), options)
            }
            _ => Err(anyhow!("No object matching requested type")),
        }
    }

    /// Resolves a human-readable reference or partial SHA to a list of matching object SHAs.
    fn resolve(repository: &Repository, name: &str) -> Result<Vec<String>> {
        match name {
            _ if name.trim().is_empty() => Err(anyhow!("Cannot resolve empty object name")),

            "HEAD" => Ok(vec![Ref::resolve(repository, &["HEAD"])?]),

            _ if name.chars().all(|c| c.is_ascii_hexdigit()) => {
                let dir = &name[..2].to_lowercase();
                let prefix = &name[2..].to_lowercase();

                let obj_dir = repository
                    .get_path_to_dir_if_exists(&["objects", dir])
                    .ok_or_else(|| anyhow!("Object directory missing: objects/{}", dir))?;

                Ok(fs::read_dir(obj_dir)?
                    .filter_map(Result::ok)
                    .filter_map(|entry| {
                        let file = entry.file_name().to_string_lossy().to_string();
                        file.strip_prefix(prefix)
                            .map(|_| format!("{}{}", dir, file))
                    })
                    .collect())
            }

            _ => {
                // Check tags, local/remote branches for matches
                let mut results = Vec::new();
                for path in [["refs", "tags"], ["refs", "heads"], ["refs", "remotes"]].iter() {
                    if let Ok(resolved) = Ref::resolve(repository, &[path[0], path[1], name]) {
                        results.push(resolved);
                    }
                }
                Ok(results)
            }
        }
    }

    /// Prints a hex dump of the provided buffer to stdout.
    pub fn dump(buf: &Vec<u8>) {
        for (i, byte) in buf.iter().enumerate() {
            if i % 16 == 0 {
                print!("\n{:08x}: ", i);
            }
            print!("{:02x} ", byte);
        }
        println!();
    }
}
