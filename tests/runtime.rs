//! Native qualification only: run this binary outside the developer namespace,
//! with the owner's explicit user-systemd/cgroup permission. Offline suite ignores it.
use rivet::{config::Project,domain::*,ledger::{Ledger,Transition},permissions::Broker,supervisor::Supervisor,tools::{ToolProposal,process::{ProcessRequest,DESCRIPTOR}}};
use std::{collections::BTreeMap,path::Path,sync::Arc,time::{SystemTime,UNIX_EPOCH,Duration}};
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
#[tokio::test]
#[ignore="requires explicitly authorized native user-systemd qualification"]
async fn qualified_worker_checks_flood_and_revocation_clean_up(){
    let root=Path::new(env!("RIVET_SOURCE_ROOT"));
    let temporary=tempfile::tempdir_in(root.join(".build/tmp")).unwrap();
    let project=Project::open(temporary.path()).unwrap();
    let ledger=Ledger::open(&project,Path::new("state")).unwrap();
    let spec=RunSpec{schema_version:1,project:ledger.project_id(),session:SessionId::default(),run:RunId::default(),task:"runtime fixture".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+300,step_budget:100,permission_epoch:ledger.epoch()};
    let task=TaskId::default();let actor=ActorId::default();
    ledger.transition(0,Transition::NewSession{id:spec.session}).await.unwrap();
    ledger.transition(0,Transition::RootTask{session:spec.session,run:spec.run,task,actor}).await.unwrap();
    let supervisor=Arc::new(Supervisor::default());let mut broker=Broker::new(spec.permission_epoch);
    for (script,flood) in [
        ("import pathlib,socket; assert pathlib.Path('code.txt').read_text()=='fixture'; assert not pathlib.Path('/home').exists(); assert not pathlib.Path('/run').exists(); print('qualified')",false),
        ("import os;\nwhile True: os.write(1,b'x'*4096)",true),
    ] {
        let request=ProcessRequest{program:"/usr/bin/python3".into(),args:vec!["-B".into(),"-c".into(),script.into()],files:BTreeMap::from([("code.txt".into(),"fixture".into())]),timeout_ms:10000};
        let proposal=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(&request).unwrap(),resources:request.resources(spec.project),preconditions:vec![],budget_reservation:"none".into()};
        let id=proposal.operation;let grant=broker.approve_process(&proposal,&proposal.resources,now()+60,now()).unwrap();
        let result=supervisor.execute_process(&spec,&ledger,&mut broker,proposal,grant).await.unwrap();
        assert_eq!(result.truncated,flood,"{}",result.stderr);
        assert!(result.stdout.len()+result.stderr.len()<=65536);
        if !flood{assert_eq!(result.exit_code,Some(0),"{}",result.stderr);assert_eq!(result.stdout.trim(),"qualified");}
        assert!(!root.join(".build/runtime").join(id.0.to_string()).exists());
    }
    let request=ProcessRequest{program:"/usr/bin/python3".into(),args:vec!["-c".into(),"import os,signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); os.fork(); time.sleep(60)".into()],files:BTreeMap::new(),timeout_ms:15000};
    let proposal=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(&request).unwrap(),resources:request.resources(spec.project),preconditions:vec![],budget_reservation:"none".into()};
    let unit=format!("rivet-worker-{}.service",proposal.operation.0);let revoke=broker.revocation();
    let grant=broker.approve_process(&proposal,&proposal.resources,now()+60,now()).unwrap();
    let operation=supervisor.execute_process(&spec,&ledger,&mut broker,proposal,grant);
    let controller=async{
        for _ in 0..100{
            let state=tokio::process::Command::new("/usr/bin/systemctl").args(["--user","show",&unit,"--property=ActiveState","--value"]).output().await.unwrap();
            if String::from_utf8_lossy(&state.stdout).trim()=="active"{tokio::time::sleep(Duration::from_millis(250)).await;revoke.cancel();return;}
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        panic!("owned worker did not become active");
    };
    let (result,())=tokio::join!(operation,controller);
    let result=result.unwrap();assert!(result.cancelled);assert!(result.elapsed_ms<9000);
    supervisor.shutdown().await;ledger.shutdown().unwrap();
}
