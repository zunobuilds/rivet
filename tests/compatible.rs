use rivet::{config::ProviderConfig, domain::*, permissions::Broker, providers::{*, compatible::Compatible}, tools::files::FileWorkspace};
use serde_json::json;
use std::{sync::{Arc,atomic::{AtomicUsize,Ordering}},time::{SystemTime,UNIX_EPOCH,Duration}};
use tokio::{io::{AsyncReadExt,AsyncWriteExt},net::TcpListener};
use tokio_util::sync::CancellationToken;
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
fn request()->InferRequest {
    InferRequest{schema_version:1,id:RequestId::default(),prompt:String::new(),context:Some(RequestContext{
        messages:vec![ChatMessage::text(MessageRole::User,"fixture only".into())],tools:vec![FileWorkspace::descriptor()],task:TaskId::default(),actor:ActorId::default(),max_output:1024,max_cost_micros:0,reservation:"fixture-reservation".into()
    })}
}
fn event(delta:serde_json::Value,finish:serde_json::Value)->String {
    format!("data: {}\n\n",json!({"choices":[{"index":0,"delta":delta,"finish_reason":finish}]}))
}
fn valid()->String {
    event(json!({"tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"rivet_","arguments":"{\"operation\":\"read\","}}]}),json!(null))+
    &event(json!({"tool_calls":[{"index":0,"function":{"name":"files_v1","arguments":"\"path\":\"a.txt\"}"}}]}),json!("tool_calls"))+
    &format!("data: {}\n\n",json!({"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":9}}))+"data: [DONE]\n\n"
}
async fn server(body:String,status:u16)->(ProviderConfig,tokio::task::JoinHandle<()>,Arc<AtomicUsize>) {
    let socket=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=socket.local_addr().unwrap();
    let calls=Arc::new(AtomicUsize::new(0));let count=calls.clone();
    let task=tokio::spawn(async move{
        let (mut stream,_)=tokio::time::timeout(Duration::from_secs(3),socket.accept()).await.unwrap().unwrap();count.fetch_add(1,Ordering::SeqCst);
        let mut bytes=vec![];let mut buffer=[0;4096];
        loop {
            let n=stream.read(&mut buffer).await.unwrap();assert!(n>0);bytes.extend_from_slice(&buffer[..n]);assert!(bytes.len()<1024*1024);
            if let Some(end)=bytes.windows(4).position(|s|s==b"\r\n\r\n"){
                let headers=String::from_utf8_lossy(&bytes[..end]);let size=headers.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length: ").and_then(|s|s.parse::<usize>().ok())).unwrap();
                if bytes.len()>=end+4+size{
                    let request:serde_json::Value=serde_json::from_slice(&bytes[end+4..]).unwrap();assert_eq!(request["model"],"synthetic");assert_eq!(request["stream"],true);break;
                }
            }
        }
        let headers=format!("HTTP/1.1 {status} Fixture\r\nContent-Type: text/event-stream\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
        if stream.write_all(headers.as_bytes()).await.is_ok(){for bytes in body.as_bytes().chunks(7){if stream.write_all(bytes).await.is_err(){break;}tokio::task::yield_now().await;}}
    });
    (ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:format!("http://{address}/v1/chat/completions"),model:"synthetic".into()},task,calls)
}
async fn collect(body:String,status:u16)->Vec<ProviderEvent>{
    let (config,server,count)=server(body,status).await;let broker=Broker::new(1);let request=request();
    let permit=broker.approve_inference(&config,&request,&request.digest().unwrap(),now()+30,now()).unwrap();
    let provider=Compatible::new(config,permit).unwrap();let mut rx=provider.infer(request.clone(),CancellationToken::new());let mut events=vec![];
    while let Some(event)=tokio::time::timeout(Duration::from_secs(5),rx.recv()).await.unwrap(){events.push(event);}
    server.await.unwrap();assert_eq!(count.load(Ordering::SeqCst),1);
    let mut denied=provider.infer(request,CancellationToken::new());assert!(matches!(denied.recv().await,Some(ProviderEvent::Error{code,..}) if code=="inference-denied"));events
}
#[tokio::test]
async fn fragmented_calls_complete_once_with_usage_and_stop(){
    let events=collect(valid(),200).await;
    assert_eq!(events.len(),5,"{events:?}");
    assert!(matches!(events[0],ProviderEvent::ToolCallDelta{..}));assert!(matches!(events[1],ProviderEvent::ToolCallDelta{..}));
    match &events[2]{ProviderEvent::ToolCallComplete{proposal,wire_id,..}=>{assert_eq!(wire_id.as_deref(),Some("call-1"));assert_eq!(proposal.arguments,json!({"operation":"read","path":"a.txt"}));assert!(proposal.resources.is_empty());},other=>panic!("{other:?}")}
    assert!(matches!(events[3],ProviderEvent::Usage{input:12,output:9,..}));assert!(matches!(events[4],ProviderEvent::Finish{..}));
}
#[tokio::test]
async fn duplicate_malformed_eof_quota_and_redirect_never_emit_calls_or_retry(){
    let duplicate=valid().replace("\"function\":{\"arguments\":\"\\\"path", "\"id\":\"call-1\",\"function\":{\"arguments\":\"\\\"path");
    // Duplicate key in a protocol object, malformed assembled args, missing DONE, and oversized frame.
    let malformed=valid().replace("a.txt\\\"}","a.txt\\\",\\\"path\\\":\\\"duplicate\\\"}");
    for (body,status) in [(duplicate,200),(malformed,200),(valid().replace("data: [DONE]\n\n",""),200),(format!("data: {}\n\n","x".repeat(65537)),200),(valid(),307)] {
        let events=collect(body,status).await;
        assert!(events.iter().any(|e|matches!(e,ProviderEvent::Error{..})),"{events:?}");
        assert!(!events.iter().any(|e|matches!(e,ProviderEvent::ToolCallComplete{..}|ProviderEvent::Finish{..})),"{events:?}");
    }
}
#[tokio::test]
async fn grants_bind_request_and_are_single_use_and_revocable(){
    let config=ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:"http://127.0.0.1:1/v1/chat/completions".into(),model:"synthetic".into()};
    let broker=Broker::new(1);let request=request();let permit=broker.approve_inference(&config,&request,&request.digest().unwrap(),now()+30,now()).unwrap();
    broker.revocation().cancel();let provider=Compatible::new(config,permit).unwrap();
    for _ in 0..2 {let mut rx=provider.infer(request.clone(),CancellationToken::new());assert!(matches!(rx.recv().await,Some(ProviderEvent::Error{code,..}) if code=="inference-denied"));assert!(rx.recv().await.is_none());}
}
#[test]
fn proxy_discovery_is_disabled(){
    if std::env::var_os("RIVET_PROXY_FIXTURE").is_some(){
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async{let events=collect(valid(),200).await;assert!(matches!(events.last(),Some(ProviderEvent::Finish{..})));});return;
    }
    let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","proxy_discovery_is_disabled","--test-threads=1"]).env("RIVET_PROXY_FIXTURE","1").env("HTTP_PROXY","http://127.0.0.1:1").env("ALL_PROXY","http://127.0.0.1:1").env_remove("NO_PROXY").output().unwrap();
    assert!(output.status.success(),"{} {}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
}

#[test]
fn endpoints_reject_plaintext_remote_credentials_redirect_targets_and_zero_remote_budget(){
    use rivet::providers::compatible::endpoint;
    for value in ["http://example.invalid/v1/chat/completions","file:///v1/chat/completions","http://user:pass@127.0.0.1:1234/v1/chat/completions","http://127.0.0.1:1234/v1/chat/completions?key=hidden","http://127.0.0.1:1234/other"]{assert!(endpoint(value).is_err());}
    assert!(endpoint("https://example.invalid/v1/chat/completions").is_ok());assert!(endpoint("http://[::1]:1234/v1/chat/completions").is_ok());
    let config=ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:"https://example.invalid/v1/chat/completions".into(),model:"synthetic".into()};let broker=Broker::new(1);let mut r=request();
    assert!(broker.approve_inference(&config,&r,&r.digest().unwrap(),now()+30,now()).is_err());r.context.as_mut().unwrap().max_cost_micros=100;
    assert!(broker.approve_inference(&config,&r,&r.digest().unwrap(),now()+30,now()).is_ok()); // No connection is made.
}
#[tokio::test]
async fn cancellation_after_send_reports_uncertainty_without_completed_proposal(){
    let socket=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=socket.local_addr().unwrap();let (started,seen)=tokio::sync::oneshot::channel();
    let server=tokio::spawn(async move{let (mut stream,_)=socket.accept().await.unwrap();let mut buffer=[0;4096];assert!(stream.read(&mut buffer).await.unwrap()>0);started.send(()).unwrap();tokio::time::sleep(Duration::from_millis(100)).await;});
    let config=ProviderConfig{profile:"llama.cpp-compatible".into(),endpoint:format!("http://{address}/v1/chat/completions"),model:"synthetic".into()};let broker=Broker::new(1);let request=request();let permit=broker.approve_inference(&config,&request,&request.digest().unwrap(),now()+30,now()).unwrap();let provider=Compatible::new(config,permit).unwrap();let cancel=CancellationToken::new();let mut events=provider.infer(request,cancel.clone());seen.await.unwrap();cancel.cancel();
    assert!(matches!(events.recv().await,Some(ProviderEvent::Error{code,..}) if code=="cancelled-outcome-unknown"));assert!(events.recv().await.is_none());server.await.unwrap();
}
