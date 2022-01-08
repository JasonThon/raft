use crate::rafterror::RaftError;
use crate::type_def;

pub trait Storage: Send + Sync {
    fn append(&self, entries: Vec<Entry>) -> Result<(), RaftError>;

    fn last_term(&self) -> type_def::TermId;

    fn last_index(&self) -> type_def::LogIndex;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    term: type_def::TermId,
    index: type_def::LogIndex,
    data: Vec<u8>,
}

impl Entry {
    pub fn term(&self) -> type_def::TermId {
        self.term.clone()
    }

    pub fn index(&self) -> type_def::LogIndex {
        self.index.clone()
    }

    pub fn new(term: type_def::TermId, index: type_def::LogIndex, data: Vec<u8>) -> Entry {
        Entry {
            term,
            index,
            data
        }
    }
}