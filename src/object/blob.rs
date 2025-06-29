// Represents a blob object type. This is used to store user files being tracked by gitrs.

use crate::object::Object;

pub struct Blob {
    data: Vec<u8>,
}

impl Object for Blob {
    fn serialize(&mut self) -> Vec<u8> {
        self.data.clone()
    }

    fn deserialize(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }
}
