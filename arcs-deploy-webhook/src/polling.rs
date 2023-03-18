use lazy_static::lazy_static;
use uuid::Uuid;
use std::time::{ Instant, SystemTime, Duration };
use chashmap::CHashMap;
use std::fmt::Display;

use crate::server::Response;

macro_rules! create_prefix {
    ($prefix:literal) => {
        macro_rules! prefix {
            ($body:literal) => { const_format::concatcp!($prefix, $body) }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployStep {
    Building,
    Pushing,
    Pulling,
    Deploying,
}

impl DeployStep {
    pub fn get_str(&self) -> &'static str {
        use DeployStep::*;
        
        create_prefix!("in_progress:");

        match self {
            Building => prefix!("building"),
            Pushing => prefix!("pushing"),
            Pulling => prefix!("pulling"),
            Deploying => prefix!("deploying"),
        }
    }

    pub fn next(&self) -> Option<Self> {
        use DeployStep::*;
        match self {
            Building => Some(Pushing),
            Pushing => Some(Pulling),
            Pulling => Some(Deploying),
            Deploying => None,
        }
    }
}


#[derive(Debug, Clone)]
pub enum DeploymentStatus {
    InProgress(Instant, DeployStep),
    Success(Instant, Vec<i32>),
    Failure(Instant, Response),
}

impl DeploymentStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Success(_, _) | Self::Failure(_, _))
    }

    pub fn get_str(&self) -> &'static str {
        use DeploymentStatus::*;
        match self {
            InProgress(_, step) => step.get_str(),
            Success(_, _) => "success",
            Failure(_, _) => "failure",
        }
    }

    pub fn finish_time(&self) -> Option<Instant> {
        match self {
            Self::Success(instant, _) | Self::Failure(instant, _) => Some(*instant),
            _ => None,
        }
    }

    pub fn start_time(&self) -> Option<Instant> {
        match self {
            Self::InProgress(instant, _) => Some(*instant),
            _ => None,
        }
    }

    pub fn last_change(&self) -> Instant {
        match self {
            Self::Success(instant, _) |
            Self::Failure(instant, _) |
            Self::InProgress(instant, _) => *instant,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollingId {
    chall_id: Uuid,
    race_lock_id: Uuid,
}
impl PollingId {
    pub fn new(chall_id: Uuid, race_lock_id: Uuid) -> Self {
        Self { chall_id, race_lock_id }
    }
}
impl Display for PollingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Polling<{}:{}>", self.chall_id, self.race_lock_id)
    }
}


lazy_static! {
    static ref CURRENT_DEPLOYMENTS: CHashMap<PollingId, DeploymentStatus> = CHashMap::new();
}

pub fn register_chall_deployment(id: PollingId) -> Result<(), DeploymentStatus> {
    if let Some(curr_status) = CURRENT_DEPLOYMENTS.get(&id) {
        Err(curr_status.clone())
    } else {
        CURRENT_DEPLOYMENTS.insert(id, DeploymentStatus::InProgress(Instant::now(), DeployStep::Building));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PollInfo {
    id: PollingId,
    status: DeploymentStatus,
    poll_time: SystemTime,
    duration_since_last_change: Duration,
}

pub fn poll_deployment(id: PollingId) -> Result<PollInfo, PollingId> {
    if let Some(status) = CURRENT_DEPLOYMENTS.get(&id) {
        let duration_since_last_change = Instant::now().duration_since(status.last_change());
        let poll_time = SystemTime::now();
        Ok(PollInfo {
            id,
            status: status.clone(),
            poll_time,
            duration_since_last_change,
        })
    } else {
        Err(id)
    }
}

pub fn update_deployment_state(id: PollingId, new_status: DeploymentStatus) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        *status = new_status;
        Ok(status.clone())
    } else {
        Err(id)
    }
}

pub fn advance_deployment_step(id: PollingId, new_step: Option<DeployStep>) -> Result<DeploymentStatus, PollingId> {
    if let Some(status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        if let DeploymentStatus::InProgress(mut _time, mut _step) = *status {
            let new_step = new_step.or_else(|| _step.next()).ok_or(id)?;
            _step = new_step;
            _time = Instant::now();
            Ok(status.clone())
        } else {
            Err(id)
        }
    } else {
        Err(id)
    }
}

pub fn fail_deployment(id: PollingId, response: Response) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        if !status.is_finished() {
            *status = DeploymentStatus::Failure(Instant::now(), response);
            Ok(status.clone())
        } else {
            Err(id)
        }
    } else {
        Err(id)
    }
}

pub fn succeed_deployment(id: PollingId, response: Vec<i32>) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        if !status.is_finished() {
            *status = DeploymentStatus::Success(Instant::now(), response);
            Ok(status.clone())
        } else {
            Err(id)
        }
    } else {
        Err(id)
    }
}

