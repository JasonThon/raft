use tokio::sync;

use crate::message;
use crate::message::Message;

pub enum RaftError {
    MissPeer(u64),
    NotSupportedMessageType(message::MessageType),
}

impl From<sync::mpsc::error::TrySendError<Message>> for RaftError {
    fn from(err: sync::mpsc::error::TrySendError<Message>) -> Self {
        todo!()
    }
}