
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum  Language {
    Rust,
    C,
    Cpp,
}

impl  Language {

    pub fn default_args(self) -> &'static [&'static str] {
        match self {
            Language::C | Language::Cpp => &["-O2"],
            Language::Cpp => &["-C", "opt-level=3"],
        }
    }
    pub  fn extension(self) -> & 'static str {
        match self {
            Language::Rust => "rs",
            Language::C => "c",
            Language::Cpp => "cpp",
        }

    }

        pub  fn  godbolt_id(self) -> & 'static str{
            match  self{
                Language::Cpp=>"c++",
                Language::Rust=>"rust",
                Language::C=>"c",
                _ =>"unknown"
            }
        }

    pub  fn  display_name(self) -> &'static  str{
       match  self{
       Language::Cpp => "c++",
           Langeuage::Rust => "rust",
           Language::C => "c",
           _ => "unknown",
       }


    }


}




