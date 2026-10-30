use rivet::{config::Project,domain::*,ledger::{self,Ledger,Transition},recovery::Intent,tools::{ToolProposal,files::{FileAction,DESCRIPTOR}}};
use std::{path::{Path,PathBuf},fs};
fn root()->tempfile::TempDir{tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap()}
async fn pending(ledger:&Ledger)->(RunId,ToolProposal){
    let session=SessionId::default();let run=RunId::default();let task=TaskId::default();let actor=ActorId::default();
    ledger.transition(0,Transition::NewSession{id:session}).await.unwrap();ledger.transition(0,Transition::RootTask{session,run,task,actor}).await.unwrap();
    let action=FileAction::Write{path:"code.txt".into(),expected:None,text:"durable patch\n".into()};
    let p=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(&action).unwrap(),resources:vec!["fixture".into()],preconditions:vec![],budget_reservation:"none".into()};
    ledger.transition(0,Transition::Propose{run,proposal:p.clone()}).await.unwrap();ledger.transition(0,Transition::Authorize{operation:p.operation,grant:"fixture".into(),expiry:1,scope:p.digest().unwrap(),permission_epoch:1}).await.unwrap();(run,p)
}
#[test]
fn crash_helper(){
    let Some(path)=std::env::var_os("RIVET_RECOVERY_FIXTURE") else{return;};let phase=std::env::var("RIVET_CRASH_PHASE").unwrap();let path=PathBuf::from(path);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{
        let project=Project::open(&path).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();let (_,p)=pending(&ledger).await;
        if phase=="authorized"{std::process::exit(91);}
        let action:FileAction=serde_json::from_value(p.arguments.clone()).unwrap();ledger.transition(1,Transition::PreparedIntent{operation:p.operation,intent:Intent::file(&action,p.resources.clone()).unwrap()}).await.unwrap();
        if phase=="intent"{std::process::exit(91);}
        ledger.transition(2,Transition::Dispatched{operation:p.operation}).await.unwrap();if phase=="dispatched"{std::process::exit(91);}
        fs::write(path.join("code.txt"),"durable patch\n").unwrap();
        if std::env::var_os("RIVET_RECOVERY_FAIL_PAUSE").is_some(){let db=rusqlite::Connection::open(path.join("state/ledger.sqlite")).unwrap();db.execute_batch("CREATE TRIGGER fail_recovery BEFORE UPDATE ON operations WHEN NEW.state='outcome-unknown' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();drop(db);}
        if phase=="effect"{std::process::exit(91);}
        let reference=ledger.retain_artifact(p.operation,b"reviewed diff".to_vec(),"text/x-diff").await.unwrap();if phase=="artifact"{std::process::exit(91);}
        ledger.transition(3,Transition::Receipt{receipt:ExecutionReceipt{schema_version:1,operation:p.operation,attempt:1,outcome:Outcome::Succeeded,exit_code:None,error:None,artifacts:vec![reference.digest],before_revision:None,after_revision:None,redacted_output:String::new()}}).await.unwrap();std::process::exit(91);
    });
}
#[tokio::test]
async fn actual_child_crashes_preserve_wal_and_intents_without_replaying_effects(){
    for (_repeat,phase) in (0..2).flat_map(|repeat|["authorized","intent","dispatched","effect","artifact","receipt"].map(|phase|(repeat,phase))){
        let temp=root();let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","crash_helper","--test-threads=1"]).env("RIVET_RECOVERY_FIXTURE",temp.path()).env("RIVET_CRASH_PHASE",phase).output().unwrap();assert_eq!(output.status.code(),Some(91),"{}",String::from_utf8_lossy(&output.stderr));
        let project=Project::open(temp.path()).unwrap();assert!(matches!(Ledger::open(&project,Path::new("state")),Err(rivet::error::Error::Recovery)));
        let before=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert!(before.sqlite_recovery_required);assert_eq!(before.integrity,"not-inspected");assert!(before.images.iter().any(|i|i.name=="ledger.sqlite-wal"));
        assert!(ledger::recover_state(&project,Path::new("state"),"wrong").await.is_err());let recovery=ledger::recover_state(&project,Path::new("state"),&before.snapshot_digest).await.unwrap();assert_eq!(recovery.state,"recovered-paused-no-replay");assert!(temp.path().join("state").join(&recovery.snapshot).join("manifest.json").is_file());
        for image in &before.images{let saved=fs::read(temp.path().join("state").join(&recovery.snapshot).join(&image.name)).unwrap();assert_eq!(saved.len() as u64,image.size);}
        let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert_eq!(report.integrity,"ok");assert!(!report.sqlite_recovery_required);
        if ["authorized","receipt"].contains(&phase){assert!(report.pending.is_empty());}else{assert_eq!(report.pending.len(),1);assert_eq!(report.pending[0].state,"outcome-unknown");assert!(matches!(report.pending[0].intent,Some(Intent::File{..})));}
        assert_eq!(temp.path().join("code.txt").exists(),["effect","artifact","receipt"].contains(&phase));if temp.path().join("code.txt").exists(){assert_eq!(fs::read_to_string(temp.path().join("code.txt")).unwrap(),"durable patch\n");}
        let backup=ledger::backup_state(&project,Path::new("state"),&report.snapshot_digest).unwrap();let db=rusqlite::Connection::open_with_flags(temp.path().join("state").join(backup.name),rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();let check:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0)).unwrap();assert_eq!(check,"ok");
    }
}
#[tokio::test]
async fn artifact_integrity_and_orphan_retention_survive_audit_failure(){
    let temp=root();let project=Project::open(temp.path()).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();let (_,p)=pending(&ledger).await;let action:FileAction=serde_json::from_value(p.arguments.clone()).unwrap();ledger.transition(1,Transition::PreparedIntent{operation:p.operation,intent:Intent::file(&action,p.resources.clone()).unwrap()}).await.unwrap();ledger.transition(2,Transition::Dispatched{operation:p.operation}).await.unwrap();
    let artifact=ledger.retain_artifact(p.operation,b"fixture evidence".to_vec(),"text/plain").await.unwrap();assert_eq!(ledger.verify_artifact(&artifact).unwrap(),b"fixture evidence");
    fs::write(temp.path().join("state/blobs").join(&artifact.digest),"corrupted").unwrap();assert!(ledger.verify_artifact(&artifact).is_err());
    let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();db.execute_batch("CREATE TRIGGER fail_artifact BEFORE INSERT ON events WHEN NEW.kind='artifact-linked' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();drop(db);
    assert!(ledger.retain_artifact(p.operation,b"durable orphan".to_vec(),"text/plain").await.is_err());ledger.shutdown().unwrap();
    assert!(Ledger::open(&project,Path::new("state")).is_err());
    let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert!(ledger::backup_state(&project,Path::new("state"),&report.snapshot_digest).is_err());assert_eq!(report.integrity,"artifact-failure");assert_eq!(report.orphan_blobs.len(),1);assert!(temp.path().join("state/blobs").join(&report.orphan_blobs[0]).is_file());
}

#[tokio::test]
async fn failed_reconciliation_keeps_the_owner_fence_and_admission_closed(){
    let temp=root();let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","crash_helper","--test-threads=1"]).env("RIVET_RECOVERY_FIXTURE",temp.path()).env("RIVET_CRASH_PHASE","effect").env("RIVET_RECOVERY_FAIL_PAUSE","1").output().unwrap();assert_eq!(output.status.code(),Some(91));
    let project=Project::open(temp.path()).unwrap();let before=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert!(ledger::recover_state(&project,Path::new("state"),&before.snapshot_digest).await.is_err());
    let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();let (required,epoch):(bool,i64)=db.query_row("SELECT required,owner_epoch FROM recovery_control,meta",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();assert!(required);assert!(epoch>=2);db.execute_batch("DROP TRIGGER fail_recovery").unwrap();drop(db);
    assert!(matches!(Ledger::open(&project,Path::new("state")),Err(rivet::error::Error::Recovery)));
    let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert!(report.control_recovery_required);
    ledger::recover_state(&project,Path::new("state"),&report.snapshot_digest).await.unwrap();let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();assert!(!report.control_recovery_required);assert_eq!(report.pending[0].state,"outcome-unknown");assert_eq!(fs::read_to_string(temp.path().join("code.txt")).unwrap(),"durable patch\n");
}

#[tokio::test]
async fn human_edit_invalidates_review_and_reconciliation_never_replays(){
    for accept in [false,true]{
        let temp=root();let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","crash_helper","--test-threads=1"]).env("RIVET_RECOVERY_FIXTURE",temp.path()).env("RIVET_CRASH_PHASE","effect").output().unwrap();assert_eq!(output.status.code(),Some(91));
        let project=Project::open(temp.path()).unwrap();let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();ledger::recover_state(&project,Path::new("state"),&report.snapshot_digest).await.unwrap();
        let report=ledger::repair_inspect(&project,Path::new("state")).unwrap();let id=OperationId(uuid::Uuid::parse_str(&report.pending[0].operation).unwrap());
        let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();let session=SessionId(uuid::Uuid::parse_str(&db.query_row("SELECT id FROM sessions",[],|r|r.get::<_,String>(0)).unwrap()).unwrap());drop(db);
        assert!(ledger::continuation(&project,Path::new("state"),session).is_err());let review=ledger::preview_reconcile(&project,Path::new("state"),id).unwrap();assert!(review.post_matches);
        fs::write(temp.path().join("code.txt"),"human edit").unwrap();assert!(ledger::reconcile_operation(&project,Path::new("state"),id,ledger::ReconcileDecision::AcceptObservedPost,&review.digest().unwrap()).await.is_err());
        let edited=ledger::preview_reconcile(&project,Path::new("state"),id).unwrap();assert!(!edited.post_matches);assert!(ledger::reconcile_operation(&project,Path::new("state"),id,ledger::ReconcileDecision::AcceptObservedPost,&edited.digest().unwrap()).await.is_err());
        if accept{fs::write(temp.path().join("code.txt"),"durable patch\n").unwrap();}
        let review=ledger::preview_reconcile(&project,Path::new("state"),id).unwrap();ledger::reconcile_operation(&project,Path::new("state"),id,if accept{ledger::ReconcileDecision::AcceptObservedPost}else{ledger::ReconcileDecision::AbandonUnknown},&review.digest().unwrap()).await.unwrap();
        let context=ledger::continuation(&project,Path::new("state"),session).unwrap();assert!(context.contains("causation_proven"));assert!(!context.contains("human edit"));assert!(ledger::preview_reconcile(&project,Path::new("state"),id).is_err());
        assert_eq!(fs::read_to_string(temp.path().join("code.txt")).unwrap(),if accept{"durable patch\n"}else{"human edit"});
    }
}
