//! Explicit owner-qualified model profiles and conservative whole-run reservations.
//! Fit/qualification declarations are inputs, never measurements made by this module.
use crate::{config::ProviderConfig,domain::*,error::{Error,Result},tools::sha256};
use serde::{Serialize,Deserialize};
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum Behavior {Shared,Concurrent,Relay}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Qualification {pub profile_digest:String,pub evidence_sha256:String,pub owner_statement:String}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {pub id:String,pub provider:ProviderConfig,pub instance:String,pub resident:bool,pub remote:bool,pub weights_ram:u64,pub weights_vram:u64,pub kv_ram_per_token:u64,pub kv_vram_per_token:u64,#[serde(default="one")]pub slots:u32,pub loading_ram:u64,pub loading_vram:u64,pub qualification:Option<Qualification>}
fn one()->u32{1}
impl Profile {pub fn digest(&self)->Result<String>{let mut profile=self.clone();profile.qualification=None;profile.resident=false;digest(&profile)}}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {pub ram:u64,pub vram:u64,pub os_ram:u64,pub tools_ram:u64,pub browser_ram:u64,pub loading_overlap_ram:u64,pub loading_overlap_vram:u64}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum PhaseGoal {WaitingOrSucceeded,Succeeded}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {pub tasks:Vec<usize>,pub profile:String,pub goal:PhaseGoal}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {#[serde(default)]pub adaptive:Option<crate::adaptive::Policy>,pub schema_version:u32,pub behavior:Behavior,pub limits:Limits,pub profiles:Vec<Profile>,pub assignments:Vec<String>,#[serde(default)]pub phases:Vec<Phase>}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reservation {pub request:RequestId,pub actor:ActorId,pub profile:String,pub context_tokens:u64,pub unknown:bool}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {pub phase:usize,pub from:Vec<String>,pub target:String,pub profile_digest:String,pub epoch:u64}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {#[serde(default)]pub adaptations:u32,pub policy_reference:String,pub policy:Policy,pub epoch:u64,pub revision:u64,pub phase:usize,pub phase_approved:bool,pub reservations:Vec<Reservation>,pub activation:Option<Activation>,pub handoffs:Vec<Vec<String>>}
pub fn digest<T:Serialize>(value:&T)->Result<String>{Ok(sha256(&serde_json::to_vec(value).map_err(|_|Error::Invalid)?))}
fn hash(s:&str)->bool{s.len()==64&&s.bytes().all(|b|b.is_ascii_hexdigit())}
fn add(a:u64,b:u64)->Result<u64>{a.checked_add(b).ok_or(Error::Invalid)}
fn mul(a:u64,b:u64)->Result<u64>{a.checked_mul(b).ok_or(Error::Invalid)}
impl Policy {
    pub fn profile(&self,id:&str)->Result<&Profile>{self.profiles.iter().find(|p|p.id==id).ok_or(Error::Invalid)}
    pub fn validate(&self,actors:usize,tasks:usize)->Result<()>{
        if let Some(adaptive)=&self.adaptive{adaptive.validate(actors)?;}
        if self.schema_version!=1||self.profiles.is_empty()||self.profiles.len()>32||self.assignments.len()!=actors||actors==0||actors>32||self.limits.os_ram<256*1024*1024||self.limits.tools_ram<1024*1024*1024||self.limits.ram==0||self.limits.ram>1<<40||self.limits.vram>1<<40||self.phases.len()>64{return Err(Error::Invalid);}
        let mut ids=std::collections::HashSet::new();for profile in &self.profiles{
            let endpoint=crate::providers::validate_endpoint(&profile.provider)?;let context=profile.provider.context_tokens.ok_or(Error::Unsupported)?;
            if profile.id.is_empty()||profile.id.len()>64||profile.instance.is_empty()||profile.instance.len()>128||!ids.insert(&profile.id)||profile.slots==0||profile.slots>32||context==0||context>1_000_000||profile.remote&&endpoint.scheme()!="https"||!profile.remote&&(profile.weights_ram==0&&profile.weights_vram==0||profile.kv_ram_per_token==0&&profile.kv_vram_per_token==0){return Err(Error::Invalid);}
            for value in [profile.weights_ram,profile.weights_vram,profile.kv_ram_per_token,profile.kv_vram_per_token,profile.loading_ram,profile.loading_vram]{if value>1<<40{return Err(Error::Invalid);}}
            if profile.slots>1{let evidence=profile.qualification.as_ref().ok_or(Error::Unsupported)?;if evidence.profile_digest!=profile.digest()?||!hash(&evidence.evidence_sha256)||evidence.owner_statement.is_empty()||evidence.owner_statement.len()>512{return Err(Error::Denied);}}
            for other in self.profiles.iter().filter(|p|p.instance==profile.instance){if other.provider.endpoint!=profile.provider.endpoint||other.provider.model!=profile.provider.model||other.provider.profile!=profile.provider.profile||other.weights_ram!=profile.weights_ram||other.weights_vram!=profile.weights_vram||other.kv_ram_per_token!=profile.kv_ram_per_token||other.kv_vram_per_token!=profile.kv_vram_per_token||other.slots!=profile.slots||other.resident!=profile.resident{return Err(Error::Invalid);}}
        }
        for id in &self.assignments{self.profile(id)?;}
        if self.behavior==Behavior::Shared{let first=&self.profile(&self.assignments[0])?.instance;if self.assignments.iter().any(|id|self.profile(id).is_ok_and(|p|&p.instance!=first)){return Err(Error::Invalid);}}
        if self.behavior==Behavior::Relay{if self.phases.is_empty(){return Err(Error::Invalid);}for phase in &self.phases{self.profile(&phase.profile)?;if phase.tasks.is_empty()||phase.tasks.len()>256||phase.tasks.iter().any(|i|*i>=tasks){return Err(Error::Invalid);}}}else if !self.phases.is_empty(){return Err(Error::Invalid);}
        if serde_json::to_vec(self).map_err(|_|Error::Invalid)?.len()>65536{return Err(Error::Unsupported);}Ok(())
    }
}
impl State {
    pub fn new(policy:Policy,epoch:u64,actors:usize,tasks:usize)->Result<Self>{policy.validate(actors,tasks)?;let policy_reference=digest(&policy)?;let state=Self{adaptations:0,policy_reference,policy,epoch,revision:0,phase:0,phase_approved:false,reservations:vec![],activation:None,handoffs:vec![]};let(ram,vram)=state.usage()?;if ram>state.policy.limits.ram||vram>state.policy.limits.vram{return Err(Error::Denied);}Ok(state)}
    pub fn usage(&self)->Result<(u64,u64)>{
        let limits=&self.policy.limits;let mut ram=add(add(limits.os_ram,limits.tools_ram)?,limits.browser_ram)?;let mut vram=0;let mut instances=std::collections::HashSet::new();
        for p in self.policy.profiles.iter().filter(|p|p.resident){if instances.insert(&p.instance){ram=add(ram,p.weights_ram)?;vram=add(vram,p.weights_vram)?;}}
        for request in &self.reservations{let p=self.policy.profile(&request.profile)?;ram=add(ram,mul(request.context_tokens,p.kv_ram_per_token)?)?;vram=add(vram,mul(request.context_tokens,p.kv_vram_per_token)?)?;}
        if let Some(activation)=&self.activation{let p=self.policy.profile(&activation.target)?;if !p.resident{ram=add(ram,p.weights_ram)?;vram=add(vram,p.weights_vram)?;}ram=add(add(ram,p.loading_ram)?,limits.loading_overlap_ram)?;vram=add(add(vram,p.loading_vram)?,limits.loading_overlap_vram)?;}
        Ok((ram,vram))
    }
    pub fn selected(&self,actor:usize)->Result<&Profile>{let id=if self.policy.behavior==Behavior::Relay{&self.policy.phases.get(self.phase).ok_or(Error::Unsupported)?.profile}else{self.policy.assignments.get(actor).ok_or(Error::Invalid)?};self.policy.profile(id)}
    pub fn eligible(&self,actor:ActorId,index:usize,task:usize)->bool{
        if self.activation.is_some()||self.reservations.iter().any(|r|r.actor==actor){return false;}if self.policy.behavior==Behavior::Relay&&(!self.phase_approved||!self.policy.phases.get(self.phase).is_some_and(|p|p.tasks.contains(&task))||!self.reservations.is_empty()){return false;}
        let Ok(profile)=self.selected(index)else{return false;};if !profile.resident{return false;}let count=self.reservations.iter().filter(|r|self.policy.profile(&r.profile).is_ok_and(|p|p.instance==profile.instance)).count();if count>=profile.slots as usize{return false;}
        let mut trial=self.clone();trial.reservations.push(Reservation{request:RequestId::default(),actor,profile:profile.id.clone(),context_tokens:profile.provider.context_tokens.unwrap_or(0) as u64,unknown:false});trial.usage().is_ok_and(|(ram,vram)|ram<=self.policy.limits.ram&&vram<=self.policy.limits.vram)
    }
    pub fn reserve(&mut self,request:RequestId,actor:ActorId,index:usize,task:usize,profile:&str,context:u64)->Result<()>{
        if self.reservations.len()>=32||self.reservations.iter().any(|r|r.request==request)||!self.eligible(actor,index,task)||self.selected(index)?.id!=profile||context==0||context>self.policy.profile(profile)?.provider.context_tokens.ok_or(Error::Unsupported)? as u64{return Err(Error::Denied);}
        self.reservations.push(Reservation{request,actor,profile:profile.into(),context_tokens:context,unknown:false});let(ram,vram)=self.usage()?;if ram>self.policy.limits.ram||vram>self.policy.limits.vram{self.reservations.pop();return Err(Error::Denied);}Ok(())
    }
    pub fn settle(&mut self,request:RequestId,known:bool)->Result<()>{let index=self.reservations.iter().position(|r|r.request==request).ok_or(Error::Conflict)?;if known{self.reservations.remove(index);}else{self.reservations[index].unknown=true;}Ok(())}
    pub fn fence(&mut self,epoch:u64){self.epoch=epoch;for r in &mut self.reservations{r.unknown=true;}self.phase_approved=false;}
    pub fn phase_proposal(&self)->Result<Activation>{let phase=self.policy.phases.get(self.phase).ok_or(Error::Invalid)?;let target=self.policy.profile(&phase.profile)?;Ok(Activation{phase:self.phase,from:self.policy.profiles.iter().filter(|p|p.resident).map(|p|p.instance.clone()).collect(),target:target.id.clone(),profile_digest:target.digest()?,epoch:self.epoch})}
    pub fn approve_phase(&mut self,approved:&str,handoff:Vec<String>)->Result<()>{if !self.reservations.is_empty()||self.activation.is_some()||handoff.len()>128{return Err(Error::Denied);}let proposal=self.phase_proposal()?;if digest(&proposal)?!=approved{return Err(Error::Denied);}let profile=self.policy.profile(&proposal.target)?;if profile.resident{self.phase_approved=true;self.handoffs.push(handoff);}else{self.activation=Some(proposal);let(ram,vram)=self.usage()?;if ram>self.policy.limits.ram||vram>self.policy.limits.vram{self.activation=None;return Err(Error::Denied);}}Ok(())}
    /// Only explicit owner-observed runtime state can complete an external activation.
    pub fn observe_activation(&mut self,approved:&str,old_unloaded:bool,new_resident:bool,evidence:&str)->Result<()>{let activation=self.activation.as_ref().ok_or(Error::Conflict)?;if digest(activation)?!=approved||!hash(evidence)||!old_unloaded||!new_resident{return Err(Error::Denied);}let target=self.policy.profile(&activation.target)?.instance.clone();for profile in &mut self.policy.profiles{profile.resident=profile.instance==target;}self.activation=None;self.phase_approved=true;self.handoffs.push(vec![format!("owner-observed:{evidence}")]);Ok(())}
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerObservation {pub request:Option<RequestId>,pub activation_digest:Option<String>,pub profile_digest:String,pub observed_complete:bool,pub old_unloaded:bool,pub new_resident:bool,pub evidence_sha256:String,pub owner_statement:String}
pub fn review_state(state:&State)->State{let mut state=state.clone();state.epoch=0;state.revision=0;state.phase_approved=false;for r in &mut state.reservations{r.unknown=true;}if let Some(a)=&mut state.activation{a.epoch=0;}state}
pub async fn observe(project:&crate::config::Project,path:&std::path::Path,run:RunId,observation:OwnerObservation,approved:Option<&str>)->Result<serde_json::Value>{
    let state=crate::ledger::model_inspect(project,path,run)?.ok_or(Error::Unsupported)?;if !hash(&observation.evidence_sha256)||observation.owner_statement.is_empty()||observation.owner_statement.len()>512{return Err(Error::Invalid);}let reviewed=digest(&(review_state(&state),&observation))?;
    let Some(approved)=approved else{return Ok(serde_json::json!({"model_pool":state,"owner_observation":observation,"review_digest":reviewed,"changed":false,"independent_measurement":false}));};if approved!=reviewed{return Err(Error::Denied);}
    let ledger=crate::ledger::Ledger::open(project,path)?;let current=ledger.model_state(run).await?.ok_or(Error::Unsupported)?;if digest(&(review_state(&current),&observation))?!=reviewed{return Err(Error::Conflict);}
    let command=if let Some(request)=observation.request{let held=current.reservations.iter().find(|r|r.request==request&&r.unknown).ok_or(Error::Conflict)?;if !observation.observed_complete||current.policy.profile(&held.profile)?.digest()?!=observation.profile_digest{return Err(Error::Denied);}crate::ledger::model_pool::Command::Reconcile{run,request,approved:digest(&current)?,evidence:observation.evidence_sha256}}
    else{let activation=current.activation.as_ref().ok_or(Error::Conflict)?;if Some(digest(activation)?)!=observation.activation_digest||activation.profile_digest!=observation.profile_digest{return Err(Error::Denied);}crate::ledger::model_pool::Command::Activation{run,approved:observation.activation_digest.ok_or(Error::Invalid)?,old_unloaded:observation.old_unloaded,new_resident:observation.new_resident,evidence:observation.evidence_sha256}};
    ledger.model_command(current.revision,command).await?;let state=ledger.model_state(run).await?;ledger.shutdown()?;Ok(serde_json::json!({"model_pool":state,"changed":true,"provenance":"explicit-owner-observation","budget_refunded":false}))
}

pub async fn restrict_modes(project:&crate::config::Project,path:&std::path::Path,run:RunId,allowed:Vec<crate::domain::Mode>,approved:Option<&str>)->Result<serde_json::Value>{let state=crate::ledger::model_inspect(project,path,run)?.ok_or(Error::Unsupported)?;let policy=state.policy.adaptive.as_ref().ok_or(Error::Unsupported)?;if allowed.len()>5||allowed.iter().enumerate().any(|(i,m)|allowed[..i].contains(m))||allowed.iter().any(|m|!policy.allowed.contains(m)){return Err(Error::Denied);}let review=digest(&(review_state(&state),&allowed))?;let Some(approved)=approved else{return Ok(serde_json::json!({"allowed":allowed,"review_digest":review,"changed":false}));};if review!=approved{return Err(Error::Denied);}let ledger=crate::ledger::Ledger::open(project,path)?;let current=ledger.model_state(run).await?.ok_or(Error::Unsupported)?;if digest(&(review_state(&current),&allowed))?!=review{return Err(Error::Conflict);}ledger.model_command(current.revision,crate::ledger::model_pool::Command::Restrict{run,allowed,approved:digest(&review_state(&current))?}).await?;ledger.shutdown()?;Ok(serde_json::json!({"allowed_modes_restricted":true,"new_permissions":false}))}
