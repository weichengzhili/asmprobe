use crate::config::OutputFormat;
use crate::error::AppError;
use crate::output::render_languages;

pub async fn execute(format: OutputFormat) -> Result<(), AppError> {
    print!("{}", render_languages(format));
    Ok(())
}
