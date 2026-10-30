//! Durable bounded run controls. No grant, credential, inference retry or transcript is restored.
use crate::{domain::*,error::{Error,Result},ledger::{Ledger,Transition}};
use rusqlite::{Transaction,params};
use serde::{Serialize,Deserialize};
use std::sync::{Arc,Mutex};
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum State {Active,Paused,Stopped,Completed,Blocked,Expired,RecoveryRequired}
impl State {
    pub fn name(self)->&'static str{match self{Self::Active=>"active",Self::Paused=>"paused",Self::Stopped=>"stopped",Self::Completed=>"completed",Self::Blocked=>"blocked",Self::Expired=>"expired",Self::RecoveryRequired=>"recovery-required"}}
}
#[derive(Clone,Debug,Serialize,Deserialize,Default)]
#[serde(rename_all="snake_case")]
pub enum Boundary {#[default] Admitted,BeforeInference,BeforeTool,ToolReceipt,Drained,SyntheticDispatch}
#[derive(Clone,Debug,Serialize,Deserialize,Default)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {pub boundary:Boundary,pub evidence:Vec<String>}
impl Checkpoint {
    fn validate(&self)->Result<()>{if self.evidence.len()>32||self.evidence.iter().any(|v|v.len()>256||v.chars().any(char::is_control)){return Err(Error::Invalid);}Ok(())}
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {pub coordination_digest:Option<String>,pub task:TaskId,pub actor:ActorId,pub spec:RunSpec,pub version:u64,pub state:String,pub revision:u64,pub steps:u32,pub last_wall:u64,pub remaining_ms:u64,pub detached:bool,pub checkpoint:Checkpoint}
impl Snapshot {pub fn digest(&self)->Result<String>{Ok(crate::tools::sha256(&serde_json::to_vec(self).map_err(|_|Error::State)?))}}
#[derive(Clone,Debug)]
pub enum Command {
    Admit{spec:RunSpec,wall:u64,detached:bool},
    Tick{run:RunId,wall:u64,remaining_ms:u64,step:bool,checkpoint:Checkpoint},
    Finish{run:RunId,state:State,wall:u64,remaining_ms:u64,checkpoint:Checkpoint},
    Resume{run:RunId,wall:u64,approved:String,detached:bool},
    Stop{run:RunId,wall:u64},
    Inspect{run:RunId,result:Arc<Mutex<Option<Snapshot>>>},
}
fn number(v:u64)->Result<i64>{v.try_into().map_err(|_|Error::Invalid)}
pub(crate) fn snapshot(db:&rusqlite::Connection,run:RunId)->Result<Snapshot>{
    let (document,version,state,revision,steps,wall,remaining_ms,detached,checkpoint):(String,i64,String,i64,u32,i64,i64,bool,String)=db.query_row("SELECT s.document,s.version,l.status,l.revision,l.steps,l.last_wall,l.remaining_ms,l.detached,l.checkpoint FROM run_lifecycle l JOIN run_specs s ON s.run_id=l.run_id WHERE l.run_id=?1 ORDER BY s.version DESC LIMIT 1",[run.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)))?;
    let (task,actor):(String,String)=db.query_row("SELECT root_task,root_actor FROM run_lifecycle WHERE run_id=?1",[run.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;
    Ok(Snapshot{coordination_digest:crate::ledger::coordination_digest(db,run)?,task:TaskId(uuid::Uuid::parse_str(&task).map_err(|_|Error::State)?),actor:ActorId(uuid::Uuid::parse_str(&actor).map_err(|_|Error::State)?),spec:serde_json::from_str(&document).map_err(|_|Error::State)?,version:version.try_into().map_err(|_|Error::State)?,state,revision:revision.try_into().map_err(|_|Error::State)?,steps,last_wall:wall.try_into().map_err(|_|Error::State)?,remaining_ms:remaining_ms.try_into().map_err(|_|Error::State)?,detached,checkpoint:serde_json::from_str(&checkpoint).map_err(|_|Error::State)?})
}
pub(crate) fn uncertain(db:&rusqlite::Connection,run:RunId)->Result<bool>{
    Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE run_id=?1 AND state NOT IN ('terminal','denied','reconciled-observed','abandoned-unknown')) OR EXISTS(SELECT 1 FROM inference_reservations i WHERE run_id=?1 AND state IN ('reserved','unknown') AND NOT EXISTS(SELECT 1 FROM recovery_resolutions x WHERE x.entity_id=i.request_id AND x.entity_kind='request' AND x.source_digest=i.request_digest))",[run.0.to_string()],|r|r.get(0))?)
}
fn spec_record(tx:&Transaction<'_>,spec:&RunSpec,version:u64)->Result<()>{let document=serde_json::to_string(spec).map_err(|_|Error::Invalid)?;tx.execute("INSERT INTO run_specs VALUES(?1,?2,?3,?4)",params![spec.run.0.to_string(),number(version)?,document,crate::tools::sha256(document.as_bytes())])?;Ok(())}
pub(crate) fn apply(tx:&Transaction<'_>,command:Command,revision:u64,epoch:u64)->Result<()> {
    let (run,kind)=match &command{Command::Admit{spec,..}=>(spec.run,"lifecycle-admitted"),Command::Tick{run,..}=>(*run,"checkpoint"),Command::Finish{run,state,..}=>(*run,state.name()),Command::Resume{run,..}=>(*run,"explicit-resume"),Command::Inspect{run,..}=>(*run,"inspect"),Command::Stop{run,..}=>(*run,"stopped")};
    if let Command::Inspect{result,..}=command{*result.lock().map_err(|_|Error::State)?=Some(snapshot(tx,run)?);return Ok(());}
    match command {
        Command::Admit{spec,wall,detached}=>{
            spec.validate()?;if revision!=0||spec.task.is_empty()||spec.task.len()>4096||spec.deadline<=wall||spec.deadline-wall>3600||spec.step_budget>100||spec.permission_epoch!=epoch{return Err(Error::Invalid);}
            let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM runs,meta WHERE runs.id=?1 AND runs.session_id=?2 AND runs.state='active' AND runs.owner_epoch=?3 AND meta.project_uuid=?4)",params![run.0.to_string(),spec.session.0.to_string(),number(epoch)?,spec.project.0.to_string()],|r|r.get(0))?;if !valid{return Err(Error::Conflict);}
            if crate::ledger::coordination_roles(tx,run)?.is_some_and(|(mode,roles)|mode!=spec.mode||roles!=spec.roles){return Err(Error::Denied);}
            spec_record(tx,&spec,1)?;
            let (task,actor)=if let Some((task,actor))=crate::ledger::coordination_root(tx,run)?{(task.0.to_string(),actor.0.to_string())}else{tx.query_row("SELECT id,owner_agent_id FROM tasks WHERE run_id=?1 ORDER BY id LIMIT 1",[run.0.to_string()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?};
            tx.execute("INSERT INTO run_lifecycle VALUES(?1,'active',0,0,?2,?3,?4,?5,?6,?7)",params![run.0.to_string(),number(wall)?,number((spec.deadline-wall)*1000)?,detached,serde_json::to_string(&Checkpoint::default()).map_err(|_|Error::State)?,task,actor])?;
        },
        Command::Tick{wall,remaining_ms,step,checkpoint,..}=>{return update(tx,run,revision,wall,remaining_ms,step,None,checkpoint);},
        Command::Finish{wall,remaining_ms,state,checkpoint,..}=>{if state==State::Active||state==State::RecoveryRequired{return Err(Error::Invalid);}return update(tx,run,revision,wall,remaining_ms,false,Some(state),checkpoint);},
        Command::Stop{wall,..}=>{
            let current=snapshot(tx,run)?;if current.revision!=revision||["completed","stopped"].contains(&current.state.as_str()){return Err(Error::Conflict);}
            tx.execute("UPDATE run_lifecycle SET status='stopped',revision=revision+1,last_wall=max(last_wall,?1) WHERE run_id=?2",params![number(wall)?,run.0.to_string()])?;
            tx.execute("UPDATE runs SET state='stopped',revision=revision+1 WHERE id=?1",[run.0.to_string()])?;
            tx.execute("DELETE FROM grants WHERE operation_id IN(SELECT id FROM operations WHERE run_id=?1)",[run.0.to_string()])?;
        },
        Command::Resume{wall,approved,detached,..}=>{
            let mut current=snapshot(tx,run)?;
            if current.revision!=revision||current.digest()?!=approved||!["paused","recovery-required","blocked"].contains(&current.state.as_str())||wall<current.last_wall||wall>=current.spec.deadline||current.remaining_ms==0||current.steps>=current.spec.step_budget||uncertain(tx,run)?{return Err(Error::Denied);}
            current.spec.permission_epoch=current.spec.permission_epoch.checked_add(1).ok_or(Error::State)?.max(epoch);
            spec_record(tx,&current.spec,current.version+1)?;
            tx.execute("DELETE FROM grants WHERE operation_id IN(SELECT id FROM operations WHERE run_id=?1)",[run.0.to_string()])?;
            tx.execute("DELETE FROM forwarding_consents WHERE run_id=?1",[run.0.to_string()])?;
            tx.execute("UPDATE run_lifecycle SET status='active',revision=revision+1,last_wall=?1,remaining_ms=min(remaining_ms,?2),detached=?4 WHERE run_id=?3",params![number(wall)?,number((current.spec.deadline-wall)*1000)?,run.0.to_string(),detached])?;
            tx.execute("UPDATE runs SET state='active',owner_epoch=?1,revision=revision+1 WHERE id=?2",params![number(epoch)?,run.0.to_string()])?;
        },
        Command::Inspect{..}=>unreachable!(),
    }
    super::ledger::audit(tx,&run.0.to_string(),"supervisor",kind)
}
fn update(tx:&Transaction<'_>,run:RunId,revision:u64,wall:u64,remaining:u64,step:bool,finish:Option<State>,checkpoint:Checkpoint)->Result<()>{
    checkpoint.validate()?;let current=snapshot(tx,run)?;
    if current.revision!=revision||current.state!="active"{return Err(Error::Conflict);}
    let remaining=remaining.min(current.remaining_ms).min(current.spec.deadline.saturating_sub(wall).saturating_mul(1000));
    let state=if wall<current.last_wall{State::Blocked}else if remaining==0{State::Expired}else if step&&current.steps>=current.spec.step_budget{State::Blocked}else if finish==Some(State::Completed)&&(uncertain(tx,run)?||!crate::ledger::coordination_done(tx,run)?){State::Blocked}else{finish.unwrap_or(State::Active)};
    let steps=current.steps+u32::from(step&&state==State::Active);
    tx.execute("UPDATE run_lifecycle SET status=?1,revision=revision+1,steps=?2,last_wall=max(last_wall,?3),remaining_ms=?4,checkpoint=?5 WHERE run_id=?6",params![state.name(),steps,number(wall)?,number(remaining)?,serde_json::to_string(&checkpoint).map_err(|_|Error::State)?,run.0.to_string()])?;
    if state!=State::Active{
        tx.execute("DELETE FROM grants WHERE operation_id IN(SELECT id FROM operations WHERE run_id=?1)",[run.0.to_string()])?;
        tx.execute("UPDATE runs SET state=?1,revision=revision+1 WHERE id=?2",params![state.name(),run.0.to_string()])?;
    }
    crate::ledger::audit(tx,&run.0.to_string(),"supervisor",if state==State::Active{"checkpoint"}else{state.name()})
}
impl Ledger {
    pub async fn lifecycle(&self,run:RunId)->Result<Snapshot>{let result=Arc::new(Mutex::new(None));self.transition(0,Transition::Lifecycle{command:Command::Inspect{run,result:result.clone()}}).await?;result.lock().map_err(|_|Error::State)?.take().ok_or(Error::State)}
    pub async fn life(&self,revision:u64,command:Command)->Result<()>{self.transition(revision,Transition::Lifecycle{command}).await}
}
pub fn inspect(project:&crate::config::Project,path:&std::path::Path,run:RunId)->Result<Snapshot>{crate::ledger::lifecycle_inspect(project,path,run)}

pub async fn stop(project:&crate::config::Project,path:&std::path::Path,run:RunId,wall:u64)->Result<Snapshot>{
    inspect(project,path,run)?;
    let report=crate::ledger::repair_inspect(project,path)?;
    crate::ledger::recover_state(project,path,&report.snapshot_digest).await?;
    let ledger=Ledger::open(project,path)?;let current=ledger.lifecycle(run).await?;
    ledger.life(current.revision,Command::Stop{run,wall}).await?;let result=ledger.lifecycle(run).await?;ledger.shutdown()?;Ok(result)
}
