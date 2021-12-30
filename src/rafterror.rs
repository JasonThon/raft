use tokio::sync;

use crate::message::Message;

pub enum RaftError {
    MissPeer(u32)
}

impl From<sync::mpsc::error::TrySendError<Message>> for RaftError {
    fn from(err: sync::mpsc::error::TrySendError<Message>) -> Self {
        todo!()
    }
}