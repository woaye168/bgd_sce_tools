//! 项目管理：初始化（含锁）、框架下载/三路哈希增量更新、最近项目记录、应用设置

use crate::config::BgdConfig;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const DEFAULT_FRAMEWORK_REPO: &str = "woaye168/bgd_sce_framework";

// ---------------------------------------------------------------- 应用设置

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// HTTP 代理地址，如 http://127.0.0.1:7897；留空表示直连
    #[serde(default)]
    pub proxy: String,
}

pub fn load_settings(app_data_dir: &Path) -> AppSettings {
    let path = app_data_dir.join("settings.json");
    fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_settings(app_data_dir: &Path, settings: &AppSettings) -> Result<()> {
    let path = app_data_dir.join("settings.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

// ---------------------------------------------------------------- 最近项目

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    pub projects: Vec<String>,
}

pub fn recent_file(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("recent_projects.json")
}

pub fn load_recent(app_data_dir: &Path) -> RecentProjects {
    let path = recent_file(app_data_dir);
    fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn add_recent(app_data_dir: &Path, project: &str) -> Result<Vec<String>> {
    let mut recent = load_recent(app_data_dir);
    recent.projects.retain(|p| p != project);
    recent.projects.insert(0, project.to_string());
    recent.projects.truncate(10);
    let path = recent_file(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&recent)?)?;
    Ok(recent.projects)
}

// ---------------------------------------------------------------- GitHub 下载

fn http_client(proxy: &str) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder().user_agent("BGD_SCE_TOOLS");
    let proxy = proxy.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).context("代理地址无效")?);
    }
    builder.build().context("无法创建 HTTP 客户端")
}

fn effective_repo(repo: &str) -> &str {
    if repo.is_empty() { DEFAULT_FRAMEWORK_REPO } else { repo }
}

/// 下载 zip 并解压到临时目录，返回 <解压根>/template 路径
fn download_and_extract(url: &str, proxy: &str) -> Result<PathBuf> {
    let resp = http_client(proxy)?
        .get(url)
        .send()
        .with_context(|| format!("下载失败: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("下载失败: HTTP {} ({url})", resp.status()));
    }
    let bytes = resp.bytes()?;

    let tmp = std::env::temp_dir().join(format!("bgd-framework-{}-{}", std::process::id(), now_ts()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("zip 解压失败")?;
    archive.extract(&tmp)?;

    let root = fs::read_dir(&tmp)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.is_dir())
        .ok_or_else(|| anyhow!("zip 内容异常"))?;
    let template = root.join("template");
    if !template.is_dir() {
        return Err(anyhow!("框架仓库缺少 template/ 目录"));
    }
    Ok(template)
}

/// 下载框架 main 分支快照
fn download_framework_template(repo: &str, proxy: &str) -> Result<PathBuf> {
    let url = format!("https://codeload.github.com/{}/zip/refs/heads/main", effective_repo(repo));
    download_and_extract(&url, proxy)
}

/// 下载框架指定 tag 快照（用于重建旧版基准）
fn download_framework_tag(repo: &str, tag: &str, proxy: &str) -> Result<PathBuf> {
    let url = format!("https://codeload.github.com/{}/zip/refs/tags/{tag}", effective_repo(repo));
    download_and_extract(&url, proxy)
}

/// 查询框架最新版本（最新 release 的 tag_name；无 release 时返回 None）
pub fn latest_framework_version(repo: &str, proxy: &str) -> Result<Option<String>> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", effective_repo(repo));
    let resp = http_client(proxy)?.get(&url).send()?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(anyhow!("查询框架版本失败: HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json()?;
    Ok(json["tag_name"].as_str().map(|s| s.to_string()))
}

// ---------------------------------------------------------------- 工具函数

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let rel = entry.path().strip_prefix(src)?;
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_file(src: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dest)?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    // 文本文件统一换行符为 LF 后哈希（避免 CRLF/LF 差异导致误报更新/冲突）
    let normalized = if bytes.contains(&0) {
        bytes
    } else {
        String::from_utf8_lossy(&bytes)
            .replace("\r\n", "\n")
            .into_bytes()
    };
    Ok(format!("{:x}", Sha256::digest(&normalized)))
}

/// 计算 libs 目录下所有文件的 SHA256（键为 "{prefix}/<相对路径>"，正斜杠）
fn collect_libs_hashes(libs_dir: &Path, prefix: &str) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    if !libs_dir.is_dir() {
        return Ok(map);
    }
    for entry in walkdir::WalkDir::new(libs_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(libs_dir)?.to_string_lossy().replace('\\', "/");
            map.insert(format!("{prefix}/{rel}"), sha256_file(entry.path())?);
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------- 框架状态（增量更新基准）

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkState {
    pub framework_repo: String,
    pub framework_version: String,
    pub installed_at: u64,
    /// ".bgd/libs/xxx" -> sha256
    pub files: BTreeMap<String, String>,
}

impl FrameworkState {
    pub fn load(bgd_root: &Path) -> Result<Self> {
        let path = bgd_root.join(".framework_state.json");
        let text = fs::read_to_string(&path).context("无本地框架基准")?;
        Ok(serde_json::from_str(&text)?)
    }
}

fn save_state(project_root: &Path, cfg: &BgdConfig) -> Result<()> {
    let files = collect_libs_hashes(&project_root.join(".bgd").join("libs"), ".bgd/libs")?;
    let state = FrameworkState {
        framework_repo: cfg.framework_repo.clone(),
        framework_version: cfg.framework_version.clone(),
        installed_at: now_ts(),
        files,
    };
    let path = project_root.join(".bgd").join(".framework_state.json");
    fs::write(&path, serde_json::to_string_pretty(&state)?)?;
    Ok(())
}

// ---------------------------------------------------------------- 初始化

/// 初始化项目：下载框架 -> 生成 .bgd/ -> 合并生成项目根配置 -> 写 init.lock 与更新基准
/// 若存在 init.lock 且未 force，则拒绝
pub fn init_project(project_root: &Path, repo: &str, proxy: &str, force: bool, log: &crate::builder::LogFn) -> Result<String> {
    let dest_bgd = project_root.join(".bgd");
    let lock_path = dest_bgd.join("init.lock");
    if lock_path.exists() && !force {
        return Err(anyhow!(
            "项目已初始化（存在 .bgd/init.lock）。如需重新初始化，请确认后强制执行（旧 .bgd 会自动备份）"
        ));
    }

    log("开始下载框架...");
    let template = download_framework_template(repo, proxy)?;
    let tpl_bgd = template.join(".bgd");
    if !tpl_bgd.is_dir() {
        return Err(anyhow!("框架模板缺少 template/.bgd 目录"));
    }

    if dest_bgd.exists() {
        let backup = project_root.join(format!(".bgd.bak-{}", now_ts()));
        fs::rename(&dest_bgd, &backup)?;
        log(&format!(".bgd 已存在，已备份为 {}", backup.display()));
    }

    copy_dir_recursive(&tpl_bgd, &dest_bgd)?;
    log("已生成 .bgd 框架目录");

    // 合并生成项目根 .emmyrc.json / .gitignore
    let cfg = BgdConfig::load(&dest_bgd)?;
    crate::builder::merge_emmyrc(&dest_bgd, &cfg, log)?;
    crate::builder::merge_gitignore(&dest_bgd, &cfg, log)?;

    // 记录框架来源与版本（tag 去 v 前缀）
    let mut cfg = cfg;
    cfg.framework_repo = effective_repo(repo).to_string();
    if let Ok(Some(ver)) = latest_framework_version(repo, proxy) {
        cfg.framework_version = ver.trim_start_matches('v').to_string();
    }
    cfg.save(&dest_bgd)?;

    // 初始化锁
    let lock = serde_json::json!({
        "initialized_at": now_ts(),
        "tool": env!("CARGO_PKG_NAME"),
        "tool_version": env!("CARGO_PKG_VERSION"),
        "framework_repo": cfg.framework_repo,
        "framework_version": cfg.framework_version,
    });
    fs::write(&lock_path, serde_json::to_string_pretty(&lock)?)?;
    log("已生成初始化锁 .bgd/init.lock");

    // 记录框架文件基准（供增量更新三路对比）
    save_state(project_root, &cfg)?;
    log("已记录框架文件基准 .bgd/.framework_state.json");

    Ok("项目初始化完成".to_string())
}

// ---------------------------------------------------------------- 增量更新（三路哈希对比）

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateReport {
    pub updated: usize,
    pub added: usize,
    pub removed: usize,
    pub kept_local: usize,
    pub conflicts: Vec<String>,
    pub notes: Vec<String>,
    pub version: String,
}

/// 更新框架：只处理 .bgd/libs（三路哈希对比），随后重新合并生成项目根配置
pub fn update_framework(project_root: &Path, repo: &str, proxy: &str, log: &crate::builder::LogFn) -> Result<UpdateReport> {
    let dest_bgd = project_root.join(".bgd");
    if !dest_bgd.is_dir() {
        return Err(anyhow!("项目尚未初始化（缺少 .bgd 目录）"));
    }
    let mut report = UpdateReport::default();

    // 1. 基准（base）
    let baseline: BTreeMap<String, String>;
    let mut conservative = false;
    match FrameworkState::load(&dest_bgd) {
        Ok(state) => {
            log(&format!("基准: 上次安装的 {} 个框架文件", state.files.len()));
            baseline = state.files;
        }
        Err(_) => {
            let cfg = BgdConfig::load(&dest_bgd)?;
            let ver = cfg.framework_version.trim_start_matches('v').to_string();
            if !ver.is_empty() {
                log(&format!("无本地基准，尝试下载旧版 v{ver} 重建..."));
                match download_framework_tag(&cfg.framework_repo, &format!("v{ver}"), proxy) {
                    Ok(tpl) => {
                        baseline = collect_libs_hashes(&tpl.join(".bgd").join("libs"), ".bgd/libs")?;
                        log(&format!("已用旧版 v{ver} 重建基准（{} 个文件）", baseline.len()));
                    }
                    Err(e) => {
                        conservative = true;
                        baseline = BTreeMap::new();
                        report.notes.push(format!("旧版 v{ver} 下载失败，进入保守模式: {e}"));
                    }
                }
            } else {
                conservative = true;
                baseline = BTreeMap::new();
                report.notes.push("无版本信息，进入保守模式（只增新文件，不覆盖不删除）".to_string());
            }
        }
    }

    // 2. 新版（remote）
    log("开始下载最新框架...");
    let template = download_framework_template(repo, proxy)?;
    let remote_libs = template.join(".bgd").join("libs");
    let remote = collect_libs_hashes(&remote_libs, ".bgd/libs")?;

    // 3. 三路对比
    for (path, remote_hash) in &remote {
        let local_path = project_root.join(path);
        let local_hash = if local_path.is_file() {
            Some(sha256_file(&local_path)?)
        } else {
            None
        };
        let remote_src = remote_libs.join(path.trim_start_matches(".bgd/libs/"));

        let conflict_file = |report: &mut UpdateReport, log: &crate::builder::LogFn| -> Result<()> {
            report.conflicts.push(path.clone());
            let new_name = format!("{}.framework-new", local_path.file_name().unwrap_or_default().to_string_lossy());
            copy_file(&remote_src, &local_path.with_file_name(new_name))?;
            log(&format!("[conflict] {path}（本地保留，新版另存 .framework-new）"));
            Ok(())
        };

        match baseline.get(path) {
            Some(base_hash) => {
                if local_hash.as_deref() == Some(base_hash.as_str()) {
                    // 用户未改：安全更新
                    if base_hash != remote_hash && !conservative {
                        copy_file(&remote_src, &local_path)?;
                        report.updated += 1;
                        log(&format!("[updated] {path}"));
                    }
                } else if base_hash == remote_hash {
                    // 用户改了，上游没改：保留
                    report.kept_local += 1;
                } else if local_hash.as_deref() == Some(remote_hash.as_str()) {
                    // 用户已手动改成与上游一致：无需处理
                } else {
                    conflict_file(&mut report, log)?;
                }
            }
            None => {
                // 上游新增
                if local_path.exists() && local_hash.as_deref() != Some(remote_hash.as_str()) {
                    conflict_file(&mut report, log)?;
                } else if !local_path.exists() {
                    copy_file(&remote_src, &local_path)?;
                    report.added += 1;
                    log(&format!("[added] {path}"));
                }
            }
        }
    }

    // 上游删除的文件
    if !conservative {
        for (path, base_hash) in &baseline {
            if remote.contains_key(path) {
                continue;
            }
            let local_path = project_root.join(path);
            if !local_path.exists() {
                continue;
            }
            let local_hash = sha256_file(&local_path)?;
            if &local_hash == base_hash {
                fs::remove_file(&local_path)?;
                report.removed += 1;
                log(&format!("[removed] {path}"));
            } else {
                report.kept_local += 1;
                report.notes.push(format!("上游已删除但本地有修改，已保留: {path}"));
            }
        }
    } else {
        report.notes.push("保守模式：未执行覆盖与删除".to_string());
    }

    // 4. 项目根配置重新合并生成（libs 片段可能已变化）
    let cfg = BgdConfig::load(&dest_bgd)?;
    crate::builder::merge_emmyrc(&dest_bgd, &cfg, log)?;
    crate::builder::merge_gitignore(&dest_bgd, &cfg, log)?;

    // 5. 版本与基准回写
    let mut cfg = cfg;
    if let Ok(Some(ver)) = latest_framework_version(repo, proxy) {
        cfg.framework_version = ver.trim_start_matches('v').to_string();
    }
    cfg.save(&dest_bgd)?;
    report.version = cfg.framework_version.clone();
    save_state(project_root, &cfg)?;

    log(&format!(
        "框架更新完成：更新 {}，新增 {}，删除 {}，保留本地 {}，冲突 {}",
        report.updated, report.added, report.removed, report.kept_local, report.conflicts.len()
    ));
    Ok(report)
}
