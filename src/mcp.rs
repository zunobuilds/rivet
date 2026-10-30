//! Explicit stdio-only MCP under the same qualified no-network worker boundary.
use crate::{config::Project,error::{Error,Result},tools::{self,process::ProcessRequest}};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};
use std::{collections::BTreeMap,path::PathBuf,sync::{Arc,atomic::{AtomicBool,Ordering}}};
use tokio::io::{AsyncReadExt,AsyncWriteExt};
pub const DESCRIPTOR:&str=concat!("rivet:mcp:v1:",env!("RIVET_BUILD_DIGEST"));
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {pub id:String,#[serde(default)]pub enabled:bool,pub program:String,pub args:Vec<String>,#[serde(default)]pub files:BTreeMap<PathBuf,String>}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(tag="operation",rename_all="snake_case",deny_unknown_fields)]
pub enum Action {List{server:String},Call{server:String,tool:String,arguments:Value,expected_toolset:String}}
impl Action {
    fn server(&self)->&str{match self{Self::List{server}|Self::Call{server,..}=>server}}
    pub fn descriptor()->tools::ToolDescriptor {
        let schema=json!({"oneOf":[{"type":"object","additionalProperties":false,"required":["operation","server"],"properties":{"operation":{"const":"list"},"server":{"type":"string"}}},{"type":"object","additionalProperties":false,"required":["operation","server","tool","arguments","expected_toolset"],"properties":{"operation":{"const":"call"},"server":{"type":"string"},"tool":{"type":"string"},"arguments":{"type":"object"},"expected_toolset":{"type":"string"}}}]});
        tools::ToolDescriptor{id:"rivet:mcp".into(),version:1,build_digest:DESCRIPTOR.into(),input_schema:schema.to_string(),output_limit:32700,effect:tools::EffectClass::External,permissions:vec!["mcp-process".into(),"mcp-tool".into()],platform:"linux".into(),cancellable:true,reconcilable:false}
    }
}
pub struct Workspace {servers:BTreeMap<String,Server>,identity:(u64,u64)}
impl Workspace {
    pub fn new(project:&Project,servers:Vec<Server>)->Result<Self>{
        use cap_std::fs::MetadataExt;
        let mut enabled=BTreeMap::new();for server in servers {
            if !server.enabled{continue;}
            if server.id.is_empty()||server.id.len()>64||!server.id.chars().all(|c|c.is_ascii_alphanumeric()||"_-".contains(c)){return Err(Error::Invalid);}
            let request=ProcessRequest{program:server.program.clone(),args:server.args.clone(),files:server.files.clone(),timeout_ms:15000};request.validate()?;
            if server.files.contains_key(&PathBuf::from("__rivet_mcp_request.json"))||enabled.insert(server.id.clone(),server).is_some(){return Err(Error::Invalid);}
        }
        let m=project.dir.dir_metadata()?;Ok(Self{servers:enabled,identity:(m.dev(),m.ino())})
    }
    pub fn has_enabled(&self)->bool{!self.servers.is_empty()}
    pub(crate) fn identity(&self)->(u64,u64){self.identity}
    pub fn resources(&self,action:&Action)->Result<Vec<String>>{
        let server=self.servers.get(action.server()).ok_or(Error::Denied)?;
        if let Action::Call{tool,arguments,expected_toolset,..}=action{
            if tool.is_empty()||tool.len()>128||tool.chars().any(char::is_control)||!arguments.is_object()||arguments.to_string().len()>65536||expected_toolset.len()!=64||!expected_toolset.bytes().all(|b|b.is_ascii_hexdigit()){return Err(Error::Invalid);}
        }
        let digest=tools::sha256(&serde_json::to_vec(server).map_err(|_|Error::Invalid)?);
        Ok(vec![format!("mcp-server:{}:{digest}",server.id),format!("server-configuration:{}",serde_json::to_string(server).map_err(|_|Error::Invalid)?),"profile:systemd-bwrap-landlock-v1;stdio;no-network;no-sampling;no-roots;no-elicitation".into()])
    }
    pub(crate) fn request(&self,action:Action)->Result<ProcessRequest>{
        self.resources(&action)?;let server=self.servers.get(action.server()).ok_or(Error::Denied)?.clone();
        let mut files=server.files.clone();files.insert("__rivet_mcp_request.json".into(),serde_json::to_string(&Job{server,action}).map_err(|_|Error::Invalid)?);
        let request=ProcessRequest{program:"/usr/bin/true".into(),args:vec![],files,timeout_ms:15000};request.validate()?;Ok(request)
    }
}
#[derive(Serialize,Deserialize)]
struct Job {server:Server,action:Action}
#[derive(Debug,Serialize,Deserialize)]
pub struct McpResult {pub protocol:String,pub toolset_digest:String,pub tools:Vec<Value>,pub result:Option<Value>}
struct Handler {changed:Arc<AtomicBool>}
impl rmcp::ClientHandler for Handler {
    fn get_info(&self)->rmcp::model::ClientConfig{
        rmcp::model::ClientConfig::default().with_protocol_version(rmcp::model::ProtocolVersion::V_2025_11_25)
    }
    async fn on_tool_list_changed(&self,_:rmcp::service::NotificationContext<rmcp::RoleClient>){self.changed.store(true,Ordering::SeqCst);}
    // The SDK's empty capabilities/default handler rejects sampling, returns no roots,
    // and declines elicitation. No provider/broker/credential is in this process.
}
async fn filter_frames(mut source:tokio::process::ChildStdout,mut output:tokio::io::DuplexStream)->Result<()> {
    let mut pending=Vec::new();let mut chunk=[0;4096];let mut total=0;let mut frames=0;
    loop{
        let n=source.read(&mut chunk).await?;if n==0{if !pending.is_empty(){return Err(Error::Invalid);}return Ok(());}
        total+=n;if total>4*1024*1024{return Err(Error::Unsupported);}pending.extend_from_slice(&chunk[..n]);
        while let Some(end)=pending.iter().position(|b|*b==b'\n'){
            frames+=1;if frames>128||end>1024*1024{return Err(Error::Unsupported);}
            let bytes=pending.drain(..=end).collect::<Vec<_>>();let frame=std::str::from_utf8(&bytes[..end]).map_err(|_|Error::Invalid)?;
            let value=crate::providers::strict_json(frame)?;if value["jsonrpc"]!="2.0"{return Err(Error::Invalid);}
            if let Some(id)=value.get("id"){
                if !(id.as_u64().is_some()||id.as_str().is_some_and(|s|!s.is_empty()&&s.len()<=128&&!s.chars().any(char::is_control))){return Err(Error::Invalid);}
            }
            // SDK owns all MCP/JSON-RPC state. This decorator only enforces framing/IDs.
            output.write_all(frame.as_bytes()).await?;output.write_all(b"\n").await?;
        }
        if pending.len()>1024*1024{return Err(Error::Unsupported);}
    }
}
/// Called only in the already-restricted internal worker. It owns no core authority.
pub async fn driver()->Result<McpResult>{
    use rmcp::service::{ClientLifecycleMode,ClientServiceExt};
    let bytes=std::fs::read("/work/input/__rivet_mcp_request.json")?;if bytes.len()>1024*1024{return Err(Error::Unsupported);}
    let job:Job=serde_json::from_slice(&bytes).map_err(|_|Error::Invalid)?;
    let process=ProcessRequest{program:job.server.program,args:job.server.args,files:job.server.files,timeout_ms:15000};process.validate()?;
    let mut child=tokio::process::Command::new(&process.program).args(&process.args).env_clear().env("PATH","/usr/bin").env("TMPDIR","/work/tmp").env("PYTHONDONTWRITEBYTECODE","1").env("PYTHONNOUSERSITE","1")
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).kill_on_drop(true).spawn()?;
    let source=child.stdout.take().ok_or(Error::State)?;let input=child.stdin.take().ok_or(Error::State)?;
    let (filtered,writer)=tokio::io::duplex(32768);let filter=tokio::spawn(filter_frames(source,writer));
    let transport=rmcp::transport::async_rw::AsyncRwTransport::<rmcp::RoleClient,_,_>::new(filtered,input).with_max_line_length(1024*1024);
    let changed=Arc::new(AtomicBool::new(false));
    let result=async {
        // Fixed legacy handshake, no SDK auto-discover/fallback transport.
        let service=tokio::time::timeout(std::time::Duration::from_secs(3),Handler{changed:changed.clone()}.serve_with_lifecycle(transport,ClientLifecycleMode::Initialize)).await.map_err(|_|Error::Unsupported)?.map_err(|_|Error::Unsupported)?;
        let protocol=service.peer_info().ok_or(Error::State)?.protocol_version.to_string();if protocol!="2025-11-25"{return Err(Error::Unsupported);}
        let list=service.list_tools(None).await.map_err(|_|Error::State)?;if list.next_cursor.is_some()||list.tools.len()>32{return Err(Error::Unsupported);}
        let tools=list.tools.iter().map(serde_json::to_value).collect::<std::result::Result<Vec<_>,_>>().map_err(|_|Error::Invalid)?;
        let encoded=serde_json::to_vec(&tools).map_err(|_|Error::Invalid)?;if encoded.len()>16384{return Err(Error::Unsupported);}let digest=tools::sha256(&encoded);
        let result=match &job.action{
            Action::List{..}=>None,
            Action::Call{tool,arguments,expected_toolset,..}=>{
                if changed.load(Ordering::SeqCst)||expected_toolset!=&digest||!list.tools.iter().any(|t|t.name==tool.as_str()){return Err(Error::Conflict);}
                let params=rmcp::model::CallToolRequestParams::new(tool.clone()).with_arguments(arguments.as_object().ok_or(Error::Invalid)?.clone());
                // Application request payload is bounded before the SDK encodes/writes it.
                if serde_json::to_vec(&params).map_err(|_|Error::Invalid)?.len()>65536{return Err(Error::Unsupported);}
                let value=service.call_tool(params).await.map_err(|_|Error::State)?;
                if changed.load(Ordering::SeqCst){return Err(Error::Recovery);}
                Some(serde_json::to_value(value).map_err(|_|Error::State)?)
            }
        };
        service.cancel().await.map_err(|_|Error::State)?;
        let result=McpResult{protocol,toolset_digest:digest,tools,result};if serde_json::to_vec(&result).map_err(|_|Error::State)?.len()>32700{return Err(Error::Unsupported);}Ok(result)
    }.await;
    let _=child.kill().await;let _=child.wait().await;filter.abort();let _=filter.await;
    result
}
