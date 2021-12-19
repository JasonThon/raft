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
    _msg_recv: mpsc::Receiver<Message>,
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
    pub(crate) fn port(&self) -> usize {
        self.port.clone()
    }

    pub(crate) fn channel_buf_size(&self) -> usize {
        self.channel_buf_size.clone()
    }
}

pub fn new_raft(conifg: Config,
                storage: Arc<dyn Storage>,
                recv: mpsc::Receiver<Message>,
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
    pub fn start(&mut self, msg_sender_chan: mpsc::Sender<Message>) {
        let mut all_handlers = Vec::new();
        let ref mut start_peers = self.start_peers();
        all_handlers.append(start_peers);

        let start_listener = tokio::spawn(
            network::NetPlan::Listener(self._raft_port.clone(), msg_sender_chan)
                .listen()
        );

        all_handlers.push(start_listener);

        let start_handler = tokio::spawn(self.handle_msg());
        all_handlers.push(start_handler);

        for handler in all_handlers {}
    }

    pub(crate) async fn handle_msg(&mut self) -> tokio::io::Result<()> {
        loop {
            while let Some(msg) = self._msg_recv.recv().await {
                match msg.message_type() {
                    message::MessageType::Heartbeat { commit: c, context: ctx } => {}
                    message::MessageType::Vote {} => {}
                }
            }
        }
    }

    fn start_peers(&self) -> Vec<JoinHandle<tokio::io::Result<()>>> {
        self._msg_recv_peers
            .iter()
            .map(|(id, mut recv)| {
                tokio::spawn(async {
                    loop {
                        while let Some(msg) = recv.recv().await {
                            match self.get_peer(id) {
                                Some(peer) => peer.send_to(msg),
                                None => Ok(())
                            }
                        }
                    }
                })
            })
            .collect()
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