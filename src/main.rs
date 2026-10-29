use clap::Parser;
use rivet::{cli::{Cli, execute}, config::Project, error::{Error, Result}};
#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json = cli.json_output();
    let result: Result<()> = async {
        let project = Project::open(&std::env::current_dir()?)?;
        tokio::select! {
            biased;
            signal = tokio::signal::ctrl_c() => { signal.map_err(|_| Error::State)?; Err(Error::Interrupted) },
            result = execute(cli, &project) => result,
        }
    }.await;
    if let Err(error) = result {
        if json { eprintln!("{}", serde_json::json!({"schema_version":1,"error":{"kind":error.kind(),"message":error.to_string(),"exit_code":error.exit_code()}})); }
        else { eprintln!("{error}"); }
        std::process::exit(error.exit_code());
    }
}
