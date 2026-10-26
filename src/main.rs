use asmprobe::cli::Cli;
use clap::Parser;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let _ = asmprobe::observability::init(&cli.log_level);

    match asmprobe::run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", asmprobe::error::report(&e));
            ExitCode::from(asmprobe::error::AppError::exit_code(&e))
        }
    }
}
