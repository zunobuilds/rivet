//! Explicit native qualification, not part of the contained offline environment.
use rivet::{config::Project,domain::*,ledger::{Ledger,Transition},permissions::Broker,supervisor::Supervisor,tools::{files::{FileWorkspace,FileAction},process::ProcessRequest,ToolProposal,git::{GitWorkspace,GitFile,GitRequest,GitAction,GitResult,DESCRIPTOR}}};
use std::{collections::BTreeMap,path::Path,sync::Arc,time::{SystemTime,UNIX_EPOCH}};
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
fn git(root:&Path,args:&[&str])->String{
    let output=std::process::Command::new("/usr/bin/git").current_dir(root).env_clear().env("PATH","/usr/bin").env("GIT_CONFIG_GLOBAL","/dev/null").env("GIT_CONFIG_NOSYSTEM","1")
        .args(["-c","core.hooksPath=/dev/null","-c","core.fsmonitor=false","-c","commit.gpgsign=false","-c","gc.auto=0","-c","user.name=Fixture","-c","user.email=fixture@example.invalid"]).args(args).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));String::from_utf8(output.stdout).unwrap().trim().into()
}
async fn execute(supervisor:&Supervisor,spec:&RunSpec,ledger:&Ledger,broker:&mut Broker,workspace:Arc<GitWorkspace>,task:TaskId,actor:ActorId,request:GitRequest)->rivet::error::Result<GitResult>{
    let proposal=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(&request).unwrap(),resources:request.resources(spec.project),preconditions:vec![],budget_reservation:"none".into()};
    let grant=broker.approve_git(&proposal,&proposal.resources,now()+60,now())?;
    supervisor.execute_git(spec,ledger,broker,workspace,proposal,grant).await
}
#[tokio::test]
#[ignore="requires explicitly authorized native user-systemd qualification"]
async fn isolated_git_disables_helpers_preserves_index_and_cas_commits(){
    use std::os::unix::fs::PermissionsExt;
    let root=Path::new(env!("RIVET_SOURCE_ROOT"));let temporary=tempfile::tempdir_in(root.join(".build/tmp")).unwrap();let path=temporary.path();
    std::fs::create_dir(path.join("empty-template")).unwrap();
    git(path,&["init","-b","main","--template=empty-template"]);
    std::fs::write(path.join("a.txt"),"old\n").unwrap();std::fs::write(path.join("user.txt"),"base\n").unwrap();
    git(path,&["add","--","a.txt","user.txt"]);git(path,&["commit","-m","seed"]);
    let head=git(path,&["rev-parse","HEAD"]);
    std::fs::write(path.join("user.txt"),"unrelated staged data\n").unwrap();git(path,&["add","--","user.txt"]);
    std::fs::create_dir_all(path.join(".git/hooks")).unwrap();std::fs::create_dir_all(path.join(".git/info")).unwrap();
    for hook in ["evil","reference-transaction","pre-commit","commit-msg","post-commit"]{
        std::fs::write(path.join(".git/hooks").join(hook),"#!/bin/sh\necho invoked > /git-metadata/objects/unapproved-helper\necho unapproved-helper >&2\nexit 99\n").unwrap();
        std::fs::set_permissions(path.join(".git/hooks").join(hook),std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    std::fs::write(path.join(".git/info/attributes"),"*.txt filter=rivet diff=rivet\n").unwrap();
    for setting in ["core.fsmonitor","filter.rivet.clean","filter.rivet.smudge","filter.rivet.process","diff.rivet.command","diff.rivet.textconv","gpg.program"]{git(path,&["config",setting,"/git-metadata/hooks/evil"]);}
    git(path,&["config","filter.rivet.required","true"]);git(path,&["config","commit.gpgsign","true"]);
    let index=std::fs::read(path.join(".git/index")).unwrap();
    let project=Project::open(path).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();
    let files_workspace=Arc::new(FileWorkspace::new(Project::open(path).unwrap(),Path::new("state")).unwrap());
    let workspace=Arc::new(GitWorkspace::new(project).unwrap());
    let spec=RunSpec{schema_version:1,project:ledger.project_id(),session:SessionId::default(),run:RunId::default(),task:"Git fixture".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+300,step_budget:100,permission_epoch:ledger.epoch()};
    let task=TaskId::default();let actor=ActorId::default();
    ledger.transition(0,Transition::NewSession{id:spec.session}).await.unwrap();ledger.transition(0,Transition::RootTask{session:spec.session,run:spec.run,task,actor}).await.unwrap();
    let supervisor=Supervisor::default();let mut broker=Broker::new(spec.permission_epoch);
    // P1b.4: approved patch, immutable checked snapshot, Git diff and audit receipt.
    async fn file(supervisor:&Supervisor,spec:&RunSpec,ledger:&Ledger,broker:&mut Broker,workspace:Arc<FileWorkspace>,task:TaskId,actor:ActorId,action:FileAction)->rivet::error::Result<rivet::tools::files::FileResult>{
        let p=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:rivet::tools::files::DESCRIPTOR.into(),arguments:serde_json::to_value(&action).unwrap(),resources:workspace.resources(&action)?,preconditions:vec![],budget_reservation:"none".into()};
        let grant=broker.approve_file(&p,&p.resources,now()+60,now())?;
        supervisor.execute_file(spec,ledger,broker,workspace,p,grant).await
    }
    let read=file(&supervisor,&spec,&ledger,&mut broker,files_workspace.clone(),task,actor,FileAction::Read{path:"a.txt".into()}).await.unwrap();
    let patched=file(&supervisor,&spec,&ledger,&mut broker,files_workspace.clone(),task,actor,FileAction::Patch{path:"a.txt".into(),expected:read.revision.unwrap(),old:"old".into(),new:"new".into()}).await.unwrap();
    assert!(patched.changed && patched.diff.contains("+new"));
    let checked=file(&supervisor,&spec,&ledger,&mut broker,files_workspace.clone(),task,actor,FileAction::Read{path:"a.txt".into()}).await.unwrap();
    let request=ProcessRequest{program:"/usr/bin/python3".into(),args:vec!["-B".into(),"-c".into(),"from pathlib import Path; assert Path('a.txt').read_text()=='new\\n'; print('fixture check passed')".into()],files:BTreeMap::from([("a.txt".into(),checked.text.clone())]),timeout_ms:10000};
    let p=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:rivet::tools::process::DESCRIPTOR.into(),arguments:serde_json::to_value(&request).unwrap(),resources:request.resources(spec.project),preconditions:vec![],budget_reservation:"none".into()};
    let grant=broker.approve_process(&p,&p.resources,now()+60,now()).unwrap();
    let check=supervisor.execute_process(&spec,&ledger,&mut broker,p,grant).await.unwrap();
    assert_eq!(check.exit_code,Some(0),"{}",check.stderr);assert_eq!(check.stdout.trim(),"fixture check passed");
    // A reviewed grant altered after review produces no side effect.
    let action=FileAction::Write{path:"denied.txt".into(),expected:None,text:"denied".into()};
    let mut p=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:rivet::tools::files::DESCRIPTOR.into(),arguments:serde_json::to_value(&action).unwrap(),resources:files_workspace.resources(&action).unwrap(),preconditions:vec![],budget_reservation:"none".into()};
    let grant=broker.approve_file(&p,&p.resources,now()+60,now()).unwrap();p.attempt+=1;
    assert!(supervisor.execute_file(&spec,&ledger,&mut broker,files_workspace,p,grant).await.is_err());
    assert!(!path.join("denied.txt").exists());
    let files=BTreeMap::from([("a.txt".into(),GitFile{text:checked.text,executable:false,deleted:false})]);
    let status=execute(&supervisor,&spec,&ledger,&mut broker,workspace.clone(),task,actor,GitRequest{action:GitAction::Status,files:files.clone()}).await.unwrap();
    assert_eq!(status.entries,vec![" M a.txt"]);
    let diff=execute(&supervisor,&spec,&ledger,&mut broker,workspace.clone(),task,actor,GitRequest{action:GitAction::Diff,files:files.clone()}).await.unwrap();assert!(diff.diff.contains("-old"));assert!(diff.diff.contains("+new"));
    let commit=GitRequest{action:GitAction::Commit{expected_head:Some(head.clone()),expected_ref:"refs/heads/main".into(),message:"approved selected file".into(),author_name:"Fixture".into(),author_email:"fixture@example.invalid".into()},files};
    let result=execute(&supervisor,&spec,&ledger,&mut broker,workspace.clone(),task,actor,commit.clone()).await.unwrap();
    let new=result.commit.unwrap();assert_ne!(new,head);
    assert_eq!(git(path,&["rev-list","--parents","-1","HEAD"]),format!("{new} {head}"));
    assert_eq!(git(path,&["cat-file","blob","HEAD:a.txt"]),"new");
    assert_eq!(git(path,&["cat-file","blob","HEAD:user.txt"]),"base");
    assert_eq!(std::fs::read(path.join(".git/index")).unwrap(),index);
    assert_eq!(std::fs::read_to_string(path.join("user.txt")).unwrap(),"unrelated staged data\n");
    assert_eq!(std::fs::read_to_string(path.join("a.txt")).unwrap(),"new\n");
    assert!(!path.join(".git/objects/unapproved-helper").exists());
    assert!(execute(&supervisor,&spec,&ledger,&mut broker,workspace,task,actor,commit).await.is_err());
    assert_eq!(git(path,&["rev-parse","HEAD"]),new);
    supervisor.shutdown().await;ledger.shutdown().unwrap();
    let db=rusqlite::Connection::open(path.join("state/ledger.sqlite")).unwrap();
    let receipts:i64=db.query_row("SELECT count(*) FROM events WHERE kind='receipt-recorded'",[],|r|r.get(0)).unwrap();assert_eq!(receipts,7);
    let unknown:i64=db.query_row("SELECT count(*) FROM events WHERE kind='outcome-unknown'",[],|r|r.get(0)).unwrap();assert_eq!(unknown,1);
    let denied:i64=db.query_row("SELECT count(*) FROM events WHERE kind='denied'",[],|r|r.get(0)).unwrap();assert_eq!(denied,1);
}
