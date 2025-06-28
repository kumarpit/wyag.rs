use crate::object::Object;

pub struct Commit {}

impl Object for Commit {
    fn serialize(&self) -> &[u8] {
        todo!()
    }

    fn deserialize(data: &[u8]) -> Self {
        todo!()
    }

    fn init(_data: &[u8]) -> Self {
        Self {}
    }
}
