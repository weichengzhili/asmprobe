pub mod formatter;
pub mod highlight;

pub use formatter::{
    render_compare_html, render_compile, render_compilers, render_languages, severity_str,
    source_str,
};
pub use highlight::{highlight_asm, Syntax};
