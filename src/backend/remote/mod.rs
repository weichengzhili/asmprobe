#[cfg(feature = "remote")]
use async_trait::async_trait;
#[cfg(feature = "remote")]
use crate::backend::remote::client::GodboltClient;
#[cfg(feature = "remote")]
use crate::backend::remote::dto::CompileRequest;
#[cfg(feature = "remote")]
use crate::backend::traits::{BackendError, CompilerBackend};
#[cfg(feature = "remote")]
use crate::config::RemoteConfig;
#[cfg(feature = "remote")]
use crate::domain::{CompilerInfo, CompilerSource, CompilerSpec, Language, SourceCode, CompileOutput};

pub mod dto;
#[cfg(feature = "remote")]
pub mod client;

#[cfg(feature = "remote")]
pub use client::GodboltClient;

#[cfg(feature = "remote")]
pub struct GodboltBackend {
    client: GodboltClient,
}

#[cfg(feature = "remote")]
impl GodboltBackend {
    pub fn new(cfg: &RemoteConfig) -> Result<Self, crate::config::ConfigError> {
        let client = GodboltClient::new(&cfg.base_url, cfg.timeout_secs, cfg.retries)
            .map_err(|e| crate::config::ConfigError::Parse {
                path: cfg.base_url.clone(),
                msg: e.to_string(),
            })?;
        Ok(Self { client })
    }
}

#[cfg(feature = "remote")]
#[async_trait]
impl CompilerBackend for GodboltBackend {
    async fn compile(
        &self,
        spec: &CompilerSpec,
        src: &SourceCode,
        args: &[String],
    ) -> Result<CompileOutput, BackendError> {
        let mut req: CompileRequest = src.clone().into();
        req.user_arguments = args.join(" ");
        let resp = self.client.compile(&spec.id_or_path, req).await?;
        let mut out: CompileOutput = resp.into();
        out.source = src.text.clone();
        if out.exit_code != 0 {
            return Err(BackendError::Compilation {
                exit_code: out.exit_code,
                diagnostics: out.diagnostics,
            });
        }
        Ok(out)
    }

    async fn list_compilers(&self, lang: Language) -> Result<Vec<CompilerInfo>, BackendError> {
        let dtos: Vec<dto::CompilerDto> = self.client.compilers(lang.godbolt_id()).await?;
        let mut infos: Vec<CompilerInfo> = dtos
            .into_iter()
            .map(|d| CompilerInfo {
                id: d.id,
                name: d.name,
                version: d.version,
                source: CompilerSource::Remote,
                is_default: false,
            })
            .collect();
        if let Some(first) = infos.first_mut() {
            first.is_default = true;
        }
        Ok(infos)
    }
}
