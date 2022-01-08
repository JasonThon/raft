use serde::{Deserialize, Serialize};

use crate::{progress, storage, type_def};
use crate::type_def::{LogIndex, TermId};


#[derive(Clone, Serialize, Deserialize)]
pub enum MessageType {
    VoteRsp {
        granted: bool
    },
    Heartbeat,
    HeartbeatResp {
        status: progress::ProgressStatus,
        reject: bool
    },
    AppEntries {
        entries: Vec<storage::Entry>,
    },
    AppEntriesResp {
        rejected: bool
    },
    Vote {
        last_log_index: LogIndex,
        last_log_term: TermId,
    },
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    msg_type: MessageType,
    from: u64,
    to: Vec<u64>,
    term: type_def::TermId,
}

impl Message {
    pub fn new(_type: MessageType, from: u64, to: Vec<u64>, term: type_def::TermId) -> Message {
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

    pub(crate) fn from(&self) -> u64 {
        self.from.clone()
    }
}
