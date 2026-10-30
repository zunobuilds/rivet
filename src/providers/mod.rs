pub(crate) mod http;
pub mod native;
pub mod compatible;
use crate::domain::{RequestId, OperationId};
use crate::tools::ToolProposal;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capabilities { pub text: bool, pub tool_calls: bool, pub usage: bool }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferRequest { pub schema_version: u32, pub id: RequestId, pub prompt: String, pub context: Option<RequestContext> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCall {pub id:String, pub name:String, pub arguments:String}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {pub role:MessageRole, pub content:String, pub tool_call_id:Option<String>, pub tool_calls:Vec<ChatCall>}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum MessageRole {System, User, Assistant, Tool}
impl ChatMessage {pub fn text(role:MessageRole, content:String)->Self {Self{role,content,tool_call_id:None,tool_calls:vec![]}}}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestContext {pub messages:Vec<ChatMessage>, pub tools:Vec<crate::tools::ToolDescriptor>, pub task:crate::domain::TaskId, pub actor:crate::domain::ActorId, pub max_output:u32, pub max_cost_micros:u64, pub credential_ref:Option<String>, pub reservation:String}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProviderEvent {
    TextDelta { request: RequestId, text: String },
    ToolCallDelta { request: RequestId, call: OperationId, arguments: String },
    ToolCallComplete { request: RequestId, proposal: ToolProposal, wire_id: Option<String> },
    Usage { request: RequestId, input: u64, output: u64 },
    Finish { request: RequestId, reason: String },
    Error { request: RequestId, code: String },
}
/// A provider receives no broker, dispatcher or ledger authority.
pub trait ProviderPort: Send + Sync {
    fn capabilities(&self) -> Capabilities;
    fn infer(&self, request: InferRequest, cancel: CancellationToken) -> mpsc::Receiver<ProviderEvent>;
}

/// Reject duplicate JSON keys at every depth, including assembled tool arguments.
pub(crate) fn strict_json(input:&str)->crate::error::Result<serde_json::Value> {
    struct Strict(serde_json::Value);
    impl<'de> Deserialize<'de> for Strict {
        fn deserialize<D:serde::Deserializer<'de>>(de:D)->std::result::Result<Self,D::Error>{
            struct V;
            impl<'de> serde::de::Visitor<'de> for V {
                type Value=Strict;
                fn expecting(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("JSON without duplicate keys")}
                fn visit_bool<E:serde::de::Error>(self,v:bool)->std::result::Result<Strict,E>{Ok(Strict(v.into()))}
                fn visit_i64<E:serde::de::Error>(self,v:i64)->std::result::Result<Strict,E>{Ok(Strict(v.into()))}
                fn visit_u64<E:serde::de::Error>(self,v:u64)->std::result::Result<Strict,E>{Ok(Strict(v.into()))}
                fn visit_f64<E:serde::de::Error>(self,v:f64)->std::result::Result<Strict,E>{serde_json::Number::from_f64(v).map(|n|Strict(n.into())).ok_or_else(||E::custom("number"))}
                fn visit_str<E:serde::de::Error>(self,v:&str)->std::result::Result<Strict,E>{Ok(Strict(v.into()))}
                fn visit_unit<E:serde::de::Error>(self)->std::result::Result<Strict,E>{Ok(Strict(serde_json::Value::Null))}
                fn visit_seq<A:serde::de::SeqAccess<'de>>(self,mut a:A)->std::result::Result<Strict,A::Error>{let mut out=vec![];while let Some(Strict(v))=a.next_element()?{out.push(v);}Ok(Strict(out.into()))}
                fn visit_map<A:serde::de::MapAccess<'de>>(self,mut a:A)->std::result::Result<Strict,A::Error>{
                    let mut out=serde_json::Map::new();while let Some((k,Strict(v)))=a.next_entry::<String,Strict>()?{if out.insert(k,v).is_some(){return Err(serde::de::Error::custom("duplicate JSON key"));}}Ok(Strict(out.into()))
                }
            }
            de.deserialize_any(V)
        }
    }
    serde_json::from_str::<Strict>(input).map(|v|v.0).map_err(|_|crate::error::Error::Invalid)
}
impl InferRequest {
    pub fn digest(&self)->crate::error::Result<String>{serde_json::to_vec(self).map(|v|crate::tools::sha256(&v)).map_err(|_|crate::error::Error::Invalid)}
}

pub fn validate_endpoint(config:&crate::config::ProviderConfig)->crate::error::Result<reqwest::Url>{
    match config.profile.as_str(){
        "llama.cpp-compatible"=>compatible::endpoint(&config.endpoint),
        "ollama"=>http::endpoint(&config.endpoint,"/api/chat"),
        "anthropic"=>http::endpoint(&config.endpoint,"/v1/messages"),
        _=>Err(crate::error::Error::Unsupported),
    }
}

/// Conservative byte-based admission, never a tokenizer or performance claim.
pub fn context_cost(request:&InferRequest)->crate::error::Result<u64>{let output=request.context.as_ref().map_or(0,|c|c.max_output as u64);(serde_json::to_vec(request).map_err(|_|crate::error::Error::Invalid)?.len() as u64).checked_mul(2).and_then(|v|v.checked_add(output+1024)).ok_or(crate::error::Error::Invalid)}
pub fn validate_context(config:&crate::config::ProviderConfig,request:&InferRequest)->crate::error::Result<()> {let limit=config.context_tokens.ok_or(crate::error::Error::Unsupported)?;if limit==0||limit>1_000_000{return Err(crate::error::Error::Invalid);}if context_cost(request)?>limit as u64{return Err(crate::error::Error::Denied);}Ok(())}
