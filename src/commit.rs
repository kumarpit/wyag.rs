use crate::object::Object;

pub struct Commit {}

impl Object for Commit {
    fn init() -> Self {
        todo!()
    }

    fn serialize(&self) -> &[u8] {
        todo!()
    }

    fn deserialize(data: &[u8]) -> Self {
        todo!()
    }
}
