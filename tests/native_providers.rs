use rivet::{config::ProviderConfig,domain::*,permissions::Broker,providers::{*,native::{Native,EphemeralSecret}},tools::files::FileWorkspace};
use serde_json::{json,Value};
use std::{sync::Arc,time::{SystemTime,UNIX_EPOCH}};
use tokio::{net::TcpListener,io::{AsyncReadExt,AsyncWriteExt}};
use tokio_util::sync::CancellationToken;
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
async fn fixture(profile:&str,body:String)->Vec<ProviderEvent>{
    let socket=TcpListener::bind("127.0.0.1:0").await.unwrap();let address=socket.local_addr().unwrap();let anthropic=profile=="anthropic";
    let server=tokio::spawn(async move{let (mut stream,_)=socket.accept().await.unwrap();let mut bytes=vec![];let mut buffer=[0;4096];
        loop{let n=stream.read(&mut buffer).await.unwrap();assert!(n>0);bytes.extend_from_slice(&buffer[..n]);if let Some(end)=bytes.windows(4).position(|s|s==b"\r\n\r\n"){
            let header=String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();let length=header.lines().find_map(|l|l.strip_prefix("content-length: ").and_then(|v|v.parse::<usize>().ok())).unwrap();if bytes.len()>=end+4+length{
                let request:Value=serde_json::from_slice(&bytes[end+4..]).unwrap();assert_eq!(request["model"],"synthetic");assert_eq!(request["stream"],true);
                if anthropic{assert!(header.contains("anthropic-version: 2023-06-01"));assert!(header.contains("x-api-key: synthetic-key"));assert_eq!(request["max_tokens"],1024);assert_eq!(request["tools"][0]["input_schema"]["$schema"],"https://json-schema.org/draft/2020-12/schema");}else{assert!(!header.contains("x-api-key"));assert_eq!(request["options"]["num_predict"],1024);}break;
            }
        }}
        let content=if anthropic{"text/event-stream"}else{"application/x-ndjson"};stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: {content}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).await.unwrap();for part in body.as_bytes().chunks(5){if stream.write_all(part).await.is_err(){break;}tokio::task::yield_now().await;}
    });
    let path=if anthropic{"/v1/messages"}else{"/api/chat"};let config=ProviderConfig{profile:profile.into(),endpoint:format!("http://{address}{path}"),model:"synthetic".into()};
    let secret=anthropic.then(||Arc::new(EphemeralSecret::new("synthetic-key".into()).unwrap()));
    let request=InferRequest{schema_version:1,id:RequestId::default(),prompt:String::new(),context:Some(RequestContext{messages:vec![ChatMessage::text(MessageRole::User,"synthetic fixture".into())],tools:vec![FileWorkspace::descriptor()],task:TaskId::default(),actor:ActorId::default(),max_output:1024,max_cost_micros:0,credential_ref:secret.as_ref().map(|s|s.reference()),reservation:"fixture".into()})};
    let broker=Broker::new(1);let permit=broker.approve_inference(&config,&request,&request.digest().unwrap(),now()+60,now()).unwrap();let provider=Native::new(config,permit,secret).unwrap();
    let mut rx=provider.infer(request,CancellationToken::new());let mut events=vec![];while let Some(e)=rx.recv().await{events.push(e);}server.await.unwrap();events
}
fn anthropic_events()->Vec<Value>{vec![
    json!({"type":"message_start","message":{"id":"fixture","usage":{"input_tokens":10,"output_tokens":0,"cache_read_input_tokens":2}}}),
    json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call-fixture","name":"rivet_files_v1","input":{}}}),
    json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"operation\":\"read\","}}),
    json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"path\":\"a.txt\"}"}}),
    json!({"type":"content_block_stop","index":0}),
    json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":5}}),
    json!({"type":"message_stop"}),
]}
fn sse(events:Vec<Value>)->String{events.into_iter().map(|v|format!("event: {}\ndata: {v}\n\n",v["type"].as_str().unwrap())).collect()}
#[tokio::test]
async fn native_anthropic_assembles_call_and_normalizes_cached_usage(){
    let events=fixture("anthropic",sse(anthropic_events())).await;assert_eq!(events.len(),3,"{events:?}");assert!(matches!(&events[0],ProviderEvent::ToolCallComplete{proposal,..} if proposal.arguments["path"]=="a.txt"));assert!(matches!(events[1],ProviderEvent::Usage{input:12,output:5,..}));assert!(matches!(events[2],ProviderEvent::Finish{..}));
}
#[tokio::test]
async fn native_ollama_normalizes_text_tools_and_usage_without_pull(){
    let body=format!("{}\n{}\n",json!({"message":{"content":"hé🐕"},"done":false}),json!({"message":{"content":"","tool_calls":[{"function":{"name":"rivet_files_v1","arguments":{"operation":"read","path":"a.txt"}}}]},"done":true,"done_reason":"stop","prompt_eval_count":10,"eval_count":5}));
    let events=fixture("ollama",body).await;assert!(matches!(&events[0],ProviderEvent::TextDelta{text,..} if text=="hé🐕"));assert!(matches!(&events[1],ProviderEvent::ToolCallComplete{proposal,..} if proposal.arguments["path"]=="a.txt"));assert!(matches!(events[2],ProviderEvent::Usage{input:10,output:5,..}));assert!(matches!(events[3],ProviderEvent::Finish{..}));
}
#[tokio::test]
async fn native_duplicate_calls_and_missing_stop_never_complete(){
    let mut duplicate=anthropic_events();duplicate.insert(2,duplicate[1].clone());let mut eof=anthropic_events();eof.pop();
    for body in [sse(duplicate),sse(eof)]{let events=fixture("anthropic",body).await;assert!(events.iter().any(|e|matches!(e,ProviderEvent::Error{..})));assert!(!events.iter().any(|e|matches!(e,ProviderEvent::ToolCallComplete{..}|ProviderEvent::Finish{..})));}
}
#[tokio::test]
async fn credentials_and_profiles_cannot_be_swapped_under_an_inference_permit(){
    let original=Arc::new(EphemeralSecret::new("synthetic-original".into()).unwrap());let changed=Arc::new(EphemeralSecret::new("synthetic-replaced".into()).unwrap());
    let config=ProviderConfig{profile:"anthropic".into(),endpoint:"http://127.0.0.1:1/v1/messages".into(),model:"synthetic".into()};
    let request=InferRequest{schema_version:1,id:RequestId::default(),prompt:String::new(),context:Some(RequestContext{messages:vec![ChatMessage::text(MessageRole::User,"fixture".into())],tools:vec![],task:TaskId::default(),actor:ActorId::default(),max_output:1,max_cost_micros:0,credential_ref:Some(original.reference()),reservation:"fixture".into()})};
    let broker=Broker::new(1);let permit=broker.approve_inference(&config,&request,&request.digest().unwrap(),now()+60,now()).unwrap();let provider=Native::new(config,permit,Some(changed)).unwrap();let mut events=provider.infer(request,CancellationToken::new());assert!(matches!(events.recv().await,Some(ProviderEvent::Error{code,..}) if code=="inference-denied"));
}
