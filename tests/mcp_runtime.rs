//! Synthetic stdio server inside the real qualified unit, never an ambient host process.
use rivet::{config::Project,domain::*,ledger::{Ledger,Transition},mcp::{Workspace,Server,Action,McpResult,DESCRIPTOR},permissions::Broker,supervisor::Supervisor,tools::ToolProposal};
use std::{path::Path,collections::BTreeMap,sync::Arc,time::{SystemTime,UNIX_EPOCH}};
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()}
fn server(id:&str,variant:&str)->Server{
    let script=r#"import json,sys,pathlib,socket
assert not pathlib.Path('/home').exists() and not pathlib.Path('/run').exists()
assert list(socket.if_nameindex())[0][1]=='lo'
MODE='VARIANT'
for line in sys.stdin:
 r=json.loads(line)
 if r.get('method')=='initialize':
  assert r['params']['protocolVersion']=='2025-11-25' and not r['params']['capabilities']
  if MODE=='flood': print('x'*(1024*1024+1),flush=True); continue
  result={'protocolVersion':'2025-11-25','capabilities':{'tools':{'listChanged':True}},'serverInfo':{'name':'synthetic','version':'1'}}
 elif r.get('method')=='tools/list':
  print(json.dumps({'jsonrpc':'2.0','id':'sampling-denied','method':'sampling/createMessage','params':{'messages':[],'maxTokens':1}}),flush=True)
  response=json.loads(sys.stdin.readline());assert response['id']=='sampling-denied' and 'error' in response
  result={'tools':[{'name':'echo','description':MODE,'inputSchema':{'type':'object','properties':{'text':{'type':'string'}},'required':['text'],'additionalProperties':False}}]}
 elif r.get('method')=='tools/call':
  assert r['params']['name']=='echo'
  result={'content':[{'type':'text','text':r['params']['arguments']['text']}],'isError':False}
 else: continue
 print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#.replace("VARIANT",variant);
    Server{id:id.into(),enabled:true,program:"/usr/bin/python3".into(),args:vec!["-B".into(),"server.py".into()],files:BTreeMap::from([("server.py".into(),script)])}
}
async fn execute(supervisor:&Supervisor,spec:&RunSpec,ledger:&Ledger,broker:&mut Broker,workspace:Arc<Workspace>,task:TaskId,actor:ActorId,action:Action)->rivet::error::Result<McpResult>{
    let proposal=ToolProposal{schema_version:1,operation:OperationId::default(),attempt:1,task,actor,descriptor_digest:DESCRIPTOR.into(),arguments:serde_json::to_value(&action).unwrap(),resources:workspace.resources(&action)?,preconditions:vec![],budget_reservation:"none".into()};let grant=broker.approve_mcp(&proposal,&proposal.resources,now()+60,now())?;supervisor.execute_mcp(spec,ledger,broker,workspace,proposal,grant).await
}
#[tokio::test]
#[ignore="requires explicitly authorized native user-systemd qualification"]
async fn brokered_mcp_lists_calls_refuses_sampling_schema_changes_and_floods(){
    let temp=tempfile::tempdir_in(Path::new(env!("RIVET_SOURCE_ROOT")).join(".build/tmp")).unwrap();let project=Project::open(temp.path()).unwrap();let ledger=Ledger::open(&project,Path::new("state")).unwrap();let task=TaskId::default();let actor=ActorId::default();
    let spec=RunSpec{schema_version:1,project:ledger.project_id(),session:SessionId::default(),run:RunId::default(),task:"MCP fixture".into(),mode:Mode::Solo,roles:vec![Role::Independent],provider:None,deadline:now()+300,step_budget:100,permission_epoch:ledger.epoch()};ledger.transition(0,Transition::NewSession{id:spec.session}).await.unwrap();ledger.transition(0,Transition::RootTask{session:spec.session,run:spec.run,task,actor}).await.unwrap();
    let supervisor=Supervisor::default();let mut broker=Broker::new(spec.permission_epoch);let workspace=Arc::new(Workspace::new(&project,vec![server("fixture","original")]).unwrap());
    let list=execute(&supervisor,&spec,&ledger,&mut broker,workspace.clone(),task,actor,Action::List{server:"fixture".into()}).await.unwrap();assert_eq!(list.tools.len(),1);assert_eq!(list.protocol,"2025-11-25");
    let action=Action::Call{server:"fixture".into(),tool:"echo".into(),arguments:serde_json::json!({"text":"fixture reply"}),expected_toolset:list.toolset_digest};let called=execute(&supervisor,&spec,&ledger,&mut broker,workspace,task,actor,action.clone()).await.unwrap();assert_eq!(called.result.unwrap()["content"][0]["text"],"fixture reply");
    let changed=Arc::new(Workspace::new(&project,vec![server("fixture","changed")]).unwrap());assert!(execute(&supervisor,&spec,&ledger,&mut broker,changed,task,actor,action).await.is_err());
    let flood=Arc::new(Workspace::new(&project,vec![server("fixture","flood")]).unwrap());assert!(execute(&supervisor,&spec,&ledger,&mut broker,flood,task,actor,Action::List{server:"fixture".into()}).await.is_err());
    let mut disabled=server("fixture","original");disabled.enabled=false;let workspace=Workspace::new(&project,vec![disabled]).unwrap();assert!(workspace.resources(&Action::List{server:"fixture".into()}).is_err());
    supervisor.shutdown().await;ledger.shutdown().unwrap();
}
