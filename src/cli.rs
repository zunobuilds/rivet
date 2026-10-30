use crate::{config::Project, domain::SessionId, error::Result, ledger::{self, Ledger, Transition}};
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
    /// A provider/tool request executes only after exact interactive review.
    Chat {#[arg(long)] task:String, #[arg(long)] config:Option<PathBuf>, #[arg(long)] state_dir:Option<PathBuf>, #[arg(long)] session:Option<String>, #[arg(long,default_value_t=100)] steps:u32, #[arg(long,default_value_t=3600)] deadline_seconds:u64, #[arg(long,default_value_t=65536)] token_budget:u64, #[arg(long,default_value_t=0)] spend_ceiling_micros:u64, #[arg(long,default_value_t=0)] request_cost_micros:u64, #[arg(long)] ephemeral_key_stdin:bool},
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
pub enum RepairCommand {Recover{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]approve:String},Inspect{#[arg(long)]state_dir:Option<PathBuf>},Backup{#[arg(long)]state_dir:Option<PathBuf>,#[arg(long)]approve:String}}
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
pub async fn execute(cli: Cli, project: &Project) -> Result<()> {
    match cli.command {
        Command::Repair{command:RepairCommand::Recover{state_dir,approve}}=>output(serde_json::to_value(ledger::recover_state(project,state(&state_dir),&approve).await?).map_err(|_|crate::error::Error::State)?,true),
        Command::Repair{command:RepairCommand::Inspect{state_dir}}=>output(serde_json::to_value(ledger::repair_inspect(project,state(&state_dir))?).map_err(|_|crate::error::Error::State)?,true),
        Command::Repair{command:RepairCommand::Backup{state_dir,approve}}=>output(serde_json::to_value(ledger::backup_state(project,state(&state_dir),&approve)?).map_err(|_|crate::error::Error::State)?,true),
        Command::Chat{task,config,state_dir,session,steps,deadline_seconds,token_budget,spend_ceiling_micros,request_cost_micros,ephemeral_key_stdin}=>{
            let configuration=project.config(config.as_deref())?;
            let provider=configuration.provider.ok_or(crate::error::Error::Unsupported)?;
            let session=match session{Some(id)=>Some(SessionId(uuid::Uuid::parse_str(&id).map_err(|_|crate::error::Error::Invalid)?)),None=>None};
            if ephemeral_key_stdin&&provider.profile!="anthropic"{return Err(crate::error::Error::Invalid);}
            let credential=if ephemeral_key_stdin{Some(crate::terminal::read_secret().await?)}else{None};
            let mut terminal=crate::terminal::Terminal::new();
            let report=crate::chat::run(project,state(&state_dir),provider,crate::chat::ChatOptions{session,task,steps,deadline_seconds,budget:crate::budget::BudgetLimits{tokens:token_budget,requests:steps,cost_micros:spend_ceiling_micros},request_cost_micros,credential,mcp:configuration.mcp,skills:configuration.skills},&mut terminal,tokio_util::sync::CancellationToken::new()).await?;
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
