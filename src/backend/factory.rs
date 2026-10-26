use crate::backend::traits::CompilerBackend;
use crate::backend::local::LocalBackend;
use crate::config::{BackendChoice, Config, ConfigError};

/// 后端选择的唯一入口。CLI 层不出现 if remote { ... } else { ... }。
pub fn create(
    choice: BackendChoice,
    cfg: &Config,
) -> Result<Box<dyn CompilerBackend>, ConfigError> {
    match choice {
        BackendChoice::Local => Ok(Box::new(LocalBackend::new(&cfg.local))),
        BackendChoice::Remote => {
            #[cfg(feature = "remote")]
            {
                Ok(Box::new(crate::backend::remote::GodboltBackend::new(
                    &cfg.remote,
                )?))
            }
            #[cfg(not(feature = "remote"))]
            {
                Err(ConfigError::RemoteDisabled)
            }
        }
    }
}
