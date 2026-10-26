use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::backend::BackendError;

/// 编译器探测与版本解析。
pub trait CompilerResolver: Send + Sync {
    fn resolve(&self, name: &str) -> Result<PathBuf, BackendError>;
    fn version(&self, path: &Path) -> Option<String>;
}

/// 系统 PATH 探测 + 版本解析，进程内缓存。
pub struct SystemResolver {
    cache: Mutex<HashMap<String, PathBuf>>,
}

impl SystemResolver {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for SystemResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerResolver for SystemResolver {
    fn resolve(&self, name: &str) -> Result<PathBuf, BackendError> {
        if let Some(p) = self.cache.lock().unwrap().get(name) {
            return Ok(p.clone());
        }
        let p = which::which(name)
            .map_err(|_| BackendError::CompilerNotFound(name.to_string()))?;
        self.cache
            .lock()
            .unwrap()
            .insert(name.to_string(), p.clone());
        Ok(p)
    }

    fn version(&self, path: &Path) -> Option<String> {
        let out = std::process::Command::new(path)
            .arg("--version")
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        parse_version(&s)
    }
}

fn parse_version(s: &str) -> Option<String> {
    let re = regex::Regex::new(r"(\d+\.\d+(?:\.\d+)?)").ok()?;
    re.captures(s)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}
