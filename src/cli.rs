use crate::{config::Project, domain::{SessionId,OperationId,RequestId}, error::Result, ledger::{self, Ledger, Transition}};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use serde_json::json;
#[derive(Parser)]
#[command(version=concat!(env!("CARGO_PKG_VERSION"), " build ", env!("RIVET_BUILD_DIGEST")), about = "Rivet administration and reviewed Solo coding")]
pub struct Cli { #[command(subcommand)] pub command: Command }
#[derive(Subcommand)]
pub enum Command {
    /// Inspect recovery evidence or create an exact-snapshot consistent backup.
    Repair {#[command(subcommand)] command:RepairCommand},
    /// Durable status, controls, exact review approval and explicit bounded continuation.
    Run {#[command(subcommand)] command:RunCommand},
    /// A provider/tool request executes only after exact interactive review.
    Chat {#[arg(long)] task:String, #[arg(long)] config:Option<PathBuf>, #[arg(long)] state_dir:Option<PathBuf>, #[arg(long)] session:Option<String>, #[arg(long,default_value_t=100)] steps:u32, #[arg(long,default_value_t=3600)] deadline_seconds:u64, #[arg(long,default_value_t=65536)] token_budget:u64, #[arg(long,default_value_t=0)] spend_ceiling_micros:u64, #[arg(long,default_value_t=0)] request_cost_micros:u64, #[arg(long)] ephemeral_key_stdin:bool,#[arg(long)] detach:bool,#[arg(long,hide=true)] detached_core:bool},
    /// Review/create the source-only Rust binary recipe; no build/install is run.
    Project {#[command(subcommand)] command:ProjectCommand},
    /// Check runtime-worker prerequisites without searching host paths or launching commands.
    Sandbox { #[command(subcommand)] command: SandboxCommand },
    Config { #[command(subcommand)] command: ConfigCommand },
    Status { #[arg(long)] state_dir: Option<PathBuf>, #[arg(long)] json: bool },
    State { #[command(subcommand)] command: StateCommand },
    Session { #[command(subcommand)] command: SessionCommand },
}
#[derive(Subcommand)]
pub enum RunCommand {
    Forward{run:uuid::Uuid,#[arg(long)]recipient:uuid::Uuid,#[arg(long)]scope:String,#[arg(long)]decision:String,#[arg(long)]approve:Option<String>,#[arg(long)]state_dir:Option<PathBuf>},
    Template{#[arg(long)]mode:ModeArg,#[arg(long)]task:String,#[arg(long)]paths:Vec<PathBuf>,#[arg(long)]minimal:bool},
    Hierarchy{#[arg(long)]plan:PathBuf,#[arg(long)]approve:Option<String>,#[arg(long)]config:Option<PathBuf>,#[arg(long)]state_dir:Option<PathBuf>,#[arg(long,default_value_t=100)]steps:u32,#[arg(long,default_value_t=3600)]deadline_seconds:u64,#[arg(long,default_value_t=65536)]token_budget:u64,#[arg(long,default_value_t=0)]spend_ceiling_micros:u64,#[arg(long,default_value_t=0)]request_cost_micros:u64,#[arg(long)]ephemeral_key_stdin:bool,#[arg(long)]detach:bool,#[arg(long,hide=true)]detached_core:bool,#[arg(long)]minimal:bool},
    Status{run:uuid::Uuid,#[arg(long)]state_dir:Option<PathBuf>},
    Pause{run:uuid::Uuid,#[arg(long)]state_dir:Option<PathBuf>},
    Stop{run:uuid::Uuid,#[arg(long)]state_dir:Option<PathBuf>},
    Disconnect{run:uuid::Uuid,#[arg(long)]state_dir:Option<PathBuf>},
    Approve{run:uuid::Uuid,challenge:String,#[arg(long)]state_dir:Option<PathBuf>},
    Resume{run:uuid::Uuid,#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]config:Option<PathBuf>,#[arg(long)]approve:Option<String>,#[arg(long,default_value_t=0)]request_cost_micros:u64,#[arg(long)]ephemeral_key_stdin:bool,#[arg(long)]detach:bool,#[arg(long,hide=true)]detached_core:bool},
}
#[derive(Clone,clap::ValueEnum)]
pub enum ModeArg {Solo,Pair,Crew,Fleet}
impl ModeArg{fn mode(&self)->crate::domain::Mode{match self{Self::Solo=>crate::domain::Mode::Solo,Self::Pair=>crate::domain::Mode::Pair,Self::Crew=>crate::domain::Mode::Crew,Self::Fleet=>crate::domain::Mode::Fleet}}}
#[derive(Subcommand)]
pub enum RepairCommand {
    /// Review an uncertain operation; committing requires its exact observation digest.
    Reconcile{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]operation:uuid::Uuid,#[arg(long)]decision:Option<ReconcileAction>,#[arg(long)]approve:Option<String>},
    /// Acknowledge an unknown inference request without refunding or resending it.
    Acknowledge{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]request:uuid::Uuid,#[arg(long)]approve:String},
    Restore{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]backup:String,#[arg(long)]approve:Option<String>},Recover{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]approve:String},Inspect{#[arg(long)]state_dir:Option<PathBuf>},Backup{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]approve:String}}
#[derive(Clone,clap::ValueEnum)]
pub enum ReconcileAction {AcceptObservedPost,AbandonUnknown}
#[derive(Subcommand)]
pub enum ProjectCommand { New {name:String, #[arg(long)] approve:Option<String>} }
#[derive(Subcommand)]
pub enum SandboxCommand { Check { #[arg(long)] control_dir: Option<PathBuf>, #[arg(long)] json: bool } }
#[derive(Subcommand)]
pub enum ConfigCommand { Check { #[arg(long)] config: Option<PathBuf>, #[arg(long)] json: bool } }
#[derive(Subcommand)]
pub enum StateCommand { Init { #[arg(long)] state_dir: Option<PathBuf> } }
#[derive(Subcommand)]
pub enum SessionCommand {
    New { #[arg(long)] state_dir: Option<PathBuf> },
    List { #[arg(long)] state_dir: Option<PathBuf>, #[arg(long)] json: bool },
}
fn state(path: &Option<PathBuf>) -> &Path { path.as_deref().unwrap_or(Path::new(".rivet-state")) }
fn output(value: serde_json::Value, structured: bool) {
    if structured { println!("{value}"); } else { println!("{}", serde_json::to_string_pretty(&value).expect("JSON value serialization")); }
}
pub async fn execute(cli: Cli, project: &Project) -> Result<()> {execute_controlled(cli,project,tokio_util::sync::CancellationToken::new()).await}
pub async fn execute_controlled(cli:Cli,project:&Project,cancel:tokio_util::sync::CancellationToken)->Result<()> {
    match cli.command {
        Command::Run{command}=>{
            use crate::ledger::control::{self,Action};use crate::domain::RunId;
            match command{
                RunCommand::Forward{run,recipient,scope,decision,approve,state_dir}=>{
                    let run=RunId(run);let recipient=crate::domain::ActorId(recipient);let status=control::request(project,state(&state_dir),run,Action::Status).await?;let epoch=status["snapshot"]["spec"]["permission_epoch"].as_u64().ok_or(crate::error::Error::State)?;let digest=control::forward_digest(run,recipient,&scope,&decision,epoch)?;
                    if let Some(approved)=approve{if approved!=digest{return Err(crate::error::Error::Denied);}output(control::request(project,state(&state_dir),run,Action::Forward{recipient,scope,text:decision,epoch,approved}).await?,true);}else{output(json!({"run":run,"recipient":recipient,"scope":scope,"decision":decision,"epoch":epoch,"review_digest":digest,"forwarded":false}),true);}
                },
                RunCommand::Template{mode,task,paths,minimal}=>{let plan=crate::scheduler::template(mode.mode(),&task,paths)?;crate::domain::validate_roles(&plan.mode,&plan.agents.iter().map(|a|a.role.clone()).collect::<Vec<_>>(),minimal)?;output(serde_json::to_value(plan).map_err(|_|crate::error::Error::State)?,true);},
                RunCommand::Hierarchy{plan,approve,config,state_dir,steps,deadline_seconds,token_budget,spend_ceiling_micros,request_cost_micros,ephemeral_key_stdin,detach,detached_core,minimal}=>{
                    if detached_core{control::verify_detached()?;}
                    let plan:crate::scheduler::Plan=serde_json::from_slice(&project.input(&plan)?).map_err(|_|crate::error::Error::Invalid)?;plan.validate()?;crate::domain::validate_roles(&plan.mode,&plan.agents.iter().map(|a|a.role.clone()).collect::<Vec<_>>(),minimal)?;let digest=crate::hierarchy::plan_digest(&plan)?;
                    let Some(approve)=approve else{output(json!({"plan":plan,"review_digest":digest,"executed":false}),true);return Ok(());};if approve!=digest{return Err(crate::error::Error::Denied);}
                    if detach&&!detached_core{let arguments=std::env::args().skip(1).collect::<Vec<_>>();output(control::detach(project,state(&state_dir),&arguments).await?,true);return Ok(());}
                    let configuration=project.config(config.as_deref())?;let provider=configuration.provider.ok_or(crate::error::Error::Unsupported)?;if ephemeral_key_stdin&&provider.profile!="anthropic"{return Err(crate::error::Error::Invalid);}let credential=if ephemeral_key_stdin{Some(crate::terminal::read_secret().await?)}else{None};let mut terminal=crate::terminal::Terminal::new();
                    let report=crate::hierarchy::run_controlled(project,state(&state_dir),provider,crate::chat::ChatOptions{session:None,task:plan.root().description.clone(),steps,deadline_seconds,budget:crate::budget::BudgetLimits{tokens:token_budget,requests:steps,cost_micros:spend_ceiling_micros},request_cost_micros,credential,mcp:configuration.mcp,skills:configuration.skills},Some(plan),Some(approve),&mut terminal,cancel,crate::chat::Controls{resume:None,detached:detach,external_reviews:detached_core}).await?;output(serde_json::to_value(report).map_err(|_|crate::error::Error::State)?,true);
                },
                RunCommand::Status{run,state_dir}=>{
                    let run=RunId(run);match crate::lifecycle::inspect(project,state(&state_dir),run){Ok(snapshot)=>{let graph=if snapshot.coordination_digest.is_some(){Some(crate::ledger::graph_inspect(project,state(&state_dir),run)?)}else{None};output(json!({"snapshot":snapshot,"coordination":graph,"resume_digest":snapshot.digest()?}),true)},Err(crate::error::Error::Busy)=>output(control::request(project,state(&state_dir),run,Action::Status).await?,true),Err(error)=>return Err(error)}
                },
                RunCommand::Stop{run,state_dir}=>{
                    let run=RunId(run);match crate::lifecycle::inspect(project,state(&state_dir),run){
                        Ok(_)=>{let wall=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_|crate::error::Error::State)?.as_secs();output(serde_json::to_value(crate::lifecycle::stop(project,state(&state_dir),run,wall).await?).map_err(|_|crate::error::Error::State)?,true);},
                        Err(crate::error::Error::Busy)=>output(control::request(project,state(&state_dir),run,Action::Stop).await?,true),Err(e)=>return Err(e)
                    }
                },
                RunCommand::Resume{run,state_dir,config,approve,request_cost_micros,ephemeral_key_stdin,detach,detached_core}=>{
                    if detached_core{control::verify_detached()?;}
                    let run=RunId(run);let snapshot=crate::lifecycle::inspect(project,state(&state_dir),run)?;
                    let Some(approved)=approve else{output(json!({"snapshot":snapshot,"resume_digest":snapshot.digest()?,"requires_fresh_reviews":true}),true);return Ok(());};
                    if detach&&!detached_core{let arguments=std::env::args().skip(1).collect::<Vec<_>>();output(control::detach(project,state(&state_dir),&arguments).await?,true);return Ok(());}
                    let configuration=project.config(config.as_deref())?;let provider=configuration.provider.ok_or(crate::error::Error::Unsupported)?;
                    if ephemeral_key_stdin&&provider.profile!="anthropic"{return Err(crate::error::Error::Invalid);}
                    let credential=if ephemeral_key_stdin{Some(crate::terminal::read_secret().await?)}else{None};
                    let mut terminal=crate::terminal::Terminal::new();
                    let graph=snapshot.coordination_digest.is_some();let options=crate::chat::ChatOptions{session:Some(snapshot.spec.session),task:snapshot.spec.task,steps:100,deadline_seconds:3600,budget:crate::budget::BudgetLimits{tokens:65536,requests:100,cost_micros:u64::from(request_cost_micros>0)},request_cost_micros,credential,mcp:configuration.mcp,skills:configuration.skills};let controls=crate::chat::Controls{resume:Some((run,approved)),detached:detach,external_reviews:detached_core};
                    let report=if graph{crate::hierarchy::run_controlled(project,state(&state_dir),provider,options,None,None,&mut terminal,cancel,controls).await?}else{crate::chat::run_controlled(project,state(&state_dir),provider,options,&mut terminal,cancel,controls).await?};
                    output(serde_json::to_value(report).map_err(|_|crate::error::Error::State)?,true);
                },
                other=>{let (run,state_dir,action)=match other{RunCommand::Pause{run,state_dir}=>(run,state_dir,Action::Pause),RunCommand::Disconnect{run,state_dir}=>(run,state_dir,Action::Disconnect),RunCommand::Approve{run,state_dir,challenge}=>(run,state_dir,Action::Approve{challenge}),_=>unreachable!()};output(control::request(project,state(&state_dir),RunId(run),action).await?,true);}
            }
        },
        Command::Repair{command:RepairCommand::Reconcile{state_dir,operation,decision,approve}}=>{
            let id=OperationId(operation);
            match (decision,approve){
                (Some(decision),Some(digest))=>{let decision=match decision{ReconcileAction::AcceptObservedPost=>ledger::ReconcileDecision::AcceptObservedPost,ReconcileAction::AbandonUnknown=>ledger::ReconcileDecision::AbandonUnknown};ledger::reconcile_operation(project,state(&state_dir),id,decision,&digest).await?;output(json!({"operation":id,"reconciled":true,"replayed":false}),true);},
                (None,None)=>{let review=ledger::preview_reconcile(project,state(&state_dir),id)?;output(json!({"review_digest":review.digest()?,"review":review}),true);},
                _=>return Err(crate::error::Error::Invalid)
            }
        },
        Command::Repair{command:RepairCommand::Acknowledge{state_dir,request,approve}}=>{ledger::acknowledge_request(project,state(&state_dir),RequestId(request),&approve)?;output(json!({"request":request,"acknowledged":true,"refunded":false,"resent":false}),true);},
        Command::Repair{command:RepairCommand::Restore{state_dir,backup,approve}}=>{
            if let Some(approved)=approve{output(serde_json::to_value(ledger::restore_state(project,state(&state_dir),&backup,&approved).await?).map_err(|_|crate::error::Error::State)?,true);}
            else{let review=ledger::preview_restore(project,state(&state_dir),&backup)?;output(json!({"review_digest":review.digest()?,"review":review}),true);}
        },
        Command::Repair{command:RepairCommand::Recover{state_dir,approve}}=>output(serde_json::to_value(ledger::recover_state(project,state(&state_dir),&approve).await?).map_err(|_|crate::error::Error::State)?,true),
        Command::Repair{command:RepairCommand::Inspect{state_dir}}=>output(serde_json::to_value(ledger::repair_inspect(project,state(&state_dir))?).map_err(|_|crate::error::Error::State)?,true),
        Command::Repair{command:RepairCommand::Backup{state_dir,approve}}=>output(serde_json::to_value(ledger::backup_state(project,state(&state_dir),&approve)?).map_err(|_|crate::error::Error::State)?,true),
        Command::Chat{task,config,state_dir,session,steps,deadline_seconds,token_budget,spend_ceiling_micros,request_cost_micros,ephemeral_key_stdin,detach,detached_core}=>{
            if detached_core{crate::ledger::control::verify_detached()?;}
            if detach&&!detached_core{let arguments=std::env::args().skip(1).collect::<Vec<_>>();output(crate::ledger::control::detach(project,state(&state_dir),&arguments).await?,true);return Ok(());}
            let configuration=project.config(config.as_deref())?;
            let provider=configuration.provider.ok_or(crate::error::Error::Unsupported)?;
            let session=match session{Some(id)=>Some(SessionId(uuid::Uuid::parse_str(&id).map_err(|_|crate::error::Error::Invalid)?)),None=>None};
            if ephemeral_key_stdin&&provider.profile!="anthropic"{return Err(crate::error::Error::Invalid);}
            let credential=if ephemeral_key_stdin{Some(crate::terminal::read_secret().await?)}else{None};
            let mut terminal=crate::terminal::Terminal::new();
            let report=crate::chat::run_controlled(project,state(&state_dir),provider,crate::chat::ChatOptions{session,task,steps,deadline_seconds,budget:crate::budget::BudgetLimits{tokens:token_budget,requests:steps,cost_micros:spend_ceiling_micros},request_cost_micros,credential,mcp:configuration.mcp,skills:configuration.skills},&mut terminal,cancel,crate::chat::Controls{resume:None,detached:detach,external_reviews:detached_core}).await?;
            output(serde_json::to_value(report).map_err(|_|crate::error::Error::State)?,true);
        },
        Command::Project{command:ProjectCommand::New{name,approve}}=>{
            let recipe=crate::scaffold::recipe(&name)?;let digest=crate::scaffold::digest(&recipe)?;
            if let Some(reviewed)=approve{crate::scaffold::create(project,&name,&reviewed)?;output(json!({"created":name,"recipe_digest":digest,"executed":false}),true);}
            else{output(json!({"recipe":"rust-bin","name":name,"files":recipe,"review_digest":digest,"created":false,"executed":false}),true);}
        },
        Command::Sandbox { command: SandboxCommand::Check { control_dir, json } } => {
            let result = crate::sandbox::preflight(project, control_dir.as_deref())?;
            let ready = result.ready;
            output(serde_json::to_value(result).expect("preflight serialization"), json);
            if !ready { return Err(crate::error::Error::Unsupported); }
        },
        Command::Config { command: ConfigCommand::Check { config, json } } => {
            let config = project.config(config.as_deref())?;
            output(json!({"schema_version":1,"valid":true,"provider":config.provider_status(),"execution_available":false}), json);
        },
        Command::Status { state_dir, json } => {
            let config = project.config(None)?;
            let inspection = ledger::inspect(project, state(&state_dir))?;
            output(json!({"schema_version":1,"state":inspection.state,"sqlite_version":inspection.sqlite_version,"provider":config.provider_status(),"adapters_available":true,"execution_available":false}), json);
        },
        Command::State { command: StateCommand::Init { state_dir } } => {
            project.config(None)?;
            Ledger::open(project, state(&state_dir))?.shutdown()?;
            output(json!({"schema_version":1,"state":"initialized"}), true);
        },
        Command::Session { command: SessionCommand::New { state_dir } } => {
            project.config(None)?;
            let ledger = Ledger::open(project, state(&state_dir))?;
            let id = SessionId::default();
            ledger.transition(0, Transition::NewSession { id }).await?;
            ledger.shutdown()?;
            output(json!({"schema_version":1,"session_id":id}), true);
        },
        Command::Session { command: SessionCommand::List { state_dir, json } } => {
            output(serde_json::to_value(ledger::inspect(project, state(&state_dir))?).expect("session serialization"), json);
        },
    }
    Ok(())
}

impl Cli {
    pub fn json_output(&self) -> bool {
        match &self.command {
            Command::Status { json, .. } => *json,
            Command::Sandbox { command: SandboxCommand::Check { json, .. } } => *json,
            Command::Config { command: ConfigCommand::Check { json, .. } } => *json,
            Command::Session { command: SessionCommand::List { json, .. } } => *json,
            _ => false,
        }
    }
}
