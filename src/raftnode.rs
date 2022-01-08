use std::sync;
use std::time::Duration;

use rand::Rng;
use tokio::sync::mpsc;
use tokio::time::Interval;

use crate::{atomic, message, progress, type_def};
use crate::message::Message;
use crate::network;
use crate::progress::{Progress, VoteQuorum};
use crate::rafterror;
use crate::rafterror::RaftError;
use crate::raftnode::log::RaftLog;
use crate::storage;
use crate::storage::Entry;
use crate::type_def::{LogIndex, TermId};
use crate::utils;

const NONE: u64 = 0;
const NONE_USIZE: usize = 0;

mod log {
    use std::sync;

    use crate::storage;
    use crate::storage::Entry;
    use crate::type_def::{LogIndex, TermId};

    pub struct RaftLog {
        _entries: Vec<storage::Entry>,
        _storage: sync::Arc<dyn storage::Storage>,
    }

    impl RaftLog {
        pub fn append(&mut self, entries: &mut Vec<Entry>) {
            self._entries.append(entries)
        }

        pub fn is_up_to_date(&self, last_log_index: LogIndex, last_log_term: TermId) -> bool {
            last_log_term > self.last_term() ||
                (last_log_term == self.last_term() && last_log_index >= self.last_index())
        }

        pub fn last_term(&self) -> TermId {
            self._entries.last()
                .map(|entry| entry.term())
                .unwrap_or(self._storage.last_term())
        }

        pub fn last_index(&self) -> LogIndex {
            self._entries.last()
                .map(|entry| entry.index())
                .unwrap_or(self._storage.last_index())
        }
    }

    pub fn new(storage: sync::Arc<dyn storage::Storage>) -> RaftLog {
        RaftLog {
            _entries: vec![],
            _storage: storage,
        }
    }
}

pub struct Raft {
    _msg_recv: mpsc::UnboundedReceiver<message::Message>,
    _state: StateType,
    _peers: Vec<network::Peer>,
    _id: u64,
    _raft_port: usize,
    _current_term: TermId,
    _election_timeout: u64,
    _election_elapse: atomic::AtomicU64,
    _heartbeat_interval: u64,
    _log_entries: Vec<storage::Entry>,
    _inner_sender: mpsc::Sender<message::Message>,
    _inner_recv: mpsc::Receiver<message::Message>,
    _leader: u64,
    _progress: progress::ProgressTracker,
    _raftlog: RaftLog,
    _vote_for: u64,
    _random_election_timeout: u64,
}

#[derive(Clone, Eq, PartialEq, Debug)]
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

pub fn new_raft(_id: u64,
                conifg: Config,
                storage: sync::Arc<dyn storage::Storage>,
                recv: mpsc::UnboundedReceiver<message::Message>,
                peers: Vec<network::Peer>) -> Raft {
    let (tx, rx) = mpsc::channel::<message::Message>(100);
    let ref mut peer_vec = peers.to_vec();
    peer_vec.sort_by(|p1, p2| p1.id().cmp(&p2.id()));

    Raft {
        _msg_recv: recv,
        _state: StateType::Follower,
        _peers: peer_vec.clone(),
        _id,
        _raft_port: conifg.port(),
        _current_term: type_def::ZERO_TERM_ID,
        _election_timeout: conifg.election_timeout(),
        _election_elapse: atomic::AtomicU64::new(0),
        _heartbeat_interval: conifg.heartbeat_interval(),
        _log_entries: vec![],
        _inner_sender: mpsc::Sender::from(tx),
        _inner_recv: mpsc::Receiver::from(rx),
        _leader: NONE,
        _progress: progress::ProgressTracker::new(_id),
        _raftlog: log::new(storage),
        _vote_for: NONE,
        _random_election_timeout: 0,
    }
}

impl Raft {
    fn is_leader(&self) -> bool {
        self._state == StateType::Leader
    }

    async fn tick(interv: &mut Interval,
                  _election_elapse: &atomic::AtomicU64) -> u64 {
        interv.tick().await;

        _election_elapse.get_and_increment()
    }

    fn become_candidate(&mut self) {
        self._state = StateType::Candidate;
        self.refresh_election_elapse();
        self._inner_sender.send(self.vote_for_self());
    }

    fn vote_for_self(&self) -> message::Message {
        message::Message::new(
            message::MessageType::Vote {
                last_log_index: self.last_log_index(),
                last_log_term: self.last_log_term(),
            },
            self._id.clone(),
            self._peers.iter()
                .map(|peer| peer.id())
                .collect(),
            self.current_term(),
        )
    }

    fn send_internal_to_peers(&self, _inner: message::Message) -> Result<(), rafterror::RaftError> {
        let node_id = self.node_id();

        match self.get_peer(&node_id) {
            Some(peer) =>
                match _inner.message_type() {
                    message::MessageType::Vote {
                        ..
                    } => peer.broadcast(_inner),
                    message::MessageType::Heartbeat => peer.broadcast(_inner),
                    _ => Ok(())
                },
            None => Err(rafterror::RaftError::MissPeer(node_id))
        }
    }

    fn post_election_timeout(&self) -> bool {
        self._election_elapse.get() >= self._election_timeout.clone()
    }

    fn is_follower(&self) -> bool {
        self._state == StateType::Follower
    }


    fn get_peer(&self, id: &u64) -> Option<&network::Peer> {
        for peer in &self._peers {
            if peer.id().eq(id) {
                return Option::Some(peer);
            }
        }

        Option::None
    }
    fn refresh_election_elapse(&mut self) {
        let mut rng = rand::thread_rng();
        self._random_election_timeout = rng.sample(
            rand::distributions::Uniform::from(0..self._election_timeout.clone())
        );
        self._election_elapse.set(0)
    }

    fn become_follower(&mut self, term: type_def::TermId, from: u64) {
        self.refresh_election_elapse();
        self._state = StateType::Follower;

        self._leader = from;
        self.reset(term);
    }
    fn last_log_index(&self) -> LogIndex {
        self._raftlog.last_index()
    }

    fn last_log_term(&self) -> TermId {
        self._raftlog.last_term()
    }
    fn node_id(&self) -> u64 {
        self._id.clone()
    }

    fn reset(&mut self, term: type_def::TermId) {
        if self.current_term() <= term {
            self._current_term = term;
        }

        self._progress.reset_vote()
    }

    fn can_vote(&self, from: u64, term: TermId) -> bool {
        self._vote_for == from ||
            (self._vote_for == NONE &&
                self._leader == NONE) ||
            (term > self.current_term())
    }

    fn try_send(&self, msg: Message) -> Result<(), RaftError> {
        self._inner_sender.try_send(msg)
            .map_err(|err| RaftError::from(err))
    }
    fn current_term(&self) -> TermId {
        self._current_term.clone()
    }

    fn state(&self) -> StateType {
        self._state.clone()
    }

    async fn try_recv(&mut self) -> Result<message::Message, mpsc::error::TryRecvError> {
        self._msg_recv.try_recv()
    }
    fn is_candidate(&self) -> bool {
        self._state == StateType::Candidate
    }

    fn step_candidate(&mut self, msg: Message) -> Result<(), RaftError> {
        match msg.message_type() {
            message::MessageType::Heartbeat => {
                let mut reject = false;

                if self.current_term() > msg.term() {
                    reject = true;
                }

                self.try_send(
                    message::Message::new(
                        message::MessageType::HeartbeatResp {
                            status: self._progress.progress(&self.node_id())
                                .unwrap()
                                .status(),
                            reject,
                        },
                        self.node_id(),
                        Vec::from([msg.from()]),
                        self.current_term(),
                    )
                )
            }
            message::MessageType::VoteRsp {
                granted
            } => {
                self._progress.record_vote(msg.from(), granted);
                match self._progress.tally_votes().status() {
                    VoteQuorum::Pending => Ok(()),
                    VoteQuorum::Won => Ok(()),
                    VoteQuorum::Lost => Ok(())
                }
            }
            message::MessageType::AppEntries {
                ref mut entries
            } => {
                self.become_follower(msg.term(), msg.from());
                self._raftlog.append(entries);

                Ok(())
            }
            message::MessageType::Vote {
                last_log_term, last_log_index
            } => {
                let mut granted = false;

                if self.can_vote(msg.from(), msg.term())
                    && self._raftlog.is_up_to_date(last_log_index, last_log_term) {
                    granted = true;
                }

                self.try_send(
                    message::Message::new(
                        message::MessageType::VoteRsp {
                            granted: granted.clone()
                        },
                        self.node_id(),
                        Vec::from([msg.from()]),
                        self.current_term(),
                    )
                )
            }
            _ => Err(RaftError::NotSupportedMessageType(msg.message_type()))
        }
    }
    fn step_follower(&mut self, msg: Message) -> Result<(), RaftError> {
        match msg.message_type() {
            message::MessageType::AppEntries {
                ref mut entries
            } => {
                let mut granted = false;

                if self.can_accept(msg.term(), entries) {
                    self._raftlog.append(entries);
                    granted = true;
                }

                self.try_send(message::Message::new(
                    message::MessageType::AppEntriesResp {
                        rejected: granted
                    },
                    self.node_id(),
                    Vec::from([msg.from()]),
                    self.current_term(),
                ))
            }
            message::MessageType::Heartbeat => {
                self.refresh_election_elapse();

                self.try_send(message::Message::new(
                    message::MessageType::HeartbeatResp {
                        status: self._progress.progress(&self.node_id())
                            .unwrap()
                            .status(),
                        reject: false,
                    },
                    self.node_id(),
                    Vec::from([msg.from()]),
                    self.current_term(),
                ))
            }
            _ => Err(rafterror::RaftError::NotSupportedMessageType(msg.message_type()))
        }
    }

    fn can_accept(&self, term: TermId, entries: &mut Vec<Entry>) -> bool {
        todo!()
    }
    fn step_leader(&self, msg: Message) -> Result<(), RaftError> {
        match msg.message_type() {
            message::MessageType::HeartbeatResp {
                status, reject
            } => {
                Ok(())
            }
            message::MessageType::Vote {
                last_log_index,
                last_log_term
            } => {
                Ok(())
            }
            message::MessageType::AppEntriesResp {
                rejected
            } => {
                Ok(())
            }
            _ => Err(rafterror::RaftError::NotSupportedMessageType(msg.message_type()))
        }
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
                result = utils::try_recv(&mut self._msg_recv) => {
                    match result {
                        Ok(msg) => self.step(msg).unwrap_or(()),
                        Err(err) => {
                            match err {
                                mpsc::error::TryRecvError::Empty => {
                                    if self.post_election_timeout() &&  self.is_follower() {
                                        self.become_candidate()
                                    }
                                },
                                mpsc::error::TryRecvError::Disconnected => panic!("channel has been closed")
                            }
                        }
                    }
                }

                _ = Self::tick(interv, &self._election_elapse), if self.is_leader() => {
                   self.try_send(
                        message::Message::new(
                            message::MessageType::Heartbeat,
                            self.node_id(),
                            self._peers
                                .iter()
                                .map(|peer| peer.id())
                                .collect(),
                            self.current_term(),
                        ))
                    .unwrap_or(())
                }

                Some(inner_msg) = utils::recv(&mut self._inner_recv) => {
                    self.send_internal_to_peers(inner_msg).unwrap_or(())
                }
            }
        }
    }

    /**
    Core Handler
     */
    fn step(&mut self, msg: message::Message) -> Result<(), RaftError> {
        match self._state {
            StateType::Candidate => self.step_candidate(msg),
            StateType::Follower => self.step_follower(msg),
            StateType::Leader => self.step_leader(msg)
        }
    }
}


#[cfg(test)]
mod raft_tests {
    use std::sync::Arc;

    use tokio::sync::mpsc;

    use crate::message::Message;
    use crate::network;
    use crate::network::Peer;
    use crate::raftnode;

    struct TestRaft(mpsc::UnboundedSender<Message>, raftnode::Raft);

    mod mem {
        use crate::rafterror::RaftError;
        use crate::storage;
        use crate::type_def::{LogIndex, TermId};

        pub struct InMemoryStorage {}

        impl storage::Storage for InMemoryStorage {
            fn append(&self, entries: Vec<storage::Entry>) -> Result<(), RaftError> {
                todo!()
            }

            fn last_term(&self) -> TermId {
                todo!()
            }

            fn last_index(&self) -> LogIndex {
                todo!()
            }
        }

        impl InMemoryStorage {
            pub fn new() -> InMemoryStorage {
                InMemoryStorage {}
            }
        }
    }


    fn new_raft() -> (mpsc::UnboundedSender<Message>, raftnode::Raft) {
        let (tx, rx) = mpsc::unbounded_channel::<Message>();

        let p: [network::Peer; 1] = [Peer::new(0, "localhost:8080", Vec::new()); 1];


        (
            tx,
            raftnode::new_raft(
                0,
                raftnode::Config::new(8080, 100, 10),
                Arc::new(mem::InMemoryStorage::new()),
                rx,
                Vec::from(p),
            ),
        )
    }

    mod state_machine_test {
        use std::sync::atomic;

        use crate::message;
        use crate::raftnode;
        use crate::storage;

        use super::new_raft;

        #[test]
        fn test_heartbeat() {
            let (tx, mut raft) = new_raft();

            tx.send(
                message::Message::new(
                    message::MessageType::Heartbeat,
                    2,
                    Vec::new(),
                    1,
                )
            );
            let result = raft._msg_recv.try_recv();
            assert!(result.is_ok());
            let msg = result.unwrap();
            let step = raft.step(msg);

            assert!(step.is_ok());
            assert_eq!(raft.current_term(), 1);
            assert_eq!(raft.state(), raftnode::StateType::Follower);
            assert_eq!(raft._leader, 2)
        }

        fn test_app_entries_accepted() {
            let (tx, mut raft) = new_raft();
            raft._current_term = 1;

            tx.send(
                message::Message::new(
                    message::MessageType::AppEntries {
                        entries: Vec::from([storage::Entry::new(1, 0, Vec::new())])
                    },
                    raft.node_id(),
                    Vec::new(),
                    raft.current_term(),
                )
            );
            let result = raft._msg_recv.try_recv();
            assert!(result.is_ok());
            let msg = result.unwrap();
            let step = raft.step(msg);
        }
    }
}