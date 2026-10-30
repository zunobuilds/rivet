use rivet::{budget::BudgetLimits,config::Project,domain::*,ledger::{self,Ledger,Transition}};
use std::{path::Path,fs};
#[tokio::test]
async fn durable_reservations_retain_uncertain_usage_and_enforce_aggregate_limits(){
    let temp=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let project=Project::open(temp.path()).unwrap();
    let ledger=Ledger::open(&project,Path::new("state")).unwrap();let session=SessionId::default();let run=RunId::default();
    ledger.transition(0,Transition::NewSession{id:session}).await.unwrap();ledger.transition(0,Transition::RootTask{session,run,task:TaskId::default(),actor:ActorId::default()}).await.unwrap();
    ledger.init_budget(run,BudgetLimits{tokens:100,requests:3,cost_micros:30}).await.unwrap();
    let first=RequestId::default();ledger.reserve_inference(run,first,60,20,"a".repeat(64),1).await.unwrap();
    assert!(ledger.reserve_inference(run,RequestId::default(),50,0,"b".repeat(64),1).await.is_err());
    assert!(ledger.reserve_inference(run,RequestId::default(),1,11,"b".repeat(64),1).await.is_err());
    ledger.settle_inference(first,Some((10,10))).await.unwrap();
    let second=RequestId::default();ledger.reserve_inference(run,second,70,10,"c".repeat(64),1).await.unwrap();ledger.settle_inference(second,None).await.unwrap();
    ledger.shutdown().unwrap();
    assert!(ledger::continuation(&project,Path::new("state"),session).is_err());
    assert!(ledger::acknowledge_request(&project,Path::new("state"),second,"wrong").is_err());
    ledger::acknowledge_request(&project,Path::new("state"),second,&"c".repeat(64)).unwrap();
    ledger::continuation(&project,Path::new("state"),session).unwrap();
    assert!(ledger::acknowledge_request(&project,Path::new("state"),second,&"c".repeat(64)).is_err());
    let ledger=Ledger::open(&project,Path::new("state")).unwrap();
    assert!(ledger.reserve_inference(run,RequestId::default(),11,0,"d".repeat(64),1).await.is_err());
    ledger.shutdown().unwrap();
    let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();
    let budget:(i64,i64,i64,i64)=db.query_row("SELECT token_charged,token_reserved,cost_charged,cost_reserved FROM run_budgets",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();assert_eq!(budget,(20,70,20,10));
}
#[test]
fn v1_upgrade_has_verified_private_backup_and_keeps_read_only_inspection_nonmutating(){
    use std::os::unix::fs::PermissionsExt;
    let temp=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let state=temp.path().join("state");fs::create_dir(&state).unwrap();fs::set_permissions(&state,fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(state.join("owner.lock"),"").unwrap();
    let db=rusqlite::Connection::open(state.join("ledger.sqlite")).unwrap();db.execute_batch(include_str!("../src/ledger/schema.sql")).unwrap();db.execute("INSERT INTO meta VALUES(1,?1,1,0)",[uuid::Uuid::new_v4().to_string()]).unwrap();db.close().unwrap();
    for name in ["owner.lock","ledger.sqlite"]{fs::set_permissions(state.join(name),fs::Permissions::from_mode(0o600)).unwrap();}
    let project=Project::open(temp.path()).unwrap();assert_eq!(ledger::inspect(&project,Path::new("state")).unwrap().state,"quiescent");assert_eq!(fs::read_dir(&state).unwrap().count(),2);
    Ledger::open(&project,Path::new("state")).unwrap().shutdown().unwrap();
    let backups=fs::read_dir(&state).unwrap().map(|e|e.unwrap().path()).filter(|p|p.is_file()&&p.file_name().unwrap().to_string_lossy().starts_with("pre-v4-")).collect::<Vec<_>>();assert_eq!(backups.len(),1);
    let backup=rusqlite::Connection::open_with_flags(&backups[0],rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();let version:i64=backup.query_row("SELECT schema_version FROM meta",[],|r|r.get(0)).unwrap();assert_eq!(version,1);let check:String=backup.query_row("PRAGMA quick_check",[],|r|r.get(0)).unwrap();assert_eq!(check,"ok");
    let db=rusqlite::Connection::open(state.join("ledger.sqlite")).unwrap();let version:i64=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0)).unwrap();assert_eq!(version,4);
}
