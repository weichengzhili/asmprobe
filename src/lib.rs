pub mod backend;
pub mod cli;
pub mod config;
pub mod domain;
pub mod error;
pub mod observability;
pub mod output;

use crate::cli::{Cli, Commands};
use crate::config::{load, BackendChoice, Config, LoadOverrides};
use crate::error::AppError;

/// 唯一的编排点：config::load → backend::factory::create → cli::commands::dispatch。
/// 集成测试直接调用 `run`，避免每次 spawn 二进制。
pub async fn run(cli: Cli) -> Result<(), AppError> {
    let overrides = LoadOverrides {
        base_url: None,
        config_path: cli.config.clone(),
        format: Some(cli.format()),
        log_level: Some(cli.log_level.clone()),
    };
    let mut cfg: Config = load(&overrides)?;

    let choice: BackendChoice = cli.backend_choice().unwrap_or_default();
    let backend = backend::create(choice, &cfg)?;

    let cmd: Commands = cli.command;
    crate::cli::commands::dispatch(cmd, backend.as_ref(), &mut cfg).await
}
