use std::borrow::{Borrow, BorrowMut};
use std::collections::BTreeMap;
use std::sync;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Interval, timeout};

use crate::{atomic, message, type_def};
use crate::message::{Message, MessageType};
use crate::network;
use crate::rafterror;
use crate::rafterror::RaftError;
use crate::storage;
use crate::type_def::{LogIndex, TermId};

pub struct Raft {
    storage: sync::Arc<dyn storage::Storage>,
    _msg_recv: mpsc::UnboundedReceiver<Message>,
    _state: std::sync::atomic::AtomicPtr<StateType>,
    _peers: Vec<network::Peer>,
    _id: u32,
    _raft_port: usize,
    _current_term: atomic::Atomic<type_def::TermId>,
    _election_timeout: u64,
    _election_elapse: atomic::AtomicU64,
    _heartbeat_interval: u64,
    _log_entries: Vec<storage::Entry>,
    _inner_sender: mpsc::Sender<Message>,
    _inner_recv: mpsc::Receiver<Message>,
    _leader: atomic::AtomicU64
}

#[derive(Clone, Eq, PartialEq)]
pub enum StateType {
    Follower,
    Leader,
    Candidate,
}

#[derive(Clone, Deserialize)]
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
                recv: mpsc::UnboundedReceiver<Message>,
                peers: Vec<network::Peer>) -> Raft {
    let (tx, rx) = mpsc::channel::<Message>(100);
    let ref mut peer_vec = peers.to_vec();
    peer_vec.sort_by(|p1, p2| p1.id().cmp(&p2.id()));

    Raft {
        storage,
        _msg_recv: recv,
        _state: sync::atomic::AtomicPtr::new(StateType::Follower.borrow_mut()),
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
        _leader: atomic::AtomicU64::new(0)
    }
}

impl Raft {
    async fn tick(interv: &mut Interval, _election_elapse: &AtomicU64) {
        interv.tick().await;
        _election_elapse.fetch_add(1, Ordering::Relaxed);
    }

    fn become_candidate(&mut self) {
        self._state.store(&mut StateType::Candidate, Ordering::SeqCst);
        self.refresh_election_elapse();
        self._inner_sender.send(self.vote_for_self());
    }

    fn vote_for_self(&self) -> Message {
        Message::new(
            MessageType::Champion {
                last_log_index: self.last_log_index(),
                last_log_term: self.last_log_term(),
            },
            self._id.clone(),
            self._peers.iter()
                .map(|peer| peer.id())
                .collect(),
            self._current_term.clone(),
        )
    }

    fn step(&self, _inner: Message) -> Result<(), rafterror::RaftError> {
        match _inner.message_type() {
            MessageType::Champion {
                last_log_index, last_log_term
            } => {
                match self.get_peer(&self._id) {
                    Some(peer) => peer.broadcast(_inner),
                    None => Err(rafterror::RaftError::MissPeer(self._id.clone()))
                }
            }
            _ => Ok(())
        }
    }

    fn post_election_timeout(&self) -> bool {
        self._election_elapse.get() >= self._election_timeout.clone()
    }

    fn is_follower(&self) -> bool {
        self._state.load(Ordering::Relaxed) == &mut StateType::Follower
    }

    pub async fn start(&mut self) {
        let ref mut interv = tokio::time::interval(
            Duration::from_millis(
                self._election_timeout.clone() / self._heartbeat_interval.clone()
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
                _ = Self::tick(interv, &self._election_elapse) => {}

                Some(inner_msg) = self._inner_recv.recv() => {
                    self.step(inner_msg).unwrap_or(())
                }
            }
        }
    }

    pub(crate) fn handle_msg(&mut self, msg: Message) -> Result<(), RaftError> {
        match msg.message_type() {
            message::MessageType::Heartbeat => {
                self.become_follower(msg.term(), msg.from());
                Ok(())
            },
            message::MessageType::Vote {} => Ok(()),
            message::MessageType::AppEntries {
                log_term,
                log_index,
                entries
            } => {
                if !self.is_follower() {
                    self.become_follower(msg.term(), msg.from())
                }

                self.storage.append(entries);

                Ok(())
            }
            message::MessageType::Champion {
                last_log_index,
                last_log_term
            } => Ok(())
        }
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
        self._election_elapse.store(0, Ordering::SeqCst)
    }

    fn become_follower(&self, term: type_def::TermId, from: u32) {
        self.refresh_election_elapse();
        self._state.store(&mut StateType::Follower, Ordering::SeqCst);
        self._current_term.set(term);
        self._leader.set(from)
    }
    fn last_log_index(&self) -> LogIndex {
        todo!()
    }

    fn last_log_term(&self) -> TermId {
        todo!()
    }
}