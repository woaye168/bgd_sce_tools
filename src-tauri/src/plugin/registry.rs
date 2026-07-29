//! 插件注册表：拉取和解析 registry.json

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// 插件唯一标识（用于文件名和 CLI 调用）
    pub id: String,
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
    let mut builder = reqwest::blocking::Client::builder().user_agent("BGD_SCE_TOOLS");
    let proxy = proxy.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|e| e.to_string())?);
    }
    let client = builder.build().map_err(|e| e.to_string())?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// 插件更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUpdate {
    pub id: String,
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
}

/// 检查插件更新（对比本地已安装插件和远程 registry）
pub fn check_updates(
    installed: &[(String, String)], // (id, current_version)
    registries: &[String],
    proxy: &str,
) -> Vec<PluginUpdate> {
    let mut updates = Vec::new();
    for url in registries {
        let registry = match fetch_registry(url, proxy) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in registry.plugins {
            if let Some((_, current)) = installed.iter().find(|(id, _)| *id == entry.id) {
                if entry.version != *current {
                    updates.push(PluginUpdate {
                        id: entry.id,
                        name: entry.name,
                        current_version: current.clone(),
                        latest_version: entry.version,
                        download_url: entry.download_url,
                    });
                }
            }
        }
    }
    updates
}
