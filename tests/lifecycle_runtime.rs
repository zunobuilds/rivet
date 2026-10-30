//! Explicit detached same-supervisor qualification against a synthetic loopback endpoint.
use std::{path::{Path,PathBuf},time::Duration};
use serde_json::{json,Value};
use tokio::{net::TcpListener,io::{AsyncReadExt,AsyncWriteExt}};
fn binary()->PathBuf{Path::new(env!("RIVET_SOURCE_ROOT")).join(".build/target/debug/rivet")}
async fn cli(root:&Path,args:&[&str])->Value{
    let mut command=tokio::process::Command::new(binary());command.current_dir(root).env_clear().env("PATH","/usr/bin");for key in ["XDG_RUNTIME_DIR","DBUS_SESSION_BUS_ADDRESS"]{if let Some(v)=std::env::var_os(key){command.env(key,v);}}
    let result=tokio::time::timeout(Duration::from_secs(15),command.args(args).output()).await.unwrap().unwrap();assert!(result.status.success(),"args={args:?}; {} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));serde_json::from_slice(&result.stdout).unwrap()
}
async fn review(root:&Path,run:&str)->String{
    tokio::time::timeout(Duration::from_secs(5),async{loop{let status=cli(root,&["run","status",run,"--state-dir","state"]).await;if let Some(challenge)=status["review"]["challenge"].as_str(){return challenge.to_string();}tokio::time::sleep(Duration::from_millis(20)).await;}}).await.unwrap()
}
async fn state(root:&Path,run:&str,expected:&str)->Value{
    tokio::time::timeout(Duration::from_secs(5),async{loop{let status=cli(root,&["run","status",run,"--state-dir","state"]).await;if status["snapshot"]["state"]==expected{return status;}tokio::time::sleep(Duration::from_millis(20)).await;}}).await.unwrap()
}
#[tokio::test]
#[ignore="requires explicitly authorized native user-systemd qualification"]
async fn explicit_detach_reconnect_pause_and_resume_keep_same_bounded_run_and_require_fresh_review(){
    let root=tempfile::tempdir_in(Path::new(env!("RIVET_SOURCE_ROOT")).join(".build/tmp")).unwrap();let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();std::fs::write(root.path().join(".rivet.toml"),format!("schema_version = 1\n[provider]\nprofile = \"llama.cpp-compatible\"\nendpoint = \"http://{address}/v1/chat/completions\"\nmodel = \"synthetic\"\ncontext_tokens = 65536\n")).unwrap();
    let detached=cli(root.path(),&["chat","--task","synthetic bounded detached task","--state-dir","state","--deadline-seconds","30","--steps","10","--detach"]).await;assert_eq!(detached["detached"],true);let run=detached["run"].as_str().unwrap().to_string();let challenge=review(root.path(),&run).await;assert_eq!(challenge.len(),64);
    assert!(tokio::time::timeout(Duration::from_millis(50),listener.accept()).await.is_err());let disconnected=cli(root.path(),&["run","disconnect",&run,"--state-dir","state"]).await;assert_eq!(disconnected["detached"],true);assert_eq!(review(root.path(),&run).await,challenge);
    let response=cli(root.path(),&["run","pause",&run,"--state-dir","state"]).await;assert_eq!(response["accepted"],true);assert_eq!(response["durable"],false);let paused=state(root.path(),&run,"paused").await;assert_eq!(paused["snapshot"]["steps"],1);let deadline=paused["snapshot"]["spec"]["deadline"].clone();let digest=paused["resume_digest"].as_str().unwrap();
    let resumed=cli(root.path(),&["run","resume",&run,"--state-dir","state","--approve",digest,"--detach"]).await;assert_eq!(resumed["run"],run);let new_challenge=review(root.path(),&run).await;assert_ne!(challenge,new_challenge);
    let server=tokio::spawn(async move{
        let(mut stream,_)=listener.accept().await.unwrap();let mut bytes=vec![];let mut buf=[0;4096];loop{let n=stream.read(&mut buf).await.unwrap();assert!(n>0);bytes.extend_from_slice(&buf[..n]);assert!(bytes.len()<1024*1024);if let Some(end)=bytes.windows(4).position(|v|v==b"\r\n\r\n"){let head=String::from_utf8_lossy(&bytes[..end]);let length=head.lines().find_map(|s|s.to_ascii_lowercase().strip_prefix("content-length: ").and_then(|v|v.parse::<usize>().ok())).unwrap();if bytes.len()>=end+4+length{break;}}}
        let body=format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",json!({"choices":[{"index":0,"delta":{"content":"Synthetic detached task complete"},"finish_reason":"stop"}]}),json!({"choices":[],"usage":{"prompt_tokens":5,"completion_tokens":5}}));let headers=format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());stream.write_all(headers.as_bytes()).await.unwrap();stream.write_all(body.as_bytes()).await.unwrap();
    });
    assert_eq!(cli(root.path(),&["run","approve",&run,&new_challenge,"--state-dir","state"]).await["approved"],true);server.await.unwrap();let completed=state(root.path(),&run,"completed").await;assert_eq!(completed["snapshot"]["steps"],2);assert_eq!(completed["snapshot"]["spec"]["deadline"],deadline);assert_eq!(completed["snapshot"]["version"],2);
    for unit in [detached["unit"].as_str().unwrap(),resumed["unit"].as_str().unwrap()]{
        let mut command=tokio::process::Command::new("/usr/bin/systemctl");command.env_clear().env("PATH","/usr/bin");for key in ["XDG_RUNTIME_DIR","DBUS_SESSION_BUS_ADDRESS"]{if let Some(v)=std::env::var_os(key){command.env(key,v);}}
        let result=command.args(["--user","show",unit,"--property=LoadState","--value"]).output().await.unwrap();assert_eq!(String::from_utf8_lossy(&result.stdout).trim(),"not-found");
    }
}
