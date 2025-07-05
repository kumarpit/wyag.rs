use crate::{kvlm::Kvlm, object::Object, refs::Ref, repository::Repository};

pub struct Tag {
    kvlm: Kvlm,
}

pub enum TagType {
    Lightweight,
    Object,
}

// Tag objects are essentially identical to commit objects
impl Object for Tag {
    fn serialize(&mut self) -> Vec<u8> {
        self.kvlm.serialize()
    }

    fn deserialize(data: &[u8]) -> Self {
        Self {
            kvlm: Kvlm::new(data),
        }
    }
}

impl Tag {
    pub fn new(kvlm: Kvlm) -> Self {
        Self { kvlm }
    }

    // TODO: again, replace the hash here with the object_find method
    pub fn create(
        repository: &Repository,
        name: &str,
        hash: &str,
        tag_type: TagType,
    ) -> anyhow::Result<()> {
        match tag_type {
            TagType::Lightweight => Ref::create_at(repository, hash, &["refs", "tags", name]),
            TagType::Object => {
                let mut kvlm = Kvlm::init();

                kvlm.insert("object", hash);
                kvlm.insert("tag", name);

                // TODO: complete
                Ok(())
            }
        }
    }
}
