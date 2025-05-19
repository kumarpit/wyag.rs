use crate::object::Object;

pub struct Tag {}

impl Object for Tag {
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
