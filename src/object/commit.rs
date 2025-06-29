use crate::{kvlm::Kvlm, object::Object};

pub struct Commit {
    kvlm: Kvlm,
}

impl Object for Commit {
    fn serialize(&self) -> Vec<u8> {
        self.kvlm.deserialize()
    }

    fn deserialize(data: &[u8]) -> Self {
        Self {
            kvlm: Kvlm::new(data),
        }
    }
}
