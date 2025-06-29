use crate::{kvlm::Kvlm, object::Object};

pub struct Commit {
    kvlm: Kvlm,
}

impl Object for Commit {
    fn serialize(&mut self) -> Vec<u8> {
        self.kvlm.serialize()
    }

    fn deserialize(data: &[u8]) -> Self {
        Self {
            kvlm: Kvlm::new(data),
        }
    }
}

impl Commit {
    pub fn short(sha: &str) -> &str {
        &sha[0..7]
    }

    pub fn message(&self) -> &str {
        self.kvlm.get_message()
    }
}
