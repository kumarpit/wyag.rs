use crate::object::Object;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
    str::FromStr,
};

pub struct Tree {
    records: Vec<Leaf>,
}

pub struct Leaf {
    file_mode: String,
    path: PathBuf, // relative to worktree
    hash: String,
}

impl Object for Tree {
    fn serialize(&mut self) -> Vec<u8> {
        // Sort leaf nodes
        self.records.sort_by_key(|leaf| {
            let is_dir = !leaf.file_mode.starts_with("10");
            let mut file_path_str = leaf.path.to_string_lossy().to_string();
            if is_dir {
                file_path_str.push('/');
            }

            file_path_str
        });

        let mut output = Vec::new();
        self.records.iter().for_each(|leaf| {
            output.extend_from_slice(
                format!(
                    "{}\x20{}\x00{}",
                    leaf.file_mode,
                    leaf.path.to_string_lossy().to_string(),
                    leaf.hash
                )
                .as_bytes(),
            );
        });

        output
    }

    fn deserialize(data: &[u8]) -> Self {
        let mut cursor = Cursor::new(data);
        let pos = cursor.position() as usize;
        let len = cursor.get_ref().len();

        let mut records = Vec::new();
        while pos < len {
            let leaf = Leaf::parse(&mut cursor, data);
            records.push(leaf);
        }

        Self { records }
    }
}

impl Leaf {
    fn parse(cursor: &mut Cursor<&[u8]>, data: &[u8]) -> Self {
        let curr_pos = cursor.position() as usize;

        // Extract the file mode
        let space_idx = data[curr_pos..]
            .iter()
            .position(|&b| b == b' ')
            .ok_or("Malformed leaf record: Missing space")
            .unwrap()
            + curr_pos;
        let mut mode = String::from_utf8_lossy(&data[curr_pos..space_idx]).into_owned();

        println!("size for mode: {}", space_idx - curr_pos);
        // Normalize to 6 bytes
        if space_idx - curr_pos == 5 {
            mode.insert(0, '0');
        }

        // Extract the file path
        let null_idx = data[space_idx..]
            .iter()
            .position(|&b| b == 0)
            .ok_or("Mallformed leaf record: Expected null byte")
            .unwrap()
            + space_idx;
        let path = String::from_utf8_lossy(&data[space_idx + 1..null_idx]).into_owned();

        // Finally, extract the SHA-1 hash
        cursor.set_position((null_idx + 1) as u64);
        let mut hash_buf = vec![0; 20];
        cursor
            .read_exact(&mut hash_buf)
            .expect("Couldn't read SHA-1 hash from leaf record");

        let hash = String::from_utf8_lossy(&hash_buf).into_owned();

        Self {
            file_mode: mode.to_string().to_owned(),
            path: PathBuf::from_str(&path).expect("Couldn't create PathBuf"),
            hash,
        }
    }
}
