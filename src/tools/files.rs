//! Descriptor-root text operations. Only the supervisor can perform an operation.
use crate::{config::Project, error::{Error, Result}};
use cap_std::fs::{Dir, MetadataExt, OpenOptions, OpenOptionsExt};
use serde::{Deserialize, Serialize};
use std::{io::{Read, Write}, path::{Component, Path, PathBuf}, sync::Mutex};
use super::sha256;
use std::os::unix::fs::MetadataExt as StdMetadataExt;
use rustix::fs::{openat2, OFlags, ResolveFlags};
use tokio_util::sync::CancellationToken;

pub const DESCRIPTOR: &str = concat!("rivet:files:v1:", env!("RIVET_BUILD_DIGEST"));
const LIMIT: usize = 64 * 1024;
const NOFOLLOW: i32 = 0o400000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision { pub digest: String, pub device: u64, pub inode: u64 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum FileAction {
    Read { path: PathBuf },
    Search { path: PathBuf, literal: String },
    Write { path: PathBuf, expected: Option<Revision>, text: String },
    Patch { path: PathBuf, expected: Revision, old: String, new: String },
    Delete { path: PathBuf, expected: Revision },
    Rename { from: PathBuf, to: PathBuf, expected: Revision },
}
#[derive(Clone, Debug, Serialize)]
pub struct FileResult {
    pub text: String,
    pub revision: Option<Revision>,
    pub diff: String,
    pub changed: bool,
}
/// Host-created capability. No provider receives this descriptor or its mutation lane.
pub struct FileWorkspace { project: Project, state: PathBuf, device: u64, inode: u64, lane: Mutex<()> }
impl FileWorkspace {
    pub fn descriptor() -> super::ToolDescriptor {
        use serde_json::json;
        fn variant(operation: &str, mut properties: serde_json::Value, required: &[&str]) -> serde_json::Value {
            properties["operation"] = json!({"const":operation});
            let mut required = required.to_vec(); required.push("operation");
            json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
        }
        let path = json!({"type":"string","minLength":1});
        let revision = json!({"$ref":"#/$defs/revision"});
        let schema = json!({
            "$schema":"https://json-schema.org/draft/2020-12/schema",
            "$defs":{"revision":{"type":"object","required":["digest","device","inode"],"additionalProperties":false,
                "properties":{"digest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"device":{"type":"integer","minimum":0},"inode":{"type":"integer","minimum":0}}}},
            "oneOf":[
                variant("read",json!({"path":path}), &["path"]),
                variant("search",json!({"path":path,"literal":{"type":"string","minLength":1}}), &["path","literal"]),
                variant("write",json!({"path":path,"expected":{"anyOf":[revision,{"type":"null"}]},"text":{"type":"string"}}), &["path","expected","text"]),
                variant("patch",json!({"path":path,"expected":revision,"old":{"type":"string","minLength":1},"new":{"type":"string"}}), &["path","expected","old","new"]),
                variant("delete",json!({"path":path,"expected":revision}), &["path","expected"]),
                variant("rename",json!({"from":path,"to":path,"expected":revision}), &["from","to","expected"])
            ]
        });
        super::ToolDescriptor {
            id:"rivet:files".into(),version:1,build_digest:DESCRIPTOR.into(), input_schema:schema.to_string(),
            output_limit:LIMIT as u64,effect:super::EffectClass::Mutation,permissions:vec!["files".into()],
            platform:"linux".into(),cancellable:true,reconcilable:false,
        }
    }
    pub fn new(project: Project, state: &Path) -> Result<Self> {
        let state = project.relative(state)?;
        let metadata = project.dir.dir_metadata()?;
        let device = metadata.dev(); let inode = metadata.ino();
        Ok(Self { project, state, device, inode, lane: Mutex::new(()) })
    }
    pub(crate) fn identity(&self) -> (u64, u64) { (self.device, self.inode) }
    fn path(&self, path: &Path) -> Result<PathBuf> {
        let path = self.project.relative(path)?;
        if path.starts_with(&self.state) { return Err(Error::Denied); }
        for component in path.components() {
            let Component::Normal(component) = component else { return Err(Error::Invalid); };
            let name = component.to_str().ok_or(Error::Invalid)?;
            if name.chars().any(char::is_control) { return Err(Error::Invalid); }
            if [".git", ".rivet-state", ".build", ".cargo", ".rivet.toml", "secrets", "credentials", ".ssh", ".gnupg", ".aws", ".netrc", ".git-credentials", ".npmrc", ".pypirc"].contains(&name)
                || name.starts_with(".env") || name.starts_with(".rivet-edit-") { return Err(Error::Denied); }
        }
        Ok(path)
    }
    pub fn resources(&self, action: &FileAction) -> Result<Vec<String>> {
        let paths = match action {
            FileAction::Read { path } | FileAction::Search { path, .. } | FileAction::Write { path, .. }
                | FileAction::Patch { path, .. } | FileAction::Delete { path, .. } => vec![self.path(path)?],
            FileAction::Rename { from, to, .. } => {
                let from = self.path(from)?; let to = self.path(to)?;
                if from == to { return Err(Error::Invalid); } vec![from, to]
            },
        };
        let mut resources: Vec<String> = paths.iter().map(|p| format!("workspace:{}:{}:file:{}", self.device, self.inode, p.display())).collect();
        resources.sort(); Ok(resources)
    }
    fn parent(&self, path: &Path) -> Result<(Dir, String)> {
        let path = self.path(path)?;
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
        let dir = match parent { Some(p) => self.directory(p)?, None => self.project.dir.try_clone()? };
        if dir.dir_metadata()?.dev() != self.device { return Err(Error::Denied); }
        Ok((dir, path.file_name().and_then(|p| p.to_str()).ok_or(Error::Invalid)?.to_owned()))
    }
    fn directory(&self, path: &Path) -> Result<Dir> {
        let fd = openat2(&self.project.dir, path, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            rustix::fs::Mode::empty(), ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV)
            .map_err(|e| if e == rustix::io::Errno::NOSYS { Error::Unsupported } else { Error::Denied })?;
        Ok(Dir::from_std_file(std::fs::File::from(fd)))
    }
    fn snapshot_at(&self, dir: &Dir, name: &str) -> Result<(String, Revision, u32)> {
        let metadata = dir.symlink_metadata(name)?;
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.dev() != self.device { return Err(Error::Denied); }
        if metadata.len() > LIMIT as u64 { return Err(Error::Unsupported); }
        let fd = openat2(dir, name, OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            rustix::fs::Mode::empty(), ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV)
            .map_err(|_| Error::Denied)?;
        let mut file = std::fs::File::from(fd);
        let opened = file.metadata()?;
        if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() || !opened.is_file() || opened.nlink() != 1 { return Err(Error::Conflict); }
        let mut bytes = Vec::new(); (&mut file).take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
        let after = file.metadata()?;
        if opened.len() != after.len() || opened.mtime() != after.mtime() || opened.mtime_nsec() != after.mtime_nsec()
            || opened.ctime() != after.ctime() || opened.ctime_nsec() != after.ctime_nsec() { return Err(Error::Conflict); }
        if bytes.len() > LIMIT { return Err(Error::Unsupported); }
        let revision = Revision { digest: sha256(&bytes), device: metadata.dev(), inode: metadata.ino() };
        let text = String::from_utf8(bytes).map_err(|_| Error::Unsupported)?;
        Ok((text, revision, metadata.mode() & 0o777))
    }
    fn snapshot(&self, path: &Path) -> Result<(String, Revision, u32)> { let (dir, name) = self.parent(path)?; self.snapshot_at(&dir, &name) }
    fn expected(&self, dir: &Dir, name: &str, expected: &Option<Revision>) -> Result<Option<(String, u32)>> {
        match (dir.symlink_metadata(name), expected) {
            (Err(e), None) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            (Ok(_), Some(expected)) => {
                let (text, actual, mode) = self.snapshot_at(dir, name)?;
                if &actual != expected { return Err(Error::Conflict); } Ok(Some((text, mode)))
            },
            (Ok(_), None) => Err(Error::Conflict),
            (Err(e), Some(_)) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::Conflict),
            _ => Err(Error::State),
        }
    }
    pub(crate) fn diff(path: &Path, before: &str, after: &str) -> Result<String> {
        let mut diff = format!("--- a/{}\n+++ b/{}\n@@ -1,{} +1,{} @@\n", path.display(), path.display(), before.lines().count(), after.lines().count());
        for (prefix, text) in [("-", before), ("+", after)] {
            for line in text.lines() { diff.push_str(prefix); diff.push_str(line); diff.push('\n'); }
            if !text.is_empty() && !text.ends_with('\n') { diff.push_str("\\ No newline at end of file\n"); }
        }
        if diff.len() > LIMIT { return Err(Error::Unsupported); } Ok(diff)
    }
    fn sync(dir: &Dir) -> Result<()> {
        // Capability directories may hold O_PATH descriptors, which cannot fsync.
        let fd=openat2(dir,".",OFlags::RDONLY|OFlags::DIRECTORY|OFlags::CLOEXEC,rustix::fs::Mode::empty(),ResolveFlags::BENEATH|ResolveFlags::NO_SYMLINKS|ResolveFlags::NO_XDEV).map_err(|_|Error::State)?;
        std::fs::File::from(fd).sync_all()?;Ok(())
    }
    fn write(&self, path: &Path, expected: &Option<Revision>, text: &str, stop: &CancellationToken) -> Result<FileResult> {
        if text.len() > LIMIT { return Err(Error::Unsupported); }
        let (dir, name) = self.parent(path)?;
        let original = self.expected(&dir, &name, expected)?;
        let before = original.as_ref().map_or("", |(text, _)| text.as_str());
        if original.is_some() && before == text { return Ok(FileResult { text: String::new(), revision: expected.clone(), diff: String::new(), changed: false }); }
        let diff = Self::diff(path, before, text)?;
        let temporary = format!(".rivet-edit-{}", uuid::Uuid::new_v4());
        let mut options = OpenOptions::new(); options.write(true).create_new(true).mode(original.as_ref().map_or(0o600, |(_, mode)| *mode)).custom_flags(NOFOLLOW);
        let mut staged = dir.open_with(&temporary, &options)?.into_std();
        let result = (|| {
            staged.write_all(text.as_bytes())?;
            if let Some((_, mode)) = &original {
                use std::os::unix::fs::PermissionsExt;
                staged.set_permissions(std::fs::Permissions::from_mode(*mode))?;
            }
            staged.sync_all()?;
            // Revalidate after staging, immediately before the atomic replacement.
            self.expected(&dir, &name, expected)?;
            if stop.is_cancelled() { return Err(Error::Interrupted); }
            if expected.is_none() {
                rustix::fs::renameat_with(&dir, &temporary, &dir, &name, rustix::fs::RenameFlags::NOREPLACE).map_err(|e| if e == rustix::io::Errno::EXIST { Error::Conflict } else { Error::State })?;
            } else { dir.rename(&temporary, &dir, &name)?; }
            Self::sync(&dir)?;
            let (_, revision, _) = self.snapshot_at(&dir, &name).map_err(|_| Error::State)?;
            Ok(FileResult { text: String::new(), revision: Some(revision), diff, changed: true })
        })();
        drop(staged);
        if dir.symlink_metadata(&temporary).is_ok() { dir.remove_file(&temporary)?; }
        result
    }
    fn search(&self, directory: &Path, literal: &str, output: &mut String, visited: &mut usize, depth: usize, stop: &CancellationToken) -> Result<()> {
        if literal.is_empty() || literal.len() > 1024 || depth > 32 { return Err(Error::Invalid); }
        let directory = self.path(directory)?;
        let dir = self.directory(&directory)?;
        if dir.dir_metadata()?.dev() != self.device { return Err(Error::Denied); }
        let mut names = dir.entries()?.take(1025).map(|entry|entry.map(|e|e.file_name())).collect::<std::io::Result<Vec<_>>>()?;
        if names.len() + *visited > 1024 { return Err(Error::Unsupported); }
        names.sort();
        for name in names {
            if stop.is_cancelled() { return Err(Error::Interrupted); }
            *visited += 1; if *visited > 1024 { return Err(Error::Unsupported); }
            let path = directory.join(name);
            match self.path(&path) { Err(Error::Denied) => continue, Err(e) => return Err(e), Ok(_) => () }
            let metadata = self.project.dir.symlink_metadata(&path)?;
            if metadata.is_dir() { self.search(&path, literal, output, visited, depth + 1, stop)?; }
            else {
                let (text, _, _) = self.snapshot(&path)?;
                for (line, value) in text.lines().enumerate().filter(|(_, value)|value.contains(literal)) {
                    let record = format!("{}:{}:{}\n", path.display(), line + 1, value);
                    if output.len() + record.len() > LIMIT { return Err(Error::Unsupported); } output.push_str(&record);
                }
            }
        }
        Ok(())
    }
    /// Called only after broker consumption and committed intent by the supervisor.
    pub(crate) fn perform(&self, action: &FileAction, stop: &CancellationToken) -> Result<FileResult> {
        let _lane = self.lane.lock().map_err(|_| Error::State)?;
        if stop.is_cancelled() { return Err(Error::Interrupted); }
        self.resources(action)?;
        match action {
            FileAction::Read { path } => { let (text, revision, _) = self.snapshot(path)?; Ok(FileResult { text, revision: Some(revision), diff: String::new(), changed: false }) },
            FileAction::Search { path, literal } => { let mut text = String::new(); self.search(path, literal, &mut text, &mut 0, 0, stop)?; Ok(FileResult { text, revision: None, diff: String::new(), changed: false }) },
            FileAction::Write { path, expected, text } => self.write(path, expected, text, stop),
            FileAction::Patch { path, expected, old, new } => {
                if old.is_empty() || old.len() > LIMIT || new.len() > LIMIT { return Err(Error::Invalid); }
                let (before, actual, _) = self.snapshot(path)?;
                if &actual != expected || before.match_indices(old).count() != 1 { return Err(Error::Conflict); }
                self.write(path, &Some(expected.clone()), &before.replacen(old, new, 1), stop)
            },
            FileAction::Delete { path, expected } => {
                let (dir, name) = self.parent(path)?;
                let original = self.expected(&dir, &name, &Some(expected.clone()))?.ok_or(Error::Conflict)?;
                let diff = Self::diff(path, &original.0, "")?;
                if stop.is_cancelled() { return Err(Error::Interrupted); }
                dir.remove_file(&name)?; Self::sync(&dir)?;
                Ok(FileResult { text: String::new(), revision: None, diff, changed: true })
            },
            FileAction::Rename { from, to, expected } => {
                let (source, name) = self.parent(from)?; let (destination, target) = self.parent(to)?;
                self.expected(&source, &name, &Some(expected.clone()))?;
                let diff = format!("rename from {}\nrename to {}\n", from.display(), to.display());
                if stop.is_cancelled() { return Err(Error::Interrupted); }
                rustix::fs::renameat_with(&source, &name, &destination, &target, rustix::fs::RenameFlags::NOREPLACE).map_err(|e| if e == rustix::io::Errno::EXIST { Error::Conflict } else { Error::State })?;
                Self::sync(&source)?; Self::sync(&destination)?;
                let (_, revision, _) = self.snapshot_at(&destination, &target).map_err(|_| Error::State)?;
                Ok(FileResult { text: String::new(), revision: Some(revision), diff, changed: true })
            },
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn path_only_capability_can_be_synced_by_reopening_the_same_directory() {
        let root=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();
        let project=Project::open(root.path()).unwrap();
        let fd=openat2(&project.dir,".",OFlags::PATH|OFlags::DIRECTORY|OFlags::CLOEXEC,rustix::fs::Mode::empty(),ResolveFlags::BENEATH|ResolveFlags::NO_SYMLINKS|ResolveFlags::NO_XDEV).unwrap();
        FileWorkspace::sync(&Dir::from_std_file(std::fs::File::from(fd))).unwrap();
    }
    #[test]
    fn racing_link_target_never_follows_outside_workspace() {
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
        use std::os::unix::fs::symlink;
        let root=tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();
        std::fs::create_dir(root.path().join("project")).unwrap();
        std::fs::write(root.path().join("sentinel"),"preserve").unwrap();
        let project=Project::open(&root.path().join("project")).unwrap();
        let files=FileWorkspace::new(project,Path::new("private-state")).unwrap();
        let running=Arc::new(AtomicBool::new(true));let control=running.clone();let path=root.path().join("project/target");
        let writer=std::thread::spawn(move||{
            while control.load(Ordering::Relaxed){
                let _=std::fs::remove_file(&path);let _=symlink("../sentinel",&path);
                let _=std::fs::remove_file(&path);let _=std::fs::write(&path,"local");
            }
        });
        let stop=CancellationToken::new();
        for _ in 0..500{
            if let Ok(read)=files.perform(&FileAction::Read{path:"target".into()},&stop){
                assert_ne!(read.text,"preserve");
                let _=files.perform(&FileAction::Write{path:"target".into(),expected:read.revision,text:"authorized".into()},&stop);
            }
        }
        running.store(false,Ordering::Relaxed);writer.join().unwrap();
        assert_eq!(std::fs::read_to_string(root.path().join("sentinel")).unwrap(),"preserve");
    }
    #[test]
    fn bind_mount_is_not_a_regular_subdirectory() {
        // Nested namespaces inherit only the already-contained developer view.
        // The bind source/target are both repository-local synthetic fixtures.
        if let Some(root) = std::env::var_os("RIVET_FILE_MOUNT_PROBE") {
            let project = Project::open(Path::new(&root)).unwrap();
            let files = FileWorkspace::new(project,Path::new("private-state")).unwrap();
            let stop = CancellationToken::new();
            assert_eq!(files.perform(&FileAction::Read {path:"source/file".into()},&stop).unwrap().text,"source");
            assert!(matches!(files.perform(&FileAction::Read {path:"mounted/file".into()},&stop),Err(Error::Denied)));
            assert!(matches!(files.perform(&FileAction::Write {path:"mounted/new".into(),expected:None,text:"must not write".into()},&stop),Err(Error::Denied)));
            assert!(!Path::new(&root).join("source/new").exists());
            return;
        }
        let root = tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap();
        std::fs::create_dir(root.path().join("source")).unwrap();
        std::fs::create_dir(root.path().join("mounted")).unwrap();
        std::fs::write(root.path().join("source/file"),"source").unwrap();
        let result = std::process::Command::new("/usr/bin/bwrap")
            .args(["--unshare-user","--unshare-pid","--die-with-parent","--ro-bind","/","/","--proc","/proc","--bind"])
            .arg(root.path()).arg(root.path()).arg("--bind").arg(root.path().join("source")).arg(root.path().join("mounted"))
            .args(["--setenv","RIVET_FILE_MOUNT_PROBE"]).arg(root.path())
            .arg("--").arg(std::env::current_exe().unwrap())
            .args(["--exact","tools::files::boundary_tests::bind_mount_is_not_a_regular_subdirectory","--test-threads=1"])
            .output().unwrap();
        assert!(result.status.success(),"nested mount fixture failed: {} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));
    }
}
