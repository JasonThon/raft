use std::sync::Arc;

use tokio::sync::mpsc;

use crate::raftnode::Config;
use crate::storage::Storage;

pub mod raftnode;
pub mod storage;
mod message;
pub mod network;

pub async fn main(storage: Arc<dyn Storage>) -> Result<(), tokio::task::JoinError> {
    let (tx, rx) = mpsc::unbounded_channel();

    let mut raft = raftnode::new_raft(Config::new(8080, 100), storage, rx, Vec::new());
    let start_listener = tokio::spawn(
        network::NetPlan::Listener(8090, tx)
            .listen()
    );
    raft.start();

    match start_listener.await {
        Ok(result) => Ok(result.unwrap()),
        Err(err) => Err(err)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
