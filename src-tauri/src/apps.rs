//! 应用管理：应用市场下载、安装、卸载、启动（独立 EXE 进程，WeGame 模式）

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用清单项（registry.json 极简条目：id/name/repo；其余元数据来自应用仓库 CI 合成的
/// app-release.json asset。旧格式全字段仍兼容反序列化，供过渡期内旧 registry 使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// GitHub 仓库（owner/repo）
    pub repo: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub asset_name: String,
    /// 下发默认：静默自启（用户本机勾选/取消的记忆优先，见 settings.auto_start_disabled）
    #[serde(default)]
    pub default_auto_start: bool,
    /// 版本说明（CI 合成 app-release.json 提供；升级按钮展示用）
    #[serde(default)]
    pub release_notes: String,
}

/// 应用仓库 CI 合成的发布元数据（release asset app-release.json）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppReleaseMeta {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub asset_name: String,
    #[serde(default)]
    pub release_notes: String,
    #[serde(default)]
    pub default_auto_start: bool,
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

/// 应用目录是否已存在（区分首次安装与升级覆盖）
pub fn app_dir_exists(id: &str) -> bool {
    app_dir(id).map(|d| d.is_dir()).unwrap_or(false)
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

/// 用应用仓库的 app-release.json（releases/latest asset）补全清单条目的元数据
/// （版本/描述/作者/asset名/版本说明/默认自启）。asset 不存在时保留 registry 原值（过渡期兼容）。
pub fn enrich_registry(registry: &mut AppRegistry, proxy: &str, token: &str) {
    for app in &mut registry.apps {
        let Ok(meta) = fetch_release_meta(&app.repo, proxy, token) else {
            continue;
        };
        if !meta.version.is_empty() {
            app.version = meta.version;
        }
        if !meta.description.is_empty() {
            app.description = meta.description;
        }
        if !meta.author.is_empty() {
            app.author = meta.author;
        }
        if !meta.asset_name.is_empty() {
            app.asset_name = meta.asset_name;
        }
        if !meta.release_notes.is_empty() {
            app.release_notes = meta.release_notes;
        }
        if meta.default_auto_start {
            app.default_auto_start = true;
        }
    }
}

/// 拉取应用仓库 releases/latest 中的 app-release.json asset（CI 合成发布元数据）
fn fetch_release_meta(repo: &str, proxy: &str, token: &str) -> Result<AppReleaseMeta> {
    let client = crate::net::http_client(proxy, token)?;
    let release_url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = client
        .get(&release_url)
        .send()
        .with_context(|| format!("查询 Release 失败: {release_url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("查询 Release 失败: HTTP {}", resp.status()));
    }
    let release: serde_json::Value = resp.json().context("解析 Release 响应失败")?;
    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let asset_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some("app-release.json"))
        .and_then(|a| a["url"].as_str().map(str::to_string))
        .ok_or_else(|| anyhow!("无 app-release.json asset"))?;
    let resp = client
        .get(&asset_url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .context("下载 app-release.json 失败")?;
    if !resp.status().is_success() {
        return Err(anyhow!("下载 app-release.json 失败: HTTP {}", resp.status()));
    }
    let text = resp.text().context("读取 app-release.json 失败")?;
    serde_json::from_str(&text).context("解析 app-release.json 失败")
}

// ---------------------------------------------------------------- 安装 / 卸载 / 列表

/// 下载并安装应用 exe 到 <宿主>/apps/{id}/
/// 流程：API 解析 Release -> 按 asset_name 找 asset -> asset API URL + Accept: octet-stream 下载
pub fn install_app(app: &AppInfo, proxy: &str, token: &str) -> Result<()> {
    let dir = app_dir(&app.id)?;
    fs::create_dir_all(&dir).with_context(|| format!("创建应用目录失败: {}", dir.display()))?;

    let client = crate::net::http_client(proxy, token)?;

    // 1. 解析 Release（tag 为 "latest" 或为空（极简 registry）时取最新 Release）
    let release_url = if app.tag.is_empty() || app.tag == "latest" {
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

// ---------------------------------------------------------------- 静默自启（0.6.6 引入，0.6.7 优化）

/// 子进程创建不显示控制台窗口（修复「黑终端」闪现）
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 进程 exe 全路径列表一次性获取（小写、反斜杠归一）。
/// 启动只调一次（~1s），所有应用的「是否在运行」判断在内存中匹配。
fn running_exe_paths() -> Vec<String> {
    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-Command",
        "Get-CimInstance Win32_Process | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let Ok(out) = cmd.output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let list = match &doc {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(_) => vec![doc],
        _ => return Vec::new(),
    };
    list.iter()
        .filter_map(|p| p["ExecutablePath"].as_str().map(|s| s.to_lowercase()))
        .collect()
}

/// 在已获取的进程列表中判断应用是否运行
fn is_running_in(exe: &std::path::Path, running: &[String]) -> bool {
    let want = exe.display().to_string().replace('/', "\\").to_lowercase();
    running.iter().any(|p| p == &want)
}

/// 应用是否已在运行（按 exe 全路径匹配进程，兼容大小写与斜杠）
pub fn is_app_running(exe: &std::path::Path) -> bool {
    is_running_in(exe, &running_exe_paths())
}

/// 运行中应用的 pid 列表（按 exe 全路径匹配）
fn app_pids(exe: &std::path::Path) -> Vec<u32> {
    let want = exe.display().to_string().replace('/', "\\").to_lowercase();
    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-Command",
        "Get-CimInstance Win32_Process | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let Ok(out) = cmd.output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let list = match &doc {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(_) => vec![doc],
        _ => return Vec::new(),
    };
    list.iter()
        .filter(|p| {
            p["ExecutablePath"]
                .as_str()
                .map(|s| s.to_lowercase() == want)
                .unwrap_or(false)
        })
        .filter_map(|p| p["ProcessId"].as_u64().map(|v| v as u32))
        .collect()
}

/// 停止运行中的应用实例：先 `--quit` 优雅退出（应用自 0.5.7/0.6.8 起支持），
/// 超时兜底 taskkill /F。全部实例消失（或本来就没在跑）返回 Ok。
pub fn stop_app(id: &str) -> Result<()> {
    let exe = app_exe_path(id)?;
    if !exe.is_file() {
        return Ok(()); // 未安装，无需停止
    }
    if is_app_running(&exe) {
        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("--quit");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = cmd.spawn();
        // 等待优雅退出（最多 3s）
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if !is_app_running(&exe) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
    // 兜底强杀
    for pid in app_pids(&exe) {
        let mut cmd = std::process::Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/F"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = cmd.output();
    }
    // 确认消失（最多 3s）
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if app_pids(&exe).is_empty() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err(anyhow!("应用 {id} 进程未能停止"))
}

/// 静默自启配置的应用（单开：已在运行跳过；有当前项目则透传 --project-path）。
/// 进程列表只查一次；调用方负责在后台线程执行（不阻塞宿主 GUI 首屏）。
pub fn autostart_apps(ids: &[String], project: Option<&std::path::Path>) {
    if ids.is_empty() {
        return;
    }
    let running = running_exe_paths();
    for id in ids {
        let Ok(exe) = app_exe_path(id) else { continue };
        if !exe.is_file() || is_running_in(&exe, &running) {
            continue;
        }
        let mut cmd = std::process::Command::new(&exe);
        if let Some(p) = project {
            cmd.arg("--project-path").arg(p);
        }
        // 静默自启语义：透传 --background，由应用自行决定是否以无窗口形态驻留
        cmd.arg("--background");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = cmd.spawn();
    }
}

// ---------------------------------------------------------------- 通知与联动（0.6.10）

/// 向应用发送 notify 通知（解耦约定：`<exe> notify key=value ...`；
/// 应用自行决定如何处理，宿主零知识。异步拉起、失败静默）
pub fn notify_app(id: &str, pairs: &[String]) {
    let Ok(exe) = app_exe_path(id) else { return };
    if !exe.is_file() {
        return;
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("notify").args(pairs);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd.spawn();
}

/// 宿主退出时联动关闭所有正在运行的已安装应用：
/// 先对全部运行实例广播 --quit 优雅退出，等待后兜底 taskkill
pub fn stop_all_running_apps() {
    let Ok(installed) = list_installed() else { return };
    let running = running_exe_paths();
    let mut alive: Vec<String> = Vec::new();
    for app in &installed {
        let Ok(exe) = app_exe_path(&app.id) else { continue };
        if exe.is_file() && is_running_in(&exe, &running) {
            // 广播优雅退出信号
            let mut cmd = std::process::Command::new(&exe);
            cmd.arg("--quit");
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            let _ = cmd.spawn();
            alive.push(app.id.clone());
        }
    }
    if alive.is_empty() {
        return;
    }
    // 等优雅退出（最多 2s），存活者逐个强杀
    std::thread::sleep(std::time::Duration::from_secs(2));
    for id in &alive {
        let _ = stop_app(id);
    }
}
