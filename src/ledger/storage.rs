//! Descriptor-local private state files. No path discovery or overwrite fallback.
use super::*;
use std::io::{Read,Write};
use serde::{Deserialize,Serialize};
const RESOLVE:rustix::fs::ResolveFlags=rustix::fs::ResolveFlags::BENEATH.union(rustix::fs::ResolveFlags::NO_SYMLINKS).union(rustix::fs::ResolveFlags::NO_XDEV);
pub(super) fn directory(parent:&Dir,name:&Path,create:bool)->Result<Dir>{
    if create {match rustix::fs::mkdirat(parent,name,rustix::fs::Mode::RUSR|rustix::fs::Mode::WUSR|rustix::fs::Mode::XUSR){Ok(())|Err(rustix::io::Errno::EXIST)=>(),Err(_)=>return Err(Error::State)}}
    let fd=rustix::fs::openat2(parent,name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),RESOLVE).map_err(|_|Error::State)?;
    let dir=Dir::from_std_file(std::fs::File::from(fd));let m=dir.dir_metadata()?;
    if m.uid()!=parent.dir_metadata()?.uid()||m.mode()&0o777!=0o700{return Err(Error::State);}Ok(dir)
}
pub(super) fn read(dir:&Dir,name:&str,limit:u64)->Result<Vec<u8>>{
    let fd=rustix::fs::openat2(dir,name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::CLOEXEC|rustix::fs::OFlags::NONBLOCK,rustix::fs::Mode::empty(),RESOLVE).map_err(|_|Error::State)?;
    let mut file=std::fs::File::from(fd);let m=file.metadata()?;
    use std::os::unix::fs::MetadataExt;
    if !m.is_file()||m.nlink()!=1||m.mode()&0o777!=0o600||m.uid()!=dir.dir_metadata()?.uid()||m.len()>limit{return Err(Error::State);}
    let mut data=vec![];(&mut file).take(limit+1).read_to_end(&mut data)?;let after=file.metadata()?;
    if data.len() as u64!=m.len()||m.mtime()!=after.mtime()||m.mtime_nsec()!=after.mtime_nsec(){return Err(Error::Conflict);}Ok(data)
}
pub(super) fn write_new(dir:&Dir,name:&str,data:&[u8])->Result<()>{
    let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
    let mut file=dir.open_with(name,&options)?.into_std();file.write_all(data)?;file.sync_all()?;sync(dir)
}
pub(super) fn sync(dir:&Dir)->Result<()>{dir.try_clone()?.into_std_file().sync_all()?;Ok(())}
pub(super) fn json<T:Serialize>(dir:&Dir,name:&str,value:&T,replace:bool)->Result<()>{
    let encoded=serde_json::to_vec(value).map_err(|_|Error::Invalid)?;
    if !replace {return write_new(dir,name,&encoded);}
    let stage=format!(".stage-{}",uuid::Uuid::new_v4());write_new(dir,&stage,&encoded)?;dir.rename(&stage,dir,name)?;sync(dir)
}
pub(super) fn read_json<T:serde::de::DeserializeOwned>(dir:&Dir,name:&str)->Result<T>{serde_json::from_slice(&read(dir,name,1024*1024)?).map_err(|_|Error::State)}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {pub project:ProjectId,pub version:u32}
pub(super) fn identity(dir:&Dir,project:ProjectId)->Result<()>{
    match dir.symlink_metadata("identity.json"){
        Ok(_)=>{let identity:Identity=read_json(dir,"identity.json")?;if identity.version!=1||identity.project!=project{return Err(Error::Conflict);}},
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>json(dir,"identity.json",&Identity{version:1,project},false)?,
        Err(_)=>return Err(Error::State),
    }Ok(())
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Worker {pub operation:OperationId,pub unit:String,pub description:String,pub epoch:u64}
pub(super) fn worker(dir:&Dir,record:&Worker)->Result<()>{
    crate::recovery::validate_worker(record.operation,&record.unit,&record.description)?;if record.epoch==0{return Err(Error::Invalid);}
    let workers=directory(dir,Path::new("workers"),true)?;let name=format!("{}.json",record.operation.0.simple());
    match workers.symlink_metadata(&name){
        Ok(_)=>{let old:Worker=read_json(&workers,&name)?;if serde_json::to_vec(&old).ok()!=serde_json::to_vec(record).ok(){return Err(Error::Conflict);}},
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>json(&workers,&name,record,false)?,
        Err(_)=>return Err(Error::State),
    }sync(dir)
}
pub(super) fn workers(dir:&Dir)->Result<Vec<Worker>>{
    let workers=directory(dir,Path::new("workers"),false)?;let mut records=vec![];
    for entry in workers.entries()?.take(4097){
        if records.len()>=4096{return Err(Error::Unsupported);}let name=entry?.file_name().into_string().map_err(|_|Error::State)?;
        let record:Worker=read_json(&workers,&name)?;
        if name!=format!("{}.json",record.operation.0.simple()){return Err(Error::State);}crate::recovery::validate_worker(record.operation,&record.unit,&record.description)?;records.push(record);
    }records.sort_by_key(|r|r.operation.0);Ok(records)
}
pub(super) fn backfill_workers(db:&Connection,dir:&Dir)->Result<()>{
    directory(dir,Path::new("workers"),true)?;
    let schema:u32=db.query_row("SELECT schema_version FROM meta",[],|r|r.get(0))?;if schema<3{return Ok(());}
    let mut query=db.prepare("SELECT operation_id,unit,description,owner_epoch FROM owned_workers LIMIT 4097")?;let mut count=0;
    for row in query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?)))?{
        count+=1;if count>4096{return Err(Error::Unsupported);}let (operation,unit,description,epoch)=row?;
        worker(dir,&Worker{operation:OperationId(uuid::Uuid::parse_str(&operation).map_err(|_|Error::State)?),unit,description,epoch:epoch.try_into().map_err(|_|Error::State)?})?;
    }Ok(())
}
