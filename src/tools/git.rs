//! Local Git primitives. No remote operations, arbitrary Git argv, hooks or filters.
use crate::{config::Project,domain::ProjectId,error::{Error,Result}};
use cap_std::fs::{Dir,MetadataExt};
use serde::{Serialize,Deserialize};
use std::{collections::BTreeMap,io::{Read,Write},path::{Path,PathBuf},process::{Command,Stdio}};
use super::process::{ProcessRequest,GitMount};
pub const DESCRIPTOR:&str=concat!("rivet:git:v1:", env!("RIVET_BUILD_DIGEST"));
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitFile{pub text:String,pub executable:bool,#[serde(default)]pub deleted:bool}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(tag="operation",rename_all="snake_case",deny_unknown_fields)]
pub enum GitAction{Status,Diff,Commit{expected_head:Option<String>,expected_ref:String,message:String,author_name:String,author_email:String}}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitRequest{pub action:GitAction,pub files:BTreeMap<PathBuf,GitFile>}
#[derive(Debug,Serialize,Deserialize)]
pub struct GitResult{pub head:Option<String>,pub reference:String,pub entries:Vec<String>,pub diff:String,pub commit:Option<String>,pub policy:String}
pub struct GitWorkspace{project:Project,metadata:Dir,identity:(u64,u64),relative:PathBuf}
impl GitWorkspace{
    pub fn new(project:Project)->Result<Self>{
        let fd=rustix::fs::openat2(&project.dir,".git",rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Denied)?;
        let metadata=Dir::from_std_file(std::fs::File::from(fd));
        if metadata.symlink_metadata("commondir").is_ok(){return Err(Error::Unsupported);}
        fn verify(dir:&Dir,entries:&mut usize,depth:usize)->Result<()>{
            if depth>32{return Err(Error::Unsupported);}
            for item in dir.entries()?{
                *entries+=1;if *entries>20000{return Err(Error::Unsupported);}
                let name=item?.file_name();let m=dir.symlink_metadata(&name)?;
                if m.is_dir(){
                    let fd=rustix::fs::openat2(dir,&name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Denied)?;
                    verify(&Dir::from_std_file(std::fs::File::from(fd)),entries,depth+1)?;
                }else if !m.is_file()||m.nlink()!=1||m.len()>64*1024*1024{return Err(Error::Denied);}
            }Ok(())
        }
        verify(&metadata,&mut 0,0)?;
        let identity=project.dir.dir_metadata()?;
        let root=project.path();
        let relative=root.strip_prefix(Path::new(env!("RIVET_SOURCE_ROOT"))).or_else(|_|root.strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR"))))
            .map_err(|_|Error::Invalid)?.join(".git");
        Ok(Self{project,metadata,identity:(identity.dev(),identity.ino()),relative})
    }
    pub(crate) fn identity(&self)->(u64,u64){self.identity}
    pub(crate) fn mount(&self,writable:bool)->Result<GitMount>{
        if writable{for name in ["HEAD","config","index"]{let m=self.metadata.symlink_metadata(name).map_err(|_|Error::Unsupported)?;if !m.is_file()||m.nlink()!=1{return Err(Error::Denied);}}}
        let m=self.metadata.dir_metadata()?;
        let actual=self.project.open_directory(Path::new(".git"))?.dir_metadata()?;
        if actual.dev()!=m.dev()||actual.ino()!=m.ino(){return Err(Error::Conflict);}
        Ok(GitMount{relative:self.relative.clone(),device:m.dev(),inode:m.ino(),writable})
    }
}
impl GitRequest{
    pub fn writable(&self)->bool{matches!(self.action,GitAction::Commit{..})}
    pub fn resources(&self,id:ProjectId)->Vec<String>{vec![format!("project:{}:local-git",id.0),if self.writable(){"git:objects-heads-reflogs-and-ref-locks:write"}else{"git:metadata:read"}.into()]}
    pub fn validate(&self)->Result<()>{
        if self.files.is_empty()||self.files.values().any(|f|f.deleted&&(!f.text.is_empty()||f.executable)){return Err(Error::Invalid);}
        let request=ProcessRequest{program:"/usr/bin/git".into(),args:vec![],files:self.files.iter().map(|(p,f)|(p.clone(),f.text.clone())).collect(),timeout_ms:15000};request.validate()?;
        if self.files.keys().any(|p|p.components().any(|c|c.as_os_str().to_string_lossy().chars().any(char::is_control))){return Err(Error::Invalid);}
        if let GitAction::Commit{expected_head,expected_ref,message,author_name,author_email}=&self.action{
            if expected_head.as_ref().is_some_and(|v|!oid(v))||!reference(expected_ref)||message.len()>4096||message.is_empty()
                ||author_name.is_empty()||author_email.is_empty()||author_name.len()>128||author_email.len()>128
                ||author_name.chars().any(|c|c.is_control()||"<>".contains(c))||author_email.chars().any(|c|c.is_control()||"<> ".contains(c)){return Err(Error::Invalid);}
        }Ok(())
    }
    pub(crate) fn process(&self)->Result<ProcessRequest>{
        self.validate()?;
        let encoded=serde_json::to_string(self).map_err(|_|Error::Invalid)?;
        Ok(ProcessRequest{program:"/usr/bin/true".into(),args:vec![],files:BTreeMap::from([("__rivet_git_request.json".into(),encoded)]),timeout_ms:15000})
    }
}
fn oid(value:&str)->bool{[40,64].contains(&value.len())&&value.bytes().all(|c|c.is_ascii_hexdigit())}
fn reference(value:&str)->bool{value.starts_with("refs/heads/")&&!value.ends_with('/')&&!value.ends_with(".lock")&&!value.contains("..")&&!value.contains("@{")&&!value.chars().any(|c|c.is_control()||" ~^:?*[\\".contains(c))}
fn git(args:&[String],input:Option<&[u8]>,index:Option<&Path>,author:Option<(&str,&str)>)->Result<Vec<u8>>{
    let mut command=Command::new("/usr/bin/git");command.env_clear().env("PATH","/usr/bin")
        .env("GIT_CONFIG_NOSYSTEM","1").env("GIT_CONFIG_GLOBAL","/dev/null").env("GIT_ATTR_NOSYSTEM","1")
        .env("GIT_WORK_TREE","/work/input").env("GIT_OPTIONAL_LOCKS","0").env("GIT_NO_REPLACE_OBJECTS","1").env("GIT_NO_LAZY_FETCH","1").env("GIT_TERMINAL_PROMPT","0")
        .env("GIT_TRACE","0").env("GIT_TRACE2","0").env("GIT_TRACE2_EVENT","0").env("GIT_TRACE2_PERF","0");
    if let Some(index)=index{command.env("GIT_INDEX_FILE",index);}
    if let Some((name,email))=author{for prefix in ["GIT_AUTHOR","GIT_COMMITTER"]{command.env(format!("{prefix}_NAME"),name).env(format!("{prefix}_EMAIL"),email);}}
    command.args(["--no-pager","--literal-pathspecs","--git-dir=/git-metadata","-c","core.hooksPath=/dev/null","-c","core.fsmonitor=false","-c","core.untrackedCache=false","-c","core.splitIndex=false","-c","core.ignorecase=false","-c","core.logAllRefUpdates=false","-c","commit.gpgsign=false","-c","gc.auto=0","-c","maintenance.auto=false","-c","trace2.eventTarget=0","-c","trace2.perfTarget=0","-c","trace2.normalTarget=0"])
        .args(args).stdin(if input.is_some(){Stdio::piped()}else{Stdio::null()}).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child=command.spawn()?;
    let stdout=child.stdout.take().ok_or(Error::State)?;let stderr=child.stderr.take().ok_or(Error::State)?;
    let output=std::thread::spawn(move||{let mut data=Vec::new();stdout.take(65537).read_to_end(&mut data).map(|_|data)});
    let errors=std::thread::spawn(move||{let mut data=Vec::new();stderr.take(65537).read_to_end(&mut data).map(|_|data)});
    if let Some(data)=input{child.stdin.take().ok_or(Error::State)?.write_all(data)?;}
    let status=child.wait()?;
    let output=output.join().map_err(|_|Error::State)??;let errors=errors.join().map_err(|_|Error::State)??;
    if output.len()>65536||errors.len()>65536{return Err(Error::Unsupported);}
    if !status.success(){let phase=args.first().map(String::as_str).unwrap_or("unknown");let diagnostic=String::from_utf8_lossy(&errors);
        let phase=if phase=="update-ref"{if diagnostic.contains("packed-refs.lock"){"update-ref-packed-lock"}else if diagnostic.contains("HEAD.lock"){"update-ref-head-lock"}else if diagnostic.contains("logs/"){"update-ref-reflog"}else{"update-ref"}}else{phase};
        eprintln!("rivet-git-phase:{phase}");return Err(Error::State);}Ok(output)
}
fn text(bytes:Vec<u8>)->Result<String>{String::from_utf8(bytes).map_err(|_|Error::Unsupported)}
fn line(args:&[&str])->Result<String>{text(git(&args.iter().map(|s|(*s).into()).collect::<Vec<_>>(),None,None,None)?).map(|s|s.trim_end().to_owned())}
fn head_entry(head:&str,path:&str)->Result<Option<(String,String)>>{
    let bytes=git(&["ls-tree".into(),"-z".into(),head.into(),"--".into(),path.into()],None,None,None)?;
    if bytes.is_empty(){return Ok(None);}
    let record=text(bytes)?;let record=record.trim_end_matches('\0');
    let (meta,name)=record.split_once('\t').ok_or(Error::State)?;let fields=meta.split_whitespace().collect::<Vec<_>>();
    if name!=path||fields.len()!=3||fields[1]!="blob"||!oid(fields[2]){return Err(Error::Unsupported);}
    Ok(Some((fields[0].into(),fields[2].into())))
}
/// Runs only inside the already-restricted worker with an explicit metadata mount.
pub fn driver()->Result<GitResult>{
    let path=Path::new("/work/input/__rivet_git_request.json");
    if std::fs::metadata(path)?.len()>1024*1024{return Err(Error::Unsupported);}
    let request:GitRequest=serde_json::from_slice(&std::fs::read(path)?).map_err(|_|Error::Invalid)?;request.validate()?;
    let current_ref=line(&["symbolic-ref","--quiet","HEAD"])?;if !reference(&current_ref){return Err(Error::Unsupported);}
    let head=line(&["rev-parse","--verify","HEAD"]).ok().filter(|s|oid(s));
    if let Some(head)=&head{let tree=git(&["ls-tree".into(),"-r".into(),"--name-only".into(),"-z".into(),head.clone()],None,None,None)?;if tree.split(|b|*b==0).filter(|p|!p.is_empty()).count()>1024{return Err(Error::Unsupported);}}
    let mut result=GitResult{head:head.clone(),reference:current_ref.clone(),entries:vec![],diff:String::new(),commit:None,policy:"raw blobs; external helpers/hooks/filters/signing disabled; selected immutable inputs".into()};
    match &request.action{
        GitAction::Status=>{
            for (path,file) in &request.files{
                let path=path.to_str().ok_or(Error::Invalid)?;
                let stage=git(&["ls-files".into(),"--stage".into(),"-z".into(),"--".into(),path.into()],None,None,None)?;
                let record=text(stage)?;let record=record.trim_end_matches('\0');
                let expected=if record.is_empty(){None}else{let (meta,name)=record.split_once('\t').ok_or(Error::State)?;if name!=path{return Err(Error::State);}let fields=meta.split_whitespace().collect::<Vec<_>>();if fields.len()!=3||fields[2]!="0"||!oid(fields[1]){return Err(Error::Conflict);}Some((fields[0].to_owned(),fields[1].to_owned()))};
                let working=if file.deleted{None}else{let hash=text(git(&["hash-object".into(),"--stdin".into(),"--no-filters".into()],Some(file.text.as_bytes()),None,None)?)?.trim().to_owned();Some((if file.executable{"100755"}else{"100644"}.into(),hash))};
                let committed=match &head{Some(head)=>head_entry(head,path)?,None=>None};
                let staged=if committed==expected{' '}else if expected.is_none(){'D'}else if committed.is_none(){'A'}else{'M'};
                let work=if expected==working{' '}else if working.is_none(){'D'}else if expected.is_none(){'?'}else{'M'};
                result.entries.push(format!("{staged}{work} {path}"));
            }
        },
        GitAction::Diff=>{
            for (path,file) in &request.files{
                let entry=match &head{Some(head)=>head_entry(head,path.to_str().ok_or(Error::Invalid)?)?,None=>None};
                let mode=entry.as_ref().map(|(mode,_)|mode.clone());
                let before=match entry{Some((_,oid))=>text(git(&["cat-file".into(),"blob".into(),oid],None,None,None)?)?,None=>String::new()};
                let next_mode=if file.executable{"100755"}else{"100644"};
                match (&mode,file.deleted){
                    (Some(mode),true)=>result.diff.push_str(&format!("deleted file mode {mode} {}\n",path.display())),
                    (None,false)=>result.diff.push_str(&format!("new file mode {next_mode} {}\n",path.display())),
                    (Some(mode),false) if mode!=next_mode=>result.diff.push_str(&format!("old mode {mode}\nnew mode {next_mode} {}\n",path.display())),
                    _=>(),
                }
                if before!=file.text{result.diff.push_str(&super::files::FileWorkspace::diff(path,&before,&file.text)?);}
                if result.diff.len()>16384{return Err(Error::Unsupported);}
            }
        },
        GitAction::Commit{expected_head,expected_ref,message,author_name,author_email}=>{
            if head!=*expected_head||current_ref!=*expected_ref{return Err(Error::Conflict);}
            let index=Path::new("/work/tmp/rivet-index");
            git(&["read-tree".into(),head.clone().unwrap_or_else(||"--empty".into())],None,Some(index),None)?;
            for (path,file) in &request.files{
                if file.deleted{git(&["update-index".into(),"--force-remove".into(),"--".into(),path.to_str().ok_or(Error::Invalid)?.into()],None,Some(index),None)?;continue;}
                let hash=text(git(&["hash-object".into(),"-w".into(),"--stdin".into(),"--no-filters".into()],Some(file.text.as_bytes()),None,None)?)?.trim().to_owned();if !oid(&hash){return Err(Error::State);}
                git(&["update-index".into(),"--add".into(),"--cacheinfo".into(),if file.executable{"100755"}else{"100644"}.into(),hash,path.to_str().ok_or(Error::Invalid)?.into()],None,Some(index),None)?;
            }
            let tree=text(git(&["write-tree".into()],None,Some(index),None)?)?.trim().to_owned();if !oid(&tree){return Err(Error::State);}
            let mut args=vec!["commit-tree".into(),tree,"-m".into(),message.clone()];if let Some(head)=&head{args.extend(["-p".into(),head.clone()]);}
            let commit=text(git(&args,None,None,Some((author_name,author_email)))?)?.trim().to_owned();if !oid(&commit){return Err(Error::State);}
            let old=head.clone().unwrap_or_else(||"0".repeat(commit.len()));
            git(&["update-ref".into(),"--no-deref".into(),expected_ref.clone(),commit.clone(),old],None,None,None)?;result.commit=Some(commit);
        },
    }
    if !request.writable()&&line(&["rev-parse","--verify","HEAD"]).ok().filter(|s|oid(s))!=head{return Err(Error::Conflict);}
    let encoded=serde_json::to_string(&result).map_err(|_|Error::State)?;if encoded.len()>32700{return Err(Error::Unsupported);}Ok(result)
}

impl GitRequest{
    pub fn descriptor()->crate::tools::ToolDescriptor{
        let schema=serde_json::json!({"type":"object","additionalProperties":false,"required":["action","files"],"properties":{"action":{"oneOf":[
            {"type":"object","additionalProperties":false,"required":["operation"],"properties":{"operation":{"const":"status"}}},
            {"type":"object","additionalProperties":false,"required":["operation"],"properties":{"operation":{"const":"diff"}}},
            {"type":"object","additionalProperties":false,"required":["operation","expected_head","expected_ref","message","author_name","author_email"],"properties":{
                "operation":{"const":"commit"},"expected_head":{"type":["string","null"]},"expected_ref":{"type":"string","pattern":"^refs/heads/"},
                "message":{"type":"string","minLength":1,"maxLength":4096},"author_name":{"type":"string","minLength":1,"maxLength":128},"author_email":{"type":"string","minLength":1,"maxLength":128}
            }}]},"files":{"type":"object","minProperties":1,"maxProperties":1024,"additionalProperties":{"type":"object","required":["text","executable"],"additionalProperties":false,"properties":{"text":{"type":"string"},"executable":{"type":"boolean"},"deleted":{"type":"boolean"}}}}}});
        crate::tools::ToolDescriptor{id:"rivet:git".into(),version:1,build_digest:DESCRIPTOR.into(),input_schema:schema.to_string(),output_limit:32700,effect:crate::tools::EffectClass::Mutation,permissions:vec!["local-git".into()],platform:"linux".into(),cancellable:true,reconcilable:false}
    }
}
