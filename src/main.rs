use clap::Parser;
use rivet::{cli::{Cli, execute}, config::Project, error::{Error, Result}};
fn main() {
    let args:Vec<String>=std::env::args().collect();
    if args.get(1).is_some_and(|a|a=="__unit-launch") {
        let result=if args.len()>6 && args[5]=="--"{rivet::tools::process::unit_launch(&args[2],&args[3],&args[4],&args[6..])}else{Err(Error::Invalid)};
        if let Err(error)=result{eprintln!("{error}");std::process::exit(error.exit_code());}
        return;
    }
    if args.get(1).is_some_and(|a|a=="__worker-bootstrap") {
        let input=if args.get(2).is_some_and(|a|a=="--"){&args[3..]}else{&args[2..]};
        if let Err(error)=rivet::tools::process::bootstrap(input){eprintln!("{error}");std::process::exit(error.exit_code());}
        return;
    }
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("runtime").block_on(administration());
}
async fn administration() {
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
