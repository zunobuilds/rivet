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

fn backup_v1(root:&StateRoot,source:&Connection)->Result<()> {
    let name=format!("pre-v2-{}.sqlite",uuid::Uuid::new_v4());
    let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
    let file=root.dir.open_with(&name,&options)?.into_std();
    let path=format!("/proc/self/fd/{}/{}",root.dir.as_raw_fd(),name);
    let mut destination=Connection::open_with_flags(path,OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    rusqlite::backup::Backup::new(source,&mut destination)?.run_to_completion(64,Duration::from_millis(2),None)?;
    let check:String=destination.query_row("PRAGMA quick_check",[],|r|r.get(0))?;
    if check!="ok"{return Err(Error::State);}
    check_schema(&destination)?;destination.close().map_err(|_|Error::State)?;
    file.sync_all()?;root.dir.try_clone()?.into_std_file().sync_all()?;Ok(())
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
    if ![1,2].contains(&version) || uuid::Uuid::parse_str(&project).is_err() || epoch < 0 || last.checked_add(1) != Some(next) { return Err(Error::State); }
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
    Authorize { operation: OperationId, grant: String, expiry: u64, scope: String, permission_epoch: u64 },
    Intent { operation: OperationId },
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
            let version:u32=check.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
            if version==1 {backup_v1(&root,&check)?;}
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
            transaction.execute_batch(include_str!("budget.sql"))?;
            transaction.execute("INSERT INTO meta VALUES(2,?1,1,0)", [ProjectId::default().0.to_string()])?;
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
        Transition::Authorize { operation, grant, expiry, scope, permission_epoch } => {
            advance(&tx, operation, revision, "proposed", "authorized")?;
            tx.execute("INSERT INTO grants VALUES(?1,?2,?3,?4,?5)", params![grant,operation.0.to_string(),sql_integer(permission_epoch)?,sql_integer(expiry)?,scope])?;
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
        Transition::CompleteRun {run,state}=>{
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
            let changed=tx.execute("UPDATE run_budgets SET token_reserved=token_reserved+?1,cost_reserved=cost_reserved+?2,requests=requests+1 WHERE run_id=?3 AND requests<request_limit AND ?1<=token_limit-token_charged-token_reserved AND ?2<=cost_limit-cost_charged-cost_reserved",params![sql_integer(tokens)?,sql_integer(cost)?,run.0.to_string()])?;
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

/// Quiescent session continuation exports only bounded evidence references, never raw transcripts.
pub fn continuation(project:&Project,path:&Path,session:SessionId)->Result<String>{
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;
    if root.has_remnants()?{return Err(Error::Recovery);}
    let connection=Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro",root.db_path()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_URI|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    settings(&connection,false)?;check_schema(&connection)?;
    let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",[session.0.to_string()],|r|r.get(0))?;
    if !exists{return Err(Error::Invalid);}
    let schema:u32=connection.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    if schema>=2 {
        let uncertain:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM inference_reservations i JOIN runs r ON i.run_id=r.id WHERE r.session_id=?1 AND i.state IN ('reserved','unknown'))",[session.0.to_string()],|r|r.get(0))?;
        if uncertain{return Err(Error::Recovery);}
    }
    let uncertain:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM operations o JOIN runs r ON o.run_id=r.id WHERE r.session_id=?1 AND o.state IN ('intent-recorded','dispatched','outcome-unknown','cancelled'))",[session.0.to_string()],|r|r.get(0))?;
    if uncertain{return Err(Error::Recovery);}
    let mut statement=connection.prepare("SELECT e.sequence,e.kind,e.payload FROM events e JOIN operations o ON e.entity_id=o.id JOIN runs r ON o.run_id=r.id WHERE r.session_id=?1 AND e.kind IN ('receipt-recorded','cancelled','outcome-unknown') ORDER BY e.sequence DESC LIMIT 8")?;
    let rows=statement.query_map([session.0.to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?;
    let mut output=vec![];
    for row in rows{let (sequence,kind,payload)=row?;let receipt:ExecutionReceipt=serde_json::from_str(&payload).map_err(|_|Error::State)?;output.push(serde_json::json!({"sequence":sequence,"kind":kind,"operation":receipt.operation,"outcome":receipt.outcome,"before":receipt.before_revision,"after":receipt.after_revision}));}
    let result=serde_json::to_string(&output).map_err(|_|Error::State)?;if result.len()>8192{return Err(Error::Unsupported);}Ok(result)
}
