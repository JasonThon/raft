use crate::rafterror::RaftError;
use crate::type_def;

pub trait Storage: Send + Sync {
    fn append(&self, entries: Vec<Entry>) -> Result<(), RaftError>;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    term: type_def::TermId,
    index: type_def::LogIndex,
    data: Vec<u8>,
}