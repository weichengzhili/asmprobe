use crate::domain::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerSpec {
    pub id_or_path: String,
    pub lang: Language,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerSource {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerInfo {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source: CompilerSource,
    pub is_default: bool,
}
