use clap::Parser;
use std::io::{self, BufRead, Write};

use crate::backend::traits::CompilerBackend;
use crate::cli::commands::dispatch;
use crate::cli::Cli;
use crate::config::Config;
use crate::error::{report, AppError};

/// 进入交互时的提示语（走 stderr，保持 stdout 只承载结果）。
const BANNER: &str = "asmprobe 交互模式：直接输入子命令，如 `compile tmp/demo.c --intel`；\
                      `help` 查看用法，`exit` 退出。";
const PROMPT: &str = "asmprobe> ";

/// 交互式命令循环：逐行读取 → 按 shell 规则切分 → 复用 clap 解析 → 复用 `dispatch`。
///
/// 与 `run` 的区别：整个会话只加载一次配置、只创建一个后端，`--format` 可在行内切换。
pub async fn execute(backend: &dyn CompilerBackend, cfg: &mut Config) -> Result<(), AppError> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut line = String::new();

    eprintln!("{}", BANNER);
    loop {
        eprint!("{}", PROMPT);
        io::stderr().flush()?;

        line.clear();
        if input.read_line(&mut line)? == 0 {
            eprintln!();
            break; // EOF（Ctrl-D 或管道结束）
        }

        let tokens = split(&line);
        let Some(first) = tokens.first().map(String::as_str) else {
            continue;
        };

        match first {
            "exit" | "quit" => break,
            "help" | "?" => Cli::print_help()?,
            "repl" => eprintln!("已在交互模式中；切换后端请退出后用 --backend 重启。"),
            _ => {
                let mut argv = Vec::with_capacity(tokens.len() + 1);
                argv.push("asmprobe".to_string());
                argv.extend(tokens.iter().cloned());

                let cli = match Cli::try_parse_from(argv) {
                    Ok(cli) => cli,
                    Err(e) => {
                        e.print()?; // clap 自行决定 stdout/stderr（`--help`/用法错误）
                        continue;
                    }
                };

                // 仅当该行显式写了 --format 才覆盖会话默认，避免默认值反向覆盖配置。
                if tokens.iter().any(|t| t == "--format" || t.starts_with("--format=")) {
                    cfg.output.format = cli.format();
                }

                let Some(cmd) = cli.command else {
                    eprintln!("需要子命令；输入 help 查看用法。");
                    continue;
                };

                // Box::pin 打断 `dispatch → repl → dispatch` 的异步递归。
                if let Err(e) = Box::pin(dispatch(cmd, backend, cfg)).await {
                    eprintln!("{}", report(&e));
                }
            }
        }
    }

    Ok(())
}

/// 极简 shell 切分：支持空格分隔、单/双引号与反斜杠转义（不引入额外依赖）。
fn split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            cur.push(ch);
            escaped = false;
            continue;
        }
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => cur.push(ch),
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    started = true;
                }
                '\\' => escaped = true,
                c if c.is_whitespace() => {
                    if started {
                        out.push(std::mem::take(&mut cur));
                        started = false;
                    }
                }
                c => {
                    cur.push(c);
                    started = true;
                }
            },
        }
    }
    if started {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn splits_plain_args() {
        assert_eq!(split("compile a.c --intel"), ["compile", "a.c", "--intel"]);
    }

    #[test]
    fn keeps_quoted_args_together() {
        assert_eq!(
            split(r#"compile a.c -a "-O2 -march=native""#),
            ["compile", "a.c", "-a", "-O2 -march=native"]
        );
    }

    #[test]
    fn treats_unclosed_quote_as_literal() {
        assert_eq!(split(r#"list-langs "#), ["list-langs"]);
    }
}
