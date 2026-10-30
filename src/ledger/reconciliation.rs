//! Explicit dispositions acknowledge observations/uncertainty; they never replay an effect.
use super::*;
use serde::{Serialize,Deserialize};
use crate::tools::files::{FileWorkspace,Revision};
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(rename_all="snake_case")]
pub enum ReconcileDecision {AcceptObservedPost,AbandonUnknown}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileReview {pub operation:OperationId,pub revision:u64,pub proposal_digest:String,pub intent:Option<crate::recovery::Intent>,pub source:Option<Revision>,pub destination:Option<Revision>,pub observation_error:Option<String>,pub post_matches:bool}
impl ReconcileReview {pub fn digest(&self)->Result<String>{serde_json::to_vec(self).map(|v|crate::tools::sha256(&v)).map_err(|_|Error::State)}}
fn operation(project:&Project,path:&Path,root:&StateRoot,id:OperationId)->Result<ReconcileReview>{
    let db=immutable(root)?;check_schema(&db)?;check_admission(&db)?;
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;if schema<4{return Err(Error::Unsupported);}
    let (state,revision,digest,intent):(String,i64,String,Option<String>)=db.query_row("SELECT o.state,o.revision,o.proposal_digest,i.metadata FROM operations o LEFT JOIN operation_intents i ON o.id=i.operation_id WHERE o.id=?1",[id.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    if !["outcome-unknown","cancelled"].contains(&state.as_str()){return Err(Error::Conflict);}
    let intent:Option<crate::recovery::Intent>=intent.map(|s|serde_json::from_str(&s).map_err(|_|Error::State)).transpose()?;
    let mut review=ReconcileReview{operation:id,revision:revision.try_into().map_err(|_|Error::State)?,proposal_digest:digest,intent,source:None,destination:None,observation_error:None,post_matches:false};
    if let Some(crate::recovery::Intent::File{path:source,after_sha256,rename_to,..})=&review.intent{
        let files=FileWorkspace::new(Project::open(project.path())?,path)?;
        let observed=(||{let current=files.observe_revision(source)?;let destination=rename_to.as_ref().map(|p|files.observe_revision(p)).transpose()?.flatten();Ok::<_,Error>((current,destination))})();
        match observed{Ok((source,destination))=>{
            review.post_matches=if rename_to.is_some(){source.is_none()&&destination.as_ref().is_some_and(|r|Some(&r.digest)==after_sha256.as_ref())}else if after_sha256.as_deref()==Some("absent"){source.is_none()}else{source.as_ref().is_some_and(|r|Some(&r.digest)==after_sha256.as_ref())};
            review.source=source;review.destination=destination;
        },Err(e)=>review.observation_error=Some(e.kind().into())}
    }Ok(review)
}
pub fn preview_reconcile(project:&Project,path:&Path,id:OperationId)->Result<ReconcileReview>{let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;operation(project,path,&root,id)}
pub async fn reconcile_operation(project:&Project,path:&Path,id:OperationId,decision:ReconcileDecision,approved:&str)->Result<()> {
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;let review=operation(project,path,&root,id)?;
    if review.digest()?!=approved||decision==ReconcileDecision::AcceptObservedPost&&!review.post_matches{return Err(Error::Denied);}
    let workers=storage::workers(&root.dir)?;for worker in workers.iter().filter(|w|w.operation==id){crate::tools::process::reconcile_owned(id,&worker.unit,&worker.description).await?;}
    // Re-observe after containment; a human edit invalidates the exact reviewed observation.
    if operation(project,path,&root,id)?.digest()?!=approved{return Err(Error::Conflict);}
    let mut db=Connection::open_with_flags(root.db_path(),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;settings(&db,true)?;
    let tx=db.transaction()?;let state=if decision==ReconcileDecision::AcceptObservedPost{"reconciled-observed"}else{"abandoned-unknown"};
    let changed=tx.execute("UPDATE operations SET state=?1,revision=revision+1 WHERE id=?2 AND revision=?3 AND state IN ('outcome-unknown','cancelled')",params![state,id.0.to_string(),sql_integer(review.revision)?])?;if changed!=1{return Err(Error::Conflict);}
    tx.execute("INSERT INTO recovery_resolutions VALUES(?1,'operation',?2,?3,?4,?5)",params![id.0.to_string(),sql_integer(review.revision)?,approved,state,serde_json::to_string(&review).map_err(|_|Error::State)?])?;audit(&tx,&id.0.to_string(),"user",state)?;tx.commit()?;db.close().map_err(|_|Error::State)?;storage::sync(&root.dir)
}
/// Exact request digest acknowledgment retains all reserved/charged budget and never resends.
pub fn acknowledge_request(project:&Project,path:&Path,request:RequestId,approved_digest:&str)->Result<()> {
    let root=StateRoot::acquire(project,path,false)?.ok_or(Error::Invalid)?;let db=immutable(&root)?;check_schema(&db)?;check_admission(&db)?;
    let (digest,state):(String,String)=db.query_row("SELECT request_digest,state FROM inference_reservations WHERE request_id=?1",[request.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;if digest!=approved_digest||state!="unknown"{return Err(Error::Denied);}drop(db);
    let mut db=Connection::open_with_flags(root.db_path(),OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX)?;settings(&db,true)?;let tx=db.transaction()?;
    tx.execute("INSERT INTO recovery_resolutions VALUES(?1,'request',0,?2,'acknowledged-unknown-no-refund','{}')",params![request.0.to_string(),digest])?;audit(&tx,&request.0.to_string(),"user","acknowledged-unknown-no-refund")?;tx.commit()?;db.close().map_err(|_|Error::State)?;storage::sync(&root.dir)
}
