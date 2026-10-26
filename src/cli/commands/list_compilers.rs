use crate::backend::traits::CompilerBackend;
use crate::cli::args::ListCompilersArgs;
use crate::config::OutputFormat;
use crate::domain::Language;
use crate::error::AppError;
use crate::output::render_compilers;

pub async fn execute(
    args: ListCompilersArgs,
    backend: &dyn CompilerBackend,
    format: OutputFormat,
) -> Result<(), AppError> {
    let lang: Language = args.language.into();
    let list = backend.list_compilers(lang).await?;
    print!("{}", render_compilers(&list, format));
    Ok(())
}
