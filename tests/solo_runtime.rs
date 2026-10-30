//! Synthetic HTTP + actual sandbox check. Requires the authorized native user-unit route.
use rivet::{budget::BudgetLimits,chat::{self,ChatOptions,Reviewer,Review},config::{Project,ProviderConfig},domain::*,error::Result,ledger,tools::{files::Revision,process::ProcessRequest}};
use std::{future::Future,pin::Pin,path::Path,collections::BTreeMap};
use tokio::{io::{AsyncReadExt,AsyncWriteExt},net::TcpListener};
use tokio_util::sync::CancellationToken;
use serde_json::{json,Value};
struct Approved {deny_tools:bool, reviews:Vec<String>}
impl Reviewer for Approved {
    fn approve(&mut self,review:Review)->Pin<Box<dyn Future<Output=Result<bool>>+Send+'_>>{self.reviews.push(review.kind.into());let allowed=!(self.deny_tools&&review.kind=="tool");Box::pin(async move{Ok(allowed)})}
}
async fn read_request(stream:&mut tokio::net::TcpStream)->Value {
    let mut bytes=vec![];let mut buffer=[0;4096];
    loop {let n=stream.read(&mut buffer).await.unwrap();assert!(n>0);bytes.extend_from_slice(&buffer[..n]);assert!(bytes.len()<1024*1024);
        if let Some(end)=bytes.windows(4).position(|s|s==b"\r\n\r\n"){
            let headers=String::from_utf8_lossy(&bytes[..end]);let len=headers.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length: ").and_then(|s|s.parse::<usize>().ok())).unwrap();
            if bytes.len()>=end+4+len{return serde_json::from_slice(&bytes[end+4..]).unwrap();}
        }
    }
}
async fn reply(stream:&mut tokio::net::TcpStream,call:Option<(&str,Value)>){
    let (delta,reason)=match call{Some((name,args))=>(json!({"tool_calls":[{"index":0,"id":uuid::Uuid::new_v4().to_string(),"type":"function","function":{"name":name,"arguments":args.to_string()}}]}),"tool_calls"),None=>(json!({"content":"Fixture task finished; consult exact receipt references."}),"stop")};
    let body=format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",json!({"choices":[{"index":0,"delta":delta,"finish_reason":reason}]}),json!({"choices":[],"usage":{"prompt_tokens":100,"completion_tokens":80}}));
    let headers=format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());stream.write_all(headers.as_bytes()).await.unwrap();stream.write_all(body.as_bytes()).await.unwrap();
}
fn options(session:Option<SessionId>)->ChatOptions{ChatOptions{session,task:"Fix add and check the exact projected fixture source".into(),steps:20,deadline_seconds:120,budget:BudgetLimits{tokens:1_000_000,requests:20,cost_micros:0},request_cost_micros:0}}
#[tokio::test]
#[ignore="requires explicitly authorized native user-systemd qualification"]
async fn synthetic_solo_reads_patches_checks_and_continues_session(){
    let root=Path::new(env!("RIVET_SOURCE_ROOT"));let temp=tempfile::tempdir_in(root.join(".build/tmp")).unwrap();
    let old="def add(a, b):\n    return a - b\n";let new="def add(a, b):\n    return a + b\n";std::fs::write(temp.path().join("code.py"),old).unwrap();
    let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();
    let server=tokio::spawn(async move{
        for step in 0..4{
            let (mut stream,_)=listener.accept().await.unwrap();let request=read_request(&mut stream).await;
            match step{
                0=>reply(&mut stream,Some(("rivet_files_v1",json!({"operation":"read","path":"code.py"})))).await,
                1=>{let last=request["messages"].as_array().unwrap().last().unwrap();assert_eq!(last["role"],"tool");let read:Value=serde_json::from_str(last["content"].as_str().unwrap()).unwrap();let revision:Revision=serde_json::from_value(read["revision"].clone()).unwrap();reply(&mut stream,Some(("rivet_files_v1",json!({"operation":"patch","path":"code.py","expected":revision,"old":"a - b","new":"a + b"})))).await;},
                2=>{let process=ProcessRequest{program:"/usr/bin/python3".into(),args:vec!["-B".into(),"-c".into(),"from code import add; assert add(1,2)==3; print('add fixture passed')".into()],files:BTreeMap::from([("code.py".into(),new.into())]),timeout_ms:10000};reply(&mut stream,Some(("rivet_process_check_v1",serde_json::to_value(process).unwrap()))).await;},
                _=>{let content=request["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap();let result:Value=serde_json::from_str(content).unwrap();assert_eq!(result["exit_code"],0);assert_eq!(result["stdout"],"add fixture passed\n");assert_eq!(result["projection_digest"].as_str().unwrap().len(),64);reply(&mut stream,None).await;},
            }
        }
    });
    let config=ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:format!("http://{address}/v1/chat/completions"),model:"synthetic".into()};let mut reviewer=Approved{deny_tools:false,reviews:vec![]};let project=Project::open(temp.path()).unwrap();
    let report=tokio::time::timeout(std::time::Duration::from_secs(20),chat::run(&project,Path::new("state"),config,options(None),&mut reviewer,CancellationToken::new())).await.unwrap().unwrap();server.await.unwrap();
    assert_eq!(report.steps,7);assert_eq!(report.checks.len(),1);assert_eq!(report.revisions.len(),2);assert_eq!(std::fs::read_to_string(temp.path().join("code.py")).unwrap(),new);
    assert_eq!(reviewer.reviews.iter().filter(|k|*k=="inference").count(),4);assert_eq!(reviewer.reviews.iter().filter(|k|*k=="tool").count(),3);
    let evidence=ledger::continuation(&project,Path::new("state"),report.session).unwrap();assert!(evidence.contains(&report.revisions[1]));assert!(!evidence.contains("def add"));
    let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();let server=tokio::spawn(async move{let (mut stream,_)=listener.accept().await.unwrap();let request=read_request(&mut stream).await;assert!(request["messages"][1]["content"].as_str().unwrap().contains("operation"));reply(&mut stream,None).await;});
    let config=ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:format!("http://{address}/v1/chat/completions"),model:"synthetic".into()};
    let continued=chat::run(&project,Path::new("state"),config,options(Some(report.session)),&mut reviewer,CancellationToken::new()).await.unwrap();assert_eq!(continued.session,report.session);assert_ne!(continued.run,report.run);server.await.unwrap();
}
#[tokio::test]
async fn denied_solo_tool_has_no_file_effect_and_preserves_audit(){
    let root=Path::new(env!("CARGO_MANIFEST_DIR"));let temp=tempfile::tempdir_in(root.join(".build/tmp")).unwrap();let project=Project::open(temp.path()).unwrap();
    let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();let server=tokio::spawn(async move{let (mut stream,_)=listener.accept().await.unwrap();read_request(&mut stream).await;reply(&mut stream,Some(("rivet_files_v1",json!({"operation":"write","path":"denied.py","expected":null,"text":"must not write"})))).await;});
    let config=ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:format!("http://{address}/v1/chat/completions"),model:"synthetic".into()};let mut reviewer=Approved{deny_tools:true,reviews:vec![]};
    assert!(chat::run(&project,Path::new("state"),config,options(None),&mut reviewer,CancellationToken::new()).await.is_err());server.await.unwrap();assert!(!temp.path().join("denied.py").exists());
    let db=rusqlite::Connection::open(temp.path().join("state/ledger.sqlite")).unwrap();let denied:i64=db.query_row("SELECT count(*) FROM events WHERE kind='denied'",[],|r|r.get(0)).unwrap();assert_eq!(denied,1);
}
