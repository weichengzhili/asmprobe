use crate::domain::Language;
use std::path::Path;
use tokio::process::Command;

/// 语言感知的命令行构造。
/// 默认参数来自 `Language::default_args()`，用户参数追加其后（可被覆盖）。
pub fn build(
    lang: Language,
    compiler: &Path,
    source: &Path,
    asm_out: &Path,
    user_args: &[String],
) -> Command {
    let mut cmd = Command::new(compiler);
    match lang {
        Language::C | Language::Cpp => {
            cmd.arg("-S").arg("-o").arg(asm_out).arg(source);
            // 让汇编中带 # file:line:col 注释，便于源码-汇编对照
            cmd.arg("-fverbose-asm");
        }
        Language::Rust => {
            cmd.arg("--emit=asm").arg("-o").arg(asm_out).arg(source);
            // 生成 .loc 行号信息，便于源码-汇编对照
            cmd.arg("-g");
        }
    }
    for a in lang.default_args() {
        cmd.arg(a);
    }
    for a in user_args {
        cmd.arg(a);
    }
    cmd
}
