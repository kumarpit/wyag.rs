// Represents a blob object type. This is used to store user files being tracked by gitrs.

use crate::object::Object;

pub struct Blob {
    data: &'static [u8],
}

impl Object for Blob {
    fn serialize(&self) -> &[u8] {
        todo!()
    }

    fn deserialize(data: &[u8]) -> Self {
        todo!()
    }
}
