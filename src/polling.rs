use lazy_static::lazy_static;
use uuid::Uuid;
use std::time::{ Instant, SystemTime, Duration };
use chashmap::CHashMap;
use serde::{ Serialize, Serializer };
use crate::server::responses::{Response, Metadata};

macro_rules! create_prefix {
    ($prefix:literal) => {
        macro_rules! prefix {
            ($body:literal) => { const_format::concatcp!($prefix, $body) }
        }
    };
}

/// Enum that represents the different states an ongoing deployment can be in
/// 
/// This is specific to the deployment process
/// 
/// ## Variants
/// - `Building` - The Docker image is in the process of being built
/// - `Pushing` - The Docker image is being pushed to the remote registry
/// - `Pulling` - The Docker image is being pulled from the remote registry
/// - `Deploying` - The challenge is being deployed to the Kubernetes cluster
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

/// Enum that represents the main states a deployment can be in 
/// 
/// ## Variants
/// - `InProgress` - The deployment is currently in progress
///     - Returns the time that the deployment started and the current step in the process it is at
/// - `Success` - The deployment was successful
///     - Returns the ports that the challenge/challenges is/are running on and the time deployment finished
/// - `Failure` - The deployment failed
///     - Returns the error that caused the failure and the time that it occurred at
#[derive(Debug, Clone, Default)]
pub enum DeploymentStatus {
    InProgress(Instant, DeployStep),
    Success(Instant, Vec<i32>),
    Failure(Instant, String),
    #[default]
    Unknown,
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
            Unknown => "unknown",
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
            Self::Unknown => Instant::now(),
        }
    }

    pub fn finished_data(&self) -> Option<serde_json::Value> {
        match self {
            Self::Success(_, ports) => Some(serde_json::to_value(ports).ok()?),
            Self::Failure(_, response) => Some(serde_json::to_value(response).ok()?),
            Self::InProgress(..) => None,
            Self::Unknown => None,
        }
    }

    pub fn since_last_change(&self) -> Duration {
        Instant::now().duration_since(self.last_change())
    }
}

#[derive(Serialize)]
struct DeploymentStatusSerializable {
    current_status: &'static str,
    seconds_since_last_change: f64,
    finished_meta: Option<serde_json::Value>,
}
impl DeploymentStatus {
    fn as_serializable(&self) -> DeploymentStatusSerializable {
        DeploymentStatusSerializable {
            current_status: self.get_str(),
            seconds_since_last_change: self.last_change().elapsed().as_secs_f64(),
            finished_meta: self.finished_data(),
        }
    }
}
impl Serialize for DeploymentStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer {
        self.as_serializable().serialize(serializer)
    }
}


/// Unique identifier that can be used to poll the status of a given deployment.
/// 
/// ## Fields
/// - `chall_id` : [Uuid] 
///     - The ID of the challenge that is being deployed
/// - `race_lock_id`: [Uuid]
///     - The ID of the request being made to deploy the challenge, prevents race conditions
/// 
/// ## Functions
/// - `new`: Creates a new `PollingId` from the given `chall_id` and `race_lock_id`
/// - `tup`: Returns a tuple of the `chall_id` and `race_lock_id`
pub type PollingId = Uuid;

lazy_static! {
    static ref CURRENT_DEPLOYMENTS: CHashMap<PollingId, DeploymentStatus> = CHashMap::new();
}

/// Registers a new deployment with the given `PollingId` and returns an error if the deployment is already in progress
pub fn register_chall_deployment(id: PollingId) -> Result<(), DeploymentStatus> {
    if let Some(curr_status) = CURRENT_DEPLOYMENTS.get(&id) {
        Err(curr_status.clone())
    } else {
        CURRENT_DEPLOYMENTS.insert(id, DeploymentStatus::InProgress(Instant::now(), DeployStep::Building));
        Ok(())
    }
}

/// Struct that contains information regarding the current status of a deployment 
/// 
/// When the server receives a poll request with a given `PollingId`, it will return this `PollInfo` struct
/// 
/// ## Fields
/// - `id`: [PollingId]
///     - The ID of the deployment that is being polled
/// - `status`: [DeploymentStatus]
///     - The current status of the deployment
/// - `poll_time`: [SystemTime]
///     - The time that the poll request was made
/// - `duration_since_last_change`: [Duration]
///    - The duration since the last change in the deployment status
#[derive(Debug, Clone, Serialize)]
pub struct PollInfo {
    pub (crate) id: PollingId,
    pub (crate) status: DeploymentStatus,
    pub (crate) poll_time: SystemTime,
    pub (crate) duration_since_last_change: Duration,
}

impl From<(PollInfo, Metadata)> for Response {
    fn from((info, meta): (PollInfo, Metadata)) -> Self {
        match serde_json::to_value(info) {
            Ok(val) => Response::success(meta, Some(val)),
            Err(e) => Response::ise(&e.to_string(), meta),
        }
    }
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

pub fn _update_deployment_state(id: PollingId, new_status: DeploymentStatus) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        *status = new_status;
        Ok(status.clone())
    } else {
        Err(id)
    }
}

pub fn advance_deployment_step(id: PollingId, new_step: Option<DeployStep>) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        if let DeploymentStatus::InProgress(time, step) = &mut *status {
            let new_step = new_step.or_else(|| step.next()).ok_or(id)?;
            *step = new_step;
            *time = Instant::now();
            Ok(status.clone())
        } else {
            Err(id)
        }
    } else {
        Err(id)
    }
}

/// Marks a given `PollingId` as `DeploymentStatus::Failure`
/// ## Returns
/// - `Ok(DeploymentStatus)` : Returns the new `DeploymentStatus` if the `PollingId` was marked as successful
/// - `Err(PollingId)` : Returns the `PollingId` if the given `PollingId` is already marked as finished
pub fn fail_deployment(id: PollingId, reason: String) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        if !status.is_finished() {
            *status = DeploymentStatus::Failure(Instant::now(), reason);
            Ok(status.clone())
        } else {
            Err(id)
        }
    } else {
        Err(id)
    }
}

/// Marks a given `PollingId` as `DeploymentStatus::Success`
/// ## Returns
/// - `Ok(DeploymentStatus)` : Returns the new `DeploymentStatus` if the `PollingId` was marked as successful
/// - `Err(PollingId)` : Returns the `PollingId` if the given `PollingId` is already marked as finished
pub fn succeed_deployment(id: PollingId, response: &[i32]) -> Result<DeploymentStatus, PollingId> {
    if let Some(mut status) = CURRENT_DEPLOYMENTS.get_mut(&id) {
        if !status.is_finished() {
            *status = DeploymentStatus::Success(Instant::now(), response.to_vec());
            Ok(status.clone())
        } else {
            Err(id)
        }
    } else {
        Err(id)
    }
}