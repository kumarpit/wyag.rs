use crate::object::Object;

pub struct Tree {}

impl Object for Tree {
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
