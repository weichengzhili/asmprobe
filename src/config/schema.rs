use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    #[serde(rename = "html")]
    Html,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendChoice {
    #[default]
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub struct RemoteConfig {
    pub base_url: String,
    pub timeout_secs: u64,
    pub retries: u32,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        RemoteConfig {
            base_url: "https://godbolt.org".to_string(),
            timeout_secs: 30,
            retries: 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LocalConfig {
    pub compiler_paths: HashMap<String, PathBuf>,
    pub default_args: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub color: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            format: OutputFormat::Text,
            color: true,
        }
    }
}

// 以下结构用于反序列化配置文件（config.toml）。
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct FileConfig {
    #[serde(default)]
    pub remote: FileRemote,
    #[serde(default)]
    pub local: FileLocal,
    #[serde(default)]
    pub output: FileOutput,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct FileRemote {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub retries: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct FileLocal {
    #[serde(default)]
    pub compiler_paths: HashMap<String, PathBuf>,
    #[serde(default)]
    pub default_args: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct FileOutput {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub color: Option<bool>,
}
