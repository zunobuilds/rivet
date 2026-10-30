use crate::{config::Project, domain::SessionId, error::Result, ledger::{self, Ledger, Transition}};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use serde_json::json;
#[derive(Parser)]
#[command(version, about = "Rivet P1a model-free administration")]
pub struct Cli { #[command(subcommand)] pub command: Command }
#[derive(Subcommand)]
pub enum Command {
    /// Check runtime-worker prerequisites without searching host paths or launching commands.
    Sandbox { #[command(subcommand)] command: SandboxCommand },
    Config { #[command(subcommand)] command: ConfigCommand },
    Status { #[arg(long)] state_dir: Option<PathBuf>, #[arg(long)] json: bool },
    State { #[command(subcommand)] command: StateCommand },
    Session { #[command(subcommand)] command: SessionCommand },
}
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
            output(json!({"schema_version":1,"state":inspection.state,"sqlite_version":inspection.sqlite_version,"provider":config.provider_status(),"adapters_available":false,"execution_available":false}), json);
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
