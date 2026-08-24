pub mod graceful_stop;
pub mod inspect;
pub mod job;

use crate::domain::{Ownership, TargetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedProcessTree {
    pub root_pid: u32,
    pub listener_pid: Option<u32>,
    pub target_id: TargetId,
    pub job_name: String,
    pub ownership: Ownership,
}
