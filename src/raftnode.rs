use std::borrow::{Borrow, BorrowMut};
use std::collections::BTreeMap;
use std::sync;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Interval, timeout};

use crate::{atomic, message, type_def};
use crate::message::Progress;
use crate::network;
use crate::rafterror;
use crate::rafterror::RaftError;
use crate::storage;
use crate::type_def::{LogIndex, TermId};

pub struct Raft {
    storage: sync::Arc<dyn storage::Storage>,
    _msg_recv: mpsc::UnboundedReceiver<message::Message>,
    _state: atomic::Atomic<StateType>,
    _peers: Vec<network::Peer>,
    _id: u32,
    _raft_port: usize,
    _current_term: atomic::Atomic<type_def::TermId>,
    _election_timeout: u64,
    _election_elapse: atomic::AtomicU64,
    _heartbeat_interval: u64,
    _log_entries: Vec<storage::Entry>,
    _inner_sender: mpsc::Sender<message::Message>,
    _inner_recv: mpsc::Receiver<message::Message>,
    _leader: sync::atomic::AtomicU32,
    _progress: BTreeMap<u32, message::Progress>,
}

#[derive(Clone, Eq, PartialEq)]
pub enum StateType {
    Follower,
    Leader,
    Candidate,
}

#[derive(Clone, serde::Deserialize)]
pub struct Config {
    port: usize,
    channel_buf_size: usize,
    election_timeout: u64,
    heartbeat_interval: u64,
}

impl Config {
    pub(crate) fn new(port: usize, channel_buf_size: usize, election_timeout: u64) -> Config {
        Config { port, channel_buf_size, election_timeout, heartbeat_interval: 3 }
    }

    pub(crate) fn port(&self) -> usize {
        self.port.clone()
    }

    pub(crate) fn channel_buf_size(&self) -> usize {
        self.channel_buf_size.clone()
    }

    pub(crate) fn election_timeout(&self) -> u64 {
        self.election_timeout.clone()
    }

    pub(crate) fn heartbeat_interval(&self) -> u64 {
        self.heartbeat_interval.clone()
    }
}

pub fn new_raft(_id: u32,
                conifg: Config,
                storage: sync::Arc<dyn storage::Storage>,
                recv: mpsc::UnboundedReceiver<message::Message>,
                peers: Vec<network::Peer>) -> Raft {
    let (tx, rx) = mpsc::channel::<message::Message>(100);
    let ref mut peer_vec = peers.to_vec();
    peer_vec.sort_by(|p1, p2| p1.id().cmp(&p2.id()));

    Raft {
        storage,
        _msg_recv: recv,
        _state: atomic::Atomic::new(StateType::Follower),
        _peers: peer_vec.clone(),
        _id,
        _raft_port: conifg.port(),
        _current_term: atomic::Atomic::<type_def::TermId>::new(0),
        _election_timeout: conifg.election_timeout(),
        _election_elapse: atomic::AtomicU64::new(0),
        _heartbeat_interval: conifg.heartbeat_interval(),
        _log_entries: vec![],
        _inner_sender: mpsc::Sender::from(tx),
        _inner_recv: mpsc::Receiver::from(rx),
        _leader: sync::atomic::AtomicU32::new(0),
        _progress: Default::default(),
    }
}

impl Raft {
    fn is_leader(&self) -> bool {
        self._state.get().unwrap() == StateType::Leader
    }

    async fn tick(interv: &mut Interval,
                  _election_elapse: &atomic::AtomicU64) -> u64 {
        interv.tick().await;

        _election_elapse.get_and_increment()
    }

    fn become_candidate(&mut self) {
        self._state.set(StateType::Candidate);
        self.refresh_election_elapse();
        self._inner_sender.send(self.vote_for_self());
    }

    fn vote_for_self(&self) -> message::Message {
        message::Message::new(
            message::MessageType::Champion {
                last_log_index: self.last_log_index(),
                last_log_term: self.last_log_term(),
            },
            self._id.clone(),
            self._peers.iter()
                .map(|peer| peer.id())
                .collect(),
            self._current_term.get().unwrap(),
        )
    }

    fn send_internal_to_peers(&self, _inner: message::Message) -> Result<(), rafterror::RaftError> {
        match self.get_peer(&self.node_id()) {
            Some(peer) =>
                match _inner.message_type() {
                    message::MessageType::Champion {
                        last_log_index, last_log_term
                    } => peer.broadcast(_inner),
                    message::MessageType::Heartbeat => peer.broadcast(_inner),
                    _ => Ok(())
                },
            None => Err(rafterror::RaftError::MissPeer(self.node_id()))
        }
    }

    fn post_election_timeout(&self) -> bool {
        self._election_elapse.get() >= self._election_timeout.clone()
    }

    fn is_follower(&self) -> bool {
        self._state.get().unwrap() == StateType::Follower
    }


    fn get_peer(&self, id: &u32) -> Option<&network::Peer> {
        for peer in &self._peers {
            if peer.id().eq(id) {
                return Option::Some(peer);
            }
        }

        Option::None
    }
    fn refresh_election_elapse(&self) {
        self._election_elapse.set(0)
    }

    fn become_follower(&self, term: type_def::TermId, from: u32) {
        if !self.is_follower() {
            self.refresh_election_elapse();
            self._state.set(StateType::Follower);
            self._current_term.set(term);
            self._leader.store(from, sync::atomic::Ordering::SeqCst)
        }
    }
    fn last_log_index(&self) -> LogIndex {
        todo!()
    }

    fn last_log_term(&self) -> TermId {
        todo!()
    }
    fn node_id(&self) -> u32 {
        self._id.clone()
    }
}

impl Raft {
    pub async fn start(&mut self) {
        let ref mut interv = tokio::time::interval(
            Duration::from_millis(
                self._heartbeat_interval.clone()
            )
        );

        loop {
            tokio::select! {
                option = self._msg_recv.recv() => {
                    match option {
                        Some(msg) => self.handle_msg(msg).unwrap_or(()),
                        None => {
                           if self.post_election_timeout() &&  self.is_follower() {
                                self.become_candidate()
                            }
                        }
                    }
                }

                _ = Self::tick(interv, &self._election_elapse), if self.is_leader() => {
                   self._inner_sender.send(
                        message::Message::new(
                            message::MessageType::Heartbeat,
                            self.node_id(),
                            self._peers
                                .iter()
                                .map(|peer| peer.id())
                                .collect(),
                            self._current_term.get().unwrap(),
                        )).await.unwrap_or(())
                }

                Some(inner_msg) = self._inner_recv.recv() => {
                    self.send_internal_to_peers(inner_msg).unwrap_or(())
                }
            }
        }
    }

    pub(crate) fn handle_msg(&mut self, msg: message::Message) -> Result<(), RaftError> {
        match msg.message_type() {
            message::MessageType::Heartbeat => {
                self.become_follower(msg.term(), msg.from());

                let arr: [u32; 1] = [msg.from(); 1];
                let mut to = Vec::from(arr);

                self._inner_sender.try_send(
                    message::Message::new(
                        message::MessageType::HeartbeatResp(
                            self._progress.get(&self.node_id())
                                .unwrap()
                                .status()
                                .clone()
                        ),
                        self.node_id(),
                        to.clone(),
                        msg.term(),
                    ))
                    .map_err(|err| RaftError::from(err))
            }
            message::MessageType::Vote {} => Ok(()),
            message::MessageType::AppEntries {
                log_term,
                log_index,
                ref mut entries
            } => {
                self.become_follower(msg.term(), msg.from());
                self._log_entries.append(entries);

                Ok(())
            }
            message::MessageType::Champion {
                last_log_index,
                last_log_term
            } => Ok(()),
            message::MessageType::HeartbeatResp(progress_status) => {
                self.refresh_election_elapse();
                match self._progress.get(&msg.from()) {
                    Some(mut progress) => {
                        let ref mut copied = progress.clone();

                        copied.update_status(progress_status);

                        self._progress.insert(
                            msg.from(),
                            copied.clone(),
                        );
                    }
                    None => {}
                }
                Ok(())
            }
        }
    }
}