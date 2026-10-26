use crate::domain::{CompileOutput, CompilerInfo, CompilerSpec, Diagnostic, Language, SourceCode};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("未找到编译器: {0}")]
    CompilerNotFound(String),
    #[error("编译失败 (退出码 {exit_code})")]
    Compilation {
        exit_code: i32,
        diagnostics: Vec<Diagnostic>,
    },
    #[error("网络错误: {0}")]
    Network(String),
    #[error("超时")]
    Timeout,
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("Godbolt API 错误: {0}")]
    Api(String),
}

/// 唯一的编译契约：上层只通过该 trait 与后端交互。
#[async_trait]
pub trait CompilerBackend: Send + Sync {
    async fn compile(
        &self,
        spec: &CompilerSpec,
        src: &SourceCode,
        args: &[String],
    ) -> Result<CompileOutput, BackendError>;

    async fn list_compilers(&self, lang: Language) -> Result<Vec<CompilerInfo>, BackendError>;
}
