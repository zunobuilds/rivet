//! Native Ollama NDJSON and versioned Anthropic Messages SSE, separately configured.
use super::*;
use crate::{config::ProviderConfig,error::{Error,Result},permissions::InferencePermit};
use serde_json::{json,Value};
use std::{collections::{BTreeMap,BTreeSet},sync::{Arc,atomic::{AtomicBool,Ordering}},time::Duration};
/// No environment/keyring/file discovery. The caller owns ephemeral credential entry.
pub struct EphemeralSecret{value:String,reference:uuid::Uuid}
impl EphemeralSecret {
    pub fn new(value:String)->Result<Self>{if value.is_empty()||value.len()>4096||!value.bytes().all(|b|b.is_ascii_graphic()){return Err(Error::Invalid);}Ok(Self{value,reference:uuid::Uuid::new_v4()})}
}
impl EphemeralSecret {pub fn reference(&self)->String{self.reference.to_string()}}
impl Drop for EphemeralSecret {fn drop(&mut self){use zeroize::Zeroize;self.value.zeroize();}}
pub struct Native {config:ProviderConfig,permit:Arc<InferencePermit>,used:AtomicBool,secret:Option<Arc<EphemeralSecret>>}
impl Native {
    pub fn new(config:ProviderConfig,permit:InferencePermit,secret:Option<Arc<EphemeralSecret>>)->Result<Self>{
        validate_endpoint(&config)?;
        if !["ollama","anthropic"].contains(&config.profile.as_str())||config.model.is_empty()||config.model.len()>256||config.model.chars().any(char::is_control){return Err(Error::Invalid);}
        if (config.profile=="anthropic"&&secret.is_none())||(config.profile!="anthropic"&&secret.is_some()){return Err(Error::Denied);}
        if config.profile=="ollama"&&(config.model.ends_with(":cloud")||config.model.contains("/cloud")){return Err(Error::Denied);}
        Ok(Self{config,permit:Arc::new(permit),used:AtomicBool::new(false),secret})
    }
}
impl ProviderPort for Native {
    fn capabilities(&self)->Capabilities{Capabilities{text:true,tool_calls:true,usage:true}}
    fn infer(&self,request:InferRequest,cancel:CancellationToken)->mpsc::Receiver<ProviderEvent>{
        let (tx,rx)=mpsc::channel(16);let auth=self.config.profile!="anthropic"||request.context.as_ref().and_then(|c|c.credential_ref.as_ref()).is_some_and(|id|self.secret.as_ref().is_some_and(|s|s.reference()==*id));let allowed=auth&&!self.used.swap(true,Ordering::SeqCst)&&self.permit.matches(&self.config,&request);let config=self.config.clone();let permit=self.permit.clone();let secret=self.secret.clone();
        tokio::spawn(async move{
            if !allowed{let _=tx.send(ProviderEvent::Error{request:request.id,code:"inference-denied".into()}).await;return;}
            let id=request.id;let work=transport(config,secret,request,&tx);tokio::pin!(work);
            let result=tokio::select!{biased;_=cancel.cancelled()=>Err("cancelled-outcome-unknown"),_=permit.revoked.cancelled()=>Err("revoked-outcome-unknown"),_=tokio::time::sleep(permit.remaining())=>Err("expired-outcome-unknown"),r=&mut work=>r};
            if let Err(code)=result{let _=tokio::time::timeout(Duration::from_millis(100),tx.send(ProviderEvent::Error{request:id,code:code.into()})).await;}
        });rx
    }
}
fn name(d:&crate::tools::ToolDescriptor)->String{format!("{}_v{}",d.id.replace([':', '.', '-'],"_"),d.version)}
fn messages(request:&InferRequest,anthropic:bool)->std::result::Result<(Vec<Value>,String),&'static str>{
    let source=request.context.as_ref().map(|c|c.messages.clone()).unwrap_or_else(||vec![ChatMessage::text(MessageRole::User,request.prompt.clone())]);
    let mut messages=vec![];let mut system=String::new();let mut tool_names=BTreeMap::new();
    for m in source {
        if anthropic {
            match m.role {
                MessageRole::System=>{system.push_str(&m.content);system.push('\n');},
                MessageRole::Tool=>messages.push(json!({"role":"user","content":[{"type":"tool_result","tool_use_id":m.tool_call_id.ok_or("missing-tool-id")?,"content":m.content}]})),
                role=>{let mut content=vec![];if !m.content.is_empty(){content.push(json!({"type":"text","text":m.content}));}for c in m.tool_calls {content.push(json!({"type":"tool_use","id":c.id,"name":c.name,"input":strict_json(&c.arguments).map_err(|_|"arguments")?}));}messages.push(json!({"role":role,"content":content}));}
            }
        }else{
            let mut value=json!({"role":m.role,"content":m.content});
            if let Some(id)=m.tool_call_id{value["tool_name"]=json!(tool_names.get(&id).ok_or("missing-tool-name")?); }
            for c in &m.tool_calls{tool_names.insert(c.id.clone(),c.name.clone());}
            if !m.tool_calls.is_empty(){value["tool_calls"]=json!(m.tool_calls.into_iter().map(|c|Ok(json!({"function":{"name":c.name,"arguments":strict_json(&c.arguments).map_err(|_|"arguments")?}}))).collect::<std::result::Result<Vec<_>,&'static str>>()?);}
            messages.push(value);
        }
    }Ok((messages,system))
}
async fn transport(config:ProviderConfig,secret:Option<Arc<EphemeralSecret>>,request:InferRequest,tx:&mpsc::Sender<ProviderEvent>)->std::result::Result<(),&'static str>{
    let anthropic=config.profile=="anthropic";let (messages,system)=messages(&request,anthropic)?;
    let context=request.context.as_ref();let max=context.map_or(1024,|c|c.max_output);if request.schema_version!=1||max==0||max>16384{return Err("request-limit");}
    let mut tools=vec![];let mut names=BTreeSet::new();
    if let Some(c)=context{for d in &c.tools {let name=name(d);if !names.insert(name.clone()){return Err("duplicate-tool");}let schema=strict_json(&d.input_schema).map_err(|_|"tool-schema")?;
        tools.push(if anthropic{json!({"name":name,"description":d.id,"input_schema":schema})}else{json!({"type":"function","function":{"name":name,"description":d.id,"parameters":schema}})});
    }}
    let mut body=if anthropic{json!({"model":config.model,"messages":messages,"system":system,"max_tokens":max,"stream":true})}else{json!({"model":config.model,"messages":messages,"options":{"num_predict":max},"stream":true})};
    if !tools.is_empty(){body["tools"]=json!(tools);}
    let encoded=serde_json::to_vec(&body).map_err(|_|"request-encoding")?;if encoded.len()>1024*1024{return Err("request-limit");}
    let mut outgoing=super::http::client()?.post(validate_endpoint(&config).map_err(|_|"endpoint")?).header("Content-Type","application/json");
    if anthropic{
        let mut key=reqwest::header::HeaderValue::from_str(&secret.as_ref().ok_or("missing-credential")?.value).map_err(|_|"credential-format")?;key.set_sensitive(true);
        outgoing=outgoing.header("x-api-key",key).header("anthropic-version","2023-06-01");
    }
    let mut response=outgoing.body(encoded).send().await.map_err(|_|"transport-outcome-unknown")?;if !response.status().is_success(){return Err("http-outcome-unknown");}
    let content=response.headers().get("content-type").and_then(|v|v.to_str().ok()).and_then(|s|s.split(';').next()).ok_or("content-type")?;
    if (anthropic&&content!="text/event-stream")||(!anthropic&&content!="application/x-ndjson"){return Err("content-type");}
    let mut decoder=NativeStream::default();let mut pending=Vec::new();let mut data=String::new();let mut total=0;let mut records=0;
    while let Some(bytes)=response.chunk().await.map_err(|_|"stream-outcome-unknown")?{
        total+=bytes.len();if total>1024*1024{return Err("response-quota");}pending.extend_from_slice(&bytes);
        while let Some(end)=pending.iter().position(|b|*b==b'\n'){
            let bytes=pending.drain(..=end).collect::<Vec<_>>();let line=std::str::from_utf8(&bytes[..end]).map_err(|_|"utf8")?.trim_end_matches('\r');
            let record=if anthropic {
                if line.is_empty(){if data.is_empty(){None}else{Some(std::mem::take(&mut data))}}
                else if let Some(value)=line.strip_prefix("data:"){data.push_str(value.strip_prefix(' ').unwrap_or(value));data.push('\n');None}
                else if line.starts_with("event:")||line.starts_with(':'){None}else{return Err("sse-field");}
            }else if line.is_empty(){None}else{Some(line.to_owned())};
            if data.len()>65536{return Err("frame-quota");}
            if let Some(record)=record{records+=1;if records>4096{return Err("record-quota");}let value=strict_json(record.trim_end()).map_err(|_|"malformed-json")?;
                let text=if anthropic{decoder.anthropic(value)?}else{decoder.ollama(value)?};
                if let Some(text)=text{tx.send(ProviderEvent::TextDelta{request:request.id,text}).await.map_err(|_|"receiver-closed")?;}
            }
        }
        if pending.len()>65536{return Err("frame-quota");}
    }
    if !pending.is_empty()||!data.is_empty()||!decoder.done{return Err("eof-outcome-unknown");}
    decoder.finish(request,tx).await
}
#[derive(Default)]
struct Block {id:String,name:String,args:String,closed:bool,text:bool}
#[derive(Default)]
struct NativeStream {blocks:BTreeMap<usize,Block>,ids:BTreeSet<String>,input:Option<u64>,output:Option<u64>,reason:Option<String>,started:bool,done:bool,text_bytes:usize}
impl NativeStream {
    fn text(&mut self,value:&Value)->std::result::Result<Option<String>,&'static str>{let text=value.as_str().ok_or("text")?;self.text_bytes+=text.len();if self.text_bytes>65536{return Err("text-quota");}Ok((!text.is_empty()).then(||text.into()))}
    fn ollama(&mut self,v:Value)->std::result::Result<Option<String>,&'static str>{
        if self.done||v.get("error").is_some(){return Err("provider-error");}
        let text=if let Some(text)=v["message"].get("content"){self.text(text)?}else{None};
        if let Some(calls)=v["message"].get("tool_calls"){for call in calls.as_array().ok_or("tool-calls")?{
            let name=call["function"]["name"].as_str().ok_or("tool-name")?.to_owned();let args=&call["function"]["arguments"];if !args.is_object(){return Err("tool-arguments");}
            let args=args.to_string();let identity=format!("{name}:{args}");if !self.ids.insert(identity)||self.blocks.len()>=8||name.len()>128||args.len()>65536{return Err("duplicate-or-large-call");}
            self.blocks.insert(self.blocks.len(),Block{id:uuid::Uuid::new_v4().to_string(),name,args,closed:true,text:false});
        }}
        if v["done"].as_bool()==Some(true){self.input=Some(v["prompt_eval_count"].as_u64().ok_or("usage")?);self.output=Some(v["eval_count"].as_u64().ok_or("usage")?);if v["done_reason"]!="stop"{return Err("incomplete-generation");}self.reason=Some(if self.blocks.is_empty(){"stop"}else{"tool_calls"}.into());self.done=true;}
        else if v["done"].as_bool()!=Some(false){return Err("missing-done");}Ok(text)
    }
    fn anthropic(&mut self,v:Value)->std::result::Result<Option<String>,&'static str>{
        if self.done{return Err("data-after-stop");}
        let kind=v["type"].as_str().ok_or("event-type")?;
        if self.reason.is_some()&&!["message_stop","ping"].contains(&kind){return Err("data-after-finish");}
        match kind{
            "ping"=>(),
            "message_start"=>{if self.started{return Err("duplicate-start");}self.started=true;let usage=&v["message"]["usage"];let input=usage["input_tokens"].as_u64().ok_or("usage")?;let cached=usage.get("cache_read_input_tokens").map_or(Some(0),Value::as_u64).ok_or("usage")?;let created=usage.get("cache_creation_input_tokens").map_or(Some(0),Value::as_u64).ok_or("usage")?;self.input=Some(input.checked_add(cached).and_then(|n|n.checked_add(created)).ok_or("usage")?);self.output=Some(usage["output_tokens"].as_u64().ok_or("usage")?);},
            "content_block_start"=>{
                if !self.started||self.blocks.len()>=16{return Err("block-limit");}let index=v["index"].as_u64().ok_or("block-index")? as usize;if self.blocks.contains_key(&index){return Err("duplicate-block");}
                let b=&v["content_block"];let block=match b["type"].as_str(){
                    Some("text")=>Block{text:true,..Block::default()},
                    Some("tool_use")=>{let id=b["id"].as_str().ok_or("call-id")?;let name=b["name"].as_str().ok_or("call-name")?;if id.is_empty()||id.len()>128||name.len()>128||!self.ids.insert(id.into()){return Err("duplicate-call");}let input=&b["input"];if !input.is_object()||!input.as_object().is_some_and(|m|m.is_empty()){return Err("initial-tool-input");}Block{id:id.into(),name:name.into(),..Block::default()}},
                    _=>return Err("unsupported-content-block"),
                };self.blocks.insert(index,block);if b["type"]=="text"{return self.text(&b["text"]);}
            },
            "content_block_delta"=>{
                let index=v["index"].as_u64().ok_or("block-index")? as usize;let b=self.blocks.get_mut(&index).ok_or("unknown-block")?;if b.closed{return Err("closed-block");}
                match v["delta"]["type"].as_str(){Some("text_delta") if b.text=>return self.text(&v["delta"]["text"]),Some("input_json_delta") if !b.text=>{b.args.push_str(v["delta"]["partial_json"].as_str().ok_or("arguments")?);if b.args.len()>65536{return Err("argument-quota");}},_=>return Err("unsupported-delta")}
            },
            "content_block_stop"=>{let index=v["index"].as_u64().ok_or("block-index")? as usize;let b=self.blocks.get_mut(&index).ok_or("unknown-block")?;if b.closed{return Err("duplicate-block-stop");}b.closed=true;},
            "message_delta"=>{if self.reason.is_some()||!self.started{return Err("duplicate-finish");}let reason=v["delta"]["stop_reason"].as_str().ok_or("stop-reason")?;self.reason=Some(match reason{"end_turn"|"stop_sequence"=>"stop","tool_use"=>"tool_calls",_=>return Err("incomplete-generation")}.into());self.output=Some(v["usage"]["output_tokens"].as_u64().ok_or("usage")?);},
            "message_stop"=>{if !self.started||self.reason.is_none()||self.blocks.values().any(|b|!b.closed){return Err("incomplete-message");}self.done=true;},
            _=>return Err("provider-or-unsupported-event"),
        }Ok(None)
    }
    async fn finish(self,request:InferRequest,tx:&mpsc::Sender<ProviderEvent>)->std::result::Result<(),&'static str>{
        let calls=self.blocks.into_values().filter(|b|!b.text).collect::<Vec<_>>();if calls.len()>8{return Err("call-quota");}
        let reason=self.reason.ok_or("missing-finish")?;if (reason=="tool_calls")!=(!calls.is_empty()){return Err("finish-mismatch");}
        let input=self.input.ok_or("usage")?;let output=self.output.ok_or("usage")?;
        let mut proposals=vec![];
        for call in calls{let context=request.context.as_ref().ok_or("unrequested-call")?;let descriptor=context.tools.iter().find(|d|name(d)==call.name).ok_or("unknown-tool")?;let arguments=strict_json(if call.args.is_empty(){"{}"}else{&call.args}).map_err(|_|"malformed-arguments")?;if !arguments.is_object(){return Err("arguments");}
            proposals.push((call.id,crate::tools::ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task:context.task,actor:context.actor,descriptor_digest:descriptor.build_digest.clone(),arguments,resources:vec![],preconditions:vec![],budget_reservation:context.reservation.clone()}));
        }
        for (id,proposal) in proposals{tx.send(ProviderEvent::ToolCallComplete{request:request.id,proposal,wire_id:Some(id)}).await.map_err(|_|"receiver-closed")?;}
        tx.send(ProviderEvent::Usage{request:request.id,input,output}).await.map_err(|_|"receiver-closed")?;
        tx.send(ProviderEvent::Finish{request:request.id,reason}).await.map_err(|_|"receiver-closed")
    }
}
