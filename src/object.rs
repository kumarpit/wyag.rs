// Definitions and methods for the various object types tracked by gitrs
pub mod blob;
pub mod commit;
pub mod error;
pub mod tag;
pub mod tree;

use core::panic;
use std::fs::File;
use std::io::{BufReader, Read};
use std::str::from_utf8;

use flate2::bufread::ZlibDecoder;
use sha1::{Digest, Sha1};

use crate::repository::Repository;
use blob::Blob;
use commit::Commit;
use error::ObjectError;
use tag::Tag;
use tree::Tree;

pub trait Object {
    fn serialize(&self) -> &[u8];
    fn deserialize(data: &[u8]) -> Self;
}

pub enum GitrsObject {
    BlobObject(Blob),
    CommitObject(Commit),
    TagObject(Tag),
    TreeObject(Tree),
}

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

impl GitrsObject {
    fn get_type(&self) -> ObjectType {
        match self {
            GitrsObject::BlobObject(_) => ObjectType::Blob,
            GitrsObject::CommitObject(_) => ObjectType::Commit,
            GitrsObject::TagObject(_) => ObjectType::Tag,
            GitrsObject::TreeObject(_) => ObjectType::Tree,
        }
    }

    fn serialize(&self) -> &[u8] {
        match self {
            GitrsObject::BlobObject(blob) => blob.serialize(),
            GitrsObject::CommitObject(commit) => commit.serialize(),
            GitrsObject::TagObject(tag) => tag.serialize(),
            GitrsObject::TreeObject(tree) => tree.serialize(),
        }
    }

    fn deserialize(data: &[u8], object_type: &str) -> Self {
        match ObjectType::try_from(object_type).unwrap() {
            ObjectType::Blob => Self::BlobObject(Blob::deserialize(data)),
            ObjectType::Commit => Self::CommitObject(Commit::deserialize(data)),
            ObjectType::Tag => Self::TagObject(Tag::deserialize(data)),
            ObjectType::Tree => Self::TreeObject(Tree::deserialize(data)),
        }
    }

    pub fn object_read(repository: &Repository, sha: String) -> Self {
        let path = repository
            .get_path_to_file(&["objects", &sha[..2], &sha[2..]])
            .expect("Object file does not exist");

        let file = File::open(path).expect("Could not open file");
        let buf_reader = BufReader::new(file);
        let mut decoder = ZlibDecoder::new(buf_reader);
        let mut decompressed_data = Vec::new();
        decoder
            .read_to_end(&mut decompressed_data)
            .expect("Failed to decompress data");

        let obj_type_end_idx = decompressed_data
            .iter()
            .position(|&byte| byte == b' ')
            .ok_or("Malformed object: Missing space in header")
            .unwrap();

        let obj_size_end_idx = decompressed_data[obj_type_end_idx..]
            .iter()
            .position(|&b| b == 0)
            .ok_or("Malformed object: Missing null byte in header")
            .unwrap()
            + obj_type_end_idx;

        let object_size: usize = from_utf8(&decompressed_data[obj_type_end_idx..obj_size_end_idx])
            .unwrap()
            .parse()
            .unwrap();

        if object_size != decompressed_data.len() - obj_size_end_idx - 1 {
            panic!("Malformed object {}: Bad length", sha);
        }

        let object_type = from_utf8(&decompressed_data[..obj_type_end_idx]).unwrap();
        let object_data = &decompressed_data[obj_type_end_idx + 1..];

        Self::deserialize(object_data, object_type)
    }

    pub fn object_write(&self, repository: &Repository) -> String {
        let data = self.serialize();

        let header = format!("{} {}\x00", self.get_type(), data.len());
        let mut payload = header.into_bytes();
        payload.extend(data);

        // Compute SHA-1 hash
        let sha = {
            let mut hasher = Sha1::new();
            hasher.update(&payload);
            hex::encode(hasher.finalize())
        };

        repository
            .upsert_file(&["objects", &sha[..2], &sha[2..]], &payload)
            .expect("Could not write object file");

        sha
    }
}
