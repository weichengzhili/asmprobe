pub mod backend;
pub mod cli;
pub mod config;
pub mod domain;
pub mod error;
pub mod observability;
pub mod output;

use std::io::{self, IsTerminal};

use crate::cli::commands::repl;
use crate::cli::Cli;
use crate::config::{load, BackendChoice, Config, LoadOverrides};
use crate::error::AppError;

/// 唯一的编排点：config::load → backend::factory::create → cli::commands::dispatch。
/// 集成测试直接调用 `run`，避免每次 spawn 二进制。
pub async fn run(cli: Cli) -> Result<(), AppError> {
    let Cli {
        command,
        backend,
        log_level,
        config,
        format,
    } = cli;

    // 裸跑（无子命令）：交互式终端进入 REPL；非交互（管道/脚本）打印帮助后成功退出。
    let interactive = command.is_none() && io::stdin().is_terminal();
    if command.is_none() && !interactive {
        Cli::print_help()?;
        return Ok(());
    }

    let overrides = LoadOverrides {
        base_url: None,
        config_path: config,
        format: Some(format.into()),
        log_level: Some(log_level),
    };
    let mut cfg: Config = load(&overrides)?;

    let choice: BackendChoice = backend.map(Into::into).unwrap_or_default();
    let backend = backend::create(choice, &cfg)?;

    match command {
        Some(cmd) => crate::cli::commands::dispatch(cmd, backend.as_ref(), &mut cfg).await,
        None => repl::execute(backend.as_ref(), &mut cfg).await,
    }
}
