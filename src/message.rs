use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum MessageType {
    Vote {},
    Heartbeat {
        commit: u64,
        context: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Message {
    msg_type: MessageType,
    from: u64,
    to: u64,
}

impl Message {
    pub(crate) fn message_type(&self) -> MessageType {
        self.msg_type.clone()
    }
}
