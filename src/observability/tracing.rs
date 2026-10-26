#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("tracing 初始化失败: {0}")]
    SetGlobal(#[from] tracing::subscriber::SetGlobalDefaultError),
}

/// 初始化 tracing：`EnvFilter` + stderr writer。
/// 日志永远写 stderr；`ASMPROBE_LOG_JSON=1` 时切换为 JSON 日志。
pub fn init(level: &str) -> Result<(), InitError> {
    use tracing_subscriber::prelude::*;

    let filter = match tracing_subscriber::EnvFilter::try_new(level) {
        Ok(f) => f,
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
    };

    let registry = tracing_subscriber::registry().with(filter);

    let result = if std::env::var("ASMPROBE_LOG_JSON").as_deref() == Ok("1") {
        tracing::subscriber::set_global_default(
            registry.with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(std::io::stderr),
            ),
        )
    } else {
        tracing::subscriber::set_global_default(
            registry.with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr),
            ),
        )
    };

    result?;
    Ok(())
}
