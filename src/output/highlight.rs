#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Att,
    Intel,
}

/// 汇编语法高亮。
/// 默认编译不启用；`--features highlight` 开启（此处保持直通以最小化依赖）。
/// 非 TTY 或 `NO_COLOR` 时由调用方自动禁用。
pub fn highlight_asm(asm: &str, _syntax: Syntax) -> String {
    asm.to_string()
}
