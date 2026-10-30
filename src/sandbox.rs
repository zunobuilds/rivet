//! Read-only worker preflight. No host cgroup discovery or worker launch.
use crate::{config::Project, error::{Error, Result}};
use serde::Serialize;
use std::{io::Read, path::Path};
use cap_std::fs::OpenOptions;

#[derive(Debug, Serialize)]
pub struct WorkerPreflight {
    pub ready: bool,
    pub controls: &'static str,
    pub controllers: Vec<String>,
    pub reason: &'static str,
}
/// Probe only an explicitly supplied, repository-contained controller directory.
/// Ordinary files cannot masquerade as a delegated kernel cgroup filesystem.
pub fn preflight(project: &Project, path: Option<&Path>) -> Result<WorkerPreflight> {
    let Some(path) = path else {
        return Ok(WorkerPreflight { ready:false, controls:"not-configured", controllers:vec![], reason:"repository-contained cgroup v2 delegation required; no host discovery permitted" });
    };
    let path = project.relative(path)?;
    project.check_components(&path)?;
    let dir = project.open_directory(&path)?;
    let filesystem = rustix::fs::fstatfs(&dir).map_err(|_| Error::State)?;
    // CGROUP2_SUPER_MAGIC, from Linux uapi. A plain fixture is never enforcement.
    if filesystem.f_type != 0x6367_7270 {
        return Ok(WorkerPreflight {ready:false,controls:"not-cgroup-v2",controllers:vec![],reason:"directory is not a kernel cgroup v2 mount"});
    }
    let mut options = OpenOptions::new(); options.read(true);
    let mut text = String::new();
    dir.open_with("cgroup.controllers", &options)?.into_std().take(4097).read_to_string(&mut text)?;
    if text.len() > 4096 { return Err(Error::State); }
    let controllers: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    if !["cpu", "memory", "pids"].iter().all(|name|controllers.iter().any(|c|c == name)) {
        return Ok(WorkerPreflight {ready:false,controls:"missing-controllers",controllers,reason:"cpu, memory and pids controllers are required"});
    }
    // Presence is not writable delegation or actual process-tree/limit qualification.
    Ok(WorkerPreflight {ready:false,controls:"candidate-unqualified",controllers,reason:"delegation writes, strict Landlock, namespace limits and process cleanup must be qualified before dispatch"})
}
