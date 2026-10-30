//! Private local UI channel, owned by the same supervisor. Never mounted into a worker.
use super::*;
use crate::chat::{Review,Reviewer};
use crate::lifecycle::Snapshot;
use serde::{Serialize,Deserialize};
use std::{sync::{Arc,Mutex,atomic::{AtomicU8,Ordering}},future::Future,pin::Pin};
use tokio::{net::{UnixListener,UnixStream},io::{AsyncReadExt,AsyncWriteExt},sync::oneshot};
const LIMIT:usize=1024*1024;
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {version:u32,run:RunId,epoch:u64,nonce:uuid::Uuid}
#[derive(Clone,Serialize,Deserialize)]
#[serde(tag="action",rename_all="snake_case",deny_unknown_fields)]
pub enum Action {Status,Pause,Stop,Disconnect,Approve{challenge:String},Forward{recipient:ActorId,scope:String,text:String,epoch:u64,approved:String}}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {run:RunId,nonce:uuid::Uuid,command:Action}
struct PendingReview {challenge:String,review:Review,reply:oneshot::Sender<bool>}
#[derive(Clone)]
pub struct Handle {pub cancel:CancellationToken,reason:Arc<AtomicU8>,pending:Arc<Mutex<Option<PendingReview>>>,pub snapshot:Arc<Mutex<Snapshot>>,forward:Arc<Mutex<std::collections::VecDeque<Forward>>>,graph:Arc<Mutex<Option<crate::scheduler::Graph>>>}
impl Handle {
    pub fn update_graph(&self,graph:crate::scheduler::Graph)->Result<()>{*self.graph.lock().map_err(|_|Error::State)?=Some(graph);Ok(())}
    pub fn take_forward(&self)->Result<Option<Forward>>{Ok(self.forward.lock().map_err(|_|Error::State)?.pop_front())}
    pub fn reason(&self)->u8{self.reason.load(Ordering::SeqCst)}
    pub fn request(&self,reason:u8){self.reason.fetch_max(reason,Ordering::SeqCst);self.cancel.cancel();}
    pub fn update(&self,value:Snapshot)->Result<()>{*self.snapshot.lock().map_err(|_|Error::State)?=value;Ok(())}
}
pub struct Plane {pub handle:Handle,worker:tokio::task::JoinHandle<()>,dir:Dir,endpoint:Endpoint,inode:u64}
impl Plane {
    pub fn bind(ledger:&Ledger,snapshot:Snapshot)->Result<Self>{
        let dir=ledger.control_directory()?;
        for name in ["control.sock","control.json"]{
            if let Ok(m)=dir.symlink_metadata(name){
                use cap_std::fs::FileTypeExt;
                let valid=if name.ends_with("sock"){m.file_type().is_socket()}else{m.is_file()&&m.nlink()==1};
                if !valid||m.uid()!=dir.dir_metadata()?.uid()||m.mode()&0o777!=0o600{return Err(Error::State);}dir.remove_file(name)?;
            }
        }
        let socket=format!("/proc/self/fd/{}/control.sock",dir.as_raw_fd());let listener=UnixListener::bind(&socket)?;dir.set_permissions("control.sock",Permissions::from_mode(0o600))?;
        let inode=dir.symlink_metadata("control.sock")?.ino();
        let endpoint=Endpoint{version:1,run:snapshot.spec.run,epoch:ledger.epoch(),nonce:uuid::Uuid::new_v4()};storage::json(&dir,"control.json",&endpoint,false)?;
        let handle=Handle{cancel:CancellationToken::new(),reason:Arc::new(AtomicU8::new(0)),pending:Arc::new(Mutex::new(None)),snapshot:Arc::new(Mutex::new(snapshot)),forward:Arc::new(Mutex::new(std::collections::VecDeque::new())),graph:Arc::new(Mutex::new(None))};
        let state=handle.clone();let expected=endpoint.clone();
        let worker=tokio::spawn(async move{
            // Sequential, bounded requests; a slow client cannot occupy the channel beyond 100 ms.
            loop{
                let accepted=listener.accept().await;let Ok((mut stream,_))=accepted else{break;};
                let response=tokio::time::timeout(Duration::from_millis(100),read_frame(&mut stream)).await;
                let reply=match response{
                    Ok(Ok(bytes))=>match serde_json::from_slice::<Wire>(&bytes){
                        Ok(wire) if wire.run==expected.run&&wire.nonce==expected.nonce=>respond(&state,wire.command),
                        _=>Err(Error::Denied)
                    },_=>Err(Error::Invalid)
                };
                let value=reply.unwrap_or_else(|e|serde_json::json!({"error":e.kind()}));
                if let Ok(bytes)=serde_json::to_vec(&value){let _=tokio::time::timeout(Duration::from_millis(100),write_frame(&mut stream,&bytes)).await;}
            }
        });Ok(Self{handle,worker,dir,endpoint,inode})
    }
    pub async fn close(mut self){self.worker.abort();let _=(&mut self.worker).await;}
}
impl Drop for Plane {
    fn drop(&mut self){
        self.worker.abort();
        if self.dir.symlink_metadata("control.sock").is_ok_and(|m|m.ino()==self.inode){let _=self.dir.remove_file("control.sock");}
        if storage::read_json::<Endpoint>(&self.dir,"control.json").is_ok_and(|e|e.nonce==self.endpoint.nonce){let _=self.dir.remove_file("control.json");}
        let _=storage::sync(&self.dir);
    }
}
fn respond(handle:&Handle,command:Action)->Result<serde_json::Value>{
    match command{
        Action::Forward{recipient,scope,text,epoch,approved}=>{
            let snapshot=handle.snapshot.lock().map_err(|_|Error::State)?;
            let expected=forward_digest(snapshot.spec.run,recipient,&scope,&text,epoch)?;
            if !handle.graph.lock().map_err(|_|Error::State)?.as_ref().is_some_and(|g|g.nodes.iter().any(|n|n.task.actor==recipient&&!matches!(n.state,crate::scheduler::Status::Succeeded|crate::scheduler::Status::Failed|crate::scheduler::Status::Cancelled)))||snapshot.coordination_digest.is_none()||handle.cancel.is_cancelled()||snapshot.spec.permission_epoch!=epoch||approved!=expected{return Err(Error::Denied);}let mut queue=handle.forward.lock().map_err(|_|Error::State)?;if queue.len()>=8{return Err(Error::Busy);}queue.push_back(Forward{recipient,scope,text,epoch,id:MessageId(uuid::Uuid::parse_str(&expected[..32]).map_err(|_|Error::State)?)});Ok(serde_json::json!({"accepted":true,"durable":false,"delivery":"queued","decision_scope_digest":expected}))
        },
        Action::Status=>{
            let snapshot=handle.snapshot.lock().map_err(|_|Error::State)?.clone();let review=handle.pending.lock().map_err(|_|Error::State)?.as_ref().map(|p|serde_json::json!({"challenge":p.challenge,"review":p.review}));
            Ok(serde_json::json!({"snapshot":snapshot,"review":review,"cancellation_requested":handle.cancel.is_cancelled(),"control_reason":handle.reason(),"coordination":handle.graph.lock().map_err(|_|Error::State)?.clone(),"queued_forwarding":handle.forward.lock().map_err(|_|Error::State)?.len()}))
        },
        Action::Disconnect if handle.snapshot.lock().map_err(|_|Error::State)?.detached=>Ok(serde_json::json!({"detached":true,"paused":false,"new_permissions":false})),
        Action::Pause|Action::Stop|Action::Disconnect=>{
            let reason=match command{Action::Stop=>4,Action::Disconnect=>2,_=>1};handle.request(reason);
            Ok(serde_json::json!({"accepted":true,"durable":false,"cleanup":"pending","reason":reason}))
        },
        Action::Approve{challenge}=>{
            if handle.cancel.is_cancelled(){return Err(Error::Denied);}
            let mut pending=handle.pending.lock().map_err(|_|Error::State)?;
            if !pending.as_ref().is_some_and(|p|p.challenge==challenge){return Err(Error::Denied);}
            let value=pending.take().ok_or(Error::Denied)?;value.reply.send(true).map_err(|_|Error::Denied)?;
            Ok(serde_json::json!({"approved":true,"scope_digest":challenge}))
        }
    }
}
impl Reviewer for Handle {
    fn approve(&mut self,review:Review)->Pin<Box<dyn Future<Output=Result<bool>>+Send+'_>>{
        Box::pin(async move{
            if self.cancel.is_cancelled(){return Err(Error::Interrupted);}
            let nonce=uuid::Uuid::new_v4();let challenge=crate::tools::sha256(&serde_json::to_vec(&(nonce,&review)).map_err(|_|Error::Invalid)?);let (tx,rx)=oneshot::channel();
            {let mut pending=self.pending.lock().map_err(|_|Error::State)?;if pending.is_some(){return Err(Error::Busy);}*pending=Some(PendingReview{challenge,review,reply:tx});}
            let result=tokio::select!{biased;_=self.cancel.cancelled()=>Err(Error::Interrupted),value=rx=>value.map_err(|_|Error::Interrupted)};
            *self.pending.lock().map_err(|_|Error::State)?=None;result
        })
    }
}
async fn read_frame(stream:&mut UnixStream)->Result<Vec<u8>>{let size=stream.read_u32().await? as usize;if size==0||size>LIMIT{return Err(Error::Invalid);}let mut data=vec![0;size];stream.read_exact(&mut data).await?;Ok(data)}
async fn write_frame(stream:&mut UnixStream,bytes:&[u8])->Result<()>{if bytes.len()>LIMIT{return Err(Error::Unsupported);}stream.write_u32(bytes.len() as u32).await?;stream.write_all(bytes).await?;Ok(())}
fn directory(project:&Project,path:&Path)->Result<Dir>{storage::directory(&project.dir,&project.relative(path)?,false)}
impl Ledger {
    fn control_directory(&self)->Result<Dir>{let scope=self.owner_scope.as_ref().ok_or(Error::Denied)?;verify_owner(scope)?;let base=if std::env::current_exe()?.starts_with(env!("CARGO_MANIFEST_DIR")){env!("CARGO_MANIFEST_DIR")}else{env!("RIVET_SOURCE_ROOT")};let project=Project::open(Path::new(base))?;let dir=directory(&project,&scope.state)?;let m=dir.dir_metadata()?;if (m.dev(),m.ino())!=(scope.device,scope.inode){return Err(Error::Denied);}Ok(dir)}
}
pub async fn request(project:&Project,path:&Path,run:RunId,command:Action)->Result<serde_json::Value>{
    let dir=directory(project,path)?;dir_admission(&dir)?;let endpoint:Endpoint=storage::read_json(&dir,"control.json")?;if endpoint.version!=1||endpoint.run!=run{return Err(Error::Denied);}
    let metadata=dir.symlink_metadata("control.sock")?;use cap_std::fs::FileTypeExt;
    if !metadata.file_type().is_socket()||metadata.uid()!=dir.dir_metadata()?.uid()||metadata.mode()&0o777!=0o600{return Err(Error::State);}
    tokio::time::timeout(Duration::from_secs(2),async{
        let mut stream=UnixStream::connect(format!("/proc/self/fd/{}/control.sock",dir.as_raw_fd())).await?;
        write_frame(&mut stream,&serde_json::to_vec(&Wire{run,nonce:endpoint.nonce,command}).map_err(|_|Error::Invalid)?).await?;
        serde_json::from_slice(&read_frame(&mut stream).await?).map_err(|_|Error::State)
    }).await.map_err(|_|Error::Busy)?
}
/// Explicitly requested detached core: same binary/supervisor, a bounded temporary owned unit.
pub async fn detach(project:&Project,path:&Path,arguments:&[String])->Result<serde_json::Value>{
    if arguments.iter().any(|s|s=="--ephemeral-key-stdin"){return Err(Error::Unsupported);}
    Ledger::open(project,path)?.shutdown()?;
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::State)?;
    let db=immutable(&root)?;let epoch:u64=db.query_row("SELECT owner_epoch FROM meta",[],|r|r.get::<_,i64>(0))?.try_into().map_err(|_|Error::State)?;drop(db);
    let operation=OperationId::default();let nonce=uuid::Uuid::new_v4();let unit=format!("rivet-worker-{}-{}.service",operation.0.simple(),nonce.simple());let description=format!("Rivet operation {} owner {}",operation.0,nonce);
    storage::worker(&root.dir,&storage::Worker{operation,unit:unit.clone(),description:description.clone(),epoch})?;
    ensure_space(&root.dir,65536)?;
    let name=format!("core-{}.log",uuid::Uuid::new_v4());storage::write_new(&root.dir,&name,b"")?;let log=project.relative(path)?.join(&name);
    let dir=root.dir.try_clone()?;drop(root);
    let mut command=tokio::process::Command::new("/usr/bin/systemd-run");command.env_clear().env("PATH","/usr/bin").env("SYSTEMD_BUS_TIMEOUT","2s");
    let mut core_environment=vec!["PATH=/usr/bin".to_string()];
    for key in ["XDG_RUNTIME_DIR","DBUS_SESSION_BUS_ADDRESS"]{if let Some(value)=std::env::var_os(key){command.env(key,&value);core_environment.push(format!("{key}={}",value.to_str().ok_or(Error::Invalid)?));}}
    command.args(["--user","--quiet","--collect","--service-type=exec"]).arg(format!("--unit={unit}")).arg(format!("--description={description}")).arg(format!("--working-directory={}",project.path().display())).args(["--property=CPUAccounting=yes","--property=CPUQuota=100%","--property=MemoryAccounting=yes","--property=MemoryMax=512M","--property=MemorySwapMax=0","--property=TasksAccounting=yes","--property=TasksMax=64","--property=RuntimeMaxSec=3610s","--property=KillMode=control-group","--property=TimeoutStopSec=5s","--property=NoNewPrivileges=yes","--property=OOMPolicy=kill"]).arg(format!("--property=StandardOutput=file:{}",project.path().join(&log).display())).arg("--property=StandardError=inherit").args(["/usr/bin/env","-i"]).args(core_environment).arg(std::env::current_exe()?).args(arguments).arg("--detached-core");
    // No credentials can be conveyed to systemd's environment/argv or a persisted input file.
    if arguments.iter().any(|s|s=="--ephemeral-key-stdin"){return Err(Error::Unsupported);}
    let launched=tokio::time::timeout(Duration::from_secs(5),command.output()).await;
    if !matches!(launched,Ok(Ok(ref result)) if result.status.success()){crate::tools::process::reconcile_owned(operation,&unit,&description).await?;return Err(Error::Unsupported);}
    let started=tokio::time::timeout(Duration::from_secs(5),async{
        loop{if let Ok(endpoint)=storage::read_json::<Endpoint>(&dir,"control.json"){if endpoint.epoch>epoch{return Ok::<_,Error>(endpoint.run);}}tokio::time::sleep(Duration::from_millis(20)).await;}
    }).await;
    match started{Ok(Ok(run))=>Ok(serde_json::json!({"run":run,"unit":unit,"log":log,"detached":true,"reviews":"run status / run approve","deadline_ceiling_seconds":3600})),_=>{crate::tools::process::reconcile_owned(operation,&unit,&description).await?;Err(Error::State)}}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::{Command,Checkpoint,State};
    fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
    async fn admitted(ledger:&Ledger,detached:bool)->Snapshot{
        let session=SessionId::default();let run=RunId::default();let spec=RunSpec{schema_version:1,project:ledger.project_id(),session,run,task:"synthetic controls".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+120,step_budget:100,permission_epoch:ledger.epoch()};
        ledger.transition(0,Transition::NewSession{id:session}).await.unwrap();ledger.transition(0,Transition::RootTask{session,run,task:TaskId::default(),actor:ActorId::default()}).await.unwrap();ledger.life(0,Command::Admit{spec,wall:now(),detached}).await.unwrap();ledger.lifecycle(run).await.unwrap()
    }
    #[tokio::test]
    async fn hundred_seeded_control_interleavings_bind_approvals_and_acknowledge_cancellation_within_target(){
        let temp=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let project=Project::open(temp.path()).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();let mut maximum=Duration::ZERO;
        for seed in 0..10{for iteration in 0..10{
            let detached=(seed+iteration)%2==0;let initial=admitted(&ledger,detached).await;let run=initial.spec.run;let plane=Plane::bind(&ledger,initial.clone()).unwrap();
            let mut reviewer=plane.handle.clone();let approval=tokio::spawn(async move{reviewer.approve(Review{kind:"synthetic-only",scope:serde_json::json!({"seed":seed,"iteration":iteration})}).await});
            let challenge=tokio::time::timeout(Duration::from_secs(1),async{loop{let reply=request(&project,Path::new("state"),run,Action::Status).await.unwrap();if let Some(value)=reply["review"]["challenge"].as_str(){break value.to_string();}tokio::task::yield_now().await;}}).await.unwrap();
            let wrong=request(&project,Path::new("state"),run,Action::Approve{challenge:"wrong".into()}).await.unwrap();assert_eq!(wrong["error"],"denied");assert!(!approval.is_finished());
            assert_eq!(request(&project,Path::new("state"),run,Action::Approve{challenge:challenge.clone()}).await.unwrap()["approved"],true);assert!(approval.await.unwrap().unwrap());
            assert_eq!(request(&project,Path::new("state"),run,Action::Approve{challenge}).await.unwrap()["error"],"denied");
            if detached{assert_eq!(request(&project,Path::new("state"),run,Action::Disconnect).await.unwrap()["detached"],true);assert!(!plane.handle.cancel.is_cancelled());}
            let start=std::time::Instant::now();let reply=request(&project,Path::new("state"),run,if iteration%3==0{Action::Stop}else if detached{Action::Pause}else{Action::Disconnect}).await.unwrap();let elapsed=start.elapsed();maximum=maximum.max(elapsed);assert!(elapsed<Duration::from_millis(250));assert_eq!(reply["accepted"],true);assert_eq!(reply["durable"],false);assert!(plane.handle.cancel.is_cancelled());
            let current=ledger.lifecycle(run).await.unwrap();ledger.life(current.revision,Command::Finish{run,state:State::Paused,wall:now(),remaining_ms:current.remaining_ms,checkpoint:Checkpoint{boundary:crate::lifecycle::Boundary::Drained,evidence:vec![]}}).await.unwrap();plane.close().await;
        }}ledger.shutdown().unwrap();println!("control_interleavings=100 seeds=10 max_ack_us={}",maximum.as_micros());
    }
    #[tokio::test]
    async fn saturated_ordinary_ledger_queue_cannot_delay_control_ack(){
        let temp=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let project=Project::open(temp.path()).unwrap();let ledger=Arc::new(Ledger::open(&project,Path::new("state")).unwrap());let initial=admitted(&ledger,false).await;let run=initial.spec.run;let plane=Plane::bind(&ledger,initial.clone()).unwrap();
        let entered=Arc::new(std::sync::Barrier::new(2));let release=Arc::new(std::sync::Barrier::new(2));let caller=ledger.clone();let enter=entered.clone();let exit=release.clone();
        let queued=tokio::spawn(async move{caller.transition(0,Transition::TestBarrier{entered:enter,release:exit}).await});
        tokio::task::spawn_blocking(move||entered.wait()).await.unwrap();
        let mut replies=vec![];for _ in 0..QUEUE{let (reply,rx)=oneshot::channel();ledger.tx.try_send(Pending{command:LedgerCommand{schema_version:1,owner_epoch:ledger.epoch(),expected_revision:0,transition:Transition::NewSession{id:SessionId::default()}},reply}).unwrap();replies.push(rx);}
        assert_eq!(ledger.tx.capacity(),0);assert!(!queued.is_finished());let started=std::time::Instant::now();let response=request(&project,Path::new("state"),run,Action::Pause).await.unwrap();assert!(started.elapsed()<Duration::from_millis(250));assert_eq!(response["accepted"],true);assert!(plane.handle.cancel.is_cancelled());
        tokio::task::spawn_blocking(move||release.wait()).await.unwrap();queued.await.unwrap().unwrap();for reply in replies{reply.await.unwrap().unwrap();}
        plane.close().await;Arc::try_unwrap(ledger).ok().unwrap().shutdown().unwrap();
    }
}

pub fn verify_detached()->Result<()>{
    let membership=std::fs::read_to_string("/proc/self/cgroup")?;let relative=membership.lines().find_map(|l|l.strip_prefix("0::")).ok_or(Error::Unsupported)?;let path=Path::new(relative);
    if path.components().any(|c|!matches!(c,std::path::Component::RootDir|std::path::Component::Normal(_))){return Err(Error::Denied);}
    let unit=path.file_name().and_then(|p|p.to_str()).ok_or(Error::Denied)?;let id=unit.strip_prefix("rivet-worker-").and_then(|s|s.strip_suffix(".service")).and_then(|s|s.split_once('-')).ok_or(Error::Denied)?;
    uuid::Uuid::parse_str(id.0).map_err(|_|Error::Denied)?;uuid::Uuid::parse_str(id.1).map_err(|_|Error::Denied)?;
    let directory=Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    for(file,expected)in[("cpu.max","100000 100000"),("memory.max","536870912"),("memory.swap.max","0"),("pids.max","64")]{if std::fs::read_to_string(directory.join(file))?.trim()!=expected{return Err(Error::Unsupported);}}
    Ok(())
}

#[derive(Clone)]
pub struct Forward {pub recipient:ActorId,pub scope:String,pub text:String,pub epoch:u64,pub id:MessageId}
pub fn forward_digest(run:RunId,recipient:ActorId,scope:&str,text:&str,epoch:u64)->Result<String>{if scope.is_empty()||scope.len()>256||text.is_empty()||text.len()>4096{return Err(Error::Invalid);}Ok(crate::tools::sha256(&serde_json::to_vec(&(run,recipient,scope,text,epoch)).map_err(|_|Error::Invalid)?))}
