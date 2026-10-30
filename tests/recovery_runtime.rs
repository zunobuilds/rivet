//! Native crash of an owned core while an already-approved sandbox worker remains alive.
use rivet::{config::Project,domain::*,ledger::{self,Ledger,Transition},permissions::Broker,supervisor::Supervisor,tools::{ToolProposal,process::{ProcessRequest,DESCRIPTOR}}};
use std::{path::Path,collections::BTreeMap,time::{SystemTime,UNIX_EPOCH,Duration},process::Stdio};
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
#[test]
fn core_exit_helper(){
    let Some(root)=std::env::var_os("RIVET_NATIVE_RECOVERY_ROOT")else{return;};
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
        let project=Project::open(Path::new(&root)).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();let task=TaskId::default();let actor=ActorId::default();
        let spec=RunSpec{schema_version:1,project:ledger.project_id(),session:SessionId::default(),run:RunId::default(),task:"synthetic recovery".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+120,step_budget:100,permission_epoch:ledger.epoch()};
        ledger.transition(0,Transition::NewSession{id:spec.session}).await.unwrap();ledger.transition(0,Transition::RootTask{session:spec.session,run:spec.run,task,actor}).await.unwrap();
        let request=ProcessRequest{program:"/usr/bin/python3".into(),args:vec!["-B".into(),"-c".into(),"import os,signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); os.fork(); time.sleep(30)".into()],files:BTreeMap::new(),timeout_ms:30000};
        let p=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(&request).unwrap(),resources:request.resources(spec.project),preconditions:vec![],budget_reservation:"none".into()};
        std::fs::write(Path::new(&root).join("operation.txt"),p.operation.0.to_string()).unwrap();let mut broker=Broker::new(spec.permission_epoch);let grant=broker.approve_process(&p,&p.resources,now()+60,now()).unwrap();let supervisor=Supervisor::default();
        let _=supervisor.execute_process(&spec,&ledger,&mut broker,p,grant).await;supervisor.shutdown().await;ledger.shutdown().unwrap();
    });
}
async fn property(unit:&str,key:&str)->String{
    let mut command=tokio::process::Command::new("/usr/bin/systemctl");command.env_clear().env("PATH","/usr/bin").env("SYSTEMD_BUS_TIMEOUT","2s");for key in ["XDG_RUNTIME_DIR","DBUS_SESSION_BUS_ADDRESS"]{if let Some(v)=std::env::var_os(key){command.env(key,v);}}
    let result=command.args(["--user","show",unit,&format!("--property={key}"),"--value"]).output().await.unwrap();String::from_utf8_lossy(&result.stdout).trim().into()
}
#[tokio::test]
#[ignore="requires explicitly authorized native user-systemd qualification"]
async fn crashed_core_worker_is_contained_by_recovery_before_new_admission(){
    for restore in [false,true]{
    let temp=tempfile::tempdir_in(Path::new(env!("RIVET_SOURCE_ROOT")).join(".build/tmp")).unwrap();
    let project=Project::open(temp.path()).unwrap();Ledger::open(&project,Path::new("state")).unwrap().shutdown().unwrap();let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();let backup=ledger::backup_state(&project,Path::new("state"),&report.snapshot_digest).unwrap();
    let mut child=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","core_exit_helper","--test-threads=1"]).env("RIVET_NATIVE_RECOVERY_ROOT",temp.path()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let unit=tokio::time::timeout(Duration::from_secs(10),async{
        loop{
            if let Ok(id)=std::fs::read_to_string(temp.path().join("operation.txt")){
                if let Ok(unit)=std::fs::read_to_string(Path::new(env!("RIVET_SOURCE_ROOT")).join(".build/runtime").join(id.trim()).join("unit.txt")){
                    if property(&unit,"ActiveState").await=="active"&&property(&unit,"TasksCurrent").await.parse::<u32>().is_ok_and(|n|n>=4){tokio::time::sleep(Duration::from_millis(100)).await;break unit;}
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await;
    child.kill().unwrap();child.wait().unwrap();let unit=unit.unwrap();
    let project=Project::open(temp.path()).unwrap();let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert!(report.sqlite_recovery_required);
    if restore{let review=ledger::preview_restore(&project,Path::new("state"),&backup.name).unwrap();let result=ledger::restore_state(&project,Path::new("state"),&backup.name,&review.digest().unwrap()).await.unwrap();assert_eq!(result.stopped_workers,1);}
    else{let recovered=ledger::recover_state(&project,Path::new("state"),&report.snapshot_digest).await.unwrap();assert_eq!(recovered.stopped_workers,1);}assert_eq!(property(&unit,"LoadState").await,"not-found");
    let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();if restore{assert!(report.pending.is_empty());}else{assert_eq!(report.pending.len(),1);assert_eq!(report.pending[0].state,"outcome-unknown");}let ledger=Ledger::open(&project,Path::new("state")).unwrap();ledger.shutdown().unwrap();
    }
}
