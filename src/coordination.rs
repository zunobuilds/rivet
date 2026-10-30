//! Typed coordinator commands and evidence. Provider/model output has no direct ledger handle.
use crate::{domain::*,scheduler::*,error::{Error,Result},ledger::{Ledger,Transition}};
use serde::{Serialize,Deserialize};
use std::sync::{Arc,Mutex};
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Observation {File{revisions:Vec<RevisionRef>},Check{projection:String,revisions:Vec<RevisionRef>,exit_code:Option<i32>,truncated:bool,cancelled:bool}}
pub const OBSERVATION_MEDIA:&str="application/vnd.rivet.observation+json";
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Payload {Progress{summary:String},Evidence{references:Vec<String>},Decision{reference:String,scope:String,text:String}}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consent {pub reference:String,pub recipient:ActorId,pub scope:String,pub epoch:u64,pub expiry:u64}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completion {pub claim:Claim,pub success:bool,pub review:Option<ReviewResult>}
#[derive(Clone,Debug)]
pub enum Command {
    Escalate{run:RunId,claim:Claim,operation:OperationId},ResumeConflicts{run:RunId},
    NextSequence{run:RunId,sender:ActorId,recipient:ActorId,result:Arc<Mutex<Option<u64>>>},
    Inbox{run:RunId,recipient:ActorId,result:Arc<Mutex<Vec<Delivery>>>},
    Withhold{run:RunId,id:MessageId,recipient:ActorId},
    Admit{run:RunId,plan:Plan},Inspect{run:RunId,result:Arc<Mutex<Option<Graph>>>},
    Claim{run:RunId,task:TaskId,now:u64,expires:u64},Yield{run:RunId,claim:Claim,now:u64},
    Finish{run:RunId,completion:Completion,now:u64},RecordEvidence{run:RunId,claim:Claim,operation:OperationId,now:u64},
    Release{run:RunId,claim:Claim},
    Expire{run:RunId,now:u64},Reconcile{run:RunId,task:TaskId},
    Forward{run:RunId,consent:Consent,message:MessageEnvelope<Payload>,now:u64},
    Consent{run:RunId,consent:Consent},Send{run:RunId,message:MessageEnvelope<Payload>,now:u64},
    Ack{run:RunId,id:MessageId,recipient:ActorId,context:Claim,completion:Option<Completion>,now:u64},
}
impl Ledger {
    pub async fn coordinate(&self,revision:u64,command:Command)->Result<()>{self.transition(revision,Transition::Coordination{command}).await}
    pub async fn graph(&self,run:RunId)->Result<Graph>{let result=Arc::new(Mutex::new(None));self.coordinate(0,Command::Inspect{run,result:result.clone()}).await?;result.lock().map_err(|_|Error::State)?.take().ok_or(Error::State)}
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Delivery {pub message:MessageEnvelope<Payload>,pub permitted:bool}
impl Ledger {pub async fn inbox(&self,run:RunId,recipient:ActorId)->Result<Vec<Delivery>>{let result=Arc::new(Mutex::new(vec![]));self.coordinate(0,Command::Inbox{run,recipient,result:result.clone()}).await?;let values=std::mem::take(&mut *result.lock().map_err(|_|Error::State)?);Ok(values)}}
