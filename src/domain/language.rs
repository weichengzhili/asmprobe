#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    C,
    Cpp,
    Rust,
}

impl Language {
    pub fn default_args(self) -> &'static [&'static str] {
        match self {
            Language::C => &["-O2"],
            Language::Cpp => &["-O2"],
            Language::Rust => &["-C", "opt-level=3"],
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::C => "c",
            Language::Cpp => "cpp",
        }
    }

    pub fn godbolt_id(self) -> &'static str {
        match self {
            Language::Cpp => "c++",
            Language::Rust => "rust",
            Language::C => "c",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Language::Cpp => "c++",
            Language::Rust => "rust",
            Language::C => "c",
        }
    }

    pub fn all() -> &'static [Language] {
        &[Language::C, Language::Cpp, Language::Rust]
    }
}
