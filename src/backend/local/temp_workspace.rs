use crate::domain::Language;
use std::io::{self, Write};
use std::path::Path;
use tempfile::Builder;

/// 临时源文件与汇编输出文件的生命周期封装。
/// 源文件按语言加正确扩展名，drop 时自动清理，不向用户暴露路径。
pub struct TempWorkspace {
    source: tempfile::NamedTempFile,
    asm: tempfile::NamedTempFile,
}

impl TempWorkspace {
    pub fn new(lang: Language, source: &str) -> io::Result<Self> {
        let mut src = Builder::new()
            .suffix(&format!(".{}", lang.extension()))
            .tempfile()?;
        src.write_all(source.as_bytes())?;
        src.flush()?;
        let asm = Builder::new().suffix(".s").tempfile()?;
        Ok(Self { source: src, asm })
    }

    pub fn source_path(&self) -> &Path {
        self.source.path()
    }

    pub fn asm_path(&self) -> &Path {
        self.asm.path()
    }
}
