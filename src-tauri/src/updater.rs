//! 自我更新：私有仓库下替代 tauri-plugin-updater
//! （插件无法携带 token，故自建：认证查询最新 Release -> 下载 NSIS 安装包 -> 启动安装）

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

const SELF_REPO: &str = "woaye168/bgd_sce_tools";

#[derive(Debug, Clone, Serialize)]
pub struct SelfUpdateInfo {
    pub current: String,
    pub latest: String,
    pub has_update: bool,
}

/// 语义化版本比较：a 比 b 新返回 true（忽略 v 前缀，逐段数值比较）
fn is_newer(a: &str, b: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.trim_start_matches('v')
            .split('.')
            .map(|s| s.trim().parse().unwrap_or(0))
            .collect()
    }
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// 查询最新 Release 的 JSON（含 assets 列表）
fn latest_release(proxy: &str, token: &str) -> Result<serde_json::Value> {
    let url = format!("https://api.github.com/repos/{SELF_REPO}/releases/latest");
    let resp = crate::net::http_client(proxy, token)?
        .get(&url)
        .send()
        .with_context(|| format!("检查更新失败: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("检查更新失败: HTTP {}", resp.status()));
    }
    resp.json().context("解析 Release 响应失败")
}

/// 检查是否有新版本
pub fn check_self_update(current: &str, proxy: &str, token: &str) -> Result<SelfUpdateInfo> {
    let release = latest_release(proxy, token)?;
    let latest = release["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("Release 缺少 tag_name"))?
        .trim_start_matches('v')
        .to_string();
    Ok(SelfUpdateInfo {
        current: current.to_string(),
        has_update: is_newer(&latest, current),
        latest,
    })
}

/// 下载最新 Release 的 NSIS 安装包并启动（安装器会自动关闭并替换当前进程）
/// 启动自我更新（async：Release 查询与安装包下载让出线程；
/// `on_progress(downloaded, total)` 回调下载进度）
pub async fn start_self_update_async(
    proxy: &str,
    token: &str,
    on_progress: impl Fn(u64, Option<u64>) + Send,
) -> Result<()> {
    let client = crate::net::async_http_client(proxy, token)?;
    let release_url = format!("https://api.github.com/repos/{SELF_REPO}/releases/latest");
    let resp = client
        .get(&release_url)
        .send()
        .await
        .with_context(|| format!("检查更新失败: {release_url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("检查更新失败: HTTP {}", resp.status()));
    }
    let release: serde_json::Value = resp.json().await.context("解析 Release 响应失败")?;

    // 找 NSIS 安装包 asset（tauri 打包命名：<product>_<version>_x64-setup.exe）
    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let asset_url = assets
        .iter()
        .filter_map(|a| {
            let name = a["name"].as_str()?;
            let url = a["url"].as_str()?;
            name.ends_with("-setup.exe").then(|| (name.to_string(), url.to_string()))
        })
        .map(|(_, url)| url)
        .next()
        .ok_or_else(|| anyhow!("最新 Release 中找不到 NSIS 安装包（*-setup.exe）"))?;

    let bytes = crate::net::download_bytes_async(&asset_url, proxy, token, on_progress).await?;

    let installer = std::env::temp_dir().join("bgd_sce_tools-update-setup.exe");
    std::fs::write(&installer, &bytes)
        .with_context(|| format!("写入安装包失败: {}", installer.display()))?;

    // 启动安装器（NSIS 安装器会自动关闭运行中的本程序并完成替换）
    std::process::Command::new(&installer)
        .spawn()
        .with_context(|| format!("启动安装器失败: {}", installer.display()))?;
    Ok(())
}

pub fn start_self_update(proxy: &str, token: &str) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow!("创建运行时失败: {e}"))?;
    rt.block_on(start_self_update_async(proxy, token, |_, _| {}))
}
