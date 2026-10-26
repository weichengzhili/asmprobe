use serde::{Deserialize, Serialize};

use crate::domain::{AsmLine, AsmSource, CompileOutput, Diagnostic, Severity, SourceCode};

/// 编译请求（对齐 Godbolt `/api/compiler/{id}/compile`）。
#[derive(Debug, Clone, Serialize)]
pub struct CompileRequest {
    pub source: String,
    #[serde(rename = "compilerOutput")]
    pub compiler_output: Option<String>,
    #[serde(rename = "userArguments")]
    pub user_arguments: String,
    pub filters: CompileFilters,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CompileFilters {
    #[serde(rename = "binary")]
    pub binary: bool,
    #[serde(rename = "commentOnly")]
    pub comment_only: bool,
    #[serde(rename = "demangle")]
    pub demangle: bool,
    #[serde(rename = "intel")]
    pub intel: bool,
    #[serde(rename = "labels")]
    pub labels: bool,
    #[serde(rename = "trim")]
    pub trim: bool,
}

/// 编译响应。
#[derive(Debug, Clone, Deserialize)]
pub struct CompileResponse {
    pub asm: Option<Vec<AsmLine>>,
    pub code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    #[serde(default)]
    pub success: bool,
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

impl From<CompileResponse> for CompileOutput {
    fn from(r: CompileResponse) -> Self {
        let asm = r.asm.unwrap_or_default();
        let mut diagnostics = Vec::new();
        if let Some(s) = &r.stderr {
            if !s.trim().is_empty() {
                for line in s.lines() {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        message: line.to_string(),
                        line: None,
                    });
                }
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

impl From<SourceCode> for CompileRequest {
    fn from(src: SourceCode) -> Self {
        CompileRequest {
            source: src.text,
            compiler_output: None,
            user_arguments: String::new(),
            filters: CompileFilters {
                binary: false,
                comment_only: true,
                demangle: true,
                intel: false,
                labels: false,
                trim: true,
            },
        }
    }
}
