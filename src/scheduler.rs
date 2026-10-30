//! One bounded deterministic task graph. Claims/leases coordinate work; they grant no tool authority.
use crate::{domain::*,error::{Error,Result}};
use serde::{Serialize,Deserialize};
use std::{collections::{HashSet,VecDeque},path::{PathBuf,Component}};
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agent {pub id:ActorId,pub role:Role,pub reports_to:Option<ActorId>,#[serde(default)]pub skills:Vec<String>}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Kind {Coordinate,Coding,Review{subjects:Vec<TaskId>}}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum Criterion {Text,Checked{paths:Vec<PathBuf>}}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(tag="kind",content="path",rename_all="snake_case")]
pub enum Resource {Path(PathBuf),Git}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {pub id:TaskId,pub parent:Option<TaskId>,pub dependencies:Vec<TaskId>,pub actor:ActorId,pub description:String,pub kind:Kind,pub criterion:Criterion,pub writes:Vec<Resource>,pub required:bool}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {pub schema_version:u32,pub mode:Mode,pub agents:Vec<Agent>,pub tasks:Vec<Task>,pub recipients:Vec<(ActorId,ActorId)>}
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum Status {Pending,Ready,Running,WaitingChildren,Reconciling,Succeeded,Failed,Cancelled}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct Claim {pub task:TaskId,pub actor:ActorId,pub fence:u64,pub epoch:u64,pub expires:u64,pub issued_sequence:u64}
impl Claim {pub fn precondition(&self)->String{format!("rivet-claim:{}:{}:{}",self.task.0,self.epoch,self.fence)}}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct RevisionRef {pub path:PathBuf,pub sha256:String}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckEvidence {pub operation:OperationId,pub artifact:String,pub projection:String,pub revisions:Vec<RevisionRef>}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq,Default)]
#[serde(deny_unknown_fields)]
pub struct Evidence {pub revisions:Vec<RevisionRef>,pub checks:Vec<CheckEvidence>}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum ReviewDecision {Approve,RequestChanges}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct Reviewed {pub task:TaskId,pub evidence:Evidence}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewResult {pub decision:ReviewDecision,pub subjects:Vec<Reviewed>}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {#[serde(default)]pub stale:Vec<PathBuf>,#[serde(default)]pub received:Vec<MessageId>,pub task:Task,pub state:Status,pub revision:u64,pub fence:u64,pub lease:Option<Claim>,pub evidence:Evidence,pub review:Option<ReviewResult>}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Queue {parent:Option<TaskId>,ready:VecDeque<TaskId>}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {pub plan:Plan,pub nodes:Vec<Node>,pub revision:u64,queues:Vec<Queue>,cursor:usize}
fn valid_path(path:&PathBuf)->bool{!path.as_os_str().is_empty()&&path.components().all(|c|matches!(c,Component::Normal(_)))&&path.components().all(|c|c.as_os_str().to_str().is_some_and(|s|!s.chars().any(char::is_control)&&![".git",".build",".cargo",".rivet.toml","credentials","secrets",".ssh",".gnupg",".aws",".netrc",".git-credentials"].contains(&s)&&!s.starts_with(".env")&&!s.starts_with(".rivet-")))}
fn overlap(a:&Resource,b:&Resource)->bool{match(a,b){(Resource::Git,Resource::Git)=>true,(Resource::Path(a),Resource::Path(b))=>a.starts_with(b)||b.starts_with(a),_=>false}}
impl Plan {
    pub fn validate(&self)->Result<()>{
        crate::domain::validate_roles(&self.mode,&self.agents.iter().map(|a|a.role.clone()).collect::<Vec<_>>(),false)?;
        if self.schema_version!=1||self.tasks.is_empty()||self.tasks.len()>256||self.agents.len()>32||self.recipients.len()>1024{return Err(Error::Invalid);}
        let agents:HashSet<_>=self.agents.iter().map(|a|a.id).collect();let ids:HashSet<_>=self.tasks.iter().map(|t|t.id).collect();if agents.len()!=self.agents.len()||ids.len()!=self.tasks.len(){return Err(Error::Invalid);}
        let roots=self.tasks.iter().filter(|t|t.parent.is_none()).collect::<Vec<_>>();if roots.len()!=1||roots[0].kind!=Kind::Coordinate||!roots[0].required{return Err(Error::Invalid);}
        for agent in &self.agents{
            if agent.skills.len()>16||agent.skills.iter().any(|s|s.is_empty()||s.len()>64||s.chars().any(char::is_control))||agent.reports_to==Some(agent.id){return Err(Error::Invalid);}
            let expected=match(&self.mode,&agent.role){(Mode::Solo,Role::Independent)|(Mode::Pair|Mode::Crew,Role::Lead)|(Mode::Fleet,Role::Director)=>None,(Mode::Fleet,Role::Lead)=>Some(Role::Director),(_,Role::Operator|Role::Reviewer)=>Some(Role::Lead),_=>return Err(Error::Invalid)};
            match expected{None if agent.reports_to.is_some()=>return Err(Error::Invalid),Some(role)=>if !self.agents.iter().any(|a|Some(a.id)==agent.reports_to&&a.role==role){return Err(Error::Invalid);},_=>()}
        }
        let root_agent=self.agents.iter().find(|a|a.id==roots[0].actor).ok_or(Error::Invalid)?;if root_agent.reports_to.is_some(){return Err(Error::Invalid);}
        for task in &self.tasks{
            if task.description.is_empty()||task.description.len()>4096||task.dependencies.len()>32||task.writes.len()>32||!agents.contains(&task.actor)||task.dependencies.iter().any(|id|*id==task.id||!ids.contains(id))||task.parent.is_some_and(|p|p==task.id||!ids.contains(&p)){return Err(Error::Invalid);}
            let actor=self.agents.iter().find(|a|a.id==task.actor).ok_or(Error::Invalid)?;
            if let Some(parent)=task.parent{let parent=self.tasks.iter().find(|t|t.id==parent).ok_or(Error::Invalid)?;if task.actor!=parent.actor&&actor.reports_to!=Some(parent.actor){return Err(Error::Denied);}}
            match &task.kind{
                Kind::Coordinate if ![Role::Independent,Role::Lead,Role::Director].contains(&actor.role)=>return Err(Error::Invalid),
                Kind::Coding if ![Role::Independent,Role::Operator].contains(&actor.role)=>return Err(Error::Invalid),
                Kind::Review{subjects}=>{if actor.role!=Role::Reviewer||subjects.is_empty()||subjects.len()>32||!task.writes.is_empty(){return Err(Error::Invalid);}for subject in subjects{let code=self.tasks.iter().find(|t|t.id==*subject).ok_or(Error::Invalid)?;if code.kind!=Kind::Coding||!matches!(code.criterion,Criterion::Checked{..})||!task.dependencies.contains(subject)||code.parent!=task.parent{return Err(Error::Invalid);}}},
                _=>()
            }
            if task.kind!=Kind::Coding&&!task.writes.is_empty(){return Err(Error::Denied);}
            if task.writes.iter().any(|r|matches!(r,Resource::Path(p) if !valid_path(p))){return Err(Error::Denied);}
            if let Criterion::Checked{paths}=&task.criterion{if paths.is_empty()||paths.len()>32||paths.iter().any(|p|!valid_path(p)){return Err(Error::Invalid);}}
            if task.kind==Kind::Coding&&task.required&&matches!(self.mode,Mode::Crew|Mode::Fleet)&&!self.tasks.iter().any(|r|r.required&&matches!(&r.kind,Kind::Review{subjects} if subjects.contains(&task.id))){return Err(Error::Invalid);}
        }
        // Parent and dependency graphs are each acyclic; parent completion is a separate barrier.
        for task in &self.tasks{
            fn walk(plan:&Plan,id:TaskId,path:&mut HashSet<TaskId>,done:&mut HashSet<TaskId>,parents:bool)->Result<()>{
                if done.contains(&id){return Ok(());}if !path.insert(id){return Err(Error::Conflict);}let t=plan.tasks.iter().find(|t|t.id==id).ok_or(Error::Invalid)?;
                let edges=if parents{t.parent.into_iter().collect()}else{t.dependencies.iter().copied().chain(plan.tasks.iter().filter(|child|child.required&&child.parent==Some(id)).map(|child|child.id)).collect::<Vec<_>>()};
                for edge in edges{walk(plan,edge,path,done,parents)?;}path.remove(&id);done.insert(id);Ok(())
            }
            walk(self,task.id,&mut HashSet::new(),&mut HashSet::new(),true)?;walk(self,task.id,&mut HashSet::new(),&mut HashSet::new(),false)?;
        }
        for (a,b) in &self.recipients{if a==b||!agents.contains(a)||!agents.contains(b){return Err(Error::Invalid);}}
        if serde_json::to_vec(self).map_err(|_|Error::Invalid)?.len()>65536{return Err(Error::Unsupported);}Ok(())
    }
    pub fn instantiate(&self)->Result<Self>{
        self.validate()?;let actors=self.agents.iter().map(|a|(a.id,ActorId::default())).collect::<Vec<_>>();let tasks=self.tasks.iter().map(|t|(t.id,TaskId::default())).collect::<Vec<_>>();let actor=|id|actors.iter().find(|(old,_)|*old==id).map(|(_,id)|*id).ok_or(Error::Invalid);let task=|id|tasks.iter().find(|(old,_)|*old==id).map(|(_,id)|*id).ok_or(Error::Invalid);
        let mut plan=self.clone();for a in &mut plan.agents{a.id=actor(a.id)?;a.reports_to=a.reports_to.map(actor).transpose()?;}for t in &mut plan.tasks{t.id=task(t.id)?;t.actor=actor(t.actor)?;t.parent=t.parent.map(task).transpose()?;t.dependencies=t.dependencies.iter().map(|id|task(*id)).collect::<Result<Vec<_>>>()?;if let Kind::Review{subjects}=&mut t.kind{*subjects=subjects.iter().map(|id|task(*id)).collect::<Result<Vec<_>>>()?;}}
        plan.recipients=plan.recipients.iter().map(|(a,b)|Ok((actor(*a)?,actor(*b)?))).collect::<Result<Vec<_>>>()?;Ok(plan)
    }
    pub fn root(&self)->&Task{self.tasks.iter().find(|t|t.parent.is_none()).expect("validated single root")}
}
impl Graph {
    pub fn new(plan:Plan)->Result<Self>{plan.validate()?;let nodes=plan.tasks.iter().cloned().map(|task|Node{stale:vec![],received:vec![],task,state:Status::Pending,revision:0,fence:0,lease:None,evidence:Evidence::default(),review:None}).collect();let mut graph=Self{plan,nodes,revision:0,queues:vec![],cursor:0};graph.refresh();Ok(graph)}
    pub fn node(&self,id:TaskId)->Result<&Node>{self.nodes.iter().find(|n|n.task.id==id).ok_or(Error::Invalid)}
    fn index(&self,id:TaskId)->Result<usize>{self.nodes.iter().position(|n|n.task.id==id).ok_or(Error::Invalid)}
    fn enqueue(&mut self,id:TaskId){let parent=self.nodes.iter().find(|n|n.task.id==id).expect("known task").task.parent;if let Some(q)=self.queues.iter_mut().find(|q|q.parent==parent){if !q.ready.contains(&id){q.ready.push_back(id);}}else{self.queues.push(Queue{parent,ready:VecDeque::from([id])});}}
    fn children_done(&self,id:TaskId)->bool{self.nodes.iter().filter(|n|n.task.parent==Some(id)&&n.task.required).all(|n|n.state==Status::Succeeded&&n.stale.is_empty())}
    pub fn refresh(&mut self){let ready=self.nodes.iter().filter(|n|(n.state==Status::Pending&&n.task.dependencies.iter().all(|d|self.node(*d).is_ok_and(|d|d.state==Status::Succeeded)))||(n.state==Status::WaitingChildren&&self.children_done(n.task.id))).map(|n|n.task.id).collect::<Vec<_>>();for id in ready{let i=self.index(id).expect("known");self.nodes[i].state=Status::Ready;self.enqueue(id);}}
    pub fn next(&self,now:u64)->Option<TaskId>{
        for offset in 0..self.queues.len(){let queue=&self.queues[(self.cursor+offset)%self.queues.len()];let Some(id)=queue.ready.front()else{continue;};let node=self.node(*id).ok()?;
            if node.state!=Status::Ready||node.lease.as_ref().is_some_and(|l|l.expires<=now){continue;}
            if self.nodes.iter().filter(|other|other.task.id!=*id&&other.lease.is_some()).any(|other|node.task.writes.iter().any(|a|other.task.writes.iter().any(|b|overlap(a,b)))){continue;}
            return Some(*id);
        }None
    }
    pub fn claim(&mut self,id:TaskId,epoch:u64,now:u64,expires:u64,sequence:u64)->Result<Claim>{
        if expires<=now||expires-now>3600||self.next(now)!=Some(id){return Err(Error::Conflict);}let i=self.index(id)?;
        let claim=if let Some(claim)=&self.nodes[i].lease{if claim.epoch!=epoch{return Err(Error::Denied);}claim.clone()}else{self.nodes[i].fence=self.nodes[i].fence.checked_add(1).ok_or(Error::State)?;Claim{task:id,actor:self.nodes[i].task.actor,fence:self.nodes[i].fence,epoch,expires,issued_sequence:sequence}};
        let queue=self.queues.iter().position(|q|q.ready.front()==Some(&id)).ok_or(Error::Conflict)?;self.queues[queue].ready.pop_front();self.cursor=(queue+1)%self.queues.len();self.nodes[i].state=Status::Running;self.nodes[i].lease=Some(claim.clone());self.nodes[i].revision+=1;Ok(claim)
    }
    pub fn validate_claim(&self,claim:&Claim,now:u64)->Result<()>{let node=self.node(claim.task)?;if node.state!=Status::Running||node.lease.as_ref()!=Some(claim)||claim.expires<=now{return Err(Error::Denied);}Ok(())}
    pub fn yield_step(&mut self,claim:&Claim,now:u64)->Result<()>{self.validate_claim(claim,now)?;let i=self.index(claim.task)?;self.nodes[i].state=Status::Ready;self.nodes[i].lease=None;self.nodes[i].revision+=1;self.enqueue(claim.task);Ok(())}
    pub fn expire(&mut self,now:u64){for node in &mut self.nodes{if node.lease.as_ref().is_some_and(|l|l.expires<=now){node.state=Status::Reconciling;node.revision+=1;}}for q in &mut self.queues{q.ready.retain(|id|self.nodes.iter().any(|n|n.task.id==*id&&n.state==Status::Ready));}}
    pub fn release(&mut self,claim:&Claim)->Result<()>{let i=self.index(claim.task)?;if self.nodes[i].lease.as_ref()!=Some(claim){return Err(Error::Conflict);}self.nodes[i].lease=None;if matches!(self.nodes[i].state,Status::Running|Status::Ready){self.nodes[i].state=Status::Ready;self.enqueue(claim.task);}self.nodes[i].revision+=1;Ok(())}
    pub fn reconcile(&mut self,id:TaskId)->Result<()>{let i=self.index(id)?;if self.nodes[i].lease.is_none()||![Status::Reconciling,Status::Running,Status::Ready].contains(&self.nodes[i].state){return Err(Error::Conflict);}self.nodes[i].lease=None;self.nodes[i].state=Status::Ready;self.nodes[i].revision+=1;self.enqueue(id);Ok(())}
    pub fn evidence_valid(&self,id:TaskId)->Result<bool>{let node=self.node(id)?;if !node.stale.is_empty(){return Ok(false);}Ok(match &node.task.criterion{Criterion::Text=>true,Criterion::Checked{paths}=>paths.iter().all(|path|node.evidence.revisions.iter().find(|r|&r.path==path).is_some_and(|revision|node.evidence.checks.iter().any(|check|check.revisions.contains(revision))))})}
    pub fn finish(&mut self,claim:&Claim,now:u64,success:bool,review:Option<ReviewResult>)->Result<()>{
        self.validate_claim(claim,now)?;let i=self.index(claim.task)?;
        if success&&!self.evidence_valid(claim.task)?{return Err(Error::Denied);}
        if let Kind::Review{subjects}=&self.nodes[i].task.kind{
            let value=review.as_ref().ok_or(Error::Invalid)?;if subjects.iter().any(|id|self.node(*id).is_ok_and(|n|!n.stale.is_empty())){return Err(Error::Denied);}let expected=subjects.iter().map(|id|self.node(*id).map(|n|Reviewed{task:*id,evidence:n.evidence.clone()})).collect::<Result<Vec<_>>>()?;
            if value.subjects!=expected||value.subjects.iter().any(|s|s.evidence.checks.is_empty()){return Err(Error::Denied);}if success&&value.decision!=ReviewDecision::Approve{return Err(Error::Denied);}
        }else if review.is_some(){return Err(Error::Invalid);}
        let state=if !success{Status::Failed}else if !self.children_done(claim.task){Status::WaitingChildren}else{Status::Succeeded};
        self.nodes[i].state=state;self.nodes[i].review=review;self.nodes[i].lease=None;self.nodes[i].revision+=1;self.refresh();Ok(())
    }
    pub fn mutation_allowed(&self,claim:&Claim,resources:&[Resource],now:u64)->Result<()>{self.validate_claim(claim,now)?;let node=self.node(claim.task)?;if resources.iter().all(|r|node.task.writes.iter().any(|allowed|match(allowed,r){(Resource::Git,Resource::Git)=>true,(Resource::Path(base),Resource::Path(path))=>path.starts_with(base),_=>false})){Ok(())}else{Err(Error::Denied)}}
    pub fn done(&self)->bool{self.node(self.plan.root().id).is_ok_and(|n|n.state==Status::Succeeded)&&self.nodes.iter().filter(|n|n.task.required).all(|n|n.state==Status::Succeeded&&n.stale.is_empty())}
}

pub fn template(mode:Mode,task:&str,paths:Vec<PathBuf>)->Result<Plan>{
    if task.is_empty()||task.len()>4096{return Err(Error::Invalid);}let root_actor=ActorId::default();let lead=if mode==Mode::Fleet{ActorId::default()}else{root_actor};let operator=if mode==Mode::Solo{root_actor}else{ActorId::default()};let reviewer=ActorId::default();
    let mut agents=vec![Agent{id:root_actor,role:if mode==Mode::Solo{Role::Independent}else if mode==Mode::Fleet{Role::Director}else{Role::Lead},reports_to:None,skills:vec![]}];
    if mode==Mode::Fleet{agents.push(Agent{id:lead,role:Role::Lead,reports_to:Some(root_actor),skills:vec![]});}
    if mode!=Mode::Solo{agents.push(Agent{id:operator,role:Role::Operator,reports_to:Some(lead),skills:vec![]});}
    if matches!(mode,Mode::Crew|Mode::Fleet){if paths.is_empty(){return Err(Error::Invalid);}agents.push(Agent{id:reviewer,role:Role::Reviewer,reports_to:Some(lead),skills:vec![]});}
    let root=TaskId::default();let group=if mode==Mode::Fleet{TaskId::default()}else{root};let code=TaskId::default();
    let mut tasks=vec![Task{id:root,parent:None,dependencies:vec![],actor:root_actor,description:task.into(),kind:Kind::Coordinate,criterion:Criterion::Text,writes:vec![],required:true}];
    if mode==Mode::Fleet{tasks.push(Task{id:group,parent:Some(root),dependencies:vec![],actor:lead,description:format!("Coordinate this explicitly scoped task and integrate the required operator and technical review: {task}"),kind:Kind::Coordinate,criterion:Criterion::Text,writes:vec![],required:true});}
    tasks.push(Task{id:code,parent:Some(group),dependencies:vec![],actor:operator,description:format!("Perform this explicitly scoped task, preserving declared paths and supplying exact tested revision evidence: {task}"),kind:Kind::Coding,criterion:if paths.is_empty(){Criterion::Text}else{Criterion::Checked{paths:paths.clone()}},writes:paths.into_iter().map(Resource::Path).collect(),required:true});
    if matches!(mode,Mode::Crew|Mode::Fleet){tasks.push(Task{id:TaskId::default(),parent:Some(group),dependencies:vec![code],actor:reviewer,description:"Independently evaluate the required coding task's exact recorded revisions and checks. Read relevant source through reviewed tools. Return only technical_review_format JSON; request_changes if insufficient. This grants no user tool permission.".into(),kind:Kind::Review{subjects:vec![code]},criterion:Criterion::Text,writes:vec![],required:true});}
    let recipients=agents.iter().filter_map(|a|a.reports_to.map(|p|[(a.id,p),(p,a.id)])).flatten().collect();let plan=Plan{schema_version:1,mode,agents,tasks,recipients};plan.validate()?;Ok(plan)
}
