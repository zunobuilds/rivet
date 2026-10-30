//! Reviewed, restartable state-directory exchange. The displaced state is never deleted.
use super::*;
use serde::{Deserialize,Serialize};
use std::path::PathBuf;
const MAX:u64=512*1024*1024;
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {version:u32,project:ProjectId,schema:u32,database:crate::artifacts::ArtifactRef,artifacts:Vec<crate::artifacts::ArtifactRef>}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreReview {pub state:PathBuf,pub state_identity:(u64,u64),pub snapshot_digest:String,pub backup:String,pub backup_digest:String,pub manifest_digest:String,pub project:ProjectId,pub workers_digest:String,pub additional_bytes:u64}
impl RestoreReview {pub fn digest(&self)->Result<String>{serde_json::to_vec(self).map(|v|crate::tools::sha256(&v)).map_err(|_|Error::State)}}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {version:u32,review:RestoreReview,sibling:String,old:(u64,u64),new:(u64,u64),prepared_digest:String,committed_digest:Option<String>}
#[derive(Debug,Serialize)]
pub struct RestoreReport {pub project:ProjectId,pub displaced_state:PathBuf,pub stopped_workers:usize,pub state:&'static str}
fn valid_backup(name:&str)->bool{
    if name.contains('/'){return false;}
    let stem=if let Some(stem)=name.strip_prefix("backup-"){stem}else if let Some(pre)=name.strip_prefix("pre-v"){
        let Some((version,stem))=pre.split_once('-')else{return false;};if !version.parse::<u32>().is_ok_and(|v|(3..=SCHEMA_VERSION).contains(&v)){return false;}stem
    }else{return false;};
    stem.strip_suffix(".sqlite").is_some_and(|id|uuid::Uuid::parse_str(id).is_ok())
}
fn valid_digest(s:&str)->bool{s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn refs(db:&Connection)->Result<Vec<crate::artifacts::ArtifactRef>>{
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;if schema<3{return Ok(vec![]);}
    let mut out=vec![];let mut total=0u64;let mut q=db.prepare("SELECT digest,size,media_type FROM artifacts ORDER BY digest LIMIT 1025")?;
    for row in q.query_map([],|r|Ok(crate::artifacts::ArtifactRef{digest:r.get(0)?,size:r.get::<_,i64>(1)?.try_into().map_err(|_|rusqlite::Error::InvalidQuery)?,media_type:r.get(2)?}))?{
        let r=row?;total=total.checked_add(r.size).ok_or(Error::Unsupported)?;
        if out.len()>=1024||total>256*1024*1024||r.size>64*1024*1024||!valid_digest(&r.digest){return Err(Error::Unsupported);}out.push(r);
    }Ok(out)
}
pub(super) fn backup_size(db:&Connection)->Result<u64>{Ok(refs(db)?.iter().map(|r|r.size).sum())}
pub(super) fn bundle(root:&StateRoot,source:&Connection,name:&str)->Result<String>{
    let artifacts=refs(source)?;let img=image(&root.dir,name)?;
    let project:String=source.query_row("SELECT project_uuid FROM meta",[],|r|r.get(0))?;let schema:u32=source.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    let assets=storage::directory(&root.dir,Path::new(&format!("{name}.assets")),true)?;
    let store=crate::artifacts::Store::open(&root.dir,false)?;
    for reference in &artifacts{let data=store.as_ref().ok_or(Error::State)?.read(reference)?;storage::write_new(&assets,&reference.digest,&data)?;}
    let manifest=Bundle{version:1,project:ProjectId(uuid::Uuid::parse_str(&project).map_err(|_|Error::State)?),schema,database:crate::artifacts::ArtifactRef{digest:img.digest,size:img.size,media_type:"application/vnd.sqlite3".into()},artifacts};
    storage::json(&assets,"manifest.json",&manifest,false)?;storage::sync(&root.dir)?;
    Ok(crate::tools::sha256(&storage::read(&assets,"manifest.json",1024*1024)?))
}
fn selected(root:&StateRoot,name:&str)->Result<(Bundle,Dir,String)>{
    if !valid_backup(name){return Err(Error::Invalid);}
    let assets=storage::directory(&root.dir,Path::new(&format!("{name}.assets")),false)?;
    let bytes=storage::read(&assets,"manifest.json",1024*1024)?;let manifest:Bundle=serde_json::from_slice(&bytes).map_err(|_|Error::State)?;
    if manifest.version!=1||!(1..=SCHEMA_VERSION).contains(&manifest.schema)||!valid_digest(&manifest.database.digest){return Err(Error::Unsupported);}
    let img=image(&root.dir,name)?;if img.digest!=manifest.database.digest||img.size!=manifest.database.size{return Err(Error::Conflict);}
    for suffix in ["-wal","-shm","-journal"]{if root.dir.symlink_metadata(format!("{name}{suffix}")).is_ok(){return Err(Error::Recovery);}}
    let db=Connection::open_with_flags(format!("file:/proc/self/fd/{}/{name}?immutable=1&mode=ro",root.dir.as_raw_fd()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_URI|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    settings(&db,false)?;check_schema(&db)?;
    let quick:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))?;if quick!="ok"||db.prepare("PRAGMA foreign_key_check")?.query([])?.next()?.is_some(){return Err(Error::State);}
    let project:String=db.query_row("SELECT project_uuid FROM meta",[],|r|r.get(0))?;
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    if schema!=manifest.schema||project!=manifest.project.0.to_string()||serde_json::to_vec(&refs(&db)?).ok()!=serde_json::to_vec(&manifest.artifacts).ok(){return Err(Error::Conflict);}
    let mut total=manifest.database.size;
    for reference in &manifest.artifacts{if !valid_digest(&reference.digest)||reference.size>64*1024*1024{return Err(Error::Invalid);}let bytes=storage::read(&assets,&reference.digest,64*1024*1024)?;total=total.checked_add(bytes.len() as u64).ok_or(Error::Unsupported)?;if total>320*1024*1024||bytes.len() as u64!=reference.size||crate::tools::sha256(&bytes)!=reference.digest{return Err(Error::State);}}
    Ok((manifest,assets,crate::tools::sha256(&bytes)))
}
fn review(project:&Project,path:&Path,root:&StateRoot,name:&str)->Result<RestoreReview>{
    dir_admission(&root.dir)?;
    let (manifest,_,manifest_digest)=selected(root,name)?;
    let identity:storage::Identity=storage::read_json(&root.dir,"identity.json")?;if identity.version!=1||identity.project!=manifest.project{return Err(Error::Conflict);}
    let workers=storage::workers(&root.dir)?;let m=root.dir.dir_metadata()?;
    let additional=manifest.database.size+manifest.artifacts.iter().map(|r|r.size).sum::<u64>()+1024*1024;
    ensure_space(&root.dir,additional)?;
    Ok(RestoreReview{state:project.relative(path)?,state_identity:(m.dev(),m.ino()),snapshot_digest:snapshot_digest(&images(root)?)?,backup:name.into(),backup_digest:manifest.database.digest,manifest_digest,project:manifest.project,workers_digest:crate::tools::sha256(&serde_json::to_vec(&workers).map_err(|_|Error::State)?),additional_bytes:additional})
}
pub fn preview_restore(project:&Project,path:&Path,backup:&str)->Result<RestoreReview>{
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;review(project,path,&root,backup)
}
fn sibling(path:&Path,name:&str)->PathBuf{path.parent().unwrap_or(Path::new("")).join(name)}
fn parent(project:&Project,path:&Path)->Result<Dir>{
    let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let fd=rustix::fs::openat2(&project.dir,parent,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
    let dir=Dir::from_std_file(std::fs::File::from(fd));let m=dir.dir_metadata()?;if m.uid()!=project.dir.dir_metadata()?.uid()||m.mode()&0o022!=0{return Err(Error::State);}Ok(dir)
}
fn identity(dir:&Dir)->Result<(u64,u64)>{let m=dir.dir_metadata()?;Ok((m.dev(),m.ino()))}
fn stage_database(root:&StateRoot,review:&RestoreReview,workers:&[storage::Worker])->Result<()>{
    let mut db=Connection::open_with_flags(root.db_path(),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    db.pragma_update(None,"foreign_keys","ON")?;db.pragma_update(None,"synchronous","FULL")?;check_schema(&db)?;
    let tx=db.transaction()?;let schema:u32=tx.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;
    if schema==1{tx.execute_batch(include_str!("budget.sql"))?;}if schema<3{tx.execute_batch(include_str!("recovery.sql"))?;}
    if schema<4{tx.execute_batch(include_str!("reconciliation.sql"))?;}
    if schema<5{tx.execute_batch(include_str!("lifecycle.sql"))?;}
    if schema<6{tx.execute_batch(include_str!("coordination.sql"))?;}
    if schema<7{tx.execute_batch(include_str!("model_pool.sql"))?;}
    if schema<8{tx.execute_batch(include_str!("adaptive.sql"))?;}
    tx.execute("UPDATE run_lifecycle SET status='recovery-required',revision=revision+1",[])?;
    tx.execute("DELETE FROM recovery_resolutions",[])?;
    let epoch:i64=tx.query_row("SELECT owner_epoch FROM meta",[],|r|r.get(0))?;
    let epoch=epoch.max(workers.iter().map(|w|sql_integer(w.epoch)).collect::<Result<Vec<_>>>()?.into_iter().max().unwrap_or(0)).checked_add(1).ok_or(Error::State)?;
    tx.execute("UPDATE meta SET schema_version=?1,owner_epoch=?2",params![SCHEMA_VERSION,epoch])?;
    model_pool::fence_all(&tx,epoch.try_into().map_err(|_|Error::State)?)?;
    tx.execute("DELETE FROM grants",[])?;tx.execute("DELETE FROM forwarding_consents",[])?;
    tx.execute("UPDATE operations SET state=CASE WHEN state IN ('proposed','authorized') THEN 'denied' WHEN state='receipt-recorded' THEN 'terminal' ELSE 'outcome-unknown' END,revision=revision+1 WHERE state NOT IN ('terminal','denied')",[])?;
    tx.execute("UPDATE runs SET state='recovery-required',revision=revision+1",[])?;
    tx.execute("UPDATE inference_reservations SET state='unknown' WHERE state IN ('reserved','settled')",[])?;
    tx.execute("UPDATE run_budgets SET token_charged=token_limit,token_reserved=0,cost_charged=cost_limit,cost_reserved=0,requests=request_limit",[])?;
    tx.execute("UPDATE recovery_control SET required=1 WHERE id=1",[])?;
    audit(&tx,"project","user","restore-prepared-no-replay")?;let payload=serde_json::to_string(review).map_err(|_|Error::State)?;
    tx.execute("UPDATE events SET payload=?1 WHERE sequence=(SELECT next_event_sequence-1 FROM meta)",[payload])?;tx.commit()?;
    db.close().map_err(|_|Error::State)?;storage::sync(&root.dir)
}
#[derive(Serialize,Deserialize)]
struct Allocations {reserved:u64}
#[derive(Serialize,Deserialize)]
struct Retained {bytes:u64,archive:PathBuf}
/// Explicit user approval also serves as the resume token after an interrupted exchange.
pub async fn restore_state(project:&Project,path:&Path,backup:&str,approved:&str)->Result<RestoreReport>{restore_inner(project,path,backup,approved,None).await}
fn fault_at(fault:Option<&str>,point:&str)->Result<()>{
    if fault==Some(point){
        #[cfg(test)] if std::env::var_os("RIVET_RESTORE_EXIT").is_some(){std::process::exit(91);}
        return Err(Error::Interrupted);
    }Ok(())
}
async fn restore_inner(project:&Project,path:&Path,backup:&str,approved:&str,fault:Option<&str>)->Result<RestoreReport>{
    let relative=project.relative(path)?;let root=StateRoot::acquire(project,&relative,false)?.ok_or(Error::Invalid)?;
    if root.dir.symlink_metadata("archived.json").is_ok(){return Err(Error::Recovery);}
    let mut marker:Marker;
    let other:StateRoot;
    if root.dir.symlink_metadata("restore.json").is_ok(){
        marker=storage::read_json(&root.dir,"restore.json")?;
        if marker.version!=1||marker.review.state!=relative||marker.review.backup!=backup||marker.review.digest()?!=approved||!marker.sibling.starts_with(".rivet-repair-")||marker.sibling.contains('/'){return Err(Error::Denied);}
        other=StateRoot::acquire(project,&sibling(&relative,&marker.sibling),false)?.ok_or(Error::Recovery)?;
        let current=identity(&root.dir)?;let second=identity(&other.dir)?;
        if !((current==marker.old&&second==marker.new)||(current==marker.new&&second==marker.old)){return Err(Error::Conflict);}
    }else{
        let inspected=review(project,&relative,&root,backup)?;if inspected.digest()?!=approved{return Err(Error::Denied);}
        let (bundle,assets,_)=selected(&root,backup)?;let workers=storage::workers(&root.dir)?;
        let reserved=if root.dir.symlink_metadata("allocations.json").is_ok(){storage::read_json::<Allocations>(&root.dir,"allocations.json")?.reserved}else{0};
        storage::json(&root.dir,"allocations.json",&Allocations{reserved:reserved.checked_add(inspected.additional_bytes).ok_or(Error::Unsupported)?},true)?;
        let name=format!(".rivet-repair-{}",uuid::Uuid::new_v4());other=StateRoot::acquire(project,&sibling(&relative,&name),true)?.ok_or(Error::State)?;
        storage::write_new(&other.dir,DB,&storage::read(&root.dir,backup,64*1024*1024)?)?;
        storage::identity(&other.dir,bundle.project)?;storage::directory(&other.dir,Path::new("workers"),true)?;
        for record in &workers{storage::worker(&other.dir,record)?;}
        let store=crate::artifacts::Store::open(&other.dir,true)?.ok_or(Error::State)?;
        for r in &bundle.artifacts{let data=storage::read(&assets,&r.digest,64*1024*1024)?;let imported=store.put(&data,&r.media_type)?;if imported.digest!=r.digest{return Err(Error::State);}}
        stage_database(&other,&inspected,&workers)?;
        let retained=Retained{bytes:state_usage(&root.dir)?,archive:sibling(&relative,&name)};storage::json(&other.dir,"retained.json",&retained,false)?;
        marker=Marker{version:1,review:inspected,sibling:name,old:identity(&root.dir)?,new:identity(&other.dir)?,prepared_digest:image(&other.dir,DB)?.digest,committed_digest:None};
        fault_at(fault,"staged")?;
        storage::json(&other.dir,"restore.json",&marker,false)?;storage::json(&root.dir,"restore.json",&marker,false)?;
    }
    fault_at(fault,"prepared")?;
    if identity(&root.dir)?==marker.old{
        if snapshot_digest(&images(&root)?)?!=marker.review.snapshot_digest||image(&other.dir,DB)?.digest!=marker.prepared_digest{return Err(Error::Conflict);}
        let parent=parent(project,&relative)?;
        rustix::fs::renameat_with(&parent,relative.file_name().ok_or(Error::Invalid)?,&parent,&marker.sibling,rustix::fs::RenameFlags::EXCHANGE).map_err(|_|Error::State)?;storage::sync(&parent)?;
    }
    fault_at(fault,"exchanged")?;
    let (active,old)=if identity(&root.dir)?==marker.new{(&root,&other)}else{(&other,&root)};
    let workers=storage::workers(&old.dir)?;
    if crate::tools::sha256(&serde_json::to_vec(&workers).map_err(|_|Error::State)?)!=marker.review.workers_digest{return Err(Error::Conflict);}
    if active.has_remnants()?{
        preserve_images(active,&images(active)?)?;
        let db=Connection::open_with_flags(active.db_path(),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        verify_engine(&db)?;check_schema(&db)?;db.close().map_err(|_|Error::State)?;storage::sync(&active.dir)?;
    }
    let digest=image(&active.dir,DB)?.digest;
    if marker.committed_digest.is_none()&&digest!=marker.prepared_digest{
        let db=Connection::open_with_flags(format!("file:{}?immutable=1&mode=ro",active.db_path()),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_URI|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        check_schema(&db)?;
        let (kind,payload):(String,String)=db.query_row("SELECT kind,payload FROM events ORDER BY sequence DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if kind!="restore-committed-no-replay"||payload!=marker.review.digest()?{return Err(Error::Conflict);}
        marker.committed_digest=Some(digest.clone());storage::json(&active.dir,"restore.json",&marker,true)?;
    }
    if marker.committed_digest.as_ref().map_or(digest!=marker.prepared_digest,|expected|expected!=&digest){return Err(Error::Conflict);}
    let mut stopped=0;for w in workers{crate::tools::process::reconcile_owned(w.operation,&w.unit,&w.description).await?;stopped+=1;}
    fault_at(fault,"contained")?;
    if marker.committed_digest.is_none(){
        let mut db=Connection::open_with_flags(active.db_path(),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;verify_referenced_artifacts(&db,&active.dir)?;
        let tx=db.transaction()?;tx.execute("UPDATE recovery_control SET required=0 WHERE id=1",[])?;audit(&tx,"project","user","restore-committed-no-replay")?;
        tx.execute("UPDATE events SET payload=?1 WHERE sequence=(SELECT next_event_sequence-1 FROM meta)",[marker.review.digest()?])?;
        tx.commit()?;fault_at(fault,"dbcommitted")?;db.close().map_err(|_|Error::State)?;
        marker.committed_digest=Some(image(&active.dir,DB)?.digest);
        storage::json(&active.dir,"restore.json",&marker,true)?;
    }
    fault_at(fault,"committed")?;
    storage::json(&old.dir,"archived.json",&marker,true)?;storage::json(&active.dir,"restore-history.json",&marker,true)?;
    active.dir.remove_file("restore.json")?;storage::sync(&active.dir)?;
    Ok(RestoreReport{project:marker.review.project,displaced_state:sibling(&relative,&marker.sibling),stopped_workers:stopped,state:"restored-paused-fresh-authorization-required"})
}
pub(super) fn state_usage(root:&Dir)->Result<u64>{
    fn visit(dir:&Dir,count:&mut usize,depth:usize)->Result<u64>{
        if depth>32{return Err(Error::Unsupported);}let mut total=0u64;
        for entry in dir.entries()?{*count+=1;if *count>16384{return Err(Error::Unsupported);}let name=entry?.file_name();let m=dir.symlink_metadata(&name)?;
            let size=if m.is_dir(){visit(&storage::directory(dir,Path::new(&name),false)?,count,depth+1)?}else if m.is_file()&&m.nlink()==1{m.len()}else{use cap_std::fs::FileTypeExt;if name==std::ffi::OsStr::new("control.sock")&&m.file_type().is_socket()&&m.uid()==dir.dir_metadata()?.uid()&&m.mode()&0o777==0o600{0}else{return Err(Error::State);}};
            total=total.checked_add(size).ok_or(Error::Unsupported)?;
        }Ok(total)
    }
    let mut used=visit(root,&mut 0,0)?;
    if root.symlink_metadata("allocations.json").is_ok(){let allocations:Allocations=storage::read_json(root,"allocations.json")?;used=used.checked_add(allocations.reserved).ok_or(Error::Unsupported)?;}
    if root.symlink_metadata("retained.json").is_ok(){let retained:Retained=storage::read_json(root,"retained.json")?;used=used.checked_add(retained.bytes).ok_or(Error::Unsupported)?;}Ok(used)
}
pub(super) fn capacity(root:&Dir,additional:u64)->Result<()>{if state_usage(root)?.checked_add(additional).is_none_or(|used|used>MAX){return Err(Error::Denied);}Ok(())}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->tempfile::TempDir{tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap()}
    async fn prepared(project:&Project)->(BackupRef,SessionId){
        let ledger=Ledger::open(project,Path::new("state")).unwrap();let session=SessionId::default();ledger.transition(0,Transition::NewSession{id:session}).await.unwrap();
        let run=RunId::default();let task=TaskId::default();let actor=ActorId::default();let operation=OperationId::default();
        ledger.transition(0,Transition::RootTask{session,run,task,actor}).await.unwrap();
        let proposal=ToolProposal{schema_version:1,operation,attempt:1,task,actor,descriptor_digest:"fixture".into(),arguments:serde_json::json!({}),resources:vec![],preconditions:vec![],budget_reservation:"none".into()};
        ledger.transition(0,Transition::Propose{run,proposal}).await.unwrap();ledger.transition(0,Transition::Authorize{operation,grant:"fixture".into(),expiry:1,scope:"fixture".into(),permission_epoch:1}).await.unwrap();ledger.transition(1,Transition::Intent{operation}).await.unwrap();ledger.transition(2,Transition::Dispatched{operation}).await.unwrap();
        let reference=ledger.retain_artifact(operation,b"durable artifact".to_vec(),"text/plain").await.unwrap();
        ledger.transition(3,Transition::Receipt{receipt:ExecutionReceipt{schema_version:1,operation,attempt:1,outcome:Outcome::Succeeded,exit_code:None,error:None,artifacts:vec![reference.digest],before_revision:None,after_revision:None,redacted_output:String::new()}}).await.unwrap();ledger.transition(4,Transition::Terminal{operation}).await.unwrap();ledger.shutdown().unwrap();
        let report=repair_inspect(project,Path::new("state")).unwrap();(backup_state(project,Path::new("state"),&report.snapshot_digest).unwrap(),session)
    }
    #[test]
    fn exit_helper(){
        let Some(path)=std::env::var_os("RIVET_RESTORE_ROOT")else{return;};
        let project=Project::open(Path::new(&path)).unwrap();let backup=std::env::var("RIVET_RESTORE_BACKUP").unwrap();let approve=std::env::var("RIVET_RESTORE_APPROVE").unwrap();let phase=std::env::var("RIVET_RESTORE_PHASE").unwrap();
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(restore_inner(&project,Path::new("state"),&backup,&approve,Some(&phase))).unwrap();panic!("fault point not reached");
    }
    #[tokio::test]
    async fn restore_crash_matrix_preserves_old_tree_and_resumes_without_grants(){
        for _repeat in 0..2{for phase in ["staged","prepared","exchanged","contained","dbcommitted","committed"]{
            let root=fixture();let project=Project::open(root.path()).unwrap();let (backup,session)=prepared(&project).await;
            let ledger=Ledger::open(&project,Path::new("state")).unwrap();let after=SessionId::default();ledger.transition(0,Transition::NewSession{id:after}).await.unwrap();ledger.shutdown().unwrap();
            let original=storage::directory(&project.dir,Path::new("state"),false).unwrap();let before=identity(&original).unwrap();let review=preview_restore(&project,Path::new("state"),&backup.name).unwrap();
            let result=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","ledger::restore::tests::exit_helper","--test-threads=1"]).env("RIVET_RESTORE_ROOT",root.path()).env("RIVET_RESTORE_BACKUP",&backup.name).env("RIVET_RESTORE_APPROVE",review.digest().unwrap()).env("RIVET_RESTORE_PHASE",phase).env("RIVET_RESTORE_EXIT","1").output().unwrap();assert_eq!(result.status.code(),Some(91),"{phase}: {}",String::from_utf8_lossy(&result.stderr));
            if phase=="staged"{assert_eq!(identity(&storage::directory(&project.dir,Path::new("state"),false).unwrap()).unwrap(),before);}
            else{assert!(matches!(Ledger::open(&project,Path::new("state")),Err(Error::Recovery)));}
            let restored=restore_state(&project,Path::new("state"),&backup.name,&review.digest().unwrap()).await.unwrap();
            let active=storage::directory(&project.dir,Path::new("state"),false).unwrap();assert_ne!(identity(&active).unwrap(),before);assert_eq!(identity(&storage::directory(&project.dir,&restored.displaced_state,false).unwrap()).unwrap(),before);
            let state=inspect(&project,Path::new("state")).unwrap();assert!(state.sessions.iter().any(|s|s.id==session.0.to_string()));assert!(!state.sessions.iter().any(|s|s.id==after.0.to_string()));
            assert!(Ledger::open(&project,&restored.displaced_state).is_err());let ledger=Ledger::open(&project,Path::new("state")).unwrap();ledger.shutdown().unwrap();
        }}
    }
    #[tokio::test]
    async fn corrupt_main_and_blob_restore_from_bundle_but_bad_review_or_backup_is_refused(){
        let root=fixture();let project=Project::open(root.path()).unwrap();let (backup,_)=prepared(&project).await;
        let review=preview_restore(&project,Path::new("state"),&backup.name).unwrap();assert!(restore_state(&project,Path::new("state"),&backup.name,"wrong").await.is_err());
        let source=StateRoot::acquire(&project,Path::new("state"),false).unwrap().unwrap();let (bundle,_,_)=selected(&source,&backup.name).unwrap();let reference=bundle.artifacts[0].clone();drop(source);
        std::fs::write(root.path().join("state/blobs").join(&reference.digest),b"corrupt blob").unwrap();
        std::fs::write(root.path().join("state/ledger.sqlite"),b"corrupt source evidence").unwrap();assert!(restore_state(&project,Path::new("state"),&backup.name,&review.digest().unwrap()).await.is_err());
        let review=preview_restore(&project,Path::new("state"),&backup.name).unwrap();let restored=restore_state(&project,Path::new("state"),&backup.name,&review.digest().unwrap()).await.unwrap();assert_eq!(storage::read(&storage::directory(&project.dir,&restored.displaced_state,false).unwrap(),DB,1024).unwrap(),b"corrupt source evidence");
        let report=repair_inspect(&project,Path::new("state")).unwrap();assert_eq!(report.integrity,"ok");let ledger=Ledger::open(&project,Path::new("state")).unwrap();assert_eq!(ledger.verify_artifact(&reference).unwrap(),b"durable artifact");ledger.shutdown().unwrap();
    }
    #[tokio::test]
    async fn unsafe_restore_inputs_are_refused_before_activation(){
        for case in ["newer","schema-mismatch","database","blob","symlink","project","missing-main"]{
            let root=fixture();let project=Project::open(root.path()).unwrap();let (backup,_)=prepared(&project).await;
            let state=StateRoot::acquire(&project,Path::new("state"),false).unwrap().unwrap();let (mut manifest,assets,_)=selected(&state,&backup.name).unwrap();let before=identity(&state.dir).unwrap();
            match case{
                "newer"=>{manifest.schema=999;storage::json(&assets,"manifest.json",&manifest,true).unwrap();},
                "schema-mismatch"=>{manifest.schema=1;storage::json(&assets,"manifest.json",&manifest,true).unwrap();},
                "database"=>std::fs::write(root.path().join("state").join(&backup.name),b"bad backup").unwrap(),
                "blob"=>std::fs::write(root.path().join("state").join(format!("{}.assets",backup.name)).join(&manifest.artifacts[0].digest),b"bad blob").unwrap(),
                "symlink"=>{let name=&manifest.artifacts[0].digest;assets.remove_file(name).unwrap();std::os::unix::fs::symlink("../../sentinel",root.path().join("state").join(format!("{}.assets",backup.name)).join(name)).unwrap();},
                "project"=>storage::json(&state.dir,"identity.json",&storage::Identity{version:1,project:ProjectId::default()},true).unwrap(),
                _=>std::fs::remove_file(root.path().join("state/ledger.sqlite")).unwrap(),
            }drop(state);
            if case=="missing-main"{assert!(matches!(Ledger::open(&project,Path::new("state")),Err(Error::Recovery)));let review=preview_restore(&project,Path::new("state"),&backup.name).unwrap();restore_state(&project,Path::new("state"),&backup.name,&review.digest().unwrap()).await.unwrap();}
            else{assert!(preview_restore(&project,Path::new("state"),&backup.name).is_err(),"{case}");assert_eq!(identity(&storage::directory(&project.dir,Path::new("state"),false).unwrap()).unwrap(),before);}
        }
    }
}
