use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::message;
use crate::message::Message;
use crate::network;
use crate::storage::Storage;

pub struct Raft {
    storage: Arc<dyn Storage>,
    _msg_recv: mpsc::UnboundedReceiver<Message>,
    _msg_sender_to_peers: BTreeMap<u32, mpsc::Sender<Message>>,
    _msg_recv_peers: BTreeMap<u32, mpsc::Receiver<Message>>,
    _state: StateType,
    _peers: Vec<network::Peer>,
    _raft_port: usize,
}

pub enum StateType {
    Follower,
    Leader,
    Candidate,
}

pub struct Config {
    port: usize,
    channel_buf_size: usize,
}

impl Config {
    pub(crate) fn new(port: usize, channel_buf_size: usize) -> Config {
        Config { port, channel_buf_size }
    }

    pub(crate) fn port(&self) -> usize {
        self.port.clone()
    }

    pub(crate) fn channel_buf_size(&self) -> usize {
        self.channel_buf_size.clone()
    }
}

pub fn new_raft(conifg: Config,
                storage: Arc<dyn Storage>,
                recv: mpsc::UnboundedReceiver<Message>,
                peers: Vec<network::Peer>) -> Raft {
    let mut send_to_peer = BTreeMap::<u32, mpsc::Sender<Message>>::new();
    let mut peer_recv = BTreeMap::<u32, mpsc::Receiver<Message>>::new();


    for mut peer in peers {
        let (tx, rx) = mpsc::channel::<Message>(conifg.channel_buf_size());

        send_to_peer.insert(peer.id(), tx);
        peer_recv.insert(peer.id(), rx);
    }

    Raft {
        storage,
        _msg_recv: recv,
        _msg_sender_to_peers: send_to_peer,
        _msg_recv_peers: peer_recv,
        _state: StateType::Candidate,
        _peers: peers,
        _raft_port: conifg.port(),
    }
}

impl Raft {
    pub fn start(mut self) {
        tokio::spawn(async {
            loop {
                while let Some(msg) = self._msg_recv.recv().await {
                    self.handle_msg(msg)
                        .unwrap_or(())
                }
            }
        });
    }

    pub(crate) fn handle_msg(&mut self, msg: Message) -> tokio::io::Result<()> {
        match msg.message_type() {
            message::MessageType::Heartbeat { commit: c, context: ctx } => Ok(()),
            message::MessageType::Vote {} => Ok(())
        }
    }

    fn get_peer(&self, id: &u32) -> Option<network::Peer> {
        for peer in self._peers {
            if peer.id().eq(id) {
                return Option::Some(peer);
            }
        }

        Option::None
    }
}