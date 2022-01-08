use std::collections::BTreeMap;

use crate::type_def::LogIndex;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum ProgressStatus {
    Probe,
    Follower,
    Snapshot,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum VoteQuorum {
    Pending,
    Won,
    Lost,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct VoteResult {
    granted: u64,
    rejected: u64,
    status: VoteQuorum,
}

impl VoteResult {
    pub fn status(&self) -> VoteQuorum {
        self.status.clone()
    }
}

#[derive(Clone)]
pub struct ProgressTracker {
    progress: BTreeMap<u64, Progress>,
    votes: BTreeMap<u64, bool>,
}

impl ProgressTracker {
    pub fn tally_votes(&self) -> VoteResult {
        todo!()
    }

    pub fn record_vote(&mut self, from: u64, granted: bool) {
        self.votes.insert(from, granted);
    }

    pub fn new(node_id: u64) -> Self {
        ProgressTracker {
            progress: BTreeMap::from([(node_id, Progress::new(ProgressStatus::Follower))]),
            votes: Default::default(),
        }
    }

    pub fn progress(&self, id: &u64) -> Option<&Progress> {
        self.progress.get(id)
    }

    pub fn reset_vote(&mut self) {
        self.votes.clear()
    }

    pub fn update_progress(&mut self, id: u64, progress: &Progress) {
        self.progress.insert(id, progress.clone());
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Progress {
    progress_status: ProgressStatus,
    _match: LogIndex,
    _next: LogIndex,
}

impl Progress {
    pub fn status(&self) -> ProgressStatus {
        self.progress_status.clone()
    }

    pub fn update_status(&mut self, status: ProgressStatus) {
        self.progress_status = status;
    }

    pub fn new(status: ProgressStatus) -> Progress {
        Progress {
            progress_status: status,
            _match: 0,
            _next: 0,
        }
    }
}