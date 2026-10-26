use crate::backend::BackendError;
use crate::config::ConfigError;

#[derive(Debug)]
pub enum AppError {
    Cli(String),
    Config(ConfigError),
    Backend(BackendError),
    Io(std::io::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Cli(m) => write!(f, "用法错误: {m}"),
            AppError::Config(e) => write!(f, "配置错误: {e}"),
            AppError::Backend(e) => write!(f, "后端错误: {e}"),
            AppError::Io(e) => write!(f, "I/O 错误: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::Config(e)
    }
}

impl From<BackendError> for AppError {
    fn from(e: BackendError) -> Self {
        AppError::Backend(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl AppError {
    /// 退出码约定见 `doc/structure.md` 附录 A。
    pub fn exit_code(&self) -> u8 {
        match self {
            AppError::Cli(_) => 2,
            AppError::Config(_) => 1,
            AppError::Io(_) => 1,
            AppError::Backend(b) => match b {
                BackendError::Compilation { .. } => 4,
                _ => 3,
            },
        }
    }
}

pub fn report(e: &AppError) -> String {
    e.to_string()
}
