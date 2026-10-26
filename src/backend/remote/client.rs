use std::time::Duration;

use crate::backend::BackendError;
use crate::backend::remote::dto::{CompileRequest, CompileResponse, CompilerDto};

/// Godbolt HTTP 封装：base URL、超时、UA、两个端点。
pub struct GodboltClient {
    http: reqwest::Client,
    base: String,
}

impl GodboltClient {
    pub fn new(base_url: &str, timeout_secs: u64, _retries: u32) -> Result<Self, BackendError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("asmprobe/0.1")
            .build()
            .map_err(|e| BackendError::Network(e.to_string()))?;
        let base = base_url.trim_end_matches('/').to_string();
        Ok(Self { http, base })
    }

    pub async fn compile(
        &self,
        id: &str,
        req: CompileRequest,
    ) -> Result<CompileResponse, BackendError> {
        let url = format!("{}/api/compiler/{}/compile", self.base, id);
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(BackendError::Api(format!("HTTP {}", resp.status())));
        }
        resp.json::<CompileResponse>()
            .await
            .map_err(|e| BackendError::Api(e.to_string()))
    }

    pub async fn compilers(&self, lang: &str) -> Result<Vec<CompilerDto>, BackendError> {
        let url = format!("{}/api/compilers/{}", self.base, lang);
        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(BackendError::Api(format!("HTTP {}", resp.status())));
        }
        resp.json::<Vec<CompilerDto>>()
            .await
            .map_err(|e| BackendError::Api(e.to_string()))
    }
}
