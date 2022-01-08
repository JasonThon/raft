use std::sync::Arc;

use tokio::sync::mpsc;

use crate::raftnode::Config;
use crate::storage::Storage;

pub mod raftnode;
pub mod storage;
mod message;
pub mod network;
mod type_def;
mod rafterror;
mod atomic;
mod progress;
mod utils;

pub async fn main(storage: Arc<dyn Storage>, node_id: &u64) {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(
        network::NetPlan::Listener(8090)
            .listen_and_send(tx)
    );

    raftnode::new_raft(
        node_id.clone(),
        Config::new(8080, 100, 3),
        storage,
        rx,
        Vec::new(),
    ).start().await
}