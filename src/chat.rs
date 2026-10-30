//! Reviewed, bounded Solo coding loop. Providers propose; only the supervisor dispatches.
use crate::{budget::BudgetLimits, config::{Project,ProviderConfig}, context::Context, domain::*, error::{Error,Result}, ledger::{Ledger,Transition}, permissions::Broker, providers::{*,compatible::Compatible}, supervisor::Supervisor, tools::{files::{FileAction,FileWorkspace},process::ProcessRequest,git::{GitRequest,GitWorkspace}}};
use serde::Serialize;
use std::{future::Future,pin::Pin,path::Path,sync::Arc,time::{SystemTime,UNIX_EPOCH}};
use tokio_util::sync::CancellationToken;
#[derive(Clone,Serialize)]
pub struct Review {pub kind:&'static str,pub scope:serde_json::Value}
pub trait Reviewer {
    fn approve(&mut self,review:Review)->Pin<Box<dyn Future<Output=Result<bool>>+Send+'_>>;
}
#[derive(Debug,Serialize)]
pub struct ChatReport {pub session:SessionId,pub run:RunId,pub steps:u32,pub result:String,pub revisions:Vec<String>,pub checks:Vec<String>}
pub struct ChatOptions {pub session:Option<SessionId>,pub task:String,pub steps:u32,pub deadline_seconds:u64,pub budget:BudgetLimits,pub request_cost_micros:u64}
fn now()->Result<u64>{SystemTime::now().duration_since(UNIX_EPOCH).map(|t|t.as_secs()).map_err(|_|Error::State)}
pub async fn run(project:&Project,state:&Path,config:ProviderConfig,options:ChatOptions,reviewer:&mut impl Reviewer,cancel:CancellationToken)->Result<ChatReport> {
    // Endpoint validation and provider configuration cannot initiate traffic.
    let endpoint=compatible::endpoint(&config.endpoint)?;
    if endpoint.scheme()=="https" && (options.request_cost_micros==0 || options.budget.cost_micros==0){return Err(Error::Denied);}
    if config.profile!="llama.cpp-compatible"{return Err(Error::Unsupported);}
    options.budget.validate()?;
    if options.steps==0||options.steps>100||options.deadline_seconds==0||options.deadline_seconds>3600{return Err(Error::Invalid);}
    let evidence=match options.session{Some(session)=>crate::ledger::continuation(project,state,session)?,None=>String::new()};
    let mut context=Context::new(&options.task,&evidence)?;
    let ledger=Ledger::open(project,state)?;
    let session=options.session.unwrap_or_default();
    if options.session.is_none(){ledger.transition(0,Transition::NewSession{id:session}).await?;}
    let spec=RunSpec{schema_version:1,project:ledger.project_id(),session,run:RunId::default(),task:options.task,mode:Mode::Solo,roles:vec![Role::Independent],provider:Some(config.profile.clone()),deadline:now()?.checked_add(options.deadline_seconds).ok_or(Error::Invalid)?,step_budget:options.steps,permission_epoch:ledger.epoch()};
    let task=TaskId::default();let actor=ActorId::default();
    ledger.transition(0,Transition::RootTask{session,run:spec.run,task,actor}).await?;
    ledger.init_budget(spec.run,options.budget).await?;
    let files=Arc::new(FileWorkspace::new(Project::open(project.path())?,state)?);
    let mut broker=Broker::new(spec.permission_epoch);let supervisor=Supervisor::default();
    let tools=vec![FileWorkspace::descriptor(),ProcessRequest::descriptor(),GitRequest::descriptor()];
    let mut report=ChatReport{session,run:spec.run,steps:0,result:String::new(),revisions:vec![],checks:vec![]};
    let result={
    let work=async {
        loop {
            if cancel.is_cancelled(){return Err(Error::Interrupted);}
            if now()?>=spec.deadline||report.steps>=spec.step_budget{return Err(Error::Denied);}
            report.steps+=1;
            let id=RequestId::default();
            let request=InferRequest{schema_version:1,id,prompt:String::new(),context:Some(RequestContext{messages:context.messages(),tools:tools.clone(),task,actor,max_output:2048,max_cost_micros:options.request_cost_micros,reservation:id.0.to_string()})};
            let digest=request.digest()?;
            if !approve(reviewer,&cancel,spec.deadline,Review{kind:"inference",scope:serde_json::json!({"endpoint":config.endpoint,"model":config.model,"profile":config.profile,"request_digest":digest,"request":request})}).await?{return Err(Error::Denied);}
            let timestamp=now()?;if timestamp>=spec.deadline{return Err(Error::Denied);}
            // Conservative byte-based admission estimate, not a tokenizer/price claim.
            let tokens=(serde_json::to_vec(&request).map_err(|_|Error::Invalid)?.len() as u64).checked_mul(2).and_then(|v|v.checked_add(3072)).ok_or(Error::Invalid)?;
            let permit=broker.approve_inference(&config,&request,&digest,spec.deadline.min(timestamp+60),timestamp)?;
            ledger.reserve_inference(spec.run,id,tokens,options.request_cost_micros,digest,broker.epoch()).await?;
            let provider=Compatible::new(config.clone(),permit)?;
            let mut events=provider.infer(request,cancel.child_token());
            let mut text=String::new();let mut proposals=vec![];let mut usage=None;let mut finished=false;let mut failed=false;
            while let Some(event)=tokio::select!{biased;_=cancel.cancelled()=>{failed=true;None},_=ledger.cancellation().cancelled_owned()=>{failed=true;None},event=events.recv()=>event}{
                match event {
                    ProviderEvent::TextDelta{request,text:delta} if request==id=>{text.push_str(&delta);if text.len()>65536{failed=true;break;}},
                    ProviderEvent::ToolCallComplete{request,proposal,wire_id:Some(wire_id)} if request==id=>{if proposals.len()>=8{failed=true;break;}proposals.push((wire_id,proposal));},
                    ProviderEvent::ToolCallDelta{request,..} if request==id=>(),
                    ProviderEvent::Usage{request,input,output} if request==id&&usage.is_none()=>usage=Some((input,output)),
                    ProviderEvent::Finish{request,..} if request==id=>{finished=true;break;},
                    _=>{failed=true;break;}
                }
            }
            ledger.settle_inference(id,if finished&&!failed{usage}else{None}).await?;
            if !finished||failed||usage.is_none()||usage.is_some_and(|(i,o)|i.checked_add(o).is_none_or(|n|n>tokens)){return Err(Error::Recovery);}
            if proposals.is_empty(){report.result=text;return Ok(());}
            if report.steps+proposals.len() as u32>spec.step_budget{return Err(Error::Denied);}
            let calls=proposals.iter().map(|(id,p)|{
                let descriptor=tools.iter().find(|d|d.build_digest==p.descriptor_digest).ok_or(Error::Denied)?;
                Ok(ChatCall{id:id.clone(),name:format!("{}_v{}",descriptor.id.replace([':', '.', '-'],"_"),descriptor.version),arguments:serde_json::to_string(&p.arguments).map_err(|_|Error::Invalid)?})
            }).collect::<Result<Vec<_>>>()?;
            context.push(ChatMessage{role:MessageRole::Assistant,content:text,tool_call_id:None,tool_calls:calls})?;
            for (wire_id,mut proposal) in proposals {
                if cancel.is_cancelled()||now()?>=spec.deadline{return Err(Error::Interrupted);}
                if proposal.task!=task||proposal.actor!=actor||proposal.attempt!=1{return Err(Error::Denied);}
                report.steps+=1;
                enum Resolved {File(FileAction),Process(ProcessRequest),Git(GitRequest)}
                let resolved=if proposal.descriptor_digest==crate::tools::files::DESCRIPTOR{
                    let action:FileAction=serde_json::from_value(proposal.arguments.clone()).map_err(|_|Error::Invalid)?;proposal.resources=files.resources(&action)?;Resolved::File(action)
                }else if proposal.descriptor_digest==crate::tools::process::DESCRIPTOR{
                    let request:ProcessRequest=serde_json::from_value(proposal.arguments.clone()).map_err(|_|Error::Invalid)?;request.validate()?;proposal.resources=request.resources(spec.project);Resolved::Process(request)
                }else if proposal.descriptor_digest==crate::tools::git::DESCRIPTOR{
                    let request:GitRequest=serde_json::from_value(proposal.arguments.clone()).map_err(|_|Error::Invalid)?;request.validate()?;proposal.resources=request.resources(spec.project);Resolved::Git(request)
                }else{return Err(Error::Unsupported);};
                if !approve(reviewer,&cancel,spec.deadline,Review{kind:"tool",scope:serde_json::to_value(&proposal).map_err(|_|Error::Invalid)?}).await?{
                    ledger.transition(0,Transition::Propose{run:spec.run,proposal:proposal.clone()}).await?;
                    ledger.transition(0,Transition::Deny{operation:proposal.operation}).await?;return Err(Error::Denied);
                }
                let timestamp=now()?;let expiry=spec.deadline.min(timestamp+60);
                let output=match resolved {
                    Resolved::File(_action)=>{
                        let grant=broker.approve_file(&proposal,&proposal.resources,expiry,timestamp)?;
                        match supervisor.execute_file(&spec,&ledger,&mut broker,files.clone(),proposal,grant).await{
                            Ok(result)=>{if let Some(revision)=&result.revision{report.revisions.push(revision.digest.clone());}serde_json::to_string(&result).map_err(|_|Error::State)?},
                            Err(Error::State|Error::Interrupted|Error::Recovery)=>return Err(Error::Recovery),
                            Err(error)=>serde_json::json!({"error":error.kind(),"success":false}).to_string(),
                        }
                    },
                    Resolved::Process(_request)=>{
                        let grant=broker.approve_process(&proposal,&proposal.resources,expiry,timestamp)?;
                        let result=supervisor.execute_process(&spec,&ledger,&mut broker,proposal,grant).await?;
                        report.checks.push(format!("projection_sha256={};exit={:?};stdout_sha256={};truncated={}",result.projection_digest,result.exit_code,crate::tools::sha256(result.stdout.as_bytes()),result.truncated));
                        serde_json::to_string(&result).map_err(|_|Error::State)?
                    },
                    Resolved::Git(_request)=>{
                        let workspace=Arc::new(GitWorkspace::new(Project::open(project.path())?)?);
                        let grant=broker.approve_git(&proposal,&proposal.resources,expiry,timestamp)?;
                        let result=supervisor.execute_git(&spec,&ledger,&mut broker,workspace,proposal,grant).await?;
                        if let Some(commit)=&result.commit{report.revisions.push(commit.clone());}serde_json::to_string(&result).map_err(|_|Error::State)?
                    }
                };
                context.push(ChatMessage{role:MessageRole::Tool,content:output,tool_call_id:Some(wire_id),tool_calls:vec![]})?;
            }
        }
    };
    tokio::pin!(work);
    tokio::select!{biased;
        _=cancel.cancelled()=>{supervisor.cancellation().cancel();work.await},
        result=&mut work=>result,
    }
    };
    supervisor.shutdown().await;
    ledger.transition(0,Transition::CompleteRun{run:spec.run,state:if result.is_ok(){"completed".into()}else{"stopped".into()}}).await?;
    ledger.shutdown()?;result?;Ok(report)
}

async fn approve(reviewer:&mut impl Reviewer,cancel:&CancellationToken,deadline:u64,review:Review)->Result<bool>{
    let remaining=deadline.saturating_sub(now()?);
    tokio::select!{biased;_=cancel.cancelled()=>Err(Error::Interrupted),_=tokio::time::sleep(std::time::Duration::from_secs(remaining))=>Err(Error::Denied),result=reviewer.approve(review)=>result}
}
