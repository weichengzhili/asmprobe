pub mod paths;
pub mod schema;

use std::path::PathBuf;
pub use schema::{
    FileConfig, FileOutput, FileRemote, LocalConfig, OutputConfig, RemoteConfig,
};

pub use schema::{BackendChoice, OutputFormat};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置文件读取失败: {0}")]
    Read(#[from] std::io::Error),
    #[error("配置文件解析失败 ({path}): {msg}")]
    Parse { path: String, msg: String },
    #[error("未知配置项: {0}")]
    UnknownKey(String),
    #[error("远程后端未启用，请用 --features remote 重新编译")]
    RemoteDisabled,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub remote: RemoteConfig,
    pub local: LocalConfig,
    pub output: OutputConfig,
}

/// 来自 CLI / 环境变量的覆盖项；config 层不依赖 cli 层。
#[derive(Debug, Clone, Default)]
pub struct LoadOverrides {
    pub base_url: Option<String>,
    pub config_path: Option<PathBuf>,
    pub format: Option<OutputFormat>,
    pub log_level: Option<String>,
}

/// 三层合并：CLI 覆盖 > 环境变量 > 配置文件 > 内置默认。
pub fn load(overrides: &LoadOverrides) -> Result<Config, ConfigError> {
    let path = match &overrides.config_path {
        Some(p) => Some(p.clone()),
        None => paths::config_file().ok(),
    };

    let file: FileConfig = if let Some(p) = path {
        if p.exists() {
            let text = std::fs::read_to_string(&p)?;
            toml::from_str(&text).map_err(|e| ConfigError::Parse {
                path: p.display().to_string(),
                msg: e.to_string(),
            })?
        } else {
            FileConfig::default()
        }
    } else {
        FileConfig::default()
    };

    let mut remote = RemoteConfig {
        base_url: file
            .remote
            .base_url
            .clone()
            .unwrap_or_else(|| RemoteConfig::default().base_url),
        timeout_secs: file.remote.timeout_secs.unwrap_or(30),
        retries: file.remote.retries.unwrap_or(1),
    };

    let local = LocalConfig {
        compiler_paths: file.local.compiler_paths.clone(),
        default_args: file.local.default_args.clone(),
    };

    let mut output = OutputConfig {
        format: match file.output.format.as_deref() {
            Some("json") => OutputFormat::Json,
            Some("html") => OutputFormat::Html,
            _ => OutputFormat::Text,
        },
        color: file.output.color.unwrap_or(true),
    };

    // 环境变量覆盖（ASMPROBE_* 前缀）
    if let Ok(v) = std::env::var("ASMPROBE_BASE_URL") {
        if !v.is_empty() {
            remote.base_url = v;
        }
    }
    if let Ok(v) = std::env::var("ASMPROBE_FORMAT") {
        output.format = match v.to_ascii_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "html" => OutputFormat::Html,
            _ => OutputFormat::Text,
        };
    }

    // CLI 覆盖（最高优先级）
    if let Some(v) = &overrides.base_url {
        remote.base_url = v.clone();
    }
    if let Some(f) = &overrides.format {
        output.format = *f;
    }

    Ok(Config {
        remote,
        local,
        output,
    })
}

impl Config {
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "remote.base_url" => Some(self.remote.base_url.clone()),
            "remote.timeout_secs" => Some(self.remote.timeout_secs.to_string()),
            "remote.retries" => Some(self.remote.retries.to_string()),
            "output.format" => Some(match self.output.format {
                OutputFormat::Json => "json".to_string(),
                OutputFormat::Html => "html".to_string(),
                OutputFormat::Text => "text".to_string(),
            }),
            "output.color" => Some(self.output.color.to_string()),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "remote.base_url" => self.remote.base_url = value.to_string(),
            "remote.timeout_secs" => {
                self.remote.timeout_secs = value
                    .parse()
                    .map_err(|_| ConfigError::UnknownKey(key.to_string()))?
            }
            "remote.retries" => {
                self.remote.retries = value
                    .parse()
                    .map_err(|_| ConfigError::UnknownKey(key.to_string()))?
            }
            "output.format" => {
                self.output.format = match value.to_ascii_lowercase().as_str() {
                    "json" => OutputFormat::Json,
                    "html" => OutputFormat::Html,
                    "text" => OutputFormat::Text,
                    _ => return Err(ConfigError::UnknownKey(key.to_string())),
                }
            }
            "output.color" => {
                self.output.color = value
                    .parse()
                    .map_err(|_| ConfigError::UnknownKey(key.to_string()))?
            }
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    pub fn to_file_config(&self) -> FileConfig {
        FileConfig {
            remote: FileRemote {
                base_url: Some(self.remote.base_url.clone()),
                timeout_secs: Some(self.remote.timeout_secs),
                retries: Some(self.remote.retries),
            },
            local: schema::FileLocal {
                compiler_paths: self.local.compiler_paths.clone(),
                default_args: self.local.default_args.clone(),
            },
            output: FileOutput {
                format: Some(match self.output.format {
                    OutputFormat::Json => "json".to_string(),
                    OutputFormat::Html => "html".to_string(),
                    OutputFormat::Text => "text".to_string(),
                }),
                color: Some(self.output.color),
            },
        }
    }

    /// 原子写回配置文件（临时文件 + rename）。
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = paths::config_file()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let fc = self.to_file_config();
        let text = toml::to_string_pretty(&fc).map_err(|e| ConfigError::Parse {
            path: path.display().to_string(),
            msg: e.to_string(),
        })?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}
