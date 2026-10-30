pub(crate) mod control;
mod reconciliation;
pub use reconciliation::{ReconcileReview,ReconcileDecision,preview_reconcile,reconcile_operation,acknowledge_request};
mod storage;
mod restore;
pub use restore::{RestoreReview,RestoreReport,preview_restore,restore_state};
use crate::{config::Project, domain::*, error::{Error, Result}, sessions::Session, tools::ToolProposal};
use cap_std::fs::{Dir, OpenOptions, OpenOptionsExt, Permissions, PermissionsExt, MetadataExt};
use rusqlite::{params, Connection, OpenFlags, Transaction};
use serde::Serialize;
use std::{os::fd::AsRawFd, path::Path, thread::{self, JoinHandle}, time::{Duration, SystemTime, UNIX_EPOCH}};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub(crate) const SCHEMA_VERSION:u32=5;
const DB: &str = "ledger.sqlite";
const LOCK: &str = "owner.lock";
const QUEUE: usize = 128;

struct StateRoot { dir: Dir, _lock: std::fs::File }
impl StateRoot {
    fn acquire(project: &Project, path: &Path, create: bool) -> Result<Option<Self>> {
        let path = project.relative(path)?;
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let flags=rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC;
        let resolve=rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV;
        let fd=match rustix::fs::openat2(&project.dir,parent,flags,rustix::fs::Mode::empty(),resolve){
            Ok(fd)=>fd,Err(rustix::io::Errno::NOENT) if !create=>return Ok(None),Err(_)=>return Err(Error::State),
        };
        let parent=Dir::from_std_file(std::fs::File::from(fd));let name=path.file_name().ok_or(Error::Invalid)?;
        if parent.symlink_metadata(name).is_err(){
            if !create{return Ok(None);}
            rustix::fs::mkdirat(&parent,name,rustix::fs::Mode::RUSR|rustix::fs::Mode::WUSR|rustix::fs::Mode::XUSR).map_err(|_|Error::State)?;
        }
        let fd=rustix::fs::openat2(&parent,name,flags,rustix::fs::Mode::empty(),resolve).map_err(|_|Error::State)?;
        let dir=Dir::from_std_file(std::fs::File::from(fd));
        let metadata = dir.dir_metadata()?;
        if metadata.uid() != project.dir.dir_metadata()?.uid() || metadata.mode() & 0o777 != 0o700 { return Err(Error::State); }
        for name in [DB, LOCK, "ledger.sqlite-wal", "ledger.sqlite-shm", "ledger.sqlite-journal"] {
            match dir.symlink_metadata(name) {
                Ok(m) if !m.is_file() || m.nlink() != 1 || m.mode() & 0o077 != 0 || m.uid() != metadata.uid() => return Err(Error::State),
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err(Error::State),
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(create).create(create).mode(0o600).custom_flags(0o400000);
        let lock = match dir.open_with(LOCK, &options) {
            Ok(file) => file.into_std(),
            Err(_) => return Err(Error::State),
        };
        match lock.try_lock() {
            Ok(()) => (),
            Err(std::fs::TryLockError::WouldBlock) => return Err(Error::Busy),
            Err(_) => return Err(Error::State),
        }
        Ok(Some(Self { dir, _lock: lock }))
    }
    fn db_path(&self) -> String { format!("/proc/self/fd/{}/{}", self.dir.as_raw_fd(), DB) }
    fn has_remnants(&self) -> Result<bool> {
        for name in ["ledger.sqlite-wal", "ledger.sqlite-shm", "ledger.sqlite-journal"] {
            match self.dir.symlink_metadata(name) {
                Ok(_) => return Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err(Error::State),
            }
        }
        Ok(false)
    }
}

fn backup_before(root:&StateRoot,source:&Connection,target:u32)->Result<()> {
    let _bounded_source=image(&root.dir,DB)?;
    ensure_space(&root.dir,restore::backup_size(source)?+_bounded_source.size+65536)?;
    let name=format!("pre-v{target}-{}.sqlite",uuid::Uuid::new_v4());
    let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
    let file=root.dir.open_with(&name,&options)?.into_std();
    let path=format!("/proc/self/fd/{}/{}",root.dir.as_raw_fd(),name);
    let mut destination=Connection::open_with_flags(path,OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    rusqlite::backup::Backup::new(source,&mut destination)?.run_to_completion(64,Duration::from_millis(2),None)?;
    destination.pragma_update(None,"journal_mode","DELETE")?;
    let check:String=destination.query_row("PRAGMA quick_check",[],|r|r.get(0))?;
    if check!="ok"{return Err(Error::State);}
    check_schema(&destination)?;destination.close().map_err(|_|Error::State)?;
    file.sync_all()?;root.dir.try_clone()?.into_std_file().sync_all()?;restore::bundle(root,source,&name)?;Ok(())
}

pub fn verify_engine(connection: &Connection) -> Result<String> {
    let version: String = connection.query_row("SELECT sqlite_version()", [], |r| r.get(0))?;
    let parts: Vec<u32> = version.split('.').map(str::parse).collect::<std::result::Result<_, _>>().map_err(|_| Error::State)?;
    if parts.as_slice() < [3, 54, 1].as_slice() { return Err(Error::Unsupported); }
    Ok(version)
}
fn check_schema(connection: &Connection) -> Result<()> {
    let rows: i64 = connection.query_row("SELECT count(*) FROM meta", [], |r| r.get(0))?;
    if rows != 1 { return Err(Error::State); }
    let (version, project, next, epoch): (u32, String, i64, i64) = connection.query_row(
        "SELECT schema_version,project_uuid,next_event_sequence,owner_epoch FROM meta", [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    let last: i64 = connection.query_row("SELECT coalesce(max(sequence),0) FROM events", [], |r| r.get(0))?;
    if !(1..=SCHEMA_VERSION).contains(&version) || uuid::Uuid::parse_str(&project).is_err() || epoch < 0 || last.checked_add(1) != Some(next) { return Err(Error::State); }
    Ok(())
}
fn dir_admission(dir:&Dir)->Result<()> {
    for name in ["restore.json","archived.json"]{match dir.symlink_metadata(name){Ok(_)=>return Err(Error::Recovery),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>(),Err(_)=>return Err(Error::State)}}Ok(())
}
fn check_admission(db:&Connection)->Result<()> {
    let version:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    if version>=3&&db.query_row("SELECT required FROM recovery_control WHERE id=1",[],|r|r.get::<_,bool>(0))?{return Err(Error::Recovery);}Ok(())
}
/// Trusted pre-sandbox service gate. No descriptor or database reaches untrusted exec.
pub(crate) fn verify_owner(scope:&OwnerScope)->Result<()> {
    let executable=std::env::current_exe()?;
    let boundary=if executable.starts_with(Path::new(env!("CARGO_MANIFEST_DIR"))){Path::new(env!("CARGO_MANIFEST_DIR"))}else{Path::new(env!("RIVET_SOURCE_ROOT"))};
    let project=Project::open(boundary)?;
    let path=project.relative(&scope.state)?;
    let fd=rustix::fs::openat2(&project.dir,&path,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Denied)?;
    let dir=Dir::from_std_file(std::fs::File::from(fd));let metadata=dir.dir_metadata()?;
    if metadata.dev()!=scope.device||metadata.ino()!=scope.inode||metadata.mode()&0o777!=0o700{return Err(Error::Denied);}
    let file=dir.symlink_metadata(DB)?;if !file.is_file()||file.nlink()!=1||file.mode()&0o777!=0o600{return Err(Error::Denied);}
    // Active WAL reader, never immutable against the live core. Reads the latest fence.
    let db=Connection::open_with_flags(format!("/proc/self/fd/{}/{DB}",dir.as_raw_fd()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;db.busy_timeout(Duration::from_secs(2))?;
    let (project,epoch):(String,i64)=db.query_row("SELECT project_uuid,owner_epoch FROM meta",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
    if project!=scope.project.0.to_string()||epoch!=sql_integer(scope.epoch)?{return Err(Error::Denied);}check_admission(&db)?;Ok(())
}
pub(crate) fn verify_dispatch(scope:&OwnerScope,operation:OperationId)->Result<()> {
    verify_owner(scope)?;
    let executable=std::env::current_exe()?;
    let boundary=if executable.starts_with(Path::new(env!("CARGO_MANIFEST_DIR"))){Path::new(env!("CARGO_MANIFEST_DIR"))}else{Path::new(env!("RIVET_SOURCE_ROOT"))};
    let project=Project::open(boundary)?;
    let fd=rustix::fs::openat2(&project.dir,&scope.state,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Denied)?;
    let dir=Dir::from_std_file(std::fs::File::from(fd));let m=dir.dir_metadata()?;if m.dev()!=scope.device||m.ino()!=scope.inode{return Err(Error::Denied);}
    let db=Connection::open_with_flags(format!("/proc/self/fd/{}/{DB}",dir.as_raw_fd()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    let expiry:i64=db.query_row("SELECT g.expiry FROM grants g JOIN operations o ON g.operation_id=o.id WHERE o.id=?1 AND o.state='dispatched'",[operation.0.to_string()],|r|r.get(0))?;
    let now=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_|Error::Denied)?.as_secs();if expiry<=sql_integer(now)?{return Err(Error::Denied);}Ok(())
}
fn settings(connection: &Connection, writable: bool) -> Result<()> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    if writable {
        let mode: String = connection.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        if mode != "wal" { return Err(Error::State); }
        connection.pragma_update(None, "synchronous", "FULL")?;
    }
    verify_engine(connection)?;
    Ok(())
}
#[derive(Debug, Serialize)]
pub struct Inspection { pub state: &'static str, pub sqlite_version: Option<String>, pub sessions: Vec<Session> }
pub fn inspect(project: &Project, path: &Path) -> Result<Inspection> {
    let Some(root) = StateRoot::acquire(project, path, false)? else {
        return Ok(Inspection { state: "missing", sqlite_version: None, sessions: vec![] });
    };
    dir_admission(&root.dir)?;
    if root.has_remnants()? { return Err(Error::Recovery); }
    if !root.dir.symlink_metadata(DB)?.is_file() { return Err(Error::State); }
    let connection = Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro", root.db_path()), OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    settings(&connection, false)?;
    check_schema(&connection)?;check_admission(&connection)?;
    let mut statement = connection.prepare("SELECT id, created_at, config_version FROM sessions ORDER BY created_at,id")?;
    let sessions = statement.query_map([], |r| Ok(Session { id: r.get(0)?, created_at: r.get(1)?, config_version: r.get(2)? }))?.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Inspection { state: "quiescent", sqlite_version: Some(verify_engine(&connection)?), sessions })
}

#[derive(Clone, Debug)]
pub enum Transition {
    #[cfg(test)] TestBarrier{entered:std::sync::Arc<std::sync::Barrier>,release:std::sync::Arc<std::sync::Barrier>},
    Lifecycle{command:crate::lifecycle::Command},
    NewSession { id: SessionId },
    RootTask { session: SessionId, run: RunId, task: TaskId, actor: ActorId },
    Propose { run: RunId, proposal: ToolProposal },
    Deny { operation: OperationId },
    Authorize { operation: OperationId, grant: String, expiry: u64, scope: String, permission_epoch: u64 },
    Intent { operation: OperationId },
    PreparedIntent {operation:OperationId,intent:crate::recovery::Intent},
    BindWorker {operation:OperationId,unit:String,description:String},
    AttachArtifact {operation:OperationId,reference:crate::artifacts::ArtifactRef},
    Dispatched { operation: OperationId },
    Receipt { receipt: ExecutionReceipt },
    Terminal { operation: OperationId },
    CompleteRun {run:RunId,state:String},
    InitBudget {run:RunId, limits:crate::budget::BudgetLimits},
    ReserveInference {run:RunId,request:RequestId,tokens:u64,cost:u64,digest:String,permission_epoch:u64},
    SettleInference {request:RequestId,usage:Option<(u64,u64)>},
}
#[derive(Clone, Debug)]
pub struct LedgerCommand { pub schema_version: u32, pub owner_epoch: u64, pub expected_revision: u64, pub transition: Transition }
struct Pending { command: LedgerCommand, reply: oneshot::Sender<Result<()>> }

#[derive(Clone,Debug,Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerScope {state:std::path::PathBuf,device:u64,inode:u64,project:ProjectId,epoch:u64}
pub struct Ledger {
    tx: mpsc::Sender<Pending>,
    stop: CancellationToken,
    worker: Option<JoinHandle<Result<()>>>,
    epoch: u64,
    project_id: ProjectId,
    workspace_identity: (u64, u64),
    artifacts:Option<std::sync::Arc<crate::artifacts::Store>>,
    owner_scope:Option<OwnerScope>,
}
#[derive(Clone)]
pub(crate) struct LedgerPort {tx:mpsc::Sender<Pending>,stop:CancellationToken,epoch:u64,scope:Option<OwnerScope>}
impl LedgerPort {
    pub(crate) fn cancellation(&self)->CancellationToken{self.stop.clone()}
    pub(crate) fn owner_scope(&self)->Result<OwnerScope>{self.scope.clone().ok_or(Error::State)}
    pub(crate) async fn transition(&self,revision:u64,transition:Transition)->Result<()> {
        let (reply,response)=oneshot::channel();
        let pending=Pending{command:LedgerCommand{schema_version:1,owner_epoch:self.epoch,expected_revision:revision,transition},reply};
        tokio::select!{biased;_=self.stop.cancelled()=>return Err(Error::Interrupted),result=self.tx.send(pending)=>result.map_err(|_|Error::State)?}
        response.await.map_err(|_|Error::State)?
    }
}
impl Ledger {
    pub(crate) fn port(&self)->LedgerPort{LedgerPort{tx:self.tx.clone(),stop:self.stop.clone(),epoch:self.epoch,scope:self.owner_scope.clone()}}
    pub fn open(project: &Project, path: &Path) -> Result<Self> {
        let metadata = project.dir.dir_metadata()?;
        let workspace_identity = (metadata.dev(), metadata.ino());
        let root = StateRoot::acquire(project, path, true)?.ok_or(Error::State)?;
        // Orphan journals are recovery evidence even when the main DB is absent.
        dir_admission(&root.dir)?;
        if root.has_remnants()? { return Err(Error::Recovery); }
        let existed = root.dir.symlink_metadata(DB).is_ok();
        if existed {
            // Reject unknown schema without even transient WAL/SHM creation.
            let check = Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro", root.db_path()), OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
            settings(&check, false)?;
            check_schema(&check)?;check_admission(&check)?;verify_referenced_artifacts(&check,&root.dir)?;
            let version:u32=check.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
            let identity:String=check.query_row("SELECT project_uuid FROM meta",[],|r|r.get(0))?;
            storage::identity(&root.dir,ProjectId(uuid::Uuid::parse_str(&identity).map_err(|_|Error::State)?))?;
            storage::backfill_workers(&check,&root.dir)?;
            if version<SCHEMA_VERSION {backup_before(&root,&check,SCHEMA_VERSION)?;qualification_exit("migration-backup");}
            check.close().map_err(|_| Error::State)?;
        }
        if !existed&&root.dir.entries()?.any(|entry|entry.map_or(true,|e|e.file_name()!=std::ffi::OsStr::new(LOCK))){return Err(Error::Recovery);}
        if !existed {
            let mut options = OpenOptions::new(); options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
            root.dir.open_with(DB, &options)?;
        }
        let mut connection = Connection::open_with_flags(root.db_path(), OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Reject unknown schema before changing journaling or owner state.
        if existed { check_schema(&connection)?;check_admission(&connection)?; }
        settings(&connection, true)?;
        if !existed {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("schema.sql"))?;
            transaction.execute_batch(include_str!("budget.sql"))?;
            transaction.execute_batch(include_str!("recovery.sql"))?;
            transaction.execute_batch(include_str!("reconciliation.sql"))?;
            transaction.execute_batch(include_str!("lifecycle.sql"))?;
            transaction.execute("INSERT INTO meta VALUES(?1,?2,1,0)", params![SCHEMA_VERSION,ProjectId::default().0.to_string()])?;
            audit(&transaction, "project", "system", "initialized")?;
            transaction.commit()?;
        }
        let transaction = connection.transaction()?;
        let version:u32=transaction.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
        if version==1 {
            transaction.execute_batch(include_str!("budget.sql"))?;
            transaction.execute("UPDATE meta SET schema_version=2",[])?;
            audit(&transaction,"project","system","migrated-v1-v2")?;
        }
        if version<3 {
            transaction.execute_batch(include_str!("recovery.sql"))?;
            transaction.execute("UPDATE meta SET schema_version=3",[])?;
            audit(&transaction,"project","system","migrated-v2-v3")?;
        }
        if version<4{transaction.execute_batch(include_str!("reconciliation.sql"))?;transaction.execute("UPDATE meta SET schema_version=4",[])?;audit(&transaction,"project","system","migrated-v3-v4")?;}
        if version<5{transaction.execute_batch(include_str!("lifecycle.sql"))?;transaction.execute("UPDATE meta SET schema_version=5",[])?;audit(&transaction,"project","system","migrated-v4-v5")?;}
        transaction.execute("UPDATE run_lifecycle SET status='recovery-required',revision=revision+1 WHERE status='active'",[])?;
        transaction.execute("UPDATE runs SET state='recovery-required',revision=revision+1 WHERE id IN(SELECT run_id FROM run_lifecycle WHERE status='recovery-required')",[])?;
        transaction.execute("UPDATE meta SET owner_epoch=owner_epoch+1", [])?;
        let epoch: u64 = transaction.query_row("SELECT owner_epoch FROM meta", [], |r| r.get::<_, i64>(0))?.try_into().map_err(|_| Error::State)?;
        audit(&transaction, "project", "system", "owner-acquired")?;
        qualification_exit("migration-before-commit");
        transaction.commit()?;
        qualification_exit("migration-after-commit");
        let project_uuid: String = connection.query_row("SELECT project_uuid FROM meta", [], |r|r.get(0))?;
        let project_id = ProjectId(uuid::Uuid::parse_str(&project_uuid).map_err(|_| Error::State)?);
        storage::identity(&root.dir,project_id)?;storage::backfill_workers(&connection,&root.dir)?;
        let state_identity=root.dir.dir_metadata()?;
        let relative=project.path().strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR"))).or_else(|_|project.path().strip_prefix(Path::new(env!("RIVET_SOURCE_ROOT")))).map_err(|_|Error::Invalid)?.join(project.relative(path)?);
        let owner_scope=Some(OwnerScope{state:relative,device:state_identity.dev(),inode:state_identity.ino(),project:project_id,epoch});
        let artifacts=Some(std::sync::Arc::new(crate::artifacts::Store::open(&root.dir,true)?.ok_or(Error::State)?));
        let (tx, mut rx) = mpsc::channel::<Pending>(QUEUE);
        let stop = CancellationToken::new(); let control = stop.clone();
        let worker = thread::spawn(move || {
            // Root and lock outlive SQLite, including clean checkpoint/close.
            let _root = root;
            while !control.is_cancelled() {
                match rx.try_recv() {
                    Ok(pending) => {
                        let result = apply(&mut connection, pending.command, epoch,&_root.dir);
                        let failed = matches!(result, Err(Error::State));
                        let _ = pending.reply.send(result);
                        if failed { control.cancel(); }
                    },
                    Err(mpsc::error::TryRecvError::Empty) => thread::sleep(Duration::from_millis(2)),
                    Err(mpsc::error::TryRecvError::Disconnected) => break,
                }
            }
            rx.close();
            while let Ok(pending) = rx.try_recv() { let _ = pending.reply.send(Err(Error::Interrupted)); }
            let checkpoint = connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get::<_, i64>(0));
            // Even a failed checkpoint must close SQLite before releasing the root/lock.
            let closed = connection.close();
            if let Err((connection, _)) = closed { drop(connection); return Err(Error::State); }
            if checkpoint? != 0 { return Err(Error::State); }
            Ok(())
        });
        Ok(Self { tx, stop, worker: Some(worker), epoch, project_id, workspace_identity,artifacts,owner_scope })
    }
    pub fn project_id(&self) -> ProjectId { self.project_id }
    pub(crate) fn workspace_identity(&self) -> (u64, u64) { self.workspace_identity }
    pub fn epoch(&self) -> u64 { self.epoch }
    pub async fn command(&self, command: LedgerCommand) -> Result<()> {
        let (reply, response) = oneshot::channel();
        tokio::select! {
            biased;
            _ = self.stop.cancelled() => return Err(Error::Interrupted),
            result = self.tx.send(Pending { command, reply }) => result.map_err(|_| Error::State)?,
        }
        // Once accepted, await the writer's definitive commit/rollback reply.
        // Cancellation closes admission and drains queued replies; it cannot erase
        // an in-flight commit result or imply that a committed effect did not occur.
        response.await.map_err(|_| Error::State)?
    }
    pub async fn transition(&self, revision: u64, transition: Transition) -> Result<()> {
        self.command(LedgerCommand { schema_version: 1, owner_epoch: self.epoch, expected_revision: revision, transition }).await
    }
    pub async fn retain_artifact(&self,operation:OperationId,bytes:Vec<u8>,media_type:&str)->Result<crate::artifacts::ArtifactRef>{
        if bytes.len()>64*1024*1024{return Err(Error::Unsupported);}
        let store=self.artifacts.clone().ok_or(Error::State)?;let media=media_type.to_owned();
        let reference=tokio::task::spawn_blocking(move||store.put(&bytes,&media)).await.map_err(|_|Error::State)??;
        self.transition(0,Transition::AttachArtifact{operation,reference:reference.clone()}).await?;Ok(reference)
    }
    pub fn verify_artifact(&self,reference:&crate::artifacts::ArtifactRef)->Result<Vec<u8>>{self.artifacts.as_ref().ok_or(Error::State)?.read(reference)}
    pub fn cancellation(&self) -> CancellationToken { self.stop.clone() }
    pub fn shutdown(mut self) -> Result<()> { self.stop.cancel(); self.join() }
    fn join(&mut self) -> Result<()> { self.worker.take().map_or(Ok(()), |w| w.join().map_err(|_| Error::State)?) }
}
impl Drop for Ledger { fn drop(&mut self) { self.stop.cancel(); let _ = self.join(); } }
pub(crate) fn audit(tx: &Transaction<'_>, entity: &str, actor: &str, kind: &str) -> Result<()> {
    tx.execute("INSERT INTO events SELECT next_event_sequence,?1,?2,?3,'{}' FROM meta", params![entity,actor,kind])?;
    tx.execute("UPDATE meta SET next_event_sequence=next_event_sequence+1", [])?; Ok(())
}
fn advance(tx: &Transaction<'_>, operation: OperationId, revision: u64, from: &str, to: &str) -> Result<()> {
    let changed = tx.execute("UPDATE operations SET state=?1,revision=revision+1 WHERE id=?2 AND state=?3 AND revision=?4", params![to,operation.0.to_string(),from,sql_integer(revision)?])?;
    if changed != 1 { return Err(Error::Conflict); }
    audit(tx, &operation.0.to_string(), "supervisor", to)
}
fn apply(connection: &mut Connection, command: LedgerCommand, epoch: u64,state:&Dir) -> Result<()> {
    if command.schema_version != 1 || command.owner_epoch != epoch { return Err(Error::Conflict); }
    if !matches!(&command.transition,Transition::Lifecycle{command:crate::lifecycle::Command::Inspect{..}}){ensure_space(state,64*1024)?;}
    let tx = connection.transaction()?;
    let actual: i64 = tx.query_row("SELECT owner_epoch FROM meta", [], |r| r.get(0))?;
    if actual != sql_integer(epoch)? { return Err(Error::Conflict); }
    let revision = command.expected_revision;
    match command.transition {
        #[cfg(test)] Transition::TestBarrier{entered,release}=>{entered.wait();release.wait();},
        Transition::Lifecycle{command}=>crate::lifecycle::apply(&tx,command,revision,epoch)?,
        Transition::NewSession { id } => {
            if revision != 0 { return Err(Error::Conflict); }
            let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| Error::State)?.as_secs();
            tx.execute("INSERT INTO sessions VALUES(?1,?2,1)", params![id.0.to_string(),sql_integer(now)?])?;
            audit(&tx, &id.0.to_string(), "user", "session-created")?;
        },
        Transition::RootTask { session, run, task, actor } => {
            if revision != 0 { return Err(Error::Conflict); }
            tx.execute("INSERT INTO runs VALUES(?1,?2,'solo','active',?3,0)", params![run.0.to_string(),session.0.to_string(),sql_integer(epoch)?])?;
            tx.execute("INSERT INTO agents VALUES(?1,?2,'independent')", params![actor.0.to_string(),run.0.to_string()])?;
            tx.execute("INSERT INTO tasks VALUES(?1,?2,?3,'active',0)", params![task.0.to_string(),run.0.to_string(),actor.0.to_string()])?;
            audit(&tx, &run.0.to_string(), "supervisor", "run-created")?;
        },
        Transition::Propose { run, proposal: p } => {
            if revision != 0 || p.schema_version != 1 { return Err(Error::Conflict); }
            // Bind actor/task/run membership, beyond independent foreign keys.
            let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks JOIN agents ON tasks.owner_agent_id=agents.id JOIN runs ON tasks.run_id=runs.id WHERE tasks.id=?1 AND tasks.run_id=?2 AND agents.id=?3 AND agents.run_id=?2 AND runs.state='active' AND runs.owner_epoch=?4 AND NOT EXISTS(SELECT 1 FROM run_lifecycle l WHERE l.run_id=runs.id AND l.status!='active'))", params![p.task.0.to_string(),run.0.to_string(),p.actor.0.to_string(),sql_integer(epoch)?], |r| r.get(0))?;
            if !valid { return Err(Error::Conflict); }
            tx.execute("INSERT INTO operations VALUES(?1,?2,?3,?4,?5,?6,'proposed',0)", params![p.operation.0.to_string(),run.0.to_string(),p.task.0.to_string(),p.actor.0.to_string(),p.descriptor_digest,p.digest()?])?;
            audit(&tx, &p.operation.0.to_string(), "provider", "proposed")?;
        },
        Transition::Deny { operation } => advance(&tx, operation, revision, "proposed", "denied")?,
        Transition::Authorize { operation, grant, expiry, scope, permission_epoch } => {
            advance(&tx, operation, revision, "proposed", "authorized")?;
            tx.execute("INSERT INTO grants VALUES(?1,?2,?3,?4,?5)", params![grant,operation.0.to_string(),sql_integer(permission_epoch)?,sql_integer(expiry)?,scope])?;
        },
        Transition::Intent { operation } => advance(&tx, operation, revision, "authorized", "intent-recorded")?,
        Transition::PreparedIntent{operation,intent}=>{
            advance(&tx,operation,revision,"authorized","intent-recorded")?;
            tx.execute("INSERT INTO operation_intents VALUES(?1,?2,?3)",params![operation.0.to_string(),intent.class(),intent.encode()?])?;
        },
        Transition::AttachArtifact{operation,reference}=>{
            if revision!=0{return Err(Error::Conflict);}
            let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1 AND state='dispatched')",[operation.0.to_string()],|r|r.get(0))?;
            if !valid{return Err(Error::Conflict);}
            tx.execute("INSERT INTO artifacts VALUES(?1,?2,?3) ON CONFLICT(digest) DO NOTHING",params![reference.digest,sql_integer(reference.size)?,reference.media_type])?;
            let actual:(i64,String)=tx.query_row("SELECT size,media_type FROM artifacts WHERE digest=?1",[&reference.digest],|r|Ok((r.get(0)?,r.get(1)?)))?;
            if actual!=(sql_integer(reference.size)?,reference.media_type){return Err(Error::Conflict);}
            tx.execute("INSERT INTO operation_artifacts VALUES(?1,?2)",params![operation.0.to_string(),reference.digest])?;
            audit(&tx,&operation.0.to_string(),"supervisor","artifact-linked")?;
        },
        Transition::BindWorker{operation,unit,description}=>{
            if revision!=0{return Err(Error::Invalid);}crate::recovery::validate_worker(operation,&unit,&description)?;
            let dispatched:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1 AND state='dispatched')",[operation.0.to_string()],|r|r.get(0))?;
            if !dispatched{return Err(Error::Conflict);}
            storage::worker(state,&storage::Worker{operation,unit:unit.clone(),description:description.clone(),epoch})?;
            tx.execute("INSERT INTO owned_workers VALUES(?1,?2,?3,?4)",params![operation.0.to_string(),unit,description,sql_integer(epoch)?])?;
            audit(&tx,&operation.0.to_string(),"supervisor","worker-bound")?;
        },
        Transition::Dispatched { operation } => advance(&tx, operation, revision, "intent-recorded", "dispatched")?,
        Transition::Receipt { receipt } => {
            if receipt.schema_version != 1 { return Err(Error::Invalid); }
            advance(&tx, receipt.operation, revision, "dispatched", match receipt.outcome { Outcome::Unknown => "outcome-unknown", Outcome::Cancelled => "cancelled", _ => "receipt-recorded" })?;
            let payload = serde_json::to_string(&receipt).map_err(|_| Error::Invalid)?;
            tx.execute("UPDATE events SET payload=?1 WHERE sequence=(SELECT next_event_sequence-1 FROM meta)", [payload])?;
        },
        Transition::Terminal { operation } => advance(&tx, operation, revision, "receipt-recorded", "terminal")?,
        Transition::CompleteRun {run,state}=>{
            let controlled:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM run_lifecycle WHERE run_id=?1)",[run.0.to_string()],|r|r.get(0))?;
            if controlled||state=="completed"&&crate::lifecycle::uncertain(&tx,run)?{return Err(Error::Denied);}
            if revision!=0||!["completed","stopped"].contains(&state.as_str()){return Err(Error::Invalid);}
            let n=tx.execute("UPDATE runs SET state=?1,revision=revision+1 WHERE id=?2 AND state='active' AND revision=0",params![state,run.0.to_string()])?;
            if n!=1{return Err(Error::Conflict);}audit(&tx,&run.0.to_string(),"supervisor",&format!("run-{state}"))?;
        },
        Transition::InitBudget {run,limits}=>{
            limits.validate()?;if revision!=0{return Err(Error::Conflict);}
            tx.execute("INSERT INTO run_budgets(run_id,token_limit,request_limit,cost_limit) VALUES(?1,?2,?3,?4)",params![run.0.to_string(),sql_integer(limits.tokens)?,limits.requests,sql_integer(limits.cost_micros)?])?;
            audit(&tx,&run.0.to_string(),"user","budget-initialized")?;
        },
        Transition::ReserveInference {run,request,tokens,cost,digest,permission_epoch}=>{
            if revision!=0||tokens==0||digest.len()!=64||!digest.bytes().all(|b|b.is_ascii_hexdigit())||permission_epoch==0{return Err(Error::Invalid);}
            let changed=tx.execute("UPDATE run_budgets SET token_reserved=token_reserved+?1,cost_reserved=cost_reserved+?2,requests=requests+1 WHERE run_id=?3 AND EXISTS(SELECT 1 FROM runs WHERE id=?3 AND state='active' AND owner_epoch=?4 AND NOT EXISTS(SELECT 1 FROM run_lifecycle l WHERE l.run_id=runs.id AND l.status!='active')) AND requests<request_limit AND ?1<=token_limit-token_charged-token_reserved AND ?2<=cost_limit-cost_charged-cost_reserved",params![sql_integer(tokens)?,sql_integer(cost)?,run.0.to_string(),sql_integer(epoch)?])?;
            if changed!=1{return Err(Error::Denied);}
            tx.execute("INSERT INTO inference_reservations(request_id,run_id,tokens,cost,request_digest,permission_epoch,state) VALUES(?1,?2,?3,?4,?5,?6,'reserved')",params![request.0.to_string(),run.0.to_string(),sql_integer(tokens)?,sql_integer(cost)?,digest,sql_integer(permission_epoch)?])?;
            audit(&tx,&request.0.to_string(),"supervisor","inference-reserved")?;
        },
        Transition::SettleInference {request,usage}=>{
            if revision!=0{return Err(Error::Conflict);}
            let (run,tokens,cost,state):(String,i64,i64,String)=tx.query_row("SELECT run_id,tokens,cost,state FROM inference_reservations WHERE request_id=?1",[request.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
            if state!="reserved"{return Err(Error::Conflict);}
            let tokens:u64=tokens.try_into().map_err(|_|Error::State)?;
            let cost:u64=cost.try_into().map_err(|_|Error::State)?;
            match usage {
                Some((input,output)) if input.checked_add(output).is_some_and(|total|total<=tokens)=>{
                    // Cost is conservatively charged at its approved upper bound; no invented price/rate.
                    tx.execute("UPDATE run_budgets SET token_reserved=token_reserved-?1,token_charged=token_charged+?2,cost_reserved=cost_reserved-?3,cost_charged=cost_charged+?3 WHERE run_id=?4",params![sql_integer(tokens)?,sql_integer(input+output)?,sql_integer(cost)?,run])?;
                    tx.execute("UPDATE inference_reservations SET state='settled',input_tokens=?1,output_tokens=?2 WHERE request_id=?3",params![sql_integer(input)?,sql_integer(output)?,request.0.to_string()])?;
                    audit(&tx,&request.0.to_string(),"provider","inference-settled")?;
                },
                _=>{
                    tx.execute("UPDATE inference_reservations SET state='unknown' WHERE request_id=?1",[request.0.to_string()])?;
                    audit(&tx,&request.0.to_string(),"supervisor","inference-usage-unknown")?;
                }
            }
        },
    }
    tx.commit()?; Ok(())
}

fn sql_integer(value: u64) -> Result<i64> { value.try_into().map_err(|_| Error::Invalid) }

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::sync::Arc;
    #[tokio::test]
    async fn accepted_command_keeps_definitive_reply_after_cancellation() {
        let (tx, mut rx) = mpsc::channel(1);
        let ledger = Arc::new(Ledger { tx, stop: CancellationToken::new(), worker: None, epoch: 1, project_id: ProjectId::default(), workspace_identity: (0,0),artifacts:None,owner_scope:None });
        let caller = ledger.clone();
        let task = tokio::spawn(async move { caller.transition(0, Transition::NewSession { id: SessionId::default() }).await });
        let pending = rx.recv().await.unwrap();
        ledger.cancellation().cancel();
        tokio::task::yield_now().await;
        assert!(!task.is_finished(), "cancellation hid an accepted command's outcome");
        pending.reply.send(Ok(())).unwrap();
        task.await.unwrap().unwrap();
        assert!(matches!(ledger.transition(0, Transition::NewSession { id: SessionId::default() }).await, Err(Error::Interrupted)));
    }
}

/// Quiescent session continuation exports only bounded evidence references, never raw transcripts.
pub fn continuation(project:&Project,path:&Path,session:SessionId)->Result<String>{
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;
    dir_admission(&root.dir)?;
    if root.has_remnants()?{return Err(Error::Recovery);}
    let connection=Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro",root.db_path()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_URI|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    settings(&connection,false)?;check_schema(&connection)?;
    let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",[session.0.to_string()],|r|r.get(0))?;
    if !exists{return Err(Error::Invalid);}
    let schema:u32=connection.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    if schema>=2 {
        let query=if schema>=4{"SELECT EXISTS(SELECT 1 FROM inference_reservations i JOIN runs r ON i.run_id=r.id WHERE r.session_id=?1 AND i.state IN ('reserved','unknown') AND NOT EXISTS(SELECT 1 FROM recovery_resolutions x WHERE x.entity_id=i.request_id AND x.entity_kind='request' AND x.source_digest=i.request_digest))"}else{"SELECT EXISTS(SELECT 1 FROM inference_reservations i JOIN runs r ON i.run_id=r.id WHERE r.session_id=?1 AND i.state IN ('reserved','unknown'))"};
        let uncertain:bool=connection.query_row(query,[session.0.to_string()],|r|r.get(0))?;
        if uncertain{return Err(Error::Recovery);}
    }
    let uncertain:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM operations o JOIN runs r ON o.run_id=r.id WHERE r.session_id=?1 AND o.state IN ('intent-recorded','dispatched','outcome-unknown','cancelled'))",[session.0.to_string()],|r|r.get(0))?;
    if uncertain{return Err(Error::Recovery);}
    let mut statement=connection.prepare("SELECT e.sequence,e.kind,e.payload,e.entity_id FROM events e JOIN operations o ON e.entity_id=o.id JOIN runs r ON o.run_id=r.id WHERE r.session_id=?1 AND e.kind IN ('receipt-recorded','cancelled','outcome-unknown','reconciled-observed','abandoned-unknown') ORDER BY e.sequence DESC LIMIT 8")?;
    let rows=statement.query_map([session.0.to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))?;
    let mut output=vec![];
    for row in rows{let (sequence,kind,payload,operation)=row?;
        if kind=="receipt-recorded" {let receipt:ExecutionReceipt=serde_json::from_str(&payload).map_err(|_|Error::State)?;output.push(serde_json::json!({"sequence":sequence,"kind":kind,"operation":receipt.operation,"outcome":receipt.outcome,"before":receipt.before_revision,"after":receipt.after_revision}));}
        else {output.push(serde_json::json!({"sequence":sequence,"kind":kind,"operation":operation,"provenance":"recovery-or-user-disposition","causation_proven":false}));}
    }
    let result=serde_json::to_string(&output).map_err(|_|Error::State)?;if result.len()>8192{return Err(Error::Unsupported);}Ok(result)
}

#[derive(Clone,Debug,Serialize,serde::Deserialize)]
pub struct StateImage {pub name:String,pub digest:String,pub size:u64,pub device:u64,pub inode:u64}
#[derive(Debug,Serialize)]
pub struct PendingEffect {pub operation:String,pub state:String,pub revision:i64,pub intent:Option<crate::recovery::Intent>}
#[derive(Debug,Serialize)]
pub struct RepairReport {pub snapshot_digest:String,pub images:Vec<StateImage>,pub sqlite_recovery_required:bool,pub control_recovery_required:bool,pub schema_version:Option<u32>,pub integrity:String,pub pending:Vec<PendingEffect>,pub owned_workers:Vec<serde_json::Value>,pub artifacts:Vec<serde_json::Value>,pub orphan_blobs:Vec<String>}
fn image(root:&Dir,name:&str)->Result<StateImage>{
    use std::os::unix::fs::MetadataExt;
    use std::io::Read;
    let fd=rustix::fs::openat2(root,name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::CLOEXEC|rustix::fs::OFlags::NOFOLLOW|rustix::fs::OFlags::NONBLOCK,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
    let mut file=std::fs::File::from(fd);let m=file.metadata()?;
    if !m.is_file()||m.nlink()!=1||m.mode()&0o777!=0o600||m.uid()!=root.dir_metadata()?.uid()||m.len()>64*1024*1024{return Err(Error::State);}
    let mut bytes=vec![];(&mut file).take(64*1024*1024+1).read_to_end(&mut bytes)?;let after=file.metadata()?;
    if m.len()!=after.len()||m.mtime()!=after.mtime()||m.mtime_nsec()!=after.mtime_nsec()||bytes.len() as u64!=m.len(){return Err(Error::Conflict);}
    Ok(StateImage{name:name.into(),digest:crate::tools::sha256(&bytes),size:m.len(),device:m.dev(),inode:m.ino()})
}
fn images(root:&StateRoot)->Result<Vec<StateImage>>{
    let mut images=vec![];for name in [DB,"ledger.sqlite-wal","ledger.sqlite-shm","ledger.sqlite-journal"]{
        match root.dir.symlink_metadata(name){Ok(_)=>images.push(image(&root.dir,name)?),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>(),Err(_)=>return Err(Error::State)}
    }Ok(images)
}
fn snapshot_digest(images:&[StateImage])->Result<String>{serde_json::to_vec(images).map(|v|crate::tools::sha256(&v)).map_err(|_|Error::State)}
fn immutable(root:&StateRoot)->Result<Connection>{
    dir_admission(&root.dir)?;
    if root.has_remnants()?{return Err(Error::Recovery);}
    let db=Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro",root.db_path()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_URI|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;settings(&db,false)?;Ok(db)
}
/// Does not replay, recover SQLite, create files, remove orphan blobs or read their contents.
pub fn repair_inspect(project:&Project,path:&Path)->Result<RepairReport>{
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;let images=images(&root)?;let digest=snapshot_digest(&images)?;
    let mut report=RepairReport{snapshot_digest:digest,images,sqlite_recovery_required:root.has_remnants()?,control_recovery_required:false,schema_version:None,integrity:"not-inspected".into(),pending:vec![],owned_workers:vec![],artifacts:vec![],orphan_blobs:vec![]};
    if dir_admission(&root.dir).is_err(){report.integrity="restore-or-archive".into();return Ok(report);}
    if report.sqlite_recovery_required{return Ok(report);}
    let db=immutable(&root)?;
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;report.schema_version=Some(schema);
    if !(1..=SCHEMA_VERSION).contains(&schema){report.integrity="unsupported-schema".into();return Ok(report);}
    check_schema(&db)?;
    let integrity:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))?;
    if integrity!="ok"{report.integrity="corrupt".into();return Ok(report);}
    let violations:bool=db.prepare("PRAGMA foreign_key_check")?.query([])?.next()?.is_some();
    if violations{report.integrity="foreign-key-violation".into();return Ok(report);}
    report.integrity="ok".into();
    if schema>=3{report.control_recovery_required=db.query_row("SELECT required FROM recovery_control WHERE id=1",[],|r|r.get(0))?;if report.control_recovery_required{report.integrity="recovery-required".into();}}
    let query=if schema>=3{"SELECT o.id,o.state,o.revision,i.metadata FROM operations o LEFT JOIN operation_intents i ON o.id=i.operation_id WHERE o.state NOT IN ('terminal','denied') ORDER BY o.id LIMIT 1025"}else{"SELECT id,state,revision,NULL FROM operations WHERE state NOT IN ('terminal','denied') ORDER BY id LIMIT 1025"};
    let mut statement=db.prepare(query)?;
    let rows=statement.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<String>>(3)?)))?;
    for row in rows{let (operation,state,revision,intent)=row?;if report.pending.len()>=1024{return Err(Error::Unsupported);}report.pending.push(PendingEffect{operation,state,revision,intent:intent.map(|s|serde_json::from_str(&s).map_err(|_|Error::State)).transpose()?});}
    if schema>=3{
        let mut statement=db.prepare("SELECT operation_id,unit,description,owner_epoch FROM owned_workers WHERE operation_id IN (SELECT id FROM operations WHERE state NOT IN ('terminal','denied')) LIMIT 1025")?;
        for row in statement.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?)))?{
            let (operation,unit,description,epoch)=row?;crate::recovery::validate_worker(OperationId(uuid::Uuid::parse_str(&operation).map_err(|_|Error::State)?),&unit,&description)?;
            if report.owned_workers.len()>=1024{return Err(Error::Unsupported);}report.owned_workers.push(serde_json::json!({"operation":operation,"unit":unit,"description":description,"owner_epoch":epoch,"liveness":"not-probed"}));
        }
        let store=crate::artifacts::Store::open(&root.dir,false)?;let mut referenced=std::collections::BTreeSet::new();let mut total=0u64;
        let mut statement=db.prepare("SELECT digest,size,media_type FROM artifacts LIMIT 1025")?;
        for row in statement.query_map([],|r|Ok(crate::artifacts::ArtifactRef{digest:r.get(0)?,size:r.get::<_,i64>(1)?.try_into().map_err(|_|rusqlite::Error::InvalidQuery)?,media_type:r.get(2)?}))?{
            let reference=row?;total=total.checked_add(reference.size).ok_or(Error::Unsupported)?;if total>256*1024*1024||referenced.len()>=1024{return Err(Error::Unsupported);}
            let valid=store.as_ref().is_some_and(|s|s.read(&reference).is_ok());referenced.insert(reference.digest.clone());
            report.artifacts.push(serde_json::json!({"reference":reference,"integrity":if valid{"verified"}else{"missing-or-corrupt"}}));
            if !valid{report.integrity="artifact-failure".into();}
        }
        if let Some(store)=store{for name in store.names()?{if !referenced.contains(&name){report.orphan_blobs.push(name);}}}
    }
    Ok(report)
}
#[derive(Debug,Serialize)]
pub struct BackupRef {pub name:String,pub digest:String,pub size:u64,pub schema_version:u32}
fn verify_referenced_artifacts(db:&Connection,root:&Dir)->Result<()> {
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;if schema<3{return Ok(());}
    let store=crate::artifacts::Store::open(root,false)?;let mut total=0u64;let mut count=0;
    let mut statement=db.prepare("SELECT digest,size,media_type FROM artifacts LIMIT 1025")?;
    for row in statement.query_map([],|r|Ok(crate::artifacts::ArtifactRef{digest:r.get(0)?,size:r.get::<_,i64>(1)?.try_into().map_err(|_|rusqlite::Error::InvalidQuery)?,media_type:r.get(2)?}))?{
        let reference=row?;count+=1;total=total.checked_add(reference.size).ok_or(Error::Unsupported)?;
        if count>1024||total>256*1024*1024{return Err(Error::Unsupported);}store.as_ref().ok_or(Error::State)?.read(&reference)?;
    }Ok(())
}
/// Explicit snapshot-bound, quiescent database and referenced-artifact backup; no overwrite.
pub fn backup_state(project:&Project,path:&Path,reviewed_snapshot:&str)->Result<BackupRef>{
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;
    if snapshot_digest(&images(&root)?)?!=reviewed_snapshot{return Err(Error::Denied);}
    let source=immutable(&root)?;check_schema(&source)?;check_admission(&source)?;verify_referenced_artifacts(&source,&root.dir)?;
    let integrity:String=source.query_row("PRAGMA quick_check",[],|r|r.get(0))?;if integrity!="ok"{return Err(Error::State);}
    ensure_space(&root.dir,restore::backup_size(&source)?+image(&root.dir,DB)?.size+65536)?;
    let name=format!("backup-{}.sqlite",uuid::Uuid::new_v4());let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
    let file=root.dir.open_with(&name,&options)?.into_std();
    let mut destination=Connection::open_with_flags(format!("/proc/self/fd/{}/{name}",root.dir.as_raw_fd()),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    rusqlite::backup::Backup::new(&source,&mut destination)?.run_to_completion(64,Duration::from_millis(2),None)?;
    destination.pragma_update(None,"journal_mode","DELETE")?;
    check_schema(&destination)?;let check:String=destination.query_row("PRAGMA quick_check",[],|r|r.get(0))?;if check!="ok"{return Err(Error::State);}
    let version:u32=destination.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    destination.close().map_err(|_|Error::State)?;file.sync_all()?;root.dir.try_clone()?.into_std_file().sync_all()?;
    restore::bundle(&root,&source,&name)?;
    let image=image(&root.dir,&name)?;Ok(BackupRef{name,digest:image.digest,size:image.size,schema_version:version})
}

#[derive(Debug,Serialize)]
pub struct RecoveryReport {pub snapshot:String,pub source_digest:String,pub stopped_workers:usize,pub state:&'static str}
fn preserve_images(root:&StateRoot,expected:&[StateImage])->Result<String>{
    use std::io::{Read,Write};
    ensure_space(&root.dir,expected.iter().map(|i|i.size).sum::<u64>()+65536)?;
    let name=format!("recovery-{}",uuid::Uuid::new_v4());root.dir.create_dir(&name)?;root.dir.set_permissions(&name,Permissions::from_mode(0o700))?;
    let fd=rustix::fs::openat2(&root.dir,&name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
    let destination=Dir::from_std_file(std::fs::File::from(fd));
    for original in expected{
        if image(&root.dir,&original.name)?.digest!=original.digest{return Err(Error::Conflict);}
        let fd=rustix::fs::openat2(&root.dir,&original.name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::CLOEXEC|rustix::fs::OFlags::NOFOLLOW,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
        let mut source=std::fs::File::from(fd);let mut bytes=vec![];(&mut source).take(64*1024*1024+1).read_to_end(&mut bytes)?;
        if bytes.len() as u64!=original.size||crate::tools::sha256(&bytes)!=original.digest{return Err(Error::Conflict);}
        let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
        let mut copy=destination.open_with(&original.name,&options)?.into_std();copy.write_all(&bytes)?;copy.sync_all()?;
    }
    let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
    let mut manifest=destination.open_with("manifest.json",&options)?.into_std();manifest.write_all(&serde_json::to_vec(expected).map_err(|_|Error::State)?)?;manifest.sync_all()?;
    destination.into_std_file().sync_all()?;root.dir.try_clone()?.into_std_file().sync_all()?;
    if snapshot_digest(&images(root)?)?!=snapshot_digest(expected)?{return Err(Error::Conflict);}Ok(name)
}
/// Explicit evidence-preserving SQLite recovery. It pauses uncertain work and never replays it.
pub async fn recover_state(project:&Project,path:&Path,reviewed_snapshot:&str)->Result<RecoveryReport>{
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;dir_admission(&root.dir)?;let expected=images(&root)?;
    if expected.iter().all(|i|i.name!=DB)||snapshot_digest(&expected)?!=reviewed_snapshot{return Err(Error::Denied);}
    let snapshot=preserve_images(&root,&expected)?;qualification_exit("recovery-evidence");
    // Raw DB/WAL/SHM/journal evidence is durable before normal SQLite recovery can write.
    let mut db=Connection::open_with_flags(root.db_path(),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;settings(&db,true)?;check_schema(&db)?;
    let integrity:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))?;if integrity!="ok"{return Err(Error::State);}
    if db.prepare("PRAGMA foreign_key_check")?.query([])?.next()?.is_some(){return Err(Error::State);}
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    let project:String=db.query_row("SELECT project_uuid FROM meta",[],|r|r.get(0))?;storage::identity(&root.dir,ProjectId(uuid::Uuid::parse_str(&project).map_err(|_|Error::State)?))?;storage::backfill_workers(&db,&root.dir)?;
    {let tx=db.transaction()?;
        tx.execute("UPDATE meta SET owner_epoch=owner_epoch+1",[])?;
        if schema>=3{tx.execute("UPDATE recovery_control SET required=1 WHERE id=1",[])?;}
        audit(&tx,"project","recovery","owner-fenced-recovery-started")?;tx.commit()?;
    }
    qualification_exit("recovery-fenced");
    // Sidecars are durable before launch and include orphan bindings whose SQL commit failed.
    let workers=storage::workers(&root.dir)?;
    for worker in &workers{
        crate::tools::process::reconcile_owned(worker.operation,&worker.unit,&worker.description).await?;
    }
    qualification_exit("recovery-contained");
    let tx=db.transaction()?;
    let mut operations=vec![];
    {let mut statement=tx.prepare("SELECT id,state FROM operations WHERE state IN ('proposed','authorized','intent-recorded','dispatched','receipt-recorded') LIMIT 1025")?;for row in statement.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?{if operations.len()>=1024{return Err(Error::Unsupported);}operations.push(row?);}}
    for (id,state) in operations{
        let next=if state=="receipt-recorded"{"terminal"}else if state=="proposed"||state=="authorized"{"denied"}else{"outcome-unknown"};
        tx.execute("UPDATE operations SET state=?1,revision=revision+1 WHERE id=?2 AND state=?3",params![next,id,state])?;audit(&tx,&id,"recovery",next)?;
    }
    if schema>=2{
        let mut requests=vec![];{let mut statement=tx.prepare("SELECT request_id FROM inference_reservations WHERE state='reserved' LIMIT 1025")?;for row in statement.query_map([],|r|r.get::<_,String>(0))?{if requests.len()>=1024{return Err(Error::Unsupported);}requests.push(row?);}}
        for id in requests{tx.execute("UPDATE inference_reservations SET state='unknown' WHERE request_id=?1",[&id])?;audit(&tx,&id,"recovery","inference-usage-unknown")?;}
    }
    tx.execute("UPDATE runs SET state='recovery-required',revision=revision+1 WHERE state='active'",[])?;
    if schema>=5{tx.execute("UPDATE run_lifecycle SET status='recovery-required',revision=revision+1 WHERE status='active'",[])?;}
    if schema>=3{tx.execute("UPDATE recovery_control SET required=0 WHERE id=1",[])?;}
    audit(&tx,"project","recovery","owner-fenced-recovered")?;tx.commit()?;qualification_exit("recovery-committed");
    let busy:i64=db.query_row("PRAGMA wal_checkpoint(TRUNCATE)",[],|r|r.get(0))?;if busy!=0{return Err(Error::Busy);}db.close().map_err(|_|Error::State)?;
    root.dir.try_clone()?.into_std_file().sync_all()?;
    Ok(RecoveryReport{snapshot,source_digest:reviewed_snapshot.into(),stopped_workers:workers.len(),state:"recovered-paused-no-replay"})
}

#[cfg(test)]
mod fencing_tests {
    use super::*;
    #[test]
    fn worker_gate_reads_active_wal_and_denies_stale_owner_or_incomplete_recovery(){
        let root=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let project=Project::open(root.path()).unwrap();
        let ledger=Ledger::open(&project,Path::new("state")).unwrap();let old=ledger.port().owner_scope().unwrap();verify_owner(&old).unwrap();ledger.shutdown().unwrap();
        let ledger=Ledger::open(&project,Path::new("state")).unwrap();let current=ledger.port().owner_scope().unwrap();assert!(matches!(verify_owner(&old),Err(Error::Denied)));verify_owner(&current).unwrap();
        let db=Connection::open(root.path().join("state/ledger.sqlite")).unwrap();db.execute("UPDATE recovery_control SET required=1 WHERE id=1",[]).unwrap();drop(db);assert!(matches!(verify_owner(&current),Err(Error::Recovery)));ledger.shutdown().unwrap();
        assert!(matches!(Ledger::open(&project,Path::new("state")),Err(Error::Recovery)));
    }
}

pub(crate) fn ensure_space(root:&Dir,additional:u64)->Result<()>{restore::capacity(root,additional)}

#[cfg(test)]
mod full_tests {
    use super::*;
    #[test]
    fn sqlite_page_ceiling_rolls_back_whole_state_and_audit_transaction(){
        let root=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let project=Project::open(root.path()).unwrap();Ledger::open(&project,Path::new("state")).unwrap().shutdown().unwrap();let state=StateRoot::acquire(&project,Path::new("state"),false).unwrap().unwrap();
        let mut db=Connection::open(state.db_path()).unwrap();settings(&db,true).unwrap();let pages:i64=db.query_row("PRAGMA page_count",[],|r|r.get(0)).unwrap();db.pragma_update(None,"max_page_count",pages).unwrap();let epoch:u64=db.query_row("SELECT owner_epoch FROM meta",[],|r|Ok(r.get::<_,i64>(0)? as u64)).unwrap();
        let mut full=false;
        for _ in 0..1000{
            let before:(i64,i64)=db.query_row("SELECT (SELECT count(*) FROM sessions),next_event_sequence FROM meta",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();let id=SessionId::default();
            let result=apply(&mut db,LedgerCommand{schema_version:1,owner_epoch:epoch,expected_revision:0,transition:Transition::NewSession{id}},epoch,&state.dir);
            if result.is_err(){assert!(matches!(result,Err(Error::State)));let after:(i64,i64)=db.query_row("SELECT (SELECT count(*) FROM sessions),next_event_sequence FROM meta",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();assert_eq!(before,after);let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",[id.0.to_string()],|r|r.get(0)).unwrap();assert!(!exists);full=true;break;}
        }assert!(full,"SQLite fixture must actually reach SQLITE_FULL");
    }
}

/// Faults exist only in test binaries; production never accepts environment-controlled exits.
pub(crate) fn qualification_exit(_point:&str){
    #[cfg(test)] if std::env::var("RIVET_QUALIFICATION_EXIT").ok().as_deref()==Some(_point){std::process::exit(91);}
}
#[cfg(test)]
mod crash_boundaries {
    use super::*;
    #[test]
    fn child(){
        let Some(path)=std::env::var_os("RIVET_QUALIFICATION_ROOT")else{return;};let project=Project::open(Path::new(&path)).unwrap();let phase=std::env::var("RIVET_QUALIFICATION_EXIT").unwrap();
        if phase.starts_with("migration"){Ledger::open(&project,Path::new("state")).unwrap().shutdown().unwrap();}
        else if phase.starts_with("recovery"){let r=repair_inspect(&project,Path::new("state")).unwrap();tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(recover_state(&project,Path::new("state"),&r.snapshot_digest)).unwrap();}
        else{let root=StateRoot::acquire(&project,Path::new("state"),false).unwrap().unwrap();crate::artifacts::Store::open(&root.dir,true).unwrap().unwrap().put(b"orphan after crash","text/plain").unwrap();}
        panic!("did not reach fault");
    }
    #[tokio::test]
    async fn migration_recovery_and_blob_activation_crashes_keep_evidence_and_safe_admission(){
        for _repeat in 0..2{for phase in ["migration-backup","migration-before-commit","migration-after-commit","recovery-evidence","recovery-fenced","recovery-contained","recovery-committed","blob-staged","blob-activated"]{
            let temp=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();let project=Project::open(temp.path()).unwrap();
            let ledger=Ledger::open(&project,Path::new("state")).unwrap();let session=SessionId::default();ledger.transition(0,Transition::NewSession{id:session}).await.unwrap();ledger.shutdown().unwrap();
            if phase.starts_with("migration"){
                let db=Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();db.execute_batch("DROP TABLE run_lifecycle; DROP TABLE run_specs; DROP TABLE recovery_resolutions; UPDATE meta SET schema_version=3").unwrap();drop(db);
            }
            let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","ledger::crash_boundaries::child","--test-threads=1"]).env("RIVET_QUALIFICATION_ROOT",temp.path()).env("RIVET_QUALIFICATION_EXIT",phase).output().unwrap();assert_eq!(output.status.code(),Some(91),"{phase}: {}",String::from_utf8_lossy(&output.stderr));
            let report=repair_inspect(&project,Path::new("state")).unwrap();if report.sqlite_recovery_required||report.control_recovery_required{recover_state(&project,Path::new("state"),&report.snapshot_digest).await.unwrap();}
            let report=repair_inspect(&project,Path::new("state")).unwrap();assert_eq!(report.integrity,"ok");assert!(report.pending.is_empty());
            if phase.starts_with("blob"){assert_eq!(report.orphan_blobs.len(),1);}
            let ledger=Ledger::open(&project,Path::new("state")).unwrap();ledger.shutdown().unwrap();assert!(inspect(&project,Path::new("state")).unwrap().sessions.iter().any(|s|s.id==session.0.to_string()));
        }}
    }
}

pub(crate) fn lifecycle_inspect(project:&Project,path:&Path,run:RunId)->Result<crate::lifecycle::Snapshot>{let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;let db=immutable(&root)?;check_schema(&db)?;check_admission(&db)?;crate::lifecycle::snapshot(&db,run)}
