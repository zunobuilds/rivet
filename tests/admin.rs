use std::{fs, path::{Path, PathBuf}, process::{Command, Output}, collections::BTreeMap};
use tempfile::TempDir;
use rivet::{config::{Config, Project}, ledger::{self, Ledger, Transition}, domain::SessionId};

fn fixture() -> TempDir { tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".build/tmp")).unwrap() }
fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rivet")).current_dir(root).args(args).env_clear().env("PATH", "/usr/bin").output().unwrap()
}
fn snapshot(path: &Path) -> BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)> {
    let mut output = BTreeMap::new();
    fn walk(base: &Path, path: &Path, output: &mut BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { output.insert(path.strip_prefix(base).unwrap().to_owned(), (Vec::new(), fs::metadata(&path).unwrap().modified().unwrap())); walk(base, &path, output); }
            else { output.insert(path.strip_prefix(base).unwrap().to_owned(), (fs::read(&path).unwrap(), fs::metadata(&path).unwrap().modified().unwrap())); }
        }
    }
    walk(path, path, &mut output); output
}
#[test]
fn administrative_reads_are_model_free_and_do_not_create_or_mutate() {
    let root = fixture(); let before = snapshot(root.path());
    for args in [vec!["--help"], vec!["--version"], vec!["config", "check", "--json"], vec!["status", "--json"], vec!["session", "list", "--json"]] {
        let output = cli(root.path(), &args);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(snapshot(root.path()) == before, "filesystem snapshot changed");
    }
    let status = cli(root.path(), &["status", "--json"]);
    let value: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(value["provider"], "absent"); assert_eq!(value["state"], "missing");
    assert_eq!(value["execution_available"], false);
    assert!(cli(root.path(), &["state", "init"]).status.success());
    assert!(cli(root.path(), &["session", "new"]).status.success());
    let before = snapshot(root.path());
    assert!(!before.keys().any(|p| p.to_string_lossy().ends_with("-wal")));
    for args in [vec!["status", "--json"], vec!["session", "list", "--json"]] {
        let output = cli(root.path(), &args);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(snapshot(root.path()) == before, "filesystem snapshot changed");
    }
    let list = cli(root.path(), &["session", "list", "--json"]);
    let value: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(value["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(value["sqlite_version"], "3.54.1");
}
#[test]
fn config_validation_redacts_and_never_discovers_parents() {
    let root = fixture(); fs::create_dir(root.path().join("child")).unwrap();
    fs::write(root.path().join(".rivet.toml"), "secret = 'SENTINEL-SECRET'").unwrap();
    assert!(cli(&root.path().join("child"), &["config", "check"]).status.success());
    for contents in ["schema_version = 999", "schema_version = 1\nsecret = 'SENTINEL-SECRET'", "SENTINEL-SECRET [ malformed", "schema_version=1\n[provider]\nprofile='x'\nendpoint='x'\nmodel='x'\nsecret='SENTINEL-SECRET'"] {
        fs::write(root.path().join(".rivet.toml"), contents).unwrap();
        let before = snapshot(root.path()); let output = cli(root.path(), &["config", "check", "--json"]);
        assert_eq!(output.status.code(), Some(2));
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["error"]["kind"], "invalid-input");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("SENTINEL"));
        assert!(snapshot(root.path()) == before, "filesystem snapshot changed");
    }
    fs::write(root.path().join(".rivet.toml"), "schema_version=1\n[provider]\nprofile='compatible'\nendpoint='http://example.invalid'\nmodel='example'").unwrap();
    let output = cli(root.path(), &["status", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["provider"], "configured-unverified");
    assert!(Config::parse("schema_version=1").unwrap().provider.is_none());
}
#[tokio::test]
async fn locks_remnants_newer_schema_and_unsafe_paths_fail_closed() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = fixture(); let project = Project::open(root.path()).unwrap(); let state = Path::new("state");
    let ledger = Ledger::open(&project, state).unwrap();
    assert!(matches!(Ledger::open(&project, state), Err(rivet::error::Error::Busy)));
    assert!(matches!(ledger::inspect(&project, state), Err(rivet::error::Error::Busy)));
    ledger.transition(0, Transition::NewSession { id: SessionId::default() }).await.unwrap();
    ledger.shutdown().unwrap();
    let metadata = fs::metadata(root.path().join("state")).unwrap(); assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
    for entry in fs::read_dir(root.path().join("state")).unwrap() { assert_eq!(entry.unwrap().metadata().unwrap().permissions().mode() & 0o077, 0); }
    fs::write(root.path().join("state/ledger.sqlite-wal"), b"recovery").unwrap();
    fs::set_permissions(root.path().join("state/ledger.sqlite-wal"), fs::Permissions::from_mode(0o600)).unwrap();
    let before = snapshot(root.path()); assert!(matches!(ledger::inspect(&project, state), Err(rivet::error::Error::Recovery)));
    assert!(snapshot(root.path()) == before, "filesystem snapshot changed"); fs::remove_file(root.path().join("state/ledger.sqlite-wal")).unwrap();
    let db = rusqlite::Connection::open(root.path().join("state/ledger.sqlite")).unwrap();
    db.execute("UPDATE meta SET schema_version=2", []).unwrap(); drop(db);
    let before = snapshot(root.path()); assert!(matches!(Ledger::open(&project, state), Err(rivet::error::Error::State)));
    assert!(matches!(ledger::inspect(&project, state), Err(rivet::error::Error::State))); assert!(snapshot(root.path()) == before, "filesystem snapshot changed");
    symlink("state", root.path().join("linked")).unwrap();
    assert!(Ledger::open(&project, Path::new("linked")).is_err());
    assert!(Ledger::open(&project, Path::new("../escape")).is_err());
    assert!(project.config(Some(Path::new("/outside"))).is_err());
}
#[tokio::test]
async fn injected_audit_failure_rolls_back_state_and_sequence() {
    let root = fixture(); let project = Project::open(root.path()).unwrap(); let state = Path::new("state");
    Ledger::open(&project, state).unwrap().shutdown().unwrap();
    let path = root.path().join("state/ledger.sqlite");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER fail_session BEFORE INSERT ON events WHEN NEW.kind='session-created' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap(); drop(db);
    let ledger = Ledger::open(&project, state).unwrap();
    assert!(ledger.transition(0, Transition::NewSession { id: SessionId::default() }).await.is_err());
    ledger.shutdown().unwrap();
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(db.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    let next: i64 = db.query_row("SELECT next_event_sequence FROM meta", [], |r| r.get(0)).unwrap();
    let last: i64 = db.query_row("SELECT max(sequence) FROM events", [], |r| r.get(0)).unwrap();
    assert_eq!(next, last + 1);
}

#[test]
fn unsafe_config_and_state_file_links_are_rejected_without_following() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    symlink("/unapproved-rivet-probe", root.path().join(".rivet.toml")).unwrap();
    let output = cli(root.path(), &["config", "check"]);
    assert_eq!(output.status.code(), Some(4));
    fs::remove_file(root.path().join(".rivet.toml")).unwrap();
    assert!(cli(root.path(), &["state", "init"]).status.success());
    let state = root.path().join(".rivet-state");
    fs::rename(state.join("ledger.sqlite"),state.join("original.sqlite")).unwrap();
    symlink("original.sqlite",state.join("ledger.sqlite")).unwrap();
    assert_eq!(cli(root.path(), &["status"]).status.code(),Some(4));
    fs::remove_file(state.join("ledger.sqlite")).unwrap();
    fs::hard_link(state.join("original.sqlite"),state.join("ledger.sqlite")).unwrap();
    assert_eq!(cli(root.path(), &["status"]).status.code(),Some(4));
    assert_eq!(cli(root.path(), &["state","init"]).status.code(),Some(4));
}

#[test]
fn orphan_journals_and_corrupt_meta_never_reinitialize_state() {
    use std::os::unix::fs::PermissionsExt;
    let root = fixture();
    assert!(cli(root.path(), &["state", "init"]).status.success());
    let state = root.path().join(".rivet-state");
    fs::rename(state.join("ledger.sqlite"), state.join("saved.sqlite")).unwrap();
    fs::write(state.join("ledger.sqlite-wal"), b"orphan-recovery-evidence").unwrap();
    fs::set_permissions(state.join("ledger.sqlite-wal"), fs::Permissions::from_mode(0o600)).unwrap();
    let before = snapshot(root.path());
    let output = cli(root.path(), &["state", "init"]);
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("recovery"));
    assert!(snapshot(root.path()) == before);
    fs::remove_file(state.join("ledger.sqlite-wal")).unwrap();
    fs::rename(state.join("saved.sqlite"), state.join("ledger.sqlite")).unwrap();
    let db = rusqlite::Connection::open(state.join("ledger.sqlite")).unwrap();
    db.execute("UPDATE meta SET next_event_sequence=9000", []).unwrap();
    drop(db);
    let before = snapshot(root.path());
    assert_eq!(cli(root.path(), &["status", "--json"]).status.code(), Some(4));
    assert_eq!(cli(root.path(), &["state", "init"]).status.code(), Some(4));
    assert!(snapshot(root.path()) == before);
}

#[test]
fn worker_preflight_denies_absent_and_fake_delegation_without_effects() {
    let root = fixture();
    let before = snapshot(root.path());
    let output = cli(root.path(), &["sandbox", "check", "--json"]);
    assert_eq!(output.status.code(),Some(3));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ready"],false); assert_eq!(value["controls"],"not-configured");
    assert!(snapshot(root.path()) == before);
    fs::create_dir(root.path().join("fake-controls")).unwrap();
    fs::write(root.path().join("fake-controls/cgroup.controllers"),"cpu memory pids").unwrap();
    let before = snapshot(root.path());
    let output = cli(root.path(), &["sandbox","check","--control-dir","fake-controls","--json"]);
    assert_eq!(output.status.code(),Some(3));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["controls"],"not-cgroup-v2");
    assert!(snapshot(root.path()) == before);
    assert_eq!(cli(root.path(), &["sandbox","check","--control-dir","../outside"]).status.code(),Some(2));
}
