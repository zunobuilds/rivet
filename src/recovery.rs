//! Durable effect descriptions contain identities/digests, never executable replay instructions.
use crate::{tools::{files::{FileAction,Revision},sha256},error::{Error,Result}};
use serde::{Serialize,Deserialize};
use std::path::PathBuf;
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(tag="class",rename_all="snake_case",deny_unknown_fields)]
pub enum Intent {
    Read {resources:Vec<String>},
    File {path:PathBuf,before:Option<Revision>,after_sha256:Option<String>,rename_to:Option<PathBuf>},
    Check {projection_sha256:String},
    Git {request_sha256:String},
    Mcp {request_sha256:String},
}
impl Intent {
    pub fn class(&self)->&'static str{match self{Self::Read{..}=>"read",Self::File{..}=>"conditional-file",Self::Check{..}=>"isolated-check",Self::Git{..}=>"git",Self::Mcp{..}=>"mcp"}}
    pub fn file(action:&FileAction,resources:Vec<String>)->Result<Self>{
        Ok(match action{
            FileAction::Read{..}|FileAction::Search{..}=>Self::Read{resources},
            FileAction::Write{path,expected,text}=>Self::File{path:path.clone(),before:expected.clone(),after_sha256:Some(sha256(text.as_bytes())),rename_to:None},
            // Patch post-state needs a granted read; use an explicit unknown post digest here.
            FileAction::Patch{path,expected,..}=>Self::File{path:path.clone(),before:Some(expected.clone()),after_sha256:None,rename_to:None},
            FileAction::Delete{path,expected}=>Self::File{path:path.clone(),before:Some(expected.clone()),after_sha256:Some("absent".into()),rename_to:None},
            FileAction::Rename{from,to,expected}=>Self::File{path:from.clone(),before:Some(expected.clone()),after_sha256:Some(expected.digest.clone()),rename_to:Some(to.clone())},
        })
    }
    pub(crate) fn encode(&self)->Result<String>{let encoded=serde_json::to_string(self).map_err(|_|Error::Invalid)?;if encoded.len()>65536{return Err(Error::Unsupported);}Ok(encoded)}
}

pub(crate) fn validate_worker(operation:crate::domain::OperationId,unit:&str,description:&str)->Result<()> {
    let prefix=format!("rivet-worker-{}-",operation.0.simple());
    let nonce=unit.strip_prefix(&prefix).and_then(|s|s.strip_suffix(".service")).ok_or(Error::Invalid)?;
    if nonce.len()!=32||!nonce.bytes().all(|b|b.is_ascii_hexdigit()){return Err(Error::Invalid);}
    let nonce=uuid::Uuid::parse_str(nonce).map_err(|_|Error::Invalid)?;
    if description!=format!("Rivet operation {} owner {}",operation.0,nonce){return Err(Error::Invalid);}Ok(())
}
