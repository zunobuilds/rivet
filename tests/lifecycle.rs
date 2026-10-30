use rivet::{budget::BudgetLimits,config::Project,domain::*,ledger::{self,Ledger,Transition},lifecycle::{self,Command,Checkpoint,State},providers::*,supervisor::Supervisor};
use std::{path::{Path,PathBuf},time::{Duration,SystemTime,UNIX_EPOCH},sync::{Arc,atomic::{AtomicUsize,Ordering}}};
use tokio_util::sync::CancellationToken;
fn root()->tempfile::TempDir{tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap()}
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
async fn admit(project:&Project)->(Ledger,RunSpec){
    let ledger=Ledger::open(project,Path::new("state")).unwrap();let spec=RunSpec{schema_version:1,project:ledger.project_id(),session:SessionId::default(),run:RunId::default(),task:"synthetic bounded task".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+3600,step_budget:100,permission_epoch:ledger.epoch()};
    ledger.transition(0,Transition::NewSession{id:spec.session}).await.unwrap();ledger.transition(0,Transition::RootTask{session:spec.session,run:spec.run,task:TaskId::default(),actor:ActorId::default()}).await.unwrap();ledger.init_budget(spec.run,BudgetLimits{tokens:1000,requests:100,cost_micros:100}).await.unwrap();ledger.life(0,Command::Admit{spec:spec.clone(),wall:now(),detached:false}).await.unwrap();(ledger,spec)
}
struct Synthetic(Arc<AtomicUsize>);
impl ProviderPort for Synthetic{
    fn capabilities(&self)->Capabilities{Capabilities{text:true,tool_calls:false,usage:true}}
    fn infer(&self,request:InferRequest,cancel:CancellationToken)->tokio::sync::mpsc::Receiver<ProviderEvent>{let(tx,rx)=tokio::sync::mpsc::channel(1);let count=self.0.clone();tokio::spawn(async move{if cancel.is_cancelled(){return;}count.fetch_add(1,Ordering::SeqCst);for event in [ProviderEvent::Usage{request:request.id,input:1,output:1},ProviderEvent::Finish{request:request.id,reason:"synthetic".into()}]{tokio::select!{biased;_=cancel.cancelled()=>return,_=tx.send(event)=>()}}});rx}
}
#[tokio::test]
async fn seeded_thousand_dispatches_hundred_pause_resume_interleavings_and_ten_restarts(){
    let started=std::time::Instant::now();let count=Arc::new(AtomicUsize::new(0));
    for seed in 1u64..=10{
        tokio::time::timeout(Duration::from_secs(30),async{
            let temp=root();let project=Project::open(temp.path()).unwrap();let(mut ledger,mut spec)=admit(&project).await;let supervisor=Supervisor::default();let mut random=seed;
            for step in 0..100{
                random=random.wrapping_mul(6364136223846793005).wrapping_add(1);let current=ledger.lifecycle(spec.run).await.unwrap();
                ledger.life(current.revision,Command::Tick{run:spec.run,wall:now(),remaining_ms:current.remaining_ms-1,step:true,checkpoint:Checkpoint{boundary:rivet::lifecycle::Boundary::SyntheticDispatch,evidence:vec![format!("seed:{seed}:step:{step}:choice:{}",random%7)]}}).await.unwrap();
                let id=RequestId::default();ledger.reserve_inference(spec.run,id,2,1,"a".repeat(64),spec.permission_epoch).await.unwrap();
                for _ in 0..random%4{tokio::task::yield_now().await;}let(tx,mut rx)=tokio::sync::mpsc::channel(1+(random%3) as usize);supervisor.drive(&spec,Synthetic(count.clone()),InferRequest{schema_version:1,id,prompt:"synthetic".into(),context:None},tx).unwrap();
                assert!(matches!(rx.recv().await,Some(ProviderEvent::Usage{input:1,output:1,..})));assert!(matches!(rx.recv().await,Some(ProviderEvent::Finish{..})));ledger.settle_inference(id,Some((1,1))).await.unwrap();
                if step%10==seed as u32%9{
                    let current=ledger.lifecycle(spec.run).await.unwrap();ledger.life(current.revision,Command::Finish{run:spec.run,state:State::Paused,wall:now(),remaining_ms:current.remaining_ms,checkpoint:current.checkpoint.clone()}).await.unwrap();
                    let current=ledger.lifecycle(spec.run).await.unwrap();let old_epoch=current.spec.permission_epoch;assert!(ledger.reserve_inference(spec.run,RequestId::default(),1,0,"d".repeat(64),old_epoch).await.is_err());
                    if step/10==4{ledger.shutdown().unwrap();ledger=Ledger::open(&project,Path::new("state")).unwrap();}
                    assert!(ledger.life(current.revision,Command::Resume{run:spec.run,wall:now(),approved:"wrong".into(),detached:false}).await.is_err());
                    ledger.life(current.revision,Command::Resume{run:spec.run,wall:now(),approved:current.digest().unwrap(),detached:false}).await.unwrap();spec=ledger.lifecycle(spec.run).await.unwrap().spec;assert!(spec.permission_epoch>old_epoch);
                }
            }
            let current=ledger.lifecycle(spec.run).await.unwrap();assert_eq!(current.steps,100);ledger.life(current.revision,Command::Finish{run:spec.run,state:State::Completed,wall:now(),remaining_ms:current.remaining_ms,checkpoint:current.checkpoint}).await.unwrap();
            let current=ledger.lifecycle(spec.run).await.unwrap();assert_eq!(current.state,"completed");assert!(ledger.reserve_inference(spec.run,RequestId::default(),1,0,"b".repeat(64),spec.permission_epoch).await.is_err());supervisor.shutdown().await;ledger.shutdown().unwrap();
            let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();let usage:(i64,i64,i64,i64)=db.query_row("SELECT requests,token_charged,cost_charged,token_reserved FROM run_budgets",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();assert_eq!(usage,(100,200,100,0));let grants:i64=db.query_row("SELECT count(*) FROM grants",[],|r|r.get(0)).unwrap();assert_eq!(grants,0);
        }).await.unwrap();
    }
    assert_eq!(count.load(Ordering::SeqCst),1000);println!("qualified seeds=10 dispatches=1000 pause/resume=100 restarts=10 elapsed_ms={}",started.elapsed().as_millis());
}
#[tokio::test]
async fn clock_rollback_suspend_expiry_and_quota_do_not_renew_time_or_permissions(){
    for case in ["rollback","forward","monotonic","larger"]{
        let temp=root();let project=Project::open(temp.path()).unwrap();let(ledger,spec)=admit(&project).await;let current=ledger.lifecycle(spec.run).await.unwrap();
        let(wall,remaining)=match case{"rollback"=>(current.last_wall-1,current.remaining_ms),"forward"=>(spec.deadline+1,current.remaining_ms),"monotonic"=>(current.last_wall,0),_=>(current.last_wall,current.remaining_ms+1000)};
        ledger.life(current.revision,Command::Tick{run:spec.run,wall,remaining_ms:remaining,step:false,checkpoint:Checkpoint::default()}).await.unwrap();let after=ledger.lifecycle(spec.run).await.unwrap();assert!(after.remaining_ms<=current.remaining_ms);
        if case=="larger"{assert_eq!(after.state,"active");}else{assert_ne!(after.state,"active");assert!(ledger.life(after.revision,Command::Resume{run:spec.run,wall:spec.deadline+2,approved:after.digest().unwrap(),detached:false}).await.is_err());}
        ledger.shutdown().unwrap();
    }
}
#[test]
fn crash_child(){let Some(path)=std::env::var_os("RIVET_LIFECYCLE_CRASH")else{return;};tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
    let path=PathBuf::from(path);let project=Project::open(&path).unwrap();let(ledger,spec)=admit(&project).await;let current=ledger.lifecycle(spec.run).await.unwrap();ledger.life(current.revision,Command::Tick{run:spec.run,wall:now(),remaining_ms:current.remaining_ms,step:true,checkpoint:Checkpoint{boundary:rivet::lifecycle::Boundary::BeforeInference,evidence:vec![]}}).await.unwrap();let request=RequestId::default();ledger.reserve_inference(spec.run,request,10,5,"c".repeat(64),spec.permission_epoch).await.unwrap();std::fs::write(path.join("fixture.json"),serde_json::to_vec(&(spec.run,request)).unwrap()).unwrap();std::process::exit(91);
});}
#[tokio::test]
async fn actual_crashed_reservation_blocks_resume_and_completion_until_explicit_disposition(){
    for _ in 0..10{
        let temp=root();let result=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","crash_child","--test-threads=1"]).env("RIVET_LIFECYCLE_CRASH",temp.path()).output().unwrap();assert_eq!(result.status.code(),Some(91));let(run,request):(RunId,RequestId)=serde_json::from_slice(&std::fs::read(temp.path().join("fixture.json")).unwrap()).unwrap();let project=Project::open(temp.path()).unwrap();let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();ledger::recover_state(&project,Path::new("state"),&report.snapshot_digest).await.unwrap();
        let before=lifecycle::inspect(&project,Path::new("state"),run).unwrap();assert_eq!(before.state,"recovery-required");let ledger=Ledger::open(&project,Path::new("state")).unwrap();assert!(ledger.life(before.revision,Command::Resume{run,wall:now(),approved:before.digest().unwrap(),detached:false}).await.is_err());ledger.shutdown().unwrap();ledger::acknowledge_request(&project,Path::new("state"),request,&"c".repeat(64)).unwrap();
        let before=lifecycle::inspect(&project,Path::new("state"),run).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();ledger.life(before.revision,Command::Resume{run,wall:now(),approved:before.digest().unwrap(),detached:false}).await.unwrap();let after=ledger.lifecycle(run).await.unwrap();assert_eq!(after.steps,1);assert!(after.remaining_ms<=before.remaining_ms);assert!(after.spec.permission_epoch>before.spec.permission_epoch);assert!(ledger.reserve_inference(run,request,10,5,"c".repeat(64),after.spec.permission_epoch).await.is_err());ledger.shutdown().unwrap();
        let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();let held:(i64,i64)=db.query_row("SELECT token_reserved,cost_reserved FROM run_budgets",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();assert_eq!(held,(10,5));
    }
}
