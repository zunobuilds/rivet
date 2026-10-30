//! Private content-addressed evidence. Blob durability precedes ledger references.
use crate::{error::{Error,Result},tools::sha256};
use cap_std::fs::{Dir,MetadataExt,OpenOptions,OpenOptionsExt,Permissions,PermissionsExt};
use serde::{Serialize,Deserialize};
use std::io::{Read,Write};
use rustix::fs::{openat2,OFlags,ResolveFlags,Mode};
const LIMIT:usize=64*1024*1024;
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {pub digest:String,pub size:u64,pub media_type:String}
pub(crate) struct Store {dir:Dir,uid:u32}
impl Store {
    pub(crate) fn open(state:&Dir,create:bool)->Result<Option<Self>>{
        if state.symlink_metadata("blobs").is_err(){
            if !create{return Ok(None);}state.create_dir("blobs")?;state.set_permissions("blobs",Permissions::from_mode(0o700))?;
        }
        let fd=openat2(state,"blobs",OFlags::RDONLY|OFlags::DIRECTORY|OFlags::CLOEXEC,Mode::empty(),ResolveFlags::BENEATH|ResolveFlags::NO_SYMLINKS|ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
        let dir=Dir::from_std_file(std::fs::File::from(fd));let m=dir.dir_metadata()?;let uid=state.dir_metadata()?.uid();if m.uid()!=uid||m.mode()&0o777!=0o700{return Err(Error::State);}Ok(Some(Self{dir,uid}))
    }
    fn validate(reference:&ArtifactRef)->Result<()> {
        if reference.digest.len()!=64||!reference.digest.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))||reference.size>LIMIT as u64||reference.media_type.is_empty()||reference.media_type.len()>128||reference.media_type.chars().any(char::is_control){return Err(Error::Invalid);}Ok(())
    }
    pub(crate) fn read(&self,reference:&ArtifactRef)->Result<Vec<u8>>{
        Self::validate(reference)?;
        let fd=openat2(&self.dir,&reference.digest,OFlags::RDONLY|OFlags::CLOEXEC|OFlags::NOFOLLOW|OFlags::NONBLOCK,Mode::empty(),ResolveFlags::BENEATH|ResolveFlags::NO_SYMLINKS|ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
        let mut file=std::fs::File::from(fd);let m=file.metadata()?;
        use std::os::unix::fs::MetadataExt;
        if !m.is_file()||m.nlink()!=1||m.uid()!=self.uid||m.mode()&0o777!=0o600||m.len()!=reference.size{return Err(Error::State);}
        let mut bytes=Vec::new();(&mut file).take((LIMIT+1) as u64).read_to_end(&mut bytes)?;
        if bytes.len() as u64!=reference.size||sha256(&bytes)!=reference.digest{return Err(Error::State);}Ok(bytes)
    }
    pub(crate) fn put(&self,bytes:&[u8],media_type:&str)->Result<ArtifactRef>{
        let reference=ArtifactRef{digest:sha256(bytes),size:bytes.len() as u64,media_type:media_type.into()};Self::validate(&reference)?;
        let name=format!(".stage-{}",uuid::Uuid::new_v4());let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
        let mut file=self.dir.open_with(&name,&options)?.into_std();
        let result=(||{file.write_all(bytes)?;file.sync_all()?;
            match rustix::fs::renameat_with(&self.dir,&name,&self.dir,&reference.digest,rustix::fs::RenameFlags::NOREPLACE){
                Ok(())=>(),Err(rustix::io::Errno::EXIST)=>{self.read(&reference)?;self.dir.remove_file(&name)?;},Err(_)=>return Err(Error::State),
            }
            self.dir.try_clone()?.into_std_file().sync_all()?;self.read(&reference)?;Ok(reference)
        })();
        if self.dir.symlink_metadata(&name).is_ok(){let _=self.dir.remove_file(&name);}result
    }
    pub(crate) fn names(&self)->Result<Vec<String>>{
        let mut names=vec![];for item in self.dir.entries()?.take(4097){names.push(item?.file_name().into_string().map_err(|_|Error::State)?);}
        if names.len()>4096{return Err(Error::Unsupported);}names.sort();Ok(names)
    }
}
