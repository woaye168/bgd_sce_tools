//! 插件注册表：拉取和解析 registry.json

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    pub download_url: String,
    #[serde(default)]
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub plugins: Vec<RegistryEntry>,
}

/// 从 URL 拉取插件注册表
pub fn fetch_registry(url: &str, proxy: &str) -> Result<Registry, String> {
    let client = reqwest::blocking::Client::builder()
        .proxy(reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}
