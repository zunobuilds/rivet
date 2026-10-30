//! Explicitly granted llama.cpp-compatible Chat Completions; no discovery/fallback.
use super::{InferRequest, ProviderEvent, ProviderPort, Capabilities};
use crate::{config::ProviderConfig, domain::OperationId, error::{Error, Result}, permissions::InferencePermit, tools::ToolProposal};
use serde_json::{json, Value};
use std::{collections::{BTreeMap, BTreeSet}, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub struct Compatible {
    config: ProviderConfig,
    permit: Arc<InferencePermit>,
    used: AtomicBool,
}
impl Compatible {
    pub fn new(config: ProviderConfig, permit: InferencePermit) -> Result<Self> {
        if config.profile != "llama.cpp-compatible" { return Err(Error::Unsupported); }
        endpoint(&config.endpoint)?;
        if config.model.is_empty() || config.model.len()>256 || config.model.chars().any(char::is_control) { return Err(Error::Invalid); }
        Ok(Self {config, permit:Arc::new(permit), used:AtomicBool::new(false)})
    }
}
/// Loopback HTTP or explicitly granted HTTPS. No redirect, userinfo, query or fragment.
pub fn endpoint(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value).map_err(|_|Error::Invalid)?;
    let host = url.host_str().ok_or(Error::Invalid)?.trim_matches(['[',']']);
    let loopback=host.parse::<std::net::IpAddr>().is_ok_and(|ip|ip.is_loopback());
    if !(url.scheme()=="https" || (url.scheme()=="http" && loopback)) { return Err(Error::Unsupported); }
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some()
        || url.path()!="/v1/chat/completions" || url.port_or_known_default().is_none() { return Err(Error::Invalid); }
    Ok(url)
}
impl ProviderPort for Compatible {
    fn capabilities(&self)->Capabilities {Capabilities{text:true,tool_calls:true,usage:true}}
    fn infer(&self, request:InferRequest, cancel:CancellationToken)->mpsc::Receiver<ProviderEvent> {
        let (tx,rx)=mpsc::channel(16);
        let allowed=!self.used.swap(true,Ordering::SeqCst) && self.permit.matches(&self.config,&request);
        let permit=self.permit.clone();let config=self.config.clone();
        tokio::spawn(async move {
            if !allowed { let _=tx.send(ProviderEvent::Error{request:request.id,code:"inference-denied".into()}).await; return; }
            let id=request.id;
            let work=transport(config,request,&tx);tokio::pin!(work);
            let result=tokio::select! {biased;
                _=cancel.cancelled()=>Err("cancelled-outcome-unknown"),
                _=permit.revoked.cancelled()=>Err("revoked-outcome-unknown"),
                _=tokio::time::sleep(permit.remaining())=>Err("expired-outcome-unknown"),
                result=&mut work=>result,
            };
            if let Err(code)=result {let _=tokio::time::timeout(Duration::from_millis(100),tx.send(ProviderEvent::Error{request:id,code:code.into()})).await;}
        });
        rx
    }
}
fn wire_name(id:&str,version:u32)->String {format!("{}_v{version}",id.replace([':', '.','-'],"_"))}
async fn transport(config:ProviderConfig,request:InferRequest,tx:&mpsc::Sender<ProviderEvent>)->std::result::Result<(), &'static str> {
    let _=rustls::crypto::ring::default_provider().install_default();
    // Compiled Mozilla roots prevent ambient native certificate/environment discovery.
    let roots=webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().map(|der|reqwest::Certificate::from_der(der.as_ref()).map_err(|_|"trust-root")).collect::<std::result::Result<Vec<_>,_>>()?;
    let client=reqwest::Client::builder().tls_certs_only(roots)
        .no_proxy().redirect(reqwest::redirect::Policy::none()).retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(60)).connection_verbose(false)
        .build().map_err(|_|"client-configuration")?;
    let context=request.context.as_ref();
    let messages=match context {
        Some(c)=>c.messages.iter().map(|m| {
            let mut value=json!({"role":m.role,"content":m.content});
            if let Some(id)=&m.tool_call_id {value["tool_call_id"]=json!(id);}
            if !m.tool_calls.is_empty(){value["tool_calls"]=json!(m.tool_calls.iter().map(|c|json!({"id":c.id,"type":"function","function":{"name":c.name,"arguments":c.arguments}})).collect::<Vec<_>>());}
            value
        }).collect::<Vec<_>>(),
        None=>vec![json!({"role":"user","content":request.prompt})],
    };
    let mut names=BTreeSet::new();
    let tools=match context{None=>vec![],Some(c)=>c.tools.iter().map(|d| {
        let name=wire_name(&d.id,d.version);names.insert(name.clone());
        Ok(json!({"type":"function","function":{"name":name,"description":d.id,"parameters":serde_json::from_str::<Value>(&d.input_schema).map_err(|_|"tool-schema")?}}))
    }).collect::<std::result::Result<Vec<_>, &'static str>>()?};
    if context.is_some_and(|c|tools.len()!=c.tools.len() || names.len()!=tools.len()) {return Err("tool-schema");}
    let max_output=context.map_or(1024,|c|c.max_output);
    if request.schema_version!=1 || max_output==0 || max_output>16384 {return Err("request-limit");}
    let mut body=json!({"model":config.model,"messages":messages,"stream":true,"stream_options":{"include_usage":true},"max_tokens":max_output,"n":1});
    if !tools.is_empty(){body["tools"]=json!(tools);body["parallel_tool_calls"]=json!(false);}
    let encoded=serde_json::to_vec(&body).map_err(|_|"request-encoding")?;
    if encoded.len()>1024*1024 {return Err("request-limit");}
    // From this send onward errors retain the reservation: the endpoint may have worked.
    let mut response=client.post(endpoint(&config.endpoint).map_err(|_|"endpoint")?).header("Content-Type","application/json")
        .body(encoded).send().await.map_err(|_|"transport-outcome-unknown")?;
    if !response.status().is_success(){return Err("http-outcome-unknown");}
    if !response.headers().get("content-type").and_then(|v|v.to_str().ok()).is_some_and(|s|s.split(';').next()==Some("text/event-stream")){return Err("unsupported-content-type");}
    let mut stream=Assembly::default();let mut pending=Vec::new();let mut event=String::new();let mut total=0;
    while let Some(chunk)=response.chunk().await.map_err(|_|"stream-outcome-unknown")? {
        total+=chunk.len();if total>1024*1024{return Err("response-quota-outcome-unknown");}
        pending.extend_from_slice(&chunk);
        while let Some(end)=pending.iter().position(|b|*b==b'\n') {
            let bytes=pending.drain(..=end).collect::<Vec<_>>();
            let line=std::str::from_utf8(&bytes[..end]).map_err(|_|"malformed-utf8")?.trim_end_matches('\r');
            if line.is_empty(){
                if !event.is_empty(){
                    let data=event.strip_suffix('\n').unwrap_or(&event);
                    let text_before=stream.text.len();
                    let call_sizes=stream.calls.iter().map(|(i,c)|(*i,c.arguments.len())).collect::<BTreeMap<_,_>>();
                    stream.event(data)?;
                    if stream.text.len()>text_before {send(tx,ProviderEvent::TextDelta{request:request.id,text:stream.text[text_before..].into()}).await?;}
                    for (index,call) in &stream.calls {
                        let before=call_sizes.get(index).copied().unwrap_or(0);
                        if call.arguments.len()>before{send(tx,ProviderEvent::ToolCallDelta{request:request.id,call:call.operation,arguments:call.arguments[before..].into()}).await?;}
                    }
                    event.clear();
                }
            }else if let Some(data)=line.strip_prefix("data:"){event.push_str(data.strip_prefix(' ').unwrap_or(data));event.push('\n');}
            else if !line.starts_with(':'){return Err("unsupported-sse-field");}
            if event.len()>65536{return Err("frame-quota-outcome-unknown");}
        }
        if pending.len()>65536{return Err("frame-quota-outcome-unknown");}
    }
    if !pending.is_empty() || !event.is_empty() || !stream.done {return Err("eof-outcome-unknown");}
    stream.emit(request,tx).await
}
#[derive(Default)]
struct Call {id:Option<String>,name:String,arguments:String,operation:OperationId}
#[derive(Default)]
struct Assembly {calls:BTreeMap<usize,Call>,ids:BTreeSet<String>,text:String,finish:Option<String>,usage:Option<(u64,u64)>,done:bool, chunks:usize}
impl Assembly {
    fn event(&mut self,data:&str)->std::result::Result<(), &'static str>{
        if self.done{return Err("data-after-done");}
        if data=="[DONE]"{if self.finish.is_none(){return Err("missing-finish");}self.done=true;return Ok(());}
        self.chunks+=1;if self.chunks>4096{return Err("chunk-quota");}
        let v=super::strict_json(data).map_err(|_|"malformed-json")?;
        if v.get("error").is_some(){return Err("provider-error-outcome-unknown");}
        if let Some(usage)=v.get("usage").filter(|v|!v.is_null()){
            if self.usage.is_some(){return Err("duplicate-usage");}
            self.usage=Some((usage["prompt_tokens"].as_u64().ok_or("invalid-usage")?,usage["completion_tokens"].as_u64().ok_or("invalid-usage")?));
        }
        let choices=v["choices"].as_array().ok_or("missing-choices")?;
        if choices.len()>1{return Err("multiple-choices");}
        for choice in choices {
            if self.finish.is_some(){return Err("choice-after-finish");}
            if choice["index"].as_u64()!=Some(0){return Err("choice-index");}
            let delta=&choice["delta"];
            if let Some(content)=delta.get("content").filter(|v|!v.is_null()) {self.text.push_str(content.as_str().ok_or("invalid-content")?);if self.text.len()>65536{return Err("text-quota");}}
            if let Some(calls)=delta.get("tool_calls") {
                for fragment in calls.as_array().ok_or("invalid-calls")? {
                    let index=fragment["index"].as_u64().ok_or("missing-call-index")? as usize;
                    if index>=8{return Err("call-quota");}
                    let call=self.calls.entry(index).or_default();
                    if let Some(id)=fragment.get("id"){
                        let id=id.as_str().ok_or("invalid-call-id")?;
                        if id.is_empty()||id.len()>128||id.chars().any(char::is_control)||call.id.is_some()||!self.ids.insert(id.into()){return Err("duplicate-call-id");}
                        call.id=Some(id.into());
                    }
                    if let Some(kind)=fragment.get("type"){if kind!="function"{return Err("unsupported-call-type");}}
                    if let Some(function)=fragment.get("function"){
                        if let Some(name)=function.get("name"){call.name.push_str(name.as_str().ok_or("invalid-function")?);}
                        if let Some(arguments)=function.get("arguments"){call.arguments.push_str(arguments.as_str().ok_or("invalid-arguments")?);}
                    }
                    if call.name.len()>128||call.arguments.len()>65536{return Err("call-quota");}
                }
            }
            if let Some(reason)=choice.get("finish_reason").filter(|v|!v.is_null()){
                let reason=reason.as_str().ok_or("invalid-finish")?;
                if !["stop","tool_calls","length","content_filter"].contains(&reason){return Err("unsupported-finish");}
                self.finish=Some(reason.into());
            }
        }
        Ok(())
    }
    async fn emit(self,request:InferRequest,tx:&mpsc::Sender<ProviderEvent>)->std::result::Result<(), &'static str>{
        let context=request.context;let reason=self.finish.ok_or("missing-finish")?;
        if reason=="length"||reason=="content_filter"{return Err("incomplete-generation");}
        if (reason=="tool_calls")!=(!self.calls.is_empty()){return Err("finish-call-mismatch");}
        let mut complete=Vec::new();
        for (expected,(index,call)) in self.calls.into_iter().enumerate(){
            if expected!=index||call.id.is_none(){return Err("incomplete-call");}
            let context=context.as_ref().ok_or("unrequested-tool-call")?;
            let descriptor=context.tools.iter().find(|d|wire_name(&d.id,d.version)==call.name).ok_or("unknown-tool")?;
            let arguments=super::strict_json(&call.arguments).map_err(|_|"malformed-arguments")?;
            if !arguments.is_object(){return Err("invalid-arguments");}
            complete.push((call.id.expect("checked"),ToolProposal{schema_version:1,operation:call.operation,attempt:1,task:context.task,actor:context.actor,descriptor_digest:descriptor.build_digest.clone(),arguments,resources:vec![],preconditions:vec![],budget_reservation:context.reservation.clone()}));
        }
        // No incomplete/malformed tool proposal is emitted before the full response validates.
        for (wire_id,proposal) in complete {send(tx,ProviderEvent::ToolCallComplete{request:request.id,proposal,wire_id:Some(wire_id)}).await?;}
        if let Some((input,output))=self.usage{send(tx,ProviderEvent::Usage{request:request.id,input,output}).await?;}
        send(tx,ProviderEvent::Finish{request:request.id,reason}).await
    }
}
async fn send(tx:&mpsc::Sender<ProviderEvent>,event:ProviderEvent)->std::result::Result<(), &'static str>{tx.send(event).await.map_err(|_|"receiver-closed")}
