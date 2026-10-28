use serde::{Deserialize, Serialize};

use crate::domain::{AsmLine, CompileOutput, Diagnostic, Language, Severity, SourceCode};

/// 编译请求（对齐 Godbolt `/api/compiler/{id}/compile`）。
///
/// 注意：Godbolt 要求 `userArguments` / `filters` 包在 `options` 里，并带 `lang`，
/// 扁平结构会被后端当成缺字段（表现为 502 Internal server error）。
#[derive(Debug, Clone, Serialize)]
pub struct CompileRequest {
    pub source: String,
    pub options: CompileOptions,
    pub lang: String,
    #[serde(rename = "allowStoreCodeDebug")]
    pub allow_store_code_debug: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileOptions {
    #[serde(rename = "userArguments")]
    pub user_arguments: String,
    pub filters: CompileFilters,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileFilters {
    pub binary: bool,
    #[serde(rename = "commentOnly")]
    pub comment_only: bool,
    pub demangle: bool,
    pub directives: bool,
    pub intel: bool,
    pub labels: bool,
    pub trim: bool,
}

/// 编译响应。`stdout` / `stderr` 在 Godbolt 里是 `{text, tag}` 对象数组（不是字符串）。
#[derive(Debug, Clone, Deserialize)]
pub struct CompileResponse {
    pub asm: Option<Vec<AsmLine>>,
    pub code: Option<i32>,
    pub stdout: Option<Vec<OutputLine>>,
    pub stderr: Option<Vec<OutputLine>>,
    #[serde(default)]
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputLine {
    pub text: String,
}

/// 编译器列表项（`/api/compilers/{lang}`）。
#[derive(Debug, Clone, Deserialize)]
pub struct CompilerDto {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    #[serde(default)]
    pub is_semver: bool,
}

impl CompileRequest {
    pub fn new(lang: Language, src: &SourceCode, args: &[String]) -> Self {
        // 本地后端用 `-masm=intel` 开关语法，远程用 filters.intel，两者不冲突。
        let intel = args.iter().any(|a| a == "-masm=intel");
        Self {
            source: src.text.clone(),
            options: CompileOptions {
                user_arguments: args.join(" "),
                filters: CompileFilters {
                    binary: false,
                    comment_only: true,
                    demangle: true,
                    directives: true,
                    intel,
                    labels: true,
                    trim: false,
                },
            },
            lang: lang.godbolt_id().to_string(),
            allow_store_code_debug: true,
        }
    }
}

impl From<CompileResponse> for CompileOutput {
    fn from(r: CompileResponse) -> Self {
        let asm = r.asm.unwrap_or_default();
        let mut diagnostics = Vec::new();
        for text in [r.stderr, r.stdout].into_iter().flatten() {
            for line in text {
                if line.text.trim().is_empty() {
                    continue;
                }
                let severity = if line.text.contains("warning:") {
                    Severity::Warning
                } else {
                    Severity::Error
                };
                diagnostics.push(Diagnostic {
                    severity,
                    message: line.text,
                    line: None,
                });
            }
        }
        let exit_code = r.code.unwrap_or(0);
        CompileOutput {
            source: String::new(),
            asm,
            diagnostics,
            exit_code,
        }
    }
}
