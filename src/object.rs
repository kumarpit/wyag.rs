// Definitions and methods for the various object types tracked by gitrs

use std::fs::File;
use std::io::{BufReader, Read};
use std::str::from_utf8;

use flate2::bufread::ZlibDecoder;

use crate::repository::Repository;

pub enum Object {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl Object {
    pub fn read(repository: &Repository, sha: String) -> Object {
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

        let object_size: usize =
            from_utf8(&decompressed_data[object_type_index..object_size_index])
                .unwrap()
                .parse()
                .unwrap();

        if object_size != decompressed_data.len() - object_size_index - 1 {
            panic!("Malformed object {}: Bad length", sha);
        }

        let object_type = from_utf8(&decompressed_data[..object_type_index]).unwrap();
        let object_data = &decompressed_data[object_type_index + 1..];

        match object_type {
            "commit" => Object::Commit.init(object_data),
            "tree" => Object::Tree.init(object_data),
            "tag" => Object::Tag.init(object_data),
            "blob" => Object::Blob.init(object_data),
            _ => panic!("Unmatched object type"),
        }
    }

    pub fn init(&self, data: &[u8]) -> Object {
        todo!();
    }

    pub fn serialize(&self) {
        todo!();
    }

    pub fn deserialize(&self) {
        todo!();
    }
}
