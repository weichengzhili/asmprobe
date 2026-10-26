use crate::config::ConfigError;
use std::path::PathBuf;

/// 解析配置文件路径。
/// 优先级：`ASMPROBE_CONFIG` 环境变量 > 平台约定目录。
pub fn config_file() -> Result<PathBuf, ConfigError> {
    if let Ok(p) = std::env::var("ASMPROBE_CONFIG") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let dirs = directories::ProjectDirs::from("", "", "asmprobe")
        .ok_or_else(|| ConfigError::UnknownKey("无法解析配置目录".to_string()))?;
    Ok(dirs.config_dir().join("config.toml"))
}

/// 解析缓存目录（跨平台约定）。
pub fn cache_dir() -> Result<PathBuf, ConfigError> {
    let dirs = directories::ProjectDirs::from("", "", "asmprobe")
        .ok_or_else(|| ConfigError::UnknownKey("无法解析缓存目录".to_string()))?;
    Ok(dirs.cache_dir().to_path_buf())
}
