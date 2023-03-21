use lazy_static::lazy_static;
use uuid::Uuid;
use std::time::{ Instant, SystemTime, Duration };
use chashmap::CHashMap;
use std::fmt::Display;
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

    pub fn finished_data(&self) -> Option<Result<Vec<i32>, Response>> {
        match self {
            Self::Success(_, ports) => Some(Ok(ports.clone())),
            Self::Failure(_, response) => Some(Err(response.clone())),
            Self::InProgress(..) => None,
        }
    }
}

#[derive(Serialize)]
struct DeploymentStatusSerializable {
    current_status: &'static str,
    seconds_since_last_change: f64,
    finished_meta: Option<Result<Vec<i32>, Response>>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PollingId {
    chall_id: Uuid,
    race_lock_id: Uuid,
}
impl PollingId {
    pub fn new(chall_id: Uuid, race_lock_id: Uuid) -> Self {
        Self { chall_id, race_lock_id }
    }
    pub fn tup(&self) -> (Uuid, Uuid) {
        (self.chall_id, self.race_lock_id)
    }
}
impl Display for PollingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Polling<{}.{}>", self.chall_id, self.race_lock_id)
    }
}
impl Serialize for PollingId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}


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
    id: PollingId,
    status: DeploymentStatus,
    poll_time: SystemTime,
    duration_since_last_change: Duration,
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

/// Marks a given `PollingId` as `DeploymentStatus::Success`
/// ## Returns
/// - `Ok(DeploymentStatus)` : Returns the new `DeploymentStatus` if the `PollingId` was marked as successful
/// - `Err(PollingId)` : Returns the `PollingId` if the given `PollingId` is already marked as finished
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


mod polling_id_deserialize {
    use super::PollingId;
    use serde::{ Deserialize, de::Visitor, Deserializer };
    use serde::de::{ Error as DeErr, MapAccess as DeMapAccess, Unexpected as DeUnexpect };
    use std::fmt;
    use uuid::Uuid;


    const FIELDS: &'static [&'static str] = &["chall_id", "deploy_race_lock_id"];

    enum PollingIdField { Chall, Race }

    impl<'de> Deserialize<'de> for PollingIdField {
        fn deserialize<D>(deserializer: D) -> Result<PollingIdField, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct FieldVisitor;

            impl<'de> Visitor<'de> for FieldVisitor {
                type Value = PollingIdField;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("`chall_id` or `deploy_race_lock_id`")
                }

                fn visit_str<E>(self, value: &str) -> Result<PollingIdField, E>
                where
                    E: DeErr,
                {
                    match value {
                        "chall_id" => Ok(PollingIdField::Chall),
                        "deploy_race_lock_id" => Ok(PollingIdField::Race),
                        _ => Err(DeErr::unknown_field(value, FIELDS)),
                    }
                }
            }

            deserializer.deserialize_identifier(FieldVisitor)
        }
    }

    struct PollingIdVisitor;
    
    impl<'de> Visitor<'de> for PollingIdVisitor {
        type Value = PollingId;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("<uuid>.<uuid> OR { chall_id: <uuid>, deploy_race_lock_id: <uuid> }")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: DeErr, {
            if let Some((chall, race_lock)) = v.split_once('.') {
                match (Uuid::parse_str(chall), Uuid::parse_str(race_lock)) {
                    (Ok(chall), Ok(race)) => Ok(PollingId::new(chall, race)),
                    (Err(_), Ok(_)) => Err(DeErr::invalid_value(DeUnexpect::Str(v), &"Valid uuid before the period.")),
                    (Ok(_), Err(_)) => Err(DeErr::invalid_value(DeUnexpect::Str(v), &"Valid uuid after the period.")),
                    (Err(_), Err(_)) => Err(DeErr::invalid_value(DeUnexpect::Str(v), &"<uuid>.<uuid>")),
                }
            } else {
                Err(DeErr::invalid_value(DeUnexpect::Str(v), &"<uuid>.<uuid>"))
            }
        }

        fn visit_map<V>(self, mut map: V) -> Result<PollingId, V::Error>
        where
            V: DeMapAccess<'de>,
        {
            let mut chall_id = None;
            let mut race_lock = None;
            while let Some(key) = map.next_key()? {
                match key {
                    PollingIdField::Chall => {
                        if chall_id.is_some() {
                            return Err(DeErr::duplicate_field(FIELDS[0]));
                        }
                        chall_id = Some(map.next_value()?);
                    }
                    PollingIdField::Race => {
                        if race_lock.is_some() {
                            return Err(DeErr::duplicate_field(FIELDS[1]));
                        }
                        race_lock = Some(map.next_value()?);
                    }
                }
            }
            let chall_id = chall_id.ok_or_else(|| DeErr::missing_field(FIELDS[0]))?;
            let race_lock_id = race_lock.ok_or_else(|| DeErr::missing_field(FIELDS[1]))?;
            Ok(PollingId::new(chall_id, race_lock_id))
        }
    }

    impl<'de> Deserialize<'de> for PollingId {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer
                .deserialize_any(PollingIdVisitor)
        }
    }
}
