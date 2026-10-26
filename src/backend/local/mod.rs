use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use regex::Regex;

use crate::backend::traits::{BackendError, CompilerBackend};
use crate::config::LocalConfig;
use crate::domain::{
    AsmLine, AsmSource, CompilerInfo, CompilerSource, CompilerSpec, CompileOutput, Diagnostic,
    Language, Severity, SourceCode,
};

pub mod command;
pub mod resolver;
pub mod temp;

pub use resolver::{CompilerResolver, SystemResolver};

pub struct LocalBackend {
    resolver: Arc<dyn CompilerResolver>,
    local_cfg: LocalConfig,
}

impl LocalBackend {
    pub fn new(cfg: &LocalConfig) -> Self {
        Self {
            resolver: Arc::new(SystemResolver::new()),
            local_cfg: cfg.clone(),
        }
    }

    /// 注入 fake 编译器，供测试使用。
    pub fn with_resolver(r: Arc<dyn CompilerResolver>) -> Self {
        Self {
            resolver: r,
            local_cfg: LocalConfig::default(),
        }
    }
}

#[async_trait]
impl CompilerBackend for LocalBackend {
    async fn compile(
        &self,
        spec: &CompilerSpec,
        src: &SourceCode,
        args: &[String],
    ) -> Result<CompileOutput, BackendError> {
        let path = resolve_compiler(&self.local_cfg, &self.resolver, &spec.id_or_path)?;

        let ws = temp::TempWorkspace::new(spec.lang, &src.text)?;

        let mut cmd = command::build(
            spec.lang,
            &path,
            ws.source_path(),
            ws.asm_path(),
            args,
        );
        let output = cmd.output().await.map_err(BackendError::Io)?;
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let asm_text = if exit_code == 0 {
            tokio::fs::read_to_string(ws.asm_path())
                .await
                .map_err(BackendError::Io)?
        } else {
            String::new()
        };

        let diagnostics = parse_diagnostics(&stderr);
        Ok(CompileOutput {
            source: src.text.clone(),
            asm: parse_asm(&asm_text),
            diagnostics,
            exit_code,
        })
    }

    async fn list_compilers(&self, lang: Language) -> Result<Vec<CompilerInfo>, BackendError> {
        let names: &[&str] = match lang {
            Language::C => &["gcc", "clang"],
            Language::Cpp => &["g++", "clang++"],
            Language::Rust => &["rustc"],
        };
        let mut out = Vec::new();
        for n in names {
            match self.resolver.resolve(n) {
                Ok(p) => {
                    let version = self.resolver.version(&p);
                    out.push(CompilerInfo {
                        id: n.to_string(),
                        name: n.to_string(),
                        version,
                        source: CompilerSource::Local,
                        is_default: false,
                    });
                }
                Err(_) => {}
            }
        }
        if let Some(first) = out.first_mut() {
            first.is_default = true;
        }
        Ok(out)
    }
}

fn resolve_compiler(
    local_cfg: &LocalConfig,
    resolver: &Arc<dyn CompilerResolver>,
    id_or_path: &str,
) -> Result<PathBuf, BackendError> {
    let p = PathBuf::from(id_or_path);
    if p.is_absolute() || id_or_path.contains('/') || id_or_path.contains('\\') {
        if p.exists() {
            return Ok(p);
        }
        return Err(BackendError::CompilerNotFound(id_or_path.to_string()));
    }
    if let Some(c) = local_cfg.compiler_paths.get(id_or_path) {
        return Ok(c.clone());
    }
    resolver.resolve(id_or_path)
}

fn diagnostic_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(?P<path>.+?):(?P<line>\d+):(?:\d+:)?\s*(?P<sev>error|warning|note):\s*(?P<msg>.*)$",
        )
        .unwrap()
    })
}

fn parse_diagnostics(s: &str) -> Vec<Diagnostic> {
    let re = diagnostic_regex();
    let mut v = Vec::new();
    for line in s.lines() {
        if let Some(c) = re.captures(line) {
            let sev = match c.name("sev").unwrap().as_str() {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                _ => Severity::Note,
            };
            let line_no = c.name("line").unwrap().as_str().parse::<u32>().ok();
            v.push(Diagnostic {
                severity: sev,
                message: c.name("msg").unwrap().as_str().to_string(),
                line: line_no,
            });
        }
    }
    v
}

/// 将汇编文本解析为结构化行，并尽量关联源码行。
/// 支持 gcc/clang `-fverbose-asm` 的 `# file:line:col` 注释，
/// 以及 rustc `-g` 产生的 `.loc <file> <line> <col>` 指令。
fn parse_asm(asm: &str) -> Vec<AsmLine> {
    let re_gcc = Regex::new(r"^\s*#\s*([^:\s]+):(\d+):(\d+)").unwrap();
    let re_rust = Regex::new(r"^\s*\.loc\s+\d+\s+(\d+)").unwrap();
    let mut current: Option<u32> = None;
    let mut out = Vec::new();
    for raw in asm.lines() {
        let trimmed = raw.trim_start();
        if let Some(c) = re_gcc.captures(trimmed) {
            current = c.get(2).unwrap().as_str().parse::<u32>().ok();
        } else if let Some(c) = re_rust.captures(trimmed) {
            current = c.get(1).unwrap().as_str().parse::<u32>().ok();
        }
        out.push(AsmLine {
            text: raw.to_string(),
            source: current.map(|l| AsmSource { line: Some(l) }),
        });
    }
    out
}
