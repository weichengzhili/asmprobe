use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::config::{BackendChoice, OutputFormat};
use crate::domain::Language;

/// 全局 `--backend` 的参数类型（ValueEnum）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BackendChoiceArg {
    Remote,
    Local,
}

impl From<BackendChoiceArg> for BackendChoice {
    fn from(a: BackendChoiceArg) -> Self {
        match a {
            BackendChoiceArg::Remote => BackendChoice::Remote,
            BackendChoiceArg::Local => BackendChoice::Local,
        }
    }
}

/// 全局 `--format` 的参数类型（ValueEnum）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormatArg {
    Text,
    Json,
    Html,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(a: OutputFormatArg) -> Self {
        match a {
            OutputFormatArg::Text => OutputFormat::Text,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Html => OutputFormat::Html,
        }
    }
}

/// `--lang` 参数类型（ValueEnum），映射到 domain::Language。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LanguageArg {
    C,
    Cpp,
    Rust,
}

impl From<LanguageArg> for Language {
    fn from(a: LanguageArg) -> Self {
        match a {
            LanguageArg::C => Language::C,
            LanguageArg::Cpp => Language::Cpp,
            LanguageArg::Rust => Language::Rust,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct CompileArgs {
    /// 源文件路径
    pub file: Option<PathBuf>,
    /// 从 stdin 读取源码（与 --file 互斥）
    #[arg(long, conflicts_with = "file")]
    pub stdin: bool,
    /// 编译器 id 或本地路径，默认 gcc
    #[arg(short, long, default_value = "gcc")]
    pub compiler: String,
    /// 额外的编译器参数（可多次出现）
    #[arg(short = 'a', long = "args", num_args = 0.., allow_hyphen_values = true)]
    pub args: Vec<String>,
    /// 语言（从扩展名推断失败时或 --stdin 时必填）
    #[arg(long, value_enum)]
    pub lang: Option<LanguageArg>,
    /// 使用 Intel 汇编语法
    #[arg(long)]
    pub intel: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ListCompilersArgs {
    #[arg(value_enum)]
    pub language: LanguageArg,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigArgs {
    /// 读取某个配置项
    Get { key: String },
    /// 写入某个配置项并持久化
    Set { key: String, value: String },
    /// 打印配置文件路径
    Path,
    /// 用 $EDITOR 打开配置文件
    Edit,
}
