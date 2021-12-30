use serde::{Deserialize, Serialize};

use crate::{storage, type_def};
use crate::type_def::TermId;

#[derive(Clone, Serialize, Deserialize)]
pub enum MessageType {
    Vote {},
    Heartbeat,
    AppEntries {
        log_term: type_def::TermId,
        log_index: type_def::LogIndex,
        entries: Vec<storage::Entry>,
    },
    Champion {
        last_log_index: type_def::LogIndex,
        last_log_term: type_def::TermId,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    msg_type: MessageType,
    from: u32,
    to: Vec<u32>,
    term: type_def::TermId,
}

impl Message {
    pub fn new(_type: MessageType, from: u32, to: Vec<u32>, term: type_def::TermId) -> Message {
        Message {
            msg_type: _type.clone(),
            from,
            to,
            term,
        }
    }


    pub fn message_type(&self) -> MessageType {
        self.msg_type.clone()
    }

    pub fn term(&self) -> TermId {
        self.term.clone()
    }

    pub(crate) fn from(&self) -> u32 {
        self.from.clone()
    }
}
