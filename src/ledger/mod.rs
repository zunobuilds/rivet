use crate::{config::Project, domain::*, error::{Error, Result}, sessions::Session, tools::ToolProposal};
use cap_std::fs::{Dir, OpenOptions, OpenOptionsExt, Permissions, PermissionsExt, MetadataExt};
use rusqlite::{params, Connection, OpenFlags, Transaction};
use serde::Serialize;
use std::{os::fd::AsRawFd, path::Path, thread::{self, JoinHandle}, time::{Duration, SystemTime, UNIX_EPOCH}};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

const DB: &str = "ledger.sqlite";
const LOCK: &str = "owner.lock";
const QUEUE: usize = 128;

struct StateRoot { dir: Dir, _lock: std::fs::File }
impl StateRoot {
    fn acquire(project: &Project, path: &Path, create: bool) -> Result<Option<Self>> {
        let path = project.relative(path)?;
        if !project.check_components(&path)? {
            if !create { return Ok(None); }
            // Only the final component may be created; never auto-create arbitrary parents.
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                if !project.check_components(parent)? { return Err(Error::State); }
            }
            project.dir.create_dir(&path)?;
            project.dir.set_permissions(&path, Permissions::from_mode(0o700))?;
        }
        let dir = project.open_directory(&path)?;
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
    if version != 1 || uuid::Uuid::parse_str(&project).is_err() || epoch < 0 || last.checked_add(1) != Some(next) { return Err(Error::State); }
    Ok(())
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
    if root.has_remnants()? { return Err(Error::Recovery); }
    if !root.dir.symlink_metadata(DB)?.is_file() { return Err(Error::State); }
    let connection = Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro", root.db_path()), OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    settings(&connection, false)?;
    check_schema(&connection)?;
    let mut statement = connection.prepare("SELECT id, created_at, config_version FROM sessions ORDER BY created_at,id")?;
    let sessions = statement.query_map([], |r| Ok(Session { id: r.get(0)?, created_at: r.get(1)?, config_version: r.get(2)? }))?.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Inspection { state: "quiescent", sqlite_version: Some(verify_engine(&connection)?), sessions })
}

#[derive(Clone, Debug)]
pub enum Transition {
    NewSession { id: SessionId },
    RootTask { session: SessionId, run: RunId, task: TaskId, actor: ActorId },
    Propose { run: RunId, proposal: ToolProposal },
    Deny { operation: OperationId },
    Authorize { operation: OperationId, grant: String, expiry: u64, scope: String },
    Intent { operation: OperationId },
    Dispatched { operation: OperationId },
    Receipt { receipt: ExecutionReceipt },
    Terminal { operation: OperationId },
}
#[derive(Clone, Debug)]
pub struct LedgerCommand { pub schema_version: u32, pub owner_epoch: u64, pub expected_revision: u64, pub transition: Transition }
struct Pending { command: LedgerCommand, reply: oneshot::Sender<Result<()>> }

pub struct Ledger {
    tx: mpsc::Sender<Pending>,
    stop: CancellationToken,
    worker: Option<JoinHandle<Result<()>>>,
    epoch: u64,
    project_id: ProjectId,
    workspace_identity: (u64, u64),
}
impl Ledger {
    pub fn open(project: &Project, path: &Path) -> Result<Self> {
        let metadata = project.dir.dir_metadata()?;
        let workspace_identity = (metadata.dev(), metadata.ino());
        let root = StateRoot::acquire(project, path, true)?.ok_or(Error::State)?;
        // Orphan journals are recovery evidence even when the main DB is absent.
        if root.has_remnants()? { return Err(Error::Recovery); }
        let existed = root.dir.symlink_metadata(DB).is_ok();
        if existed {
            // Reject unknown schema without even transient WAL/SHM creation.
            let check = Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro", root.db_path()), OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
            settings(&check, false)?;
            check_schema(&check)?;
            check.close().map_err(|_| Error::State)?;
        }
        if !existed {
            let mut options = OpenOptions::new(); options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
            root.dir.open_with(DB, &options)?;
        }
        let mut connection = Connection::open_with_flags(root.db_path(), OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Reject unknown schema before changing journaling or owner state.
        if existed { check_schema(&connection)?; }
        settings(&connection, true)?;
        if !existed {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("schema.sql"))?;
            transaction.execute("INSERT INTO meta VALUES(1,?1,1,0)", [ProjectId::default().0.to_string()])?;
            audit(&transaction, "project", "system", "initialized")?;
            transaction.commit()?;
        }
        let transaction = connection.transaction()?;
        transaction.execute("UPDATE meta SET owner_epoch=owner_epoch+1", [])?;
        let epoch: u64 = transaction.query_row("SELECT owner_epoch FROM meta", [], |r| r.get::<_, i64>(0))?.try_into().map_err(|_| Error::State)?;
        audit(&transaction, "project", "system", "owner-acquired")?;
        transaction.commit()?;
        let project_uuid: String = connection.query_row("SELECT project_uuid FROM meta", [], |r|r.get(0))?;
        let project_id = ProjectId(uuid::Uuid::parse_str(&project_uuid).map_err(|_| Error::State)?);
        let (tx, mut rx) = mpsc::channel::<Pending>(QUEUE);
        let stop = CancellationToken::new(); let control = stop.clone();
        let worker = thread::spawn(move || {
            // Root and lock outlive SQLite, including clean checkpoint/close.
            let _root = root;
            while !control.is_cancelled() {
                match rx.try_recv() {
                    Ok(pending) => {
                        let result = apply(&mut connection, pending.command, epoch);
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
        Ok(Self { tx, stop, worker: Some(worker), epoch, project_id, workspace_identity })
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
    pub fn cancellation(&self) -> CancellationToken { self.stop.clone() }
    pub fn shutdown(mut self) -> Result<()> { self.stop.cancel(); self.join() }
    fn join(&mut self) -> Result<()> { self.worker.take().map_or(Ok(()), |w| w.join().map_err(|_| Error::State)?) }
}
impl Drop for Ledger { fn drop(&mut self) { self.stop.cancel(); let _ = self.join(); } }
fn audit(tx: &Transaction<'_>, entity: &str, actor: &str, kind: &str) -> Result<()> {
    tx.execute("INSERT INTO events SELECT next_event_sequence,?1,?2,?3,'{}' FROM meta", params![entity,actor,kind])?;
    tx.execute("UPDATE meta SET next_event_sequence=next_event_sequence+1", [])?; Ok(())
}
fn advance(tx: &Transaction<'_>, operation: OperationId, revision: u64, from: &str, to: &str) -> Result<()> {
    let changed = tx.execute("UPDATE operations SET state=?1,revision=revision+1 WHERE id=?2 AND state=?3 AND revision=?4", params![to,operation.0.to_string(),from,sql_integer(revision)?])?;
    if changed != 1 { return Err(Error::Conflict); }
    audit(tx, &operation.0.to_string(), "supervisor", to)
}
fn apply(connection: &mut Connection, command: LedgerCommand, epoch: u64) -> Result<()> {
    if command.schema_version != 1 || command.owner_epoch != epoch { return Err(Error::Conflict); }
    let tx = connection.transaction()?;
    let actual: i64 = tx.query_row("SELECT owner_epoch FROM meta", [], |r| r.get(0))?;
    if actual != sql_integer(epoch)? { return Err(Error::Conflict); }
    let revision = command.expected_revision;
    match command.transition {
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
            let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks JOIN agents ON tasks.owner_agent_id=agents.id WHERE tasks.id=?1 AND tasks.run_id=?2 AND agents.id=?3 AND agents.run_id=?2)", params![p.task.0.to_string(),run.0.to_string(),p.actor.0.to_string()], |r| r.get(0))?;
            if !valid { return Err(Error::Conflict); }
            tx.execute("INSERT INTO operations VALUES(?1,?2,?3,?4,?5,?6,'proposed',0)", params![p.operation.0.to_string(),run.0.to_string(),p.task.0.to_string(),p.actor.0.to_string(),p.descriptor_digest,p.digest()?])?;
            audit(&tx, &p.operation.0.to_string(), "provider", "proposed")?;
        },
        Transition::Deny { operation } => advance(&tx, operation, revision, "proposed", "denied")?,
        Transition::Authorize { operation, grant, expiry, scope } => {
            advance(&tx, operation, revision, "proposed", "authorized")?;
            tx.execute("INSERT INTO grants VALUES(?1,?2,?3,?4,?5)", params![grant,operation.0.to_string(),sql_integer(epoch)?,sql_integer(expiry)?,scope])?;
        },
        Transition::Intent { operation } => advance(&tx, operation, revision, "authorized", "intent-recorded")?,
        Transition::Dispatched { operation } => advance(&tx, operation, revision, "intent-recorded", "dispatched")?,
        Transition::Receipt { receipt } => {
            if receipt.schema_version != 1 { return Err(Error::Invalid); }
            advance(&tx, receipt.operation, revision, "dispatched", match receipt.outcome { Outcome::Unknown => "outcome-unknown", Outcome::Cancelled => "cancelled", _ => "receipt-recorded" })?;
            let payload = serde_json::to_string(&receipt).map_err(|_| Error::Invalid)?;
            tx.execute("UPDATE events SET payload=?1 WHERE sequence=(SELECT next_event_sequence-1 FROM meta)", [payload])?;
        },
        Transition::Terminal { operation } => advance(&tx, operation, revision, "receipt-recorded", "terminal")?,
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
        let ledger = Arc::new(Ledger { tx, stop: CancellationToken::new(), worker: None, epoch: 1, project_id: ProjectId::default(), workspace_identity: (0,0) });
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
