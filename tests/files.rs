use rivet::{config::Project, domain::*, error::{Error, Result}, ledger::{Ledger, Transition}, permissions::Broker, supervisor::Supervisor, tools::{ToolProposal, files::{FileAction, FileResult, FileWorkspace, DESCRIPTOR}}};
use std::{fs, path::{Path, PathBuf}, sync::Arc, time::{SystemTime, UNIX_EPOCH}};

struct Fixture {
    temp: tempfile::TempDir, ledger: Ledger, spec: RunSpec, task: TaskId, actor: ActorId,
    files: Arc<FileWorkspace>, broker: Broker, supervisor: Supervisor,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();
        let project = Project::open(temp.path()).unwrap();
        let ledger = Ledger::open(&project, Path::new("private-state")).unwrap();
        let spec = RunSpec {schema_version:1, project:ledger.project_id(),session:SessionId::default(),run:RunId::default(),task:"fixture".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+60,step_budget:100,permission_epoch:ledger.epoch()};
        let task = TaskId::default(); let actor = ActorId::default();
        ledger.transition(0,Transition::NewSession {id:spec.session}).await.unwrap();
        ledger.transition(0,Transition::RootTask {session:spec.session,run:spec.run,task,actor}).await.unwrap();
        let files = Arc::new(FileWorkspace::new(project,Path::new("private-state")).unwrap());
        let broker = Broker::new(ledger.epoch());
        Self {temp,ledger,spec,task,actor,files,broker,supervisor:Supervisor::default()}
    }
    fn proposal(&self, action: &FileAction) -> ToolProposal {
        ToolProposal {schema_version:1,operation:OperationId::default(),attempt:1,task:self.task,actor:self.actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(action).unwrap(),resources:self.files.resources(action).unwrap(),preconditions:vec![],budget_reservation:"none".into()}
    }
    async fn execute(&mut self, action: FileAction) -> Result<FileResult> {
        let proposal = self.proposal(&action);
        let grant = self.broker.approve_file(&proposal,&proposal.resources,now()+60,now())?;
        self.supervisor.execute_file(&self.spec,&self.ledger,&mut self.broker,self.files.clone(),proposal,grant).await
    }
}
fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() }
fn path(p: &str) -> PathBuf { p.into() }

#[tokio::test]
async fn supervised_file_lifecycle_records_intent_and_preserves_stale_edits() {
    let mut f = Fixture::new().await;
    let create = f.execute(FileAction::Write {path:path("source.txt"),expected:None,text:"alpha\nbeta\n".into()}).await.unwrap();
    assert!(create.diff.contains("+alpha"));
    let revision = create.revision.unwrap();
    let read = f.execute(FileAction::Read {path:path("source.txt")}).await.unwrap(); assert_eq!(read.text,"alpha\nbeta\n");
    let patched = f.execute(FileAction::Patch {path:path("source.txt"),expected:revision.clone(),old:"beta".into(),new:"gamma".into()}).await.unwrap();
    assert!(patched.diff.contains("-beta")); assert!(patched.diff.contains("+gamma"));
    assert!(matches!(f.execute(FileAction::Write {path:path("source.txt"),expected:Some(revision),text:"stale".into()}).await,Err(Error::Conflict)));
    assert_eq!(fs::read_to_string(f.temp.path().join("source.txt")).unwrap(),"alpha\ngamma\n");
    fs::create_dir(f.temp.path().join("sources")).unwrap();
    f.execute(FileAction::Rename {from:path("source.txt"),to:path("sources/renamed.txt"),expected:patched.revision.unwrap()}).await.unwrap();
    let found = f.execute(FileAction::Search {path:path("sources"),literal:"gamma".into()}).await.unwrap();
    assert_eq!(found.text,"sources/renamed.txt:2:gamma\n");
    let read = f.execute(FileAction::Read {path:path("sources/renamed.txt")}).await.unwrap();
    fs::write(f.temp.path().join("sources/renamed.txt"),"human edit").unwrap();
    assert!(matches!(f.execute(FileAction::Delete {path:path("sources/renamed.txt"),expected:read.revision.unwrap()}).await,Err(Error::Conflict)));
    let read = f.execute(FileAction::Read {path:path("sources/renamed.txt")}).await.unwrap();
    f.execute(FileAction::Delete {path:path("sources/renamed.txt"),expected:read.revision.unwrap()}).await.unwrap();
    assert!(!f.temp.path().join("sources/renamed.txt").exists());
    let root = f.temp.path().to_owned(); f.supervisor.shutdown().await; f.ledger.shutdown().unwrap();
    let db = rusqlite::Connection::open(root.join("private-state/ledger.sqlite")).unwrap();
    let count: i64 = db.query_row("SELECT count(*) FROM events WHERE kind='intent-recorded'",[],|r|r.get(0)).unwrap(); assert!(count>=10);
    let mut statement = db.prepare("SELECT kind FROM events WHERE entity_id=(SELECT id FROM operations ORDER BY rowid LIMIT 1) ORDER BY sequence").unwrap();
    let events = statement.query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<std::result::Result<Vec<_>,_>>().unwrap();
    assert_eq!(events,["proposed","authorized","intent-recorded","dispatched","artifact-linked","artifact-linked","receipt-recorded","terminal"]);
}
#[tokio::test]
async fn no_clobber_bounds_and_ambiguous_patch_fail_without_file_changes() {
    let mut f = Fixture::new().await;
    let first = f.execute(FileAction::Write {path:path("one"),expected:None,text:"same same".into()}).await.unwrap().revision.unwrap();
    let second = f.execute(FileAction::Write {path:path("two"),expected:None,text:"preserve".into()}).await.unwrap().revision.unwrap();
    assert!(matches!(f.execute(FileAction::Write {path:path("two"),expected:None,text:"clobber".into()}).await,Err(Error::Conflict)));
    assert!(matches!(f.execute(FileAction::Rename {from:path("one"),to:path("two"),expected:first.clone()}).await,Err(Error::Conflict)));
    assert!(matches!(f.execute(FileAction::Patch {path:path("one"),expected:first,old:"same".into(),new:"changed".into()}).await,Err(Error::Conflict)));
    assert!(matches!(f.execute(FileAction::Write {path:path("two"),expected:Some(second),text:"x".repeat(65537)}).await,Err(Error::Unsupported)));
    assert_eq!(fs::read_to_string(f.temp.path().join("one")).unwrap(),"same same");
    assert_eq!(fs::read_to_string(f.temp.path().join("two")).unwrap(),"preserve");
    assert!(!fs::read_dir(f.temp.path()).unwrap().any(|p|p.unwrap().file_name().to_string_lossy().starts_with(".rivet-edit-")));
}
#[tokio::test]
async fn protected_paths_links_special_files_and_traversal_are_denied() {
    use std::os::unix::fs::symlink;
    let mut f = Fixture::new().await;
    for name in ["../outside","private-state/ledger.sqlite",".rivet-state/x",".git/config",".env","secrets/key",".cargo/config.toml",".ssh/id_rsa",".aws/credentials",".netrc"] {
        assert!(f.files.resources(&FileAction::Read {path:path(name)}).is_err());
    }
    fs::write(f.temp.path().join("safe"),"safe").unwrap();
    symlink("safe",f.temp.path().join("link")).unwrap();
    assert!(f.execute(FileAction::Read {path:path("link")}).await.is_err());
    fs::hard_link(f.temp.path().join("safe"),f.temp.path().join("hard")).unwrap();
    assert!(f.execute(FileAction::Read {path:path("hard")}).await.is_err());
    rustix::fs::mknodat(rustix::fs::CWD, f.temp.path().join("pipe"), rustix::fs::FileType::Fifo, rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR, 0).unwrap();
    assert!(f.execute(FileAction::Read {path:path("pipe")}).await.is_err());
    fs::create_dir(f.temp.path().join("alias")).unwrap();
    fs::write(f.temp.path().join("alias/file"),"nested").unwrap();
    fs::rename(f.temp.path().join("alias"),f.temp.path().join("moved")).unwrap();
    symlink("moved",f.temp.path().join("alias")).unwrap();
    assert!(f.execute(FileAction::Read {path:path("alias/file")}).await.is_err());
}
#[tokio::test]
async fn mutated_or_foreign_scope_and_critical_ledger_failure_cannot_dispatch() {
    let mut f = Fixture::new().await;
    let action = FileAction::Write {path:path("denied"),expected:None,text:"not written".into()};
    let mut proposal = f.proposal(&action);
    let grant = f.broker.approve_file(&proposal,&proposal.resources,now()+60,now()).unwrap();
    proposal.arguments=serde_json::to_value(FileAction::Write {path:path("changed"),expected:None,text:"mutation".into()}).unwrap();
    assert!(f.supervisor.execute_file(&f.spec,&f.ledger,&mut f.broker,f.files.clone(),proposal,grant).await.is_err());
    assert!(!f.temp.path().join("denied").exists()); assert!(!f.temp.path().join("changed").exists());
    let other = Fixture::new().await;
    let proposal = f.proposal(&action); let grant = f.broker.approve_file(&proposal,&proposal.resources,now()+60,now()).unwrap();
    assert!(f.supervisor.execute_file(&f.spec,&f.ledger,&mut f.broker,other.files.clone(),proposal,grant).await.is_err());
    let db = rusqlite::Connection::open(f.temp.path().join("private-state/ledger.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_file_intent BEFORE INSERT ON events WHEN NEW.kind='intent-recorded' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap(); drop(db);
    assert!(f.execute(action.clone()).await.is_err());
    assert!(!f.temp.path().join("denied").exists());
    f.supervisor.cancellation().cancel();
    assert!(matches!(f.execute(action).await,Err(Error::Interrupted)));
    assert!(!f.temp.path().join("denied").exists());
}
