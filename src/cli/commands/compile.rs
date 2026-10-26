use tokio::io::AsyncReadExt;
use tracing::instrument;

use crate::backend::traits::CompilerBackend;
use crate::backend::BackendError;
use crate::cli::args::CompileArgs;
use crate::config::{Config, OutputFormat};
use crate::domain::{CompilerSpec, Language, SourceCode, SourceOrigin};
use crate::error::AppError;
use crate::output::{render_compare_html, render_compile, severity_str};

#[instrument(skip(backend, cfg), fields(compiler = %args.compiler))]
pub async fn execute(
    args: CompileArgs,
    backend: &dyn CompilerBackend,
    cfg: &Config,
) -> Result<(), AppError> {
    let name = match &args.file {
        Some(f) => f.display().to_string(),
        None => "<stdin>".to_string(),
    };

    let (text, origin) = if args.stdin {
        let t = read_stdin().await?;
        (t, SourceOrigin::Stdin)
    } else {
        let file = args
            .file
            .as_ref()
            .ok_or_else(|| AppError::Cli("缺少源文件，请使用 --file <路径> 或 --stdin".into()))?;
        let t = tokio::fs::read_to_string(file).await.map_err(AppError::Io)?;
        (t, SourceOrigin::File)
    };

    let lang = infer_lang(&args)?;
    let spec = CompilerSpec {
        id_or_path: args.compiler.clone(),
        lang,
    };

    let mut full_args: Vec<String> = Vec::new();
    if args.intel {
        match lang {
            Language::C | Language::Cpp => full_args.push("-masm=intel".to_string()),
            Language::Rust => {
                full_args.push("-C".to_string());
                full_args.push("llvm-args=-x86-asm-syntax=intel".to_string());
            }
        }
    }
    full_args.extend(args.args.iter().cloned());

    let src = SourceCode { text, origin };
    let out = backend.compile(&spec, &src, &full_args).await?;

    if out.exit_code == 0 {
        match cfg.output.format {
            OutputFormat::Json => {
                println!("{}", render_compile(&out, cfg.output.format, cfg.output.color));
            }
            OutputFormat::Html => {
                // 诊断（警告）写 stderr；HTML 对照页写 stdout
                for d in &out.diagnostics {
                    eprintln!("[{}] {}", severity_str(d.severity), d.message);
                }
                println!("{}", render_compare_html(&out, &name));
            }
            OutputFormat::Text => {
                // 汇编只写 stdout；诊断（警告）写 stderr
                for d in &out.diagnostics {
                    eprintln!("[{}] {}", severity_str(d.severity), d.message);
                }
                let asm_text: String = out
                    .asm
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                print!("{asm_text}");
                if !asm_text.ends_with('\n') {
                    print!("\n");
                }
            }
        }
        Ok(())
    } else {
        for d in &out.diagnostics {
            eprintln!("[{}] {}", severity_str(d.severity), d.message);
        }
        Err(AppError::Backend(BackendError::Compilation {
            exit_code: out.exit_code,
            diagnostics: out.diagnostics.clone(),
        }))
    }
}

fn infer_lang(args: &CompileArgs) -> Result<Language, AppError> {
    if let Some(l) = args.lang {
        return Ok(l.into());
    }
    if let Some(file) = &args.file {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        return Ok(match ext {
            "c" => Language::C,
            "cpp" | "cc" | "cxx" | "c++" => Language::Cpp,
            "rs" => Language::Rust,
            other => {
                return Err(AppError::Cli(format!(
                    "无法从扩展名 `{other}` 推断语言，请用 --lang 指定"
                )))
            }
        });
    }
    Err(AppError::Cli("使用 --stdin 时需用 --lang 指定语言".into()))
}

async fn read_stdin() -> Result<String, AppError> {
    let mut buf = String::new();
    tokio::io::stdin()
        .read_to_string(&mut buf)
        .await
        .map_err(AppError::Io)?;
    Ok(buf)
}
