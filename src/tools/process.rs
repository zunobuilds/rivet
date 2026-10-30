//! Cgroup-bound, no-network checks against an explicit read-only source projection.
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::{Component, Path, PathBuf}, process::Stdio, time::{Duration, Instant}};
use tokio::{io::{AsyncRead, AsyncReadExt}, process::Command};
use tokio_util::sync::CancellationToken;

pub const DESCRIPTOR: &str = "rivet:process-check:v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessRequest {
    pub program: String,
    pub args: Vec<String>,
    pub files: BTreeMap<PathBuf, String>,
    pub timeout_ms: u64,
}
#[derive(Debug, Serialize)]
pub struct ProcessResult {
    pub exit_code: Option<i32>, pub stdout: String, pub stderr: String,
    pub truncated: bool, pub cancelled: bool, pub elapsed_ms: u64, pub unit: String,
}
impl ProcessRequest {
    pub fn validate(&self) -> Result<()> {
        let program=Path::new(&self.program);
        if !program.is_absolute() || !self.program.starts_with("/usr/bin/")
            || program.components().any(|c| !matches!(c,Component::RootDir|Component::Normal(_)))
            || self.args.len()>128 || self.args.iter().map(String::len).sum::<usize>()>65536
            || self.timeout_ms==0 || self.timeout_ms>60000 || self.files.len()>1024 { return Err(Error::Invalid); }
        let mut size=0;
        for (path,text) in &self.files {
            if path.as_os_str().is_empty() || path.components().any(|c| !matches!(c,Component::Normal(_))) {return Err(Error::Invalid);}
            for component in path.components() {
                let name=component.as_os_str().to_str().ok_or(Error::Invalid)?;
                if name.chars().any(char::is_control) || [".git",".rivet-state",".cargo",".rivet.toml","secrets","credentials"].contains(&name) || name.starts_with(".env") {return Err(Error::Denied);}
            }
            size+=text.len(); if size>1024*1024 {return Err(Error::Unsupported);}
        }
        Ok(())
    }
}
fn manager(program: &str) -> Command {
    let mut command=Command::new(program);
    command.env_clear().env("PATH","/usr/bin").env("SYSTEMD_BUS_TIMEOUT","2s");
    for key in ["XDG_RUNTIME_DIR","DBUS_SESSION_BUS_ADDRESS"] {
        if let Some(value)=std::env::var_os(key) {command.env(key,value);}
    }
    command
}
struct OwnedUnit { name:String }
impl OwnedUnit {
    async fn signal(&self, signal:&str) -> Result<()> {
        let status=manager("/usr/bin/systemctl").args(["--user","kill","--kill-whom=all",signal,&self.name])
            .stdout(Stdio::null()).stderr(Stdio::null()).status().await?;
        if status.success() {Ok(())} else {Err(Error::State)}
    }
    async fn stop(&self) -> Result<()> {
        // Only our UUID-named unit is affected. Unit TimeoutStopSec bounds grace.
        let result=tokio::time::timeout(Duration::from_secs(7),manager("/usr/bin/systemctl")
            .args(["--user","stop",&self.name]).stdout(Stdio::null()).stderr(Stdio::null()).status()).await;
        if !matches!(result,Ok(Ok(_))){return Err(Error::State);}
        for _ in 0..20 {
            let state=manager("/usr/bin/systemctl").args(["--user","show",&self.name,"--property=LoadState","--value"]).output().await?;
            if String::from_utf8_lossy(&state.stdout).trim()=="not-found"{return Ok(());}
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err(Error::State)
    }
}
impl Drop for OwnedUnit {
    fn drop(&mut self) {
        // Also executes on dropped dispatch futures. No broad kill/name matching.
        let _=std::process::Command::new("/usr/bin/systemctl").args(["--user","stop","--no-block",&self.name])
            .env("SYSTEMD_BUS_TIMEOUT","2s").stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}
async fn drain<R:AsyncRead+Unpin>(mut input:R, overflow:CancellationToken)->Result<(Vec<u8>,bool)> {
    let mut bytes=Vec::new(); let mut truncated=false; let mut chunk=[0;4096];
    loop {
        let count=input.read(&mut chunk).await?; if count==0 {break;}
        let remaining=32768_usize.saturating_sub(bytes.len());
        bytes.extend_from_slice(&chunk[..count.min(remaining)]);
        if count>remaining {truncated=true;overflow.cancel();}
    }
    Ok((bytes,truncated))
}
struct Stage { parent:cap_std::fs::Dir, name:String, input:cap_std::fs::Dir }
impl Stage {
    fn directory(parent:&cap_std::fs::Dir,name:&str)->Result<cap_std::fs::Dir>{
        match parent.create_dir(name){Ok(())=>(),Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>(),Err(_)=>return Err(Error::State)}
        let fd=rustix::fs::openat2(parent,name,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Denied)?;
        Ok(cap_std::fs::Dir::from_std_file(std::fs::File::from(fd)))
    }
    fn new(root:&Path,id:crate::domain::OperationId,files:&BTreeMap<PathBuf,String>)->Result<Self>{
        use cap_std::fs::{OpenOptions,OpenOptionsExt,Permissions,PermissionsExt};
        let project=crate::config::Project::open(root)?;
        let build=Self::directory(&project.dir,".build")?;
        let parent=Self::directory(&build,"runtime")?;
        let name=id.0.to_string();parent.create_dir(&name)?;
        parent.set_permissions(&name,Permissions::from_mode(0o700))?;
        let base=Self::directory(&parent,&name)?;
        let input=Self::directory(&base,"input")?;
        let stage=Self{parent,name,input};
        for (path,text) in files {
            let mut dir=stage.input.try_clone()?;
            if let Some(parent)=path.parent(){for part in parent.components(){dir=Self::directory(&dir,part.as_os_str().to_str().ok_or(Error::Invalid)?)?;}}
            let mut options=OpenOptions::new();options.write(true).create_new(true).mode(0o600).custom_flags(0o400000);
            let mut file=dir.open_with(path.file_name().ok_or(Error::Invalid)?,&options)?.into_std();
            std::io::Write::write_all(&mut file,text.as_bytes())?;
        }
        Ok(stage)
    }
    fn identity(&self)->Result<(u64,u64)>{use cap_std::fs::MetadataExt;let m=self.input.dir_metadata()?;Ok((m.dev(),m.ino()))}
    fn cleanup(&self)->Result<()>{self.parent.remove_dir_all(&self.name)?;Ok(())}
}
impl Drop for Stage{fn drop(&mut self){let _=self.parent.remove_dir_all(&self.name);}}
/// Invoked by the supervisor only after a committed intent. Projection files are explicit inputs.
pub(crate) async fn perform(request:ProcessRequest, operation:crate::domain::OperationId, cancel:CancellationToken)->Result<ProcessResult> {
    request.validate()?; if cancel.is_cancelled(){return Err(Error::Interrupted);}
    let root=Path::new(env!("RIVET_SOURCE_ROOT"));
    let stage=Stage::new(root,operation,&request.files)?;
    let executable=root.join(".build/target/debug/rivet");
    let name=format!("rivet-worker-{}.service",operation.0);
    let unit=OwnedUnit{name:name.clone()};
    let mut command=manager("/usr/bin/systemd-run");
    command.args(["--user","--collect","--wait","--pipe","--service-type=exec","--quiet","--unit"]).arg(&name);
    command.args(["--property=Delegate=cpu memory pids","--property=CPUQuota=100%","--property=CPUQuotaPeriodSec=100ms","--property=MemoryMax=512M",
        "--property=MemorySwapMax=0","--property=TasksMax=32","--property=KillMode=control-group","--property=TimeoutStopSec=5s",
        "--property=NoNewPrivileges=yes","--property=OOMPolicy=kill"]);
    command.arg(format!("--property=RuntimeMaxSec={}ms",request.timeout_ms+7000));
    let identity=stage.identity()?;
    command.args(["/usr/bin/env","-i","PATH=/usr/bin"]).arg(&executable).arg("__unit-launch").arg(&name).arg(identity.0.to_string()).arg(identity.1.to_string()).arg("--");
    command.args(["/usr/bin/bwrap","--unshare-all","--unshare-user","--disable-userns","--die-with-parent","--new-session","--cap-drop","ALL"]);
    for path in ["/usr/bin","/usr/lib","/usr/lib64","/usr/libexec","/usr/include"] {command.args(["--ro-bind",path,path]);}
    command.args(["--symlink","usr/bin","/bin","--symlink","usr/lib","/lib","--symlink","usr/lib64","/lib64",
        "--ro-bind","/usr/bin/ld.bfd","/etc/alternatives/ld","--proc","/proc","--dev","/dev"]);
    command.args(["--ro-bind-fd","__RIVET_INPUT_FD__","/work/input"]);
    command.arg("--ro-bind").arg(&executable).arg("/rivet-worker");
    command.args(["--size","67108864","--tmpfs","/work/tmp","--symlink","work/tmp","/tmp","--chdir","/work/input",
        "--clearenv","--setenv","PATH","/usr/bin","--setenv","TMPDIR","/work/tmp","--setenv","PYTHONDONTWRITEBYTECODE","1",
        "--remount-ro","/","--","/rivet-worker","__worker-bootstrap","--"]);
    command.arg(&request.program).args(&request.args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let started=Instant::now(); let mut child=command.spawn()?;
    let overflow=CancellationToken::new();
    let out=tokio::spawn(drain(child.stdout.take().ok_or(Error::State)?,overflow.clone()));
    let err=tokio::spawn(drain(child.stderr.take().ok_or(Error::State)?,overflow.clone()));
    let interrupted=tokio::select! {
        biased;
        _=cancel.cancelled()=>true,
        _=overflow.cancelled()=>true,
        _=tokio::time::sleep(Duration::from_millis(request.timeout_ms))=>true,
        result=child.wait()=>{result?;false},
    };
    if interrupted {
        let _=unit.signal("--signal=TERM").await;
        if tokio::time::timeout(Duration::from_secs(5),child.wait()).await.is_err(){let _=unit.signal("--signal=KILL").await;}
    }
    unit.stop().await?;
    let status=tokio::time::timeout(Duration::from_secs(2),child.wait()).await.map_err(|_|Error::State)??;
    let (stdout,out_cut)=tokio::time::timeout(Duration::from_secs(2),out).await.map_err(|_|Error::State)?.map_err(|_|Error::State)??;
    let (stderr,err_cut)=tokio::time::timeout(Duration::from_secs(2),err).await.map_err(|_|Error::State)?.map_err(|_|Error::State)??;
    stage.cleanup()?;
    fn bounded(bytes:&[u8])->(String,bool){let mut value=String::from_utf8_lossy(bytes).into_owned();let cut=value.len()>32768;if cut{let mut end=32768;while !value.is_char_boundary(end){end-=1;}value.truncate(end);} (value,cut)}
    let (stdout,out_loss)=bounded(&stdout);let (stderr,err_loss)=bounded(&stderr);
    Ok(ProcessResult{exit_code:status.code(),stdout,stderr,truncated:out_cut||err_cut||out_loss||err_loss,cancelled:interrupted,elapsed_ms:started.elapsed().as_millis() as u64,unit:name})
}
/// Single-threaded bootstrap: apply mandatory rules before any untrusted exec.
pub fn bootstrap(args:&[String])->Result<()> {
    use landlock::{Access,AccessFs,AccessNet,Compatible,CompatLevel,PathBeneath,PathFd,Ruleset,RulesetAttr,RulesetCreatedAttr,RulesetStatus,Scope,ABI};
    use std::os::unix::process::CommandExt;
    if args.is_empty() || !args[0].starts_with("/usr/bin/"){return Err(Error::Invalid);}
    let descriptors=std::fs::read_dir("/proc/self/fd")?.map(|e|e.map(|e|e.path())).collect::<std::io::Result<Vec<_>>>()?;
    for path in descriptors {
        let number=path.file_name().and_then(|p|p.to_str()).and_then(|p|p.parse::<u32>().ok()).ok_or(Error::State)?;
        if number>2 && std::fs::read_link(path).is_ok(){return Err(Error::State);}
    }
    if !std::fs::read_to_string("/proc/self/status")?.lines().any(|line|line.split_whitespace().collect::<Vec<_>>()==["NoNewPrivs:","1"]){return Err(Error::Unsupported);}
    let abi=ABI::V6;
    let mut rules=Ruleset::default().set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(abi)).map_err(|_|Error::Unsupported)?
        .handle_access(AccessNet::BindTcp|AccessNet::ConnectTcp).map_err(|_|Error::Unsupported)?
        .scope(Scope::AbstractUnixSocket).map_err(|_|Error::Unsupported)?
        .scope(Scope::Signal).map_err(|_|Error::Unsupported)?
        .create().map_err(|_|Error::Unsupported)?;
    for path in ["/usr","/work/input","/proc"] {
        rules=rules.add_rule(PathBeneath::new(PathFd::new(path).map_err(|_|Error::State)?,AccessFs::from_read(abi))).map_err(|_|Error::Unsupported)?;
    }
    rules=rules.add_rule(PathBeneath::new(PathFd::new("/work/tmp").map_err(|_|Error::State)?,AccessFs::from_all(abi))).map_err(|_|Error::Unsupported)?;
    for path in ["/dev/null","/dev/zero","/dev/random","/dev/urandom"] {
        let rights=if path=="/dev/random"||path=="/dev/urandom"{AccessFs::ReadFile.into()}else{AccessFs::ReadFile|AccessFs::WriteFile};
        rules=rules.add_rule(PathBeneath::new(PathFd::new(path).map_err(|_|Error::State)?,rights)).map_err(|_|Error::Unsupported)?;
    }
    let status=rules.restrict_self().map_err(|_|Error::Unsupported)?;
    if status.ruleset!=RulesetStatus::FullyEnforced {return Err(Error::Unsupported);}
    tracing::debug!("strict worker restrictions applied");
    let _error=std::process::Command::new(&args[0]).args(&args[1..]).env_clear()
        .env("PATH","/usr/bin").env("TMPDIR","/work/tmp").env("PYTHONDONTWRITEBYTECODE","1").env("PYTHONNOUSERSITE","1").exec();
    Err(Error::State)
}

/// Trusted service entry: verify actual kernel limits before mounting/executing the worker.
pub fn unit_launch(unit:&str, device:&str, inode:&str, args:&[String])->Result<()> {
    use std::os::unix::process::CommandExt;
    let id=unit.strip_prefix("rivet-worker-").and_then(|s|s.strip_suffix(".service")).ok_or(Error::Invalid)?;
    uuid::Uuid::parse_str(id).map_err(|_|Error::Invalid)?;
    if args.first().map(String::as_str)!=Some("/usr/bin/bwrap"){return Err(Error::Invalid);}
    let membership=std::fs::read_to_string("/proc/self/cgroup")?;
    let relative=membership.lines().find_map(|l|l.strip_prefix("0::")).ok_or(Error::Unsupported)?;
    let path=Path::new(relative);
    if path.file_name().and_then(|p|p.to_str())!=Some(unit) || path.components().any(|c|!matches!(c,Component::RootDir|Component::Normal(_))){return Err(Error::Denied);}
    let root=Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    for (file,expected) in [("cpu.max","100000 100000"),("memory.max","536870912"),("memory.swap.max","0"),("pids.max","32")] {
        if std::fs::read_to_string(root.join(file))?.trim()!=expected{return Err(Error::Unsupported);}
    }
    use cap_std::fs::MetadataExt;
    use std::os::fd::AsRawFd;
    let project=crate::config::Project::open(Path::new(env!("RIVET_SOURCE_ROOT")))?;
    let relative=format!(".build/runtime/{id}/input");
    let fd=rustix::fs::openat2(&project.dir,relative,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Denied)?;
    let input=cap_std::fs::Dir::from_std_file(std::fs::File::from(fd));let metadata=input.dir_metadata()?;
    if metadata.dev()!=device.parse::<u64>().map_err(|_|Error::Invalid)? || metadata.ino()!=inode.parse::<u64>().map_err(|_|Error::Invalid)?{return Err(Error::Denied);}
    let mut args=args.to_vec();let position=args.iter().position(|a|a=="--ro-bind-fd").ok_or(Error::Invalid)?;
    if args.get(position+1).map(String::as_str)!=Some("__RIVET_INPUT_FD__"){return Err(Error::Invalid);}
    args[position+1]=input.as_raw_fd().to_string();
    rustix::io::fcntl_setfd(&input,rustix::io::FdFlags::empty()).map_err(|_|Error::State)?;
    let _error=std::process::Command::new(&args[0]).args(&args[1..]).env_clear().env("PATH","/usr/bin").exec();
    Err(Error::State)
}
impl ProcessRequest {
    pub fn resources(&self,project:crate::domain::ProjectId)->Vec<String>{vec![format!("project:{}:isolated-check",project.0),format!("program:{}",self.program),"profile:systemd-bwrap-landlock-v1".into()]}
}

impl ProcessRequest {
    pub fn descriptor()->crate::tools::ToolDescriptor{
        let schema=serde_json::json!({"type":"object","additionalProperties":false,"required":["program","args","files","timeout_ms"],"properties":{
            "program":{"type":"string","pattern":"^/usr/bin/"},"args":{"type":"array","maxItems":128,"items":{"type":"string"}},
            "files":{"type":"object","maxProperties":1024,"additionalProperties":{"type":"string"}},"timeout_ms":{"type":"integer","minimum":1,"maximum":60000}}});
        crate::tools::ToolDescriptor{id:"rivet:process-check".into(),version:1,build_digest:DESCRIPTOR.into(),input_schema:schema.to_string(),output_limit:65536,effect:crate::tools::EffectClass::External,permissions:vec!["process".into(),"source-projection".into()],platform:"linux".into(),cancellable:true,reconcilable:false}
    }
}
