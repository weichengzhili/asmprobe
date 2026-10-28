use std::error::Error as _;
use std::time::Duration;

use crate::backend::BackendError;
use crate::backend::remote::dto::{CompileRequest, CompileResponse, CompilerDto};

/// reqwest 的错误信息只保留最外层，根因（DNS/TLS/连接重置）在 source 链里，
/// 这里整条串起来，否则用户只能看到 “error sending request for url”。
fn net_error(e: reqwest::Error) -> BackendError {
    let mut msg = e.to_string();
    let mut src = e.source();
    while let Some(cause) = src {
        msg.push_str(" <- ");
        msg.push_str(&cause.to_string());
        src = cause.source();
    }
    BackendError::Network(msg)
}

/// 出错时给一小段响应体，便于定位（Godbolt 502 只回 `{"error":"Internal server error"}`）。
fn preview(body: &str) -> String {
    const MAX: usize = 200;
    let end = body
        .char_indices()
        .nth(MAX)
        .map_or(body.len(), |(i, _)| i);
    let s = &body[..end];
    if end < body.len() {
        format!("{}…（共 {} 字节）", s, body.len())
    } else {
        s.to_string()
    }
}

/// Godbolt HTTP 封装：base URL、超时、UA、两个端点。
pub struct GodboltClient {
    http: reqwest::Client,
    base: String,
    retries: usize,
}

impl GodboltClient {
    pub fn new(base_url: &str, timeout_secs: u64, retries: u32) -> Result<Self, BackendError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("asmprobe/0.1")
            .build()
            .map_err(net_error)?;
        let base = base_url.trim_end_matches('/').to_string();
        Ok(Self {
            http,
            base,
            retries: retries as usize,
        })
    }

    /// 发送并对**传输层**错误（DNS/连接/超时）重试；HTTP 状态码与解析错误不重试。
    async fn send(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, BackendError> {
        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let req = builder
                .try_clone()
                .ok_or_else(|| BackendError::Network("请求无法复制，无法重试".to_string()))?;
            match req.send().await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    let transport = !e.is_body() && !e.is_decode() && !e.is_status();
                    if transport && attempt <= self.retries {
                        tracing::warn!(attempt, error = %e, "请求失败，重试中");
                        tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
                        continue;
                    }
                    return Err(net_error(e));
                }
            }
        }
    }

    pub async fn compile(
        &self,
        id: &str,
        req: CompileRequest,
    ) -> Result<CompileResponse, BackendError> {
        let url = format!("{}/api/compiler/{}/compile", self.base, id);
        let resp = self
            .send(self
                .http
                .post(&url)
                .json(&req)
                .header("Accept", "application/json"))
            .await?;
        let status = resp.status();
        let body = resp.text().await.map_err(net_error)?;
        if !status.is_success() {
            return Err(BackendError::Api(format!("HTTP {} {}", status, preview(&body))));
        }
        serde_json::from_str::<CompileResponse>(&body).map_err(|e| {
            BackendError::Api(format!("响应解析失败: {}；响应片段: {}", e, preview(&body)))
        })
    }

    pub async fn compilers(&self, lang: &str) -> Result<Vec<CompilerDto>, BackendError> {
        let url = format!("{}/api/compilers/{}", self.base, lang);
        let resp = self
            .send(self.http.get(&url).header("Accept", "application/json"))
            .await?;
        let status = resp.status();
        let body = resp.text().await.map_err(net_error)?;
        if !status.is_success() {
            return Err(BackendError::Api(format!("HTTP {} {}", status, preview(&body))));
        }
        serde_json::from_str::<Vec<CompilerDto>>(&body).map_err(|e| {
            BackendError::Api(format!("响应解析失败: {}；响应片段: {}", e, preview(&body)))
        })
    }
}
