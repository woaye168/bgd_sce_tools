//! 应用管理：应用市场下载、安装、卸载、启动（独立 EXE 进程，WeGame 模式）

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用清单项（registry.json 中一个应用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
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

/// 应用清单（registry.json 顶层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRegistry {
    #[serde(default)]
    pub apps: Vec<AppInfo>,
}

/// 已安装应用（apps/{id}/app.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
}

/// 宿主安装目录（exe 所在目录，便携绿色版约定）
fn host_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("无法获取宿主可执行文件路径")?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow!("无法获取宿主安装目录"))
}

/// 应用安装根目录：<宿主>/apps
pub fn apps_root() -> Result<PathBuf> {
    Ok(host_dir()?.join("apps"))
}

/// 单个应用目录：<宿主>/apps/{id}
fn app_dir(id: &str) -> Result<PathBuf> {
    Ok(apps_root()?.join(id))
}

/// 应用 exe 路径：<宿主>/apps/{id}/{id}.exe
pub fn app_exe_path(id: &str) -> Result<PathBuf> {
    Ok(app_dir(id)?.join(format!("{id}.exe")))
}

// ---------------------------------------------------------------- 清单拉取

fn http_client(proxy: &str) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder().user_agent("BGD_SCE_TOOLS");
    let proxy = proxy.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).context("代理地址无效")?);
    }
    builder.build().context("创建 HTTP 客户端失败")
}

/// 从远程清单 URL 拉取应用列表
pub fn fetch_registry(url: &str, proxy: &str) -> Result<AppRegistry> {
    let resp = http_client(proxy)?
        .get(url)
        .send()
        .with_context(|| format!("请求应用清单失败: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("应用清单请求失败: HTTP {}", resp.status()));
    }
    let text = resp.text().context("读取应用清单响应失败")?;
    serde_json::from_str(&text).with_context(|| format!("解析应用清单失败: {url}"))
}

// ---------------------------------------------------------------- 安装 / 卸载 / 列表

/// 下载并安装应用 exe 到 <宿主>/apps/{id}/
pub fn install_app(app: &AppInfo, proxy: &str) -> Result<()> {
    let dir = app_dir(&app.id)?;
    fs::create_dir_all(&dir).with_context(|| format!("创建应用目录失败: {}", dir.display()))?;

    let resp = http_client(proxy)?
        .get(&app.download_url)
        .send()
        .with_context(|| format!("下载应用失败: {}", app.download_url))?;
    if !resp.status().is_success() {
        return Err(anyhow!("下载应用失败: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().context("读取应用下载内容失败")?;

    let exe_path = app_exe_path(&app.id)?;
    fs::write(&exe_path, &bytes).with_context(|| format!("写入应用文件失败: {}", exe_path.display()))?;

    // 写元数据 app.json
    let meta = InstalledApp {
        id: app.id.clone(),
        name: app.name.clone(),
        version: app.version.clone(),
        description: app.description.clone(),
        author: app.author.clone(),
    };
    let meta_path = dir.join("app.json");
    fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)
        .with_context(|| format!("写入应用元数据失败: {}", meta_path.display()))?;
    Ok(())
}

/// 卸载应用：删除 <宿主>/apps/{id}/ 整个目录
pub fn uninstall_app(id: &str) -> Result<()> {
    let dir = app_dir(id)?;
    if dir.is_dir() {
        fs::remove_dir_all(&dir).with_context(|| format!("删除应用目录失败: {}", dir.display()))?;
    }
    Ok(())
}

/// 列出已安装应用（扫描 <宿主>/apps/* /app.json）
pub fn list_installed() -> Result<Vec<InstalledApp>> {
    let root = apps_root()?;
    let mut apps = Vec::new();
    if !root.is_dir() {
        return Ok(apps);
    }
    for entry in fs::read_dir(&root).with_context(|| format!("读取应用目录失败: {}", root.display()))? {
        let entry = entry?;
        let meta_path = entry.path().join("app.json");
        if meta_path.is_file() {
            if let Ok(text) = fs::read_to_string(&meta_path) {
                if let Ok(app) = serde_json::from_str::<InstalledApp>(&text) {
                    apps.push(app);
                }
            }
        }
    }
    Ok(apps)
}
