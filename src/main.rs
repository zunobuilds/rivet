use clap::Parser;
use rivet::{cli::{Cli, execute}, config::Project, error::{Error, Result}};
fn main() {
    let args:Vec<String>=std::env::args().collect();
    if args.get(1).is_some_and(|a|a=="__unit-launch") {
        let result=if args.len()>9 && args[2]==env!("RIVET_BUILD_DIGEST") && args[8]=="--"{rivet::tools::process::unit_launch(&args[3],&args[4],&args[5],&args[6],&args[7],&args[9..])}else{Err(Error::Invalid)};
        if let Err(error)=result{eprintln!("{error}");std::process::exit(error.exit_code());}
        return;
    }
    if args.get(1).is_some_and(|a|a=="__worker-bootstrap") {
        if args.get(2).map(String::as_str)!=Some(env!("RIVET_BUILD_DIGEST")){eprintln!("worker build mismatch");std::process::exit(4);}
        let git=match args.get(3).map(String::as_str){Some("--git-ro")=>Some(false),Some("--git-rw")=>Some(true),_=>None};
        let mcp=args.get(3).is_some_and(|a|a=="--mcp");
        let offset=if git.is_some()||mcp{5}else{4};
        let input=if args.get(offset-1).is_some_and(|a|a=="--"){&args[offset..]}else{&args[2..]};
        if let Err(error)=rivet::tools::process::bootstrap(input,git,mcp){eprintln!("{error}");std::process::exit(error.exit_code());}
        return;
    }
    if args.get(1).is_some_and(|a|a=="__mcp-driver"){
        let result=tokio::runtime::Builder::new_current_thread().enable_all().build().expect("worker runtime").block_on(rivet::mcp::driver());
        match result{Ok(result)=>println!("{}",serde_json::to_string(&result).expect("MCP result")),Err(error)=>{eprintln!("{error}");std::process::exit(error.exit_code());}}return;
    }
    if args.get(1).is_some_and(|a|a=="__git-driver"){
        match rivet::tools::git::driver(){Ok(result)=>println!("{}",serde_json::to_string(&result).expect("Git result")),Err(error)=>{eprintln!("{error}");std::process::exit(error.exit_code());}}
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
