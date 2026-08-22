use clap::Parser;
use dsh_work::cli::{self, Cli};

#[tokio::main]
async fn main() {
    dsh_work::logging::init_logging();

    let cli = Cli::parse();
    let result = cli::run(cli).await;

    if let Err(err) = result {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
