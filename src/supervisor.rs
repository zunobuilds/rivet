use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use crate::{domain::RunSpec, error::{Error, Result}, providers::{InferRequest, ProviderEvent, ProviderPort}};

/// Only controller owns task lifetime; separate control does not enter the ordinary queue.
pub struct Supervisor { stop: CancellationToken, tasks: TaskTracker, registry: crate::tools::Registry, git_lane:tokio::sync::Mutex<()> }
impl Default for Supervisor {
    fn default() -> Self {
        let mut registry = crate::tools::Registry::default();
        registry.register(crate::tools::files::FileWorkspace::descriptor()).expect("built-in descriptor is valid");
        registry.register(crate::tools::process::ProcessRequest::descriptor()).expect("built-in descriptor is valid");
        registry.register(crate::tools::git::GitRequest::descriptor()).expect("built-in descriptor is valid");
        Self { stop: CancellationToken::new(), tasks: TaskTracker::new(), registry,git_lane:tokio::sync::Mutex::new(()) }
    }
}
impl Supervisor {
    pub fn cancellation(&self) -> CancellationToken { self.stop.clone() }
    pub fn admit(&self, spec: &RunSpec) -> Result<()> { spec.validate() }
    pub fn drive<P: ProviderPort + 'static>(&self, spec: &RunSpec, provider: P, request: InferRequest, ordinary: mpsc::Sender<ProviderEvent>) -> Result<()> {
        self.admit(spec)?;
        self.check_cancelled()?;
        let stop = self.stop.child_token();
        self.tasks.spawn(async move {
            let mut events = provider.infer(request, stop.clone());
            loop {
                let event = tokio::select! { biased; _ = stop.cancelled() => break, event = events.recv() => match event { Some(e) => e, None => break } };
                tokio::select! { biased; _ = stop.cancelled() => break, result = ordinary.send(event) => if result.is_err() { break; } }
            }
        });
        Ok(())
    }
    pub async fn shutdown(&self) { self.stop.cancel(); self.tasks.close(); self.tasks.wait().await; }
    pub fn check_cancelled(&self) -> Result<()> { if self.stop.is_cancelled() { Err(Error::Interrupted) } else { Ok(()) } }
}
impl Drop for Supervisor { fn drop(&mut self) { self.stop.cancel(); self.tasks.close(); } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::*, ledger::{Ledger, Transition}, permissions::{Broker, BrokerDecision}, tools::*, config::Project};
    use std::{path::Path, sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};

    fn proposal() -> ToolProposal { ToolProposal { schema_version: 1, operation: OperationId::default(), attempt: 1, task: TaskId::default(), actor: ActorId::default(), descriptor_digest: "test:inert:v1".into(), arguments: serde_json::json!({}), resources: vec!["in-memory-counter".into()], preconditions: vec!["value=0".into()], budget_reservation: "none".into() } }
    fn spec() -> RunSpec { RunSpec { schema_version:1, project: ProjectId::default(), session: SessionId::default(), run: RunId::default(), task:"test".into(), mode:Mode::Solo, roles:vec![Role::Independent], provider:None, deadline:60, step_budget:100, permission_epoch:1 } }
    #[test]
    fn solo_and_registry_contracts() {
        let mut spec = spec(); assert!(spec.validate().is_ok());
        spec.roles.push(Role::Independent); assert!(spec.validate().is_err());
        spec.roles = vec![Role::Independent];
        for mode in [Mode::Pair,Mode::Crew,Mode::Fleet,Mode::Mesh,Mode::Adaptive] { spec.mode = mode; assert!(matches!(spec.validate(), Err(Error::Unsupported))); }
        let supervisor = Supervisor::default();
        let started = Arc::new(AtomicUsize::new(0)); let (tx,_rx) = mpsc::channel(1);
        assert!(matches!(supervisor.drive(&spec,Fake {proposal:None,started:started.clone(),closed:Arc::new(AtomicUsize::new(0))},InferRequest {schema_version:1,id:RequestId::default(),prompt:"must not start".into()},tx),Err(Error::Unsupported)));
        assert_eq!(started.load(Ordering::SeqCst),0);
        let descriptor = ToolDescriptor { id:"test:inert".into(),version:1,build_digest:"v1".into(),input_schema:"{}".into(),output_limit:64,effect:EffectClass::Inert,permissions:vec![],platform:"test".into(),cancellable:true,reconcilable:true };
        let mut registry = Registry::default(); registry.register(descriptor.clone()).unwrap(); assert!(registry.register(descriptor.clone()).is_err()); assert_eq!(registry.get("test:inert",1),Some(&descriptor));
    }
    #[test]
    fn grants_bind_all_proposal_fields_epoch_expiry_and_revocation() {
        let proposal = proposal(); let mut broker = Broker::new(1);
        assert!(matches!(broker.decide(&proposal), BrokerDecision::Deny(_)));
        let grant = broker.approve_inert(&proposal,10).unwrap(); broker.consume(grant,&proposal,9).unwrap();
        let grant = broker.approve_inert(&proposal,10).unwrap(); assert!(broker.consume(grant,&proposal,10).is_err());
        let grant = broker.approve_inert(&proposal,10).unwrap(); broker.revoke_all(); assert!(broker.consume(grant,&proposal,1).is_err());
        for index in 0..9 {
            let grant = broker.approve_inert(&proposal,10).unwrap(); let mut changed = proposal.clone();
            match index { 0 => changed.actor = ActorId::default(), 1 => changed.operation = OperationId::default(), 2 => changed.resources.push("other".into()), 3 => changed.arguments=serde_json::json!({"changed":true}), 4 => changed.attempt+=1, 5 => changed.task=TaskId::default(), 6 => changed.preconditions.clear(), 7=>changed.budget_reservation="other".into(), _=>changed.descriptor_digest="other".into() }
            assert!(broker.consume(grant,&changed,1).is_err());
        }
    }
    struct Fake { proposal: Option<ToolProposal>, started: Arc<AtomicUsize>, closed: Arc<AtomicUsize> }
    impl ProviderPort for Fake {
        fn capabilities(&self) -> crate::providers::Capabilities { crate::providers::Capabilities {text:false,tool_calls:true,usage:false} }
        fn infer(&self, request: InferRequest, cancel: CancellationToken) -> mpsc::Receiver<ProviderEvent> {
            let (tx,rx) = mpsc::channel(1); let proposal = self.proposal.clone(); let started = self.started.clone(); let closed = self.closed.clone();
            tokio::spawn(async move {
                if let Some(proposal) = proposal { let _ = tx.send(ProviderEvent::ToolCallComplete { request:request.id, proposal }).await; }
                started.store(1,Ordering::SeqCst); cancel.cancelled().await; drop(tx); closed.store(1,Ordering::SeqCst);
            }); rx
        }
    }
    async fn await_count(count: &AtomicUsize) { tokio::time::timeout(Duration::from_secs(2), async { while count.load(Ordering::SeqCst)==0 { tokio::task::yield_now().await; } }).await.unwrap(); }
    #[tokio::test]
    async fn cancellation_bypasses_waiting_provider_and_saturated_queue() {
        for saturated in [false,true] {
            let supervisor = Supervisor::default(); let (tx,_rx) = mpsc::channel(1);
            if saturated { tx.send(ProviderEvent::Finish {request:RequestId::default(),reason:"occupied".into()}).await.unwrap(); }
            let started = Arc::new(AtomicUsize::new(0)); let closed = Arc::new(AtomicUsize::new(0));
            supervisor.drive(&spec(),Fake {proposal:saturated.then(proposal),started:started.clone(),closed:closed.clone()}, InferRequest {schema_version:1,id:RequestId::default(),prompt:"fixture".into()},tx).unwrap();
            await_count(&started).await;
            tokio::time::timeout(Duration::from_secs(2),supervisor.shutdown()).await.unwrap(); await_count(&closed).await;
            assert!(supervisor.check_cancelled().is_err());
        }
    }
    #[tokio::test]
    async fn fake_provider_stops_at_proposal_and_only_supervisor_dispatches_after_intent() {
        let temp = tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();
        let project = Project::open(temp.path()).unwrap(); let ledger = Ledger::open(&project,Path::new("state")).unwrap();
        let spec = spec(); let p = proposal();
        ledger.transition(0,Transition::NewSession {id:spec.session}).await.unwrap();
        ledger.transition(0,Transition::RootTask {session:spec.session,run:spec.run,task:p.task,actor:p.actor}).await.unwrap();
        let supervisor = Supervisor::default(); supervisor.admit(&spec).unwrap();
        let (tx,mut rx) = mpsc::channel(1); let effects = AtomicUsize::new(0);
        supervisor.drive(&spec,Fake {proposal:Some(p.clone()),started:Arc::new(AtomicUsize::new(0)),closed:Arc::new(AtomicUsize::new(0))},InferRequest {schema_version:1,id:RequestId::default(),prompt:"test".into()},tx).unwrap();
        let event = rx.recv().await.unwrap(); assert!(matches!(event,ProviderEvent::ToolCallComplete {..})); assert_eq!(effects.load(Ordering::SeqCst),0);
        ledger.transition(0,Transition::Propose {run:spec.run,proposal:p.clone()}).await.unwrap();
        let mut broker = Broker::new(ledger.epoch());
        supervisor.dispatch_inert(&ledger,&mut broker,&p,true,&effects).await.unwrap();
        assert_eq!(effects.load(Ordering::SeqCst),1);
        let denied = ToolProposal {operation:OperationId::default(),..p.clone()};
        ledger.transition(0,Transition::Propose {run:spec.run,proposal:denied.clone()}).await.unwrap();
        assert!(supervisor.dispatch_inert(&ledger,&mut broker,&denied,false,&effects).await.is_err()); assert_eq!(effects.load(Ordering::SeqCst),1);
        assert!(matches!(ledger.transition(0,Transition::Intent {operation:p.operation}).await,Err(Error::Conflict)));
        let unknown = ToolProposal {operation:OperationId::default(),..p.clone()};
        ledger.transition(0,Transition::Propose {run:spec.run,proposal:unknown.clone()}).await.unwrap();
        ledger.transition(0,Transition::Authorize {operation:unknown.operation,grant:uuid::Uuid::new_v4().to_string(),expiry:100,scope:unknown.digest().unwrap(),permission_epoch:broker.epoch()}).await.unwrap();
        ledger.transition(1,Transition::Intent {operation:unknown.operation}).await.unwrap();
        ledger.transition(2,Transition::Dispatched {operation:unknown.operation}).await.unwrap();
        ledger.transition(3,Transition::Receipt {receipt:ExecutionReceipt {schema_version:1,operation:unknown.operation,attempt:1,outcome:Outcome::Unknown,exit_code:None,error:None,artifacts:vec![],before_revision:None,after_revision:None,redacted_output:String::new()}}).await.unwrap();
        assert!(matches!(ledger.transition(4,Transition::Terminal {operation:unknown.operation}).await,Err(Error::Conflict)));
        supervisor.shutdown().await; ledger.shutdown().unwrap();
        let db = rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();
        let mut statement = db.prepare("SELECT kind FROM events WHERE entity_id=?1 ORDER BY sequence").unwrap();
        let events: Vec<String> = statement.query_map([p.operation.0.to_string()],|r|r.get(0)).unwrap().collect::<std::result::Result<_,_>>().unwrap();
        assert_eq!(events,["proposed","authorized","intent-recorded","dispatched","receipt-recorded","terminal"]);
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use crate::{config::Project, domain::*, ledger::{Ledger, LedgerCommand, Transition}, permissions::Broker, tools::ToolProposal};
    use std::{path::Path, sync::atomic::{AtomicUsize, Ordering}};
    #[tokio::test]
    async fn stale_owner_and_critical_ledger_failure_prevent_effect() {
        let temp = tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();
        let project = Project::open(temp.path()).unwrap();
        let state = Path::new("state");
        Ledger::open(&project,state).unwrap().shutdown().unwrap();
        let db = rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();
        db.execute_batch("CREATE TRIGGER fail_intent BEFORE INSERT ON events WHEN NEW.kind='intent-recorded' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap(); drop(db);
        let ledger = Ledger::open(&project,state).unwrap();
        assert!(matches!(ledger.command(LedgerCommand {schema_version:1,owner_epoch:ledger.epoch()-1,expected_revision:0,transition:Transition::NewSession {id:SessionId::default()}}).await,Err(Error::Conflict)));
        let session = SessionId::default(); let run = RunId::default();
        let p = ToolProposal {schema_version:1,operation:OperationId::default(),attempt:1,task:TaskId::default(),actor:ActorId::default(),descriptor_digest:"test:inert:v1".into(),arguments:serde_json::json!({"secret":"not-persisted-raw"}),resources:vec![],preconditions:vec![],budget_reservation:"none".into()};
        ledger.transition(0,Transition::NewSession {id:session}).await.unwrap();
        ledger.transition(0,Transition::RootTask {session,run,task:p.task,actor:p.actor}).await.unwrap();
        ledger.transition(0,Transition::Propose {run,proposal:p.clone()}).await.unwrap();
        let effects = AtomicUsize::new(0); let mut broker = Broker::new(ledger.epoch());
        let supervisor = Supervisor::default();
        assert!(supervisor.dispatch_inert(&ledger,&mut broker,&p,true,&effects).await.is_err());
        assert_eq!(effects.load(Ordering::SeqCst),0); ledger.shutdown().unwrap();
        let db = rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();
        let (state,revision): (String,i64) = db.query_row("SELECT state,revision FROM operations",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!((state.as_str(),revision),("authorized",1));
        let digest: String = db.query_row("SELECT proposal_digest FROM operations",[],|r|r.get(0)).unwrap();
        assert_eq!(digest.len(),64); assert!(!digest.contains("not-persisted"));
    }
}

#[cfg(test)]
use crate::{domain::*, ledger::{Ledger, Transition}, permissions::Broker, tools::ToolProposal};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(test)]
impl Supervisor {
    // Sole inert dispatcher: compiled only for tests, never handed to ProviderPort.
    async fn dispatch_inert(&self, ledger: &Ledger, broker: &mut Broker, p: &ToolProposal, allow: bool, effects: &AtomicUsize) -> Result<()> {
        self.check_cancelled()?;
        if !allow { ledger.transition(0,Transition::Deny {operation:p.operation}).await?; return Err(Error::Denied); }
        let grant = broker.approve_inert(p,100)?;
        ledger.transition(0,Transition::Authorize {operation:p.operation,grant:grant.reference(),expiry:100,scope:p.digest()?,permission_epoch:broker.epoch()}).await?;
        broker.consume(grant,p,1)?;
        ledger.transition(1,Transition::Intent {operation:p.operation}).await?;
        ledger.transition(2,Transition::Dispatched {operation:p.operation}).await?;
        self.check_cancelled()?;
        effects.fetch_add(1,Ordering::SeqCst);
        ledger.transition(3,Transition::Receipt {receipt:ExecutionReceipt {schema_version:1,operation:p.operation,attempt:1,outcome:Outcome::Succeeded,exit_code:None,error:None,artifacts:vec![],before_revision:None,after_revision:None,redacted_output:String::new()}}).await?;
        ledger.transition(4,Transition::Terminal {operation:p.operation}).await
    }
}

impl Supervisor {
    /// Single-operation file dispatch for trusted host orchestration; no CLI/model approval path.
    pub async fn execute_file(
        &self,
        spec: &RunSpec,
        ledger: &crate::ledger::Ledger,
        broker: &mut crate::permissions::Broker,
        files: std::sync::Arc<crate::tools::files::FileWorkspace>,
        proposal: crate::tools::ToolProposal,
        grant: crate::permissions::Grant,
    ) -> Result<crate::tools::files::FileResult> {
        use crate::{domain::{ExecutionReceipt, Outcome}, ledger::Transition, tools::files::{FileAction, DESCRIPTOR}};
        self.admit(spec)?;
        self.check_cancelled()?;
        let descriptor = self.registry.get("rivet:files",1).ok_or(Error::Unsupported)?;
        if descriptor.build_digest != proposal.descriptor_digest { return Err(Error::Denied); }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| Error::State)?.as_secs();
        if proposal.descriptor_digest != DESCRIPTOR
            || spec.permission_epoch != broker.epoch() || now >= spec.deadline
            || spec.project != ledger.project_id() || files.identity() != ledger.workspace_identity() { return Err(Error::Denied); }
        ledger.transition(0, Transition::Propose { run: spec.run, proposal: proposal.clone() }).await?;
        let validation = (|| {
            let action: FileAction = serde_json::from_value(proposal.arguments.clone()).map_err(|_| Error::Invalid)?;
            if files.resources(&action)? != proposal.resources { return Err(Error::Denied); }
            Ok(action)
        })();
        let action = match validation { Ok(action) => action, Err(e) => {
            ledger.transition(0, Transition::Deny { operation: proposal.operation }).await?; return Err(e);
        }};
        let reference = grant.reference(); let expiry = grant.expiry();
        // Consume before any file read/write; mutated/revoked/expired approval cannot dispatch.
        if let Err(error) = broker.consume(grant, &proposal, now) {
            ledger.transition(0, Transition::Deny { operation: proposal.operation }).await?; return Err(error);
        }
        ledger.transition(0, Transition::Authorize { operation: proposal.operation, grant: reference, expiry, scope: proposal.digest()?, permission_epoch:broker.epoch() }).await?;
        self.check_cancelled()?;
        ledger.transition(1, Transition::Intent { operation: proposal.operation }).await?;
        ledger.transition(2, Transition::Dispatched { operation: proposal.operation }).await?;
        let before_revision = match &action {
            FileAction::Write {expected, ..} => expected.as_ref().map(|r|r.digest.clone()),
            FileAction::Patch {expected, ..} | FileAction::Delete {expected, ..} | FileAction::Rename {expected, ..} => Some(expected.digest.clone()),
            _ => None,
        };
        let stop = self.stop.child_token();
        let worker_stop = stop.clone();
        let ledger_stop = ledger.cancellation();
        let revoked = broker.revocation();
        let remaining = expiry.min(spec.deadline).saturating_sub(now);
        let work = self.tasks.spawn_blocking(move || files.perform(&action, &worker_stop));
        tokio::pin!(work);
        let effect = tokio::select! { biased;
            _ = ledger_stop.cancelled() => { stop.cancel(); work.await },
            _ = revoked.cancelled() => { stop.cancel(); work.await },
            _ = tokio::time::sleep(std::time::Duration::from_secs(remaining)) => { stop.cancel(); work.await },
            result = &mut work => result,
        }.map_err(|_| Error::State)?;
        let outcome = match &effect {
            Ok(_) => Outcome::Succeeded,
            Err(Error::State) => Outcome::Unknown,
            Err(Error::Interrupted) => Outcome::Cancelled,
            Err(_) => Outcome::Failed,
        };
        let receipt = ExecutionReceipt {
            schema_version: 1, operation: proposal.operation, attempt: proposal.attempt, outcome,
            exit_code: None, error: effect.as_ref().err().map(|e|e.kind().to_owned()), artifacts: vec![],
            before_revision, after_revision: effect.as_ref().ok().and_then(|r|r.revision.as_ref().map(|r|r.digest.clone())),
            redacted_output: effect.as_ref().ok().map_or_else(String::new, |r|format!("changed={};diff_sha256={}",r.changed,crate::tools::sha256(r.diff.as_bytes()))),
        };
        ledger.transition(3, Transition::Receipt { receipt }).await?;
        if effect.is_ok() { ledger.transition(4, Transition::Terminal { operation: proposal.operation }).await?; }
        effect
    }
}

impl Supervisor {
    pub async fn execute_process(&self,spec:&RunSpec,ledger:&crate::ledger::Ledger,broker:&mut crate::permissions::Broker,proposal:crate::tools::ToolProposal,grant:crate::permissions::Grant)->Result<crate::tools::process::ProcessResult> {
        use crate::{domain::{ExecutionReceipt,Outcome},ledger::Transition,tools::process::{ProcessRequest,DESCRIPTOR}};
        self.admit(spec)?;self.check_cancelled()?;
        let descriptor=self.registry.get("rivet:process-check",1).ok_or(Error::Unsupported)?;
        if descriptor.build_digest!=proposal.descriptor_digest{return Err(Error::Denied);}
        let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|Error::State)?.as_secs();
        if proposal.descriptor_digest!=DESCRIPTOR || spec.project!=ledger.project_id() || spec.permission_epoch!=broker.epoch() || now>=spec.deadline{return Err(Error::Denied);}
        ledger.transition(0,Transition::Propose{run:spec.run,proposal:proposal.clone()}).await?;
        let request=serde_json::from_value::<ProcessRequest>(proposal.arguments.clone()).map_err(|_|Error::Invalid);
        let validation=request.and_then(|r|{r.validate()?;if r.resources(spec.project)!=proposal.resources || now+(r.timeout_ms+999)/1000>spec.deadline{return Err(Error::Denied);}Ok(r)});
        let request=match validation{Ok(r)=>r,Err(e)=>{ledger.transition(0,Transition::Deny{operation:proposal.operation}).await?;return Err(e);}};
        let reference=grant.reference();let expiry=grant.expiry();
        if let Err(e)=broker.consume(grant,&proposal,now){ledger.transition(0,Transition::Deny{operation:proposal.operation}).await?;return Err(e);}
        ledger.transition(0,Transition::Authorize{operation:proposal.operation,grant:reference,expiry,scope:proposal.digest()?,permission_epoch:broker.epoch()}).await?;
        self.check_cancelled()?;
        ledger.transition(1,Transition::Intent{operation:proposal.operation}).await?;
        ledger.transition(2,Transition::Dispatched{operation:proposal.operation}).await?;
        let stop=self.stop.child_token();let operation=proposal.operation;let ledger_stop=ledger.cancellation();let revoked=broker.revocation();
        let mut request=request;
        request.timeout_ms=request.timeout_ms.min(expiry.saturating_sub(now)*1000);
        let effect=self.tasks.spawn(async move{
            let work=crate::tools::process::perform(request,operation,stop.clone());tokio::pin!(work);
            tokio::select!{biased;
                _=ledger_stop.cancelled()=>{stop.cancel();work.await},
                _=revoked.cancelled()=>{stop.cancel();work.await},
                result=&mut work=>result,
            }
        }).await.map_err(|_|Error::State)?;
        let outcome=match &effect{Ok(r) if r.cancelled=>Outcome::Cancelled,Ok(r) if r.exit_code==Some(0)&&!r.truncated=>Outcome::Succeeded,Ok(_)=>Outcome::Failed,Err(_)=>Outcome::Unknown};
        let receipt=ExecutionReceipt{schema_version:1,operation,attempt:proposal.attempt,outcome,exit_code:effect.as_ref().ok().and_then(|r|r.exit_code),error:effect.as_ref().err().map(|e|e.kind().into()),artifacts:vec![],before_revision:None,after_revision:None,redacted_output:effect.as_ref().ok().map_or_else(String::new,|r|format!("stdout_sha256={};stderr_sha256={};truncated={}",crate::tools::sha256(r.stdout.as_bytes()),crate::tools::sha256(r.stderr.as_bytes()),r.truncated))};
        let terminal=matches!(receipt.outcome,Outcome::Succeeded|Outcome::Failed);
        ledger.transition(3,Transition::Receipt{receipt}).await?;
        if terminal{ledger.transition(4,Transition::Terminal{operation}).await?;}
        effect
    }
}

impl Supervisor{
    pub async fn execute_git(&self,spec:&RunSpec,ledger:&crate::ledger::Ledger,broker:&mut crate::permissions::Broker,workspace:std::sync::Arc<crate::tools::git::GitWorkspace>,proposal:crate::tools::ToolProposal,grant:crate::permissions::Grant)->Result<crate::tools::git::GitResult>{
        use crate::{domain::{ExecutionReceipt,Outcome},ledger::Transition,tools::git::{GitRequest,DESCRIPTOR}};
        self.admit(spec)?;self.check_cancelled()?;
        let descriptor=self.registry.get("rivet:git",1).ok_or(Error::Unsupported)?;if descriptor.build_digest!=proposal.descriptor_digest{return Err(Error::Denied);}
        let revoked=broker.revocation();
        let _lane=tokio::select!{biased;_=self.stop.cancelled()=>return Err(Error::Interrupted),_=revoked.cancelled()=>return Err(Error::Denied),lane=self.git_lane.lock()=>lane};
        let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|Error::State)?.as_secs();
        if proposal.descriptor_digest!=DESCRIPTOR||spec.project!=ledger.project_id()||workspace.identity()!=ledger.workspace_identity()||spec.permission_epoch!=broker.epoch()||now>=spec.deadline{return Err(Error::Denied);}
        ledger.transition(0,Transition::Propose{run:spec.run,proposal:proposal.clone()}).await?;
        let validation=serde_json::from_value::<GitRequest>(proposal.arguments.clone()).map_err(|_|Error::Invalid).and_then(|r|{r.validate()?;if r.resources(spec.project)!=proposal.resources{return Err(Error::Denied);}Ok(r)});
        let request=match validation{Ok(r)=>r,Err(e)=>{ledger.transition(0,Transition::Deny{operation:proposal.operation}).await?;return Err(e);}};
        let reference=grant.reference();let expiry=grant.expiry();
        if let Err(e)=broker.consume(grant,&proposal,now){ledger.transition(0,Transition::Deny{operation:proposal.operation}).await?;return Err(e);}
        let mount=workspace.mount(request.writable())?;let mut process=request.process()?;process.timeout_ms=process.timeout_ms.min(expiry.saturating_sub(now)*1000).min(spec.deadline.saturating_sub(now)*1000);
        ledger.transition(0,Transition::Authorize{operation:proposal.operation,grant:reference,expiry,scope:proposal.digest()?,permission_epoch:broker.epoch()}).await?;
        self.check_cancelled()?;ledger.transition(1,Transition::Intent{operation:proposal.operation}).await?;ledger.transition(2,Transition::Dispatched{operation:proposal.operation}).await?;
        let operation=proposal.operation;let stop=self.stop.child_token();let ledger_stop=ledger.cancellation();
        let effect=self.tasks.spawn(async move{
            let _retained_workspace=workspace;let work=crate::tools::process::perform_git(process,operation,stop.clone(),mount);tokio::pin!(work);
            tokio::select!{biased;_=ledger_stop.cancelled()=>{stop.cancel();work.await},_=revoked.cancelled()=>{stop.cancel();work.await},r=&mut work=>r}
        }).await.map_err(|_|Error::State)?;
        let result=effect.and_then(|r|{if r.cancelled||r.truncated||r.exit_code!=Some(0){
            let phase=["symbolic-ref","rev-parse","ls-tree","ls-files","hash-object","read-tree","update-index","write-tree","commit-tree","update-ref","update-ref-packed-lock","update-ref-head-lock","update-ref-reflog","cat-file"].into_iter().find(|phase|r.stderr.lines().any(|line|line==format!("rivet-git-phase:{phase}")));
            return Err(phase.map_or(Error::State,Error::GitPhase));}serde_json::from_str::<crate::tools::git::GitResult>(&r.stdout).map_err(|_|Error::State)});
        let receipt=ExecutionReceipt{schema_version:1,operation,attempt:proposal.attempt,outcome:if result.is_ok(){Outcome::Succeeded}else{Outcome::Unknown},exit_code:None,error:result.as_ref().err().map(|e|e.kind().into()),artifacts:vec![],before_revision:result.as_ref().ok().and_then(|r|r.head.clone()),after_revision:result.as_ref().ok().and_then(|r|r.commit.clone()),redacted_output:result.as_ref().ok().map_or_else(String::new,|r|format!("entries={};diff_sha256={}",r.entries.len(),crate::tools::sha256(r.diff.as_bytes())))};
        ledger.transition(3,Transition::Receipt{receipt}).await?;if result.is_ok(){ledger.transition(4,Transition::Terminal{operation}).await?;}result
    }
}
