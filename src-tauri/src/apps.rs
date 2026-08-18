//! 应用管理：应用市场下载、安装、卸载、启动（独立 EXE 进程，WeGame 模式）

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用清单项（registry.json 中一个应用）
/// 私有仓库下 release asset 直链不可用，下载必须走 API：repo + tag + asset_name 定位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// GitHub 仓库（owner/repo）
    pub repo: String,
    /// Release tag（"latest" 表示最新 Release）
    pub tag: String,
    /// Release asset 文件名
    pub asset_name: String,
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

/// 从远程清单 URL 拉取应用列表
pub fn fetch_registry(url: &str, proxy: &str, token: &str) -> Result<AppRegistry> {
    let resp = crate::net::http_client(proxy, token)?
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
/// 流程：API 解析 Release -> 按 asset_name 找 asset -> asset API URL + Accept: octet-stream 下载
pub fn install_app(app: &AppInfo, proxy: &str, token: &str) -> Result<()> {
    let dir = app_dir(&app.id)?;
    fs::create_dir_all(&dir).with_context(|| format!("创建应用目录失败: {}", dir.display()))?;

    let client = crate::net::http_client(proxy, token)?;

    // 1. 解析 Release（tag 为 "latest" 时取最新 Release）
    let release_url = if app.tag == "latest" {
        format!("https://api.github.com/repos/{}/releases/latest", app.repo)
    } else {
        format!("https://api.github.com/repos/{}/releases/tags/{}", app.repo, app.tag)
    };
    let resp = client
        .get(&release_url)
        .send()
        .with_context(|| format!("查询应用 Release 失败: {release_url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("查询应用 Release 失败: HTTP {}（{}）", resp.status(), app.repo));
    }
    let release: serde_json::Value = resp.json().context("解析 Release 响应失败")?;

    // 2. 按文件名定位 asset（asset["url"] 是 API 地址，带 token 才能下载）
    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let asset_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(app.asset_name.as_str()))
        .and_then(|a| a["url"].as_str().map(str::to_string))
        .ok_or_else(|| anyhow!("Release {} 中找不到 asset: {}", app.tag, app.asset_name))?;

    // 3. 下载 asset（Accept: octet-stream；302 到 CDN 时敏感头自动剥离）
    let resp = client
        .get(&asset_url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .with_context(|| format!("下载应用失败: {}", app.asset_name))?;
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

// ---------------------------------------------------------------- 自启动（随主程序静默启动，0.6.6）

/// 应用是否已在运行（按 exe 全路径匹配进程，兼容大小写与斜杠）
pub fn is_app_running(exe: &std::path::Path) -> bool {
    let want = exe.display().to_string().replace('/', "\\").to_lowercase();
    let Ok(out) = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress",
        ])
        .output()
    else {
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let list = match &doc {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(_) => vec![doc],
        _ => return false,
    };
    list.iter().any(|p| {
        p["ExecutablePath"]
            .as_str()
            .map(|s| s.to_lowercase() == want)
            .unwrap_or(false)
    })
}

/// 随主程序静默启动配置的应用（单开：已在运行跳过；有当前项目则透传 --project-path）
pub fn autostart_apps(ids: &[String], project: Option<&std::path::Path>) {
    for id in ids {
        let Ok(exe) = app_exe_path(id) else { continue };
        if !exe.is_file() || is_app_running(&exe) {
            continue;
        }
        let mut cmd = std::process::Command::new(&exe);
        if let Some(p) = project {
            cmd.arg("--project-path").arg(p);
        }
        let _ = cmd.spawn();
    }
}
