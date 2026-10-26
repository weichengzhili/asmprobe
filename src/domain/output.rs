use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOrigin {
    File,
    Stdin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCode {
    pub text: String,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub line: Option<u32>,
}

/// 单条汇编行与其对应的源码行（来自编译器行号标记）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsmSource {
    pub line: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsmLine {
    pub text: String,
    pub source: Option<AsmSource>,
}

/// 一次编译的输出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    /// 原始源码（用于源码-汇编对照页）。
    pub source: String,
    /// 汇编，按行给出，并尽量关联源码行。
    pub asm: Vec<AsmLine>,
    pub diagnostics: Vec<Diagnostic>,
    pub exit_code: i32,
}
