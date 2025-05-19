// Definitions and methods for the various object types tracked by gitrs

use core::panic;
use std::fs::File;
use std::io::{BufReader, Read};
use std::str::from_utf8;

use flate2::bufread::ZlibDecoder;
use sha1::Digest;

use crate::blob::Blob;
use crate::commit::Commit;
use crate::repository::Repository;
use crate::tag::Tag;
use crate::tree::Tree;

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

// TODO: improve error handling everywhere
#[derive(Debug)]
pub enum ObjectError {}

impl TryFrom<&str> for ObjectType {
    type Error = ObjectError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "blob" => Ok(ObjectType::Blob),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            "tree" => Ok(ObjectType::Tree),
            _ => panic!("Unrecognized object type"),
        }
    }
}

impl GitrsObject {
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
}

pub fn object_read(repository: &Repository, sha: String) -> GitrsObject {
    let path = repository
        .get_path_to_file(&["objects", &sha[..2], &sha[2..]])
        .expect("Object file does not exist");

    if !path.exists() {
        panic!("Object file does not exist");
    }

    let file = File::open(path).expect("Could not open file");
    let buf_reader = BufReader::new(file);
    let mut decoder = ZlibDecoder::new(buf_reader);
    let mut decompressed_data = Vec::new();
    decoder
        .read_to_end(&mut decompressed_data)
        .expect("Failed to decompress data");

    let object_type_index = decompressed_data
        .iter()
        .position(|&byte| byte == b' ')
        .ok_or("Malformed object: Missing space in header")
        .unwrap();

    let object_size_index = decompressed_data[object_type_index..]
        .iter()
        .position(|&b| b == 0)
        .ok_or("Malformed object: Missing null byte in header")
        .unwrap()
        + object_type_index;

    let object_size: usize = from_utf8(&decompressed_data[object_type_index..object_size_index])
        .unwrap()
        .parse()
        .unwrap();

    if object_size != decompressed_data.len() - object_size_index - 1 {
        panic!("Malformed object {}: Bad length", sha);
    }

    let object_type = from_utf8(&decompressed_data[..object_type_index]).unwrap();
    let object_data = &decompressed_data[object_type_index + 1..];

    GitrsObject::deserialize(object_data, object_type)
}
