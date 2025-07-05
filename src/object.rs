// Definitions and methods for the various object types tracked by gitrs
pub mod blob;
pub mod commit;
pub mod error;
pub mod tag;
pub mod tree;

use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::str::{FromStr, from_utf8};

use anyhow::anyhow;
use flate2::bufread::ZlibDecoder;
use sha1::{Digest, Sha1};

use crate::refs::Ref;
use crate::repository::Repository;
use blob::Blob;
use commit::Commit;
use error::ObjectError;
use tag::Tag;
use tree::Tree;

/////////////////////////////////////
///Object Representation
/////////////////////////////////////

pub trait Object {
    fn serialize(&mut self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Self;
}

pub enum GitrsObject {
    BlobObject(Blob),
    CommitObject(Commit),
    TagObject(Tag),
    TreeObject(Tree),
}

#[derive(Clone, Debug)]
pub enum ObjectType {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ObjectType::*;

        let format_name = match self {
            Blob => "blob",
            Commit => "commit",
            Tag => "tag",
            Tree => "tree",
        };

        write!(f, "{format_name}")
    }
}

impl TryFrom<&str> for ObjectType {
    type Error = ObjectError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "blob" => Ok(ObjectType::Blob),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            "tree" => Ok(ObjectType::Tree),
            rest => Err(ObjectError::UnrecognizedObjectType(rest.to_string())),
        }
    }
}

impl FromStr for ObjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ObjectType::try_from(s).map_err(|e| e.to_string())
    }
}

impl GitrsObject {
    pub fn get_type(&self) -> ObjectType {
        match self {
            GitrsObject::BlobObject(_) => ObjectType::Blob,
            GitrsObject::CommitObject(_) => ObjectType::Commit,
            GitrsObject::TagObject(_) => ObjectType::Tag,
            GitrsObject::TreeObject(_) => ObjectType::Tree,
        }
    }

    // Objects are stored in the following format:
    // <TYPE>0x20<SIZE>0x00<CONTENTS>
    // The header part, and the contents, are then compressed using Zlib
    pub fn serialize(&mut self) -> Vec<u8> {
        match self {
            GitrsObject::BlobObject(blob) => blob.serialize(),
            GitrsObject::CommitObject(commit) => commit.serialize(),
            GitrsObject::TagObject(tag) => tag.serialize(),
            GitrsObject::TreeObject(tree) => tree.serialize(),
        }
    }

    pub fn deserialize(data: &[u8], object_type: &str) -> Self {
        match ObjectType::try_from(object_type).unwrap() {
            ObjectType::Blob => Self::BlobObject(Blob::deserialize(data)),
            ObjectType::Commit => Self::CommitObject(Commit::deserialize(data)),
            ObjectType::Tag => Self::TagObject(Tag::deserialize(data)),
            ObjectType::Tree => Self::TreeObject(Tree::deserialize(data)),
        }
    }

    pub fn deserialize_and_write(
        repository: &Repository,
        data: &[u8],
        object_type: ObjectType,
    ) -> String {
        Self::deserialize(data, object_type.to_string().as_str()).write(repository)
    }

    /// Read and parse the object specified by `sha` in the given repository
    pub fn read(repository: &Repository, sha: &str) -> anyhow::Result<Self> {
        let path = repository
            .get_path_to_file(&["objects", &sha[..2], &sha[2..]])
            .ok_or_else(|| anyhow!("Object file does not exist"))?;

        // Decompressing object (header + contents)
        let file = File::open(path).expect("Could not open file");
        let buf_reader = BufReader::new(file);
        let mut decoder = ZlibDecoder::new(buf_reader);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;

        // hex dump
        print!("Raw object");
        Self::dump(&decompressed_data);

        // Extract the object type
        let obj_type_end_idx = decompressed_data
            .iter()
            .position(|&byte| byte == b' ')
            .ok_or_else(|| anyhow!("Malformed object: Missing space in header"))?;

        let object_type_str = from_utf8(&decompressed_data[..obj_type_end_idx])?;

        // Extract the object size
        let obj_size_end_idx = decompressed_data[obj_type_end_idx..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow!("Malformed object: Missing null byte in header"))?
            + obj_type_end_idx;

        let object_size: usize =
            from_utf8(&decompressed_data[obj_type_end_idx + 1..obj_size_end_idx])?.parse()?;

        let expected_length = decompressed_data.len() - (obj_size_end_idx + 1);

        if object_size == expected_length {
            let object_data = &decompressed_data[obj_size_end_idx + 1..];
            Ok(Self::deserialize(object_data, object_type_str))
        } else {
            Err(anyhow!(
                "Malformed object {}: Bad length - actual {} expected {}",
                sha,
                object_size,
                expected_length
            ))
        }
    }

    /// Write the current object to the repository
    pub fn write(&mut self, repository: &Repository) -> String {
        let data = self.serialize();

        let header = format!("{}\x20{}\x00", self.get_type(), data.len());
        let mut payload = header.into_bytes();
        payload.extend(data);

        // Compute SHA-1 hash
        let sha = {
            let mut hasher = Sha1::new();
            hasher.update(&payload);
            hex::encode(hasher.finalize()) // SHA-1 produces a 160-bit hash
        };

        repository
            .upsert_file(&["objects", &sha[..2], &sha[2..]], &payload)
            .expect("Could not write object file");

        sha
    }

    pub fn find(repository: &Repository, name: &str) -> anyhow::Result<String> {
        let shas = Self::resolve(repository, name)?;
        match shas.len() {
            0 => Err(anyhow!("Couldn't find object with name: {}", name)),
            1 => Ok(shas.get(0).unwrap().clone()),
            _ => Err(anyhow!(
                "Ambigious reference ({}). Candidates are: {:?}",
                name,
                shas
            )),
        }
    }

    // Hex dump
    pub fn dump(buf: &Vec<u8>) {
        for (i, byte) in buf.iter().enumerate() {
            if i % 16 == 0 {
                print!("\n{:08x}: ", i);
            }
            print!("{:02x} ", byte);
        }
        println!();
    }

    /// Resolves a human-readable name to an object hash
    fn resolve(repository: &Repository, name: &str) -> anyhow::Result<Vec<String>> {
        match name {
            _ if name.trim().is_empty() => {
                Err(anyhow!("Cannot resolve empty string as object name"))
            }
            "HEAD" => Ok(vec![Ref::resolve(repository, &["HEAD"])?]),
            hash if hash.chars().all(|c| c.is_digit(16)) => {
                let dir = &hash.to_lowercase()[..2];

                // Read objects
                let obj_path = repository
                    .get_path_to_dir(&["objects", &dir])
                    .ok_or_else(|| anyhow!("Object dir doesn't exist: objects/{}", dir))?;
                let obj_name_prefix = &hash.to_lowercase()[2..];
                let objs = fs::read_dir(obj_path)?;

                let mut candidates = objs.fold(Vec::new(), |mut acc, entry_res| {
                    if let Ok(entry) = entry_res {
                        let file_name = entry.file_name().to_string_lossy().into_owned();
                        if file_name.starts_with(obj_name_prefix) {
                            acc.push(format!("{}{}", dir, file_name.to_string()));
                        }
                    }
                    acc
                });

                // Read references (tags)
                if let Some(tag) = Ref::resolve(repository, &["refs", "tags", name]).ok() {
                    candidates.push(tag);
                }

                // Find local branches
                if let Some(branch) = Ref::resolve(repository, &["refs", "heads", name]).ok() {
                    candidates.push(branch);
                }

                // Find remote branches
                if let Some(remote) = Ref::resolve(repository, &["refs", "remotes", name]).ok() {
                    candidates.push(remote);
                }

                Ok(candidates)
            }
            _ => Err(anyhow!("Invalid object name: {}", name)),
        }
    }
}
