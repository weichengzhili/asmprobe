use crate::cli::args::ConfigArgs;
use crate::config::{paths, Config, ConfigError};
use crate::error::AppError;

pub async fn execute(args: ConfigArgs, cfg: &mut Config) -> Result<(), AppError> {
    match args {
        ConfigArgs::Get { key } => match cfg.get(&key) {
            Some(v) => {
                println!("{v}");
                Ok(())
            }
            None => Err(AppError::Config(ConfigError::UnknownKey(key))),
        },
        ConfigArgs::Set { key, value } => {
            cfg.set(&key, &value)?;
            cfg.save()?;
            Ok(())
        }
        ConfigArgs::Path => {
            let p = paths::config_file().map_err(AppError::Config)?;
            println!("{}", p.display());
            Ok(())
        }
        ConfigArgs::Edit => {
            let p = paths::config_file().map_err(AppError::Config)?;
            if !p.exists() {
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent).map_err(AppError::Io)?;
                }
                std::fs::write(&p, "").map_err(AppError::Io)?;
            }
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
            let status = std::process::Command::new(&editor)
                .arg(&p)
                .status()
                .map_err(AppError::Io)?;
            if status.success() {
                Ok(())
            } else {
                Err(AppError::Cli("编辑器退出异常".into()))
            }
        }
    }
}
