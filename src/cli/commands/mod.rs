pub mod compile;
pub mod config;
pub mod list_compilers;
pub mod list_langs;
pub mod repl;

use crate::backend::traits::CompilerBackend;
use crate::cli::Commands;
use crate::config::Config;
use crate::error::AppError;

/// 子命令分发器：仅 match 分发，无业务逻辑。
pub async fn dispatch(
    cmd: Commands,
    backend: &dyn CompilerBackend,
    cfg: &mut Config,
) -> Result<(), AppError> {
    match cmd {
        Commands::Compile(a) => compile::execute(a, backend, cfg).await,
        Commands::ListLangs => list_langs::execute(cfg.output.format).await,
        Commands::ListCompilers(a) => list_compilers::execute(a, backend, cfg.output.format).await,
        Commands::Config(a) => config::execute(a, cfg).await,
        Commands::Repl => repl::execute(backend, cfg).await,
    }
}
