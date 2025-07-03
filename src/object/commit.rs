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

    pub fn get_tree_hash(&self) -> &String {
        self.kvlm
            .get_key("tree")
            .expect("Each commit must provide a tree")
            .first()
            .expect("Tree cannot be assigned to empty value")
    }
}
