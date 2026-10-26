pub mod args;
pub mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::cli::args::{
    BackendChoiceArg, CompileArgs, ConfigArgs, ListCompilersArgs, OutputFormatArg,
};
use crate::config::{BackendChoice, OutputFormat};

#[derive(Debug, Parser)]
#[command(name = "asmprobe", version, about = "Inspect compiler assembly output (local or godbolt.org)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 选择后端：remote(godbolt) 或 local
    #[arg(long, global = true, value_enum)]
    pub backend: Option<BackendChoiceArg>,

    /// 日志级别 (trace|debug|info|warn|error)
    #[arg(long, global = true, env = "ASMPROBE_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// 配置文件路径（覆盖 ASMPROBE_CONFIG 与平台约定）
    #[arg(long, global = true, env = "ASMPROBE_CONFIG")]
    pub config: Option<PathBuf>,

    /// 输出格式：text 或 json
    #[arg(long, global = true, value_enum, default_value = "text")]
    pub format: OutputFormatArg,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 编译源码并输出汇编
    Compile(CompileArgs),
    /// 列出支持的语言及默认参数
    ListLangs,
    /// 列出某语言可用编译器
    ListCompilers(ListCompilersArgs),
    /// 读取/写入配置
    #[command(subcommand)]
    Config(ConfigArgs),
}

impl Cli {
    pub fn format(&self) -> OutputFormat {
        self.format.into()
    }

    pub fn backend_choice(&self) -> Option<BackendChoice> {
        self.backend.map(Into::into)
    }
}
