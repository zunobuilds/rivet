use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA: u32 = 1;
macro_rules! ids {
    ($($name:ident),+) => {$ (
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);
        impl Default for $name { fn default() -> Self { Self(Uuid::new_v4()) } }
    )+};
}
ids!(ProjectId, SessionId, RunId, TaskId, ActorId, OperationId, MessageId, RequestId);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Solo, Pair, Crew, Fleet, Mesh, Adaptive }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role { Independent, Lead, Reviewer, Operator, Director }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSpec {
    pub schema_version: u32,
    pub project: ProjectId,
    pub session: SessionId,
    pub run: RunId,
    pub task: String,
    pub mode: Mode,
    pub roles: Vec<Role>,
    pub provider: Option<String>,
    pub deadline: u64,
    pub step_budget: u32,
    pub permission_epoch: u64,
}
impl RunSpec {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA || self.step_budget == 0 || self.deadline == 0 { return Err(Error::Invalid); }
        validate_roles(&self.mode,&self.roles,false)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageEnvelope<T> {
    pub schema_version: u32,
    pub id: MessageId,
    pub causal_id: Option<MessageId>,
    pub sender: ActorId,
    pub recipient: ActorId,
    pub task: TaskId,
    pub sequence: u64,
    pub evidence: Vec<String>,
    pub user_decisions: Vec<String>,
    pub payload: T,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome { Succeeded, Failed, Cancelled, Unknown }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub schema_version: u32,
    pub operation: OperationId,
    pub attempt: u32,
    pub outcome: Outcome,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub artifacts: Vec<String>,
    pub before_revision: Option<String>,
    pub after_revision: Option<String>,
    pub redacted_output: String,
}

pub fn validate_roles(mode:&Mode,roles:&[Role],minimal:bool)->Result<()>{
    if minimal&&*mode!=Mode::Solo{return Err(Error::Denied);}if roles.len()>32{return Err(Error::Invalid);}
    let n=|role|roles.iter().filter(|r|**r==role).count();
    let valid=match mode{
        Mode::Solo=>roles==[Role::Independent],
        Mode::Pair=>roles.len()==2&&n(Role::Lead)==1&&n(Role::Operator)==1,
        Mode::Crew=>n(Role::Lead)==1&&n(Role::Reviewer)==1&&n(Role::Operator)>=1&&roles.len()==2+n(Role::Operator),
        Mode::Fleet=>n(Role::Director)==1&&n(Role::Lead)>=1&&n(Role::Reviewer)>=1&&n(Role::Operator)>=1&&roles.len()==1+n(Role::Lead)+n(Role::Reviewer)+n(Role::Operator),
        Mode::Mesh=>!roles.is_empty()&&roles.iter().all(|r|*r==Role::Independent),Mode::Adaptive=>return Err(Error::Unsupported),
    };if valid{Ok(())}else{Err(Error::Invalid)}
}
