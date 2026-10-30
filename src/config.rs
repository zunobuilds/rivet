use crate::error::{Error, Result};
use cap_std::{ambient_authority, fs::{Dir, OpenOptions, OpenOptionsExt}};
use serde::{Deserialize, Serialize};
use std::{path::{Component, Path, PathBuf}};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig { pub profile: String, pub endpoint: String, pub model: String }
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    #[serde(default)] pub provider: Option<ProviderConfig>,
    #[serde(default)] pub mcp: Vec<crate::mcp::Server>,
    #[serde(default)] pub skills: Vec<crate::skills::Skill>,
}
impl Default for Config { fn default() -> Self { Self { schema_version: 1, provider: None, mcp:vec![],skills:vec![] } } }
impl Config {
    pub fn parse(input: &str) -> Result<Self> {
        let config: Self = toml::from_str(input).map_err(|_| Error::Invalid)?;
        if config.schema_version != 1 { return Err(Error::Invalid); }
        if let Some(p) = &config.provider {
            if p.profile.is_empty() || p.endpoint.is_empty() || p.model.is_empty() { return Err(Error::Invalid); }
        }
        crate::skills::validate(&config.skills)?;
        Ok(config)
    }
    pub fn provider_status(&self) -> &'static str {
        if self.provider.is_some() { "configured-unverified" } else { "absent" }
    }
}
/// Project descriptor and lexical paths. No parent/home discovery or canonicalize of user paths.
pub struct Project { pub(crate) dir: Dir, root: PathBuf }
impl Project {
    pub fn open(root: &Path) -> Result<Self> {
        let executable = std::env::current_exe()?;
        let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let source_root = Path::new(env!("RIVET_SOURCE_ROOT"));
        let boundary = if executable.starts_with(manifest_root) { manifest_root }
            else if executable.starts_with(source_root) { source_root }
            else { return Err(Error::Invalid); };
        let relative = root.strip_prefix(boundary).map_err(|_| Error::Invalid)?;
        let base = Self { dir: Dir::open_ambient_dir(boundary, ambient_authority())?, root: boundary.to_owned() };
        let dir = if relative.as_os_str().is_empty() { base.dir.try_clone()? } else { base.open_directory(relative)? };
        Ok(Self { dir, root: root.to_owned() })
    }

    pub(crate) fn path(&self)->&Path {&self.root}
    pub fn relative(&self, path: &Path) -> Result<PathBuf> {
        let path = if path.is_absolute() { path.strip_prefix(&self.root).map_err(|_| Error::Invalid)? } else { path };
        if path.as_os_str().is_empty() || path.components().any(|c| !matches!(c, Component::Normal(_))) { return Err(Error::Invalid); }
        Ok(path.to_owned())
    }
    pub(crate) fn check_components(&self, path: &Path) -> Result<bool> {
        let mut current = PathBuf::new();
        for part in path.components() {
            current.push(part);
            match self.dir.symlink_metadata(&current) {
                Ok(m) if m.file_type().is_symlink() => return Err(Error::State),
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(_) => return Err(Error::State),
            }
        }
        Ok(true)
    }
    pub(crate) fn open_directory(&self, path: &Path) -> Result<Dir> {
        let mut current = self.dir.try_clone()?;
        for component in path.components() {
            if !matches!(component, Component::Normal(_)) { return Err(Error::Invalid); }
            let mut options = OpenOptions::new();
            // Linux O_DIRECTORY | O_NOFOLLOW; application qualification is Linux-only.
            options.read(true).custom_flags(0o200000 | 0o400000);
            let file = current.open_with(component.as_os_str(), &options)?;
            current = Dir::from_std_file(file.into_std());
        }
        Ok(current)
    }
    pub fn config(&self, path: Option<&Path>) -> Result<Config> {
        let relative = self.relative(path.unwrap_or(Path::new(".rivet.toml")))?;
        let parent=relative.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let resolve=rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV;
        let parent=rustix::fs::openat2(&self.dir,parent,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::DIRECTORY|rustix::fs::OFlags::CLOEXEC,rustix::fs::Mode::empty(),resolve).map_err(|e|if e==rustix::io::Errno::LOOP||e==rustix::io::Errno::XDEV{Error::State}else{Error::Invalid})?;
        let fd=match rustix::fs::openat2(&parent,relative.file_name().ok_or(Error::Invalid)?,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::CLOEXEC|rustix::fs::OFlags::NONBLOCK,rustix::fs::Mode::empty(),resolve){
            Ok(fd)=>fd,Err(rustix::io::Errno::NOENT) if path.is_none()=>return Ok(Config::default()),Err(rustix::io::Errno::LOOP|rustix::io::Errno::XDEV)=>return Err(Error::State),Err(_)=>return Err(Error::Invalid),
        };
        let mut file=std::fs::File::from(fd);let opened=file.metadata()?;
        use std::os::unix::fs::MetadataExt;
        if !opened.is_file()||opened.nlink()!=1||opened.len()>1024*1024{return Err(Error::Invalid);}
        use std::io::Read;
        let mut input=String::new();(&mut file).take(1024*1024+1).read_to_string(&mut input).map_err(|_|Error::Invalid)?;
        let after=file.metadata()?;
        if input.len()>1024*1024||opened.len()!=after.len()||opened.mtime()!=after.mtime()||opened.mtime_nsec()!=after.mtime_nsec()||opened.ctime()!=after.ctime()||opened.ctime_nsec()!=after.ctime_nsec(){return Err(Error::Invalid);}
        Config::parse(&input)
    }
}

impl Project {
    /// Explicit trusted CLI input, never exposed as a model-visible file capability.
    pub(crate) fn input(&self,path:&Path)->Result<Vec<u8>>{
        use std::io::Read;use std::os::unix::fs::MetadataExt;
        let relative=self.relative(path)?;let fd=rustix::fs::openat2(&self.dir,&relative,rustix::fs::OFlags::RDONLY|rustix::fs::OFlags::CLOEXEC|rustix::fs::OFlags::NONBLOCK,rustix::fs::Mode::empty(),rustix::fs::ResolveFlags::BENEATH|rustix::fs::ResolveFlags::NO_SYMLINKS|rustix::fs::ResolveFlags::NO_XDEV).map_err(|_|Error::Invalid)?;let mut file=std::fs::File::from(fd);let before=file.metadata()?;
        if !before.is_file()||before.nlink()!=1||before.len()>65536{return Err(Error::Invalid);}let mut bytes=vec![];(&mut file).take(65537).read_to_end(&mut bytes)?;let after=file.metadata()?;if bytes.len() as u64!=before.len()||before.mtime()!=after.mtime()||before.mtime_nsec()!=after.mtime_nsec(){return Err(Error::Conflict);}Ok(bytes)
    }
}
