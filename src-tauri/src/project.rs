//! 项目管理：初始化项目、框架下载/更新、最近项目记录

use crate::config::BgdConfig;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
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

fn codeload_url(repo: &str) -> String {
    format!("https://codeload.github.com/{repo}/zip/refs/heads/main")
}

/// 下载框架仓库 zip 并解压到临时目录，返回 <解压根>/template 路径
fn download_framework_template(repo: &str, proxy: &str) -> Result<PathBuf> {
    let repo = if repo.is_empty() { DEFAULT_FRAMEWORK_REPO } else { repo };
    let url = codeload_url(repo);
    let resp = http_client(proxy)?
        .get(&url)
        .send()
        .with_context(|| format!("框架下载失败: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("框架下载失败: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes()?;

    let tmp = std::env::temp_dir().join(format!("bgd-framework-{}", std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("框架 zip 解压失败")?;
    archive.extract(&tmp)?;

    // zip 根目录为 <repo>-main/
    let root = fs::read_dir(&tmp)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.is_dir())
        .ok_or_else(|| anyhow!("框架 zip 内容异常"))?;
    let template = root.join("template");
    if !template.is_dir() {
        return Err(anyhow!("框架仓库缺少 template/ 目录"));
    }
    Ok(template)
}

/// 查询框架最新版本（最新 release 的 tag_name；无 release 时返回 None）
pub fn latest_framework_version(repo: &str, proxy: &str) -> Result<Option<String>> {
    let repo = if repo.is_empty() { DEFAULT_FRAMEWORK_REPO } else { repo };
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
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

// ---------------------------------------------------------------- 初始化 / 更新

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

/// 初始化项目：下载框架 -> 生成 .bgd/、.emmyrc.json、.gitignore
/// 若 .bgd 已存在则备份为 .bgd.bak-<时间戳>
pub fn init_project(project_root: &Path, repo: &str, proxy: &str, log: &dyn Fn(&str)) -> Result<String> {
    log("开始下载框架...");
    let template = download_framework_template(repo, proxy)?;
    let tpl_bgd = template.join(".bgd");
    if !tpl_bgd.is_dir() {
        return Err(anyhow!("框架模板缺少 template/.bgd 目录"));
    }

    let dest_bgd = project_root.join(".bgd");
    if dest_bgd.exists() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let backup = project_root.join(format!(".bgd.bak-{ts}"));
        fs::rename(&dest_bgd, &backup)?;
        log(&format!(".bgd 已存在，已备份为 {}", backup.display()));
    }

    copy_dir_recursive(&tpl_bgd, &dest_bgd)?;
    log("已生成 .bgd 框架目录");

    // 项目根级模板文件（.emmyrc.json 覆盖写入；.gitignore 不存在才写入）
    let emmyrc = template.join(".emmyrc.json");
    if emmyrc.exists() {
        fs::copy(&emmyrc, project_root.join(".emmyrc.json"))?;
        log("已写入 .emmyrc.json");
    }
    let gitignore = template.join(".gitignore");
    if gitignore.exists() && !project_root.join(".gitignore").exists() {
        fs::copy(&gitignore, project_root.join(".gitignore"))?;
        log("已写入 .gitignore");
    }

    // 记录框架来源与版本
    let cfg_path = dest_bgd.join("bgd.json");
    if cfg_path.exists() {
        let mut cfg = BgdConfig::load(&dest_bgd)?;
        cfg.framework_repo = if repo.is_empty() {
            DEFAULT_FRAMEWORK_REPO.to_string()
        } else {
            repo.to_string()
        };
        if let Ok(Some(ver)) = latest_framework_version(repo, proxy) {
            cfg.framework_version = ver;
        }
        cfg.save(&dest_bgd)?;
    }

    Ok("项目初始化完成".to_string())
}

/// 更新框架：只覆盖 .bgd/libs 与 .emmyrc.json，不动 src；更新 framework_version
pub fn update_framework(project_root: &Path, repo: &str, proxy: &str, log: &dyn Fn(&str)) -> Result<String> {
    let dest_bgd = project_root.join(".bgd");
    if !dest_bgd.is_dir() {
        return Err(anyhow!("项目尚未初始化（缺少 .bgd 目录）"));
    }
    log("开始下载最新框架...");
    let template = download_framework_template(repo, proxy)?;

    let src_libs = template.join(".bgd").join("libs");
    let dest_libs = dest_bgd.join("libs");
    if dest_libs.exists() {
        fs::remove_dir_all(&dest_libs)?;
    }
    copy_dir_recursive(&src_libs, &dest_libs)?;
    log("已更新 .bgd/libs");

    // 模板同步更新（构建行为相关）
    let src_templates = template.join(".bgd").join("templates");
    let dest_templates = dest_bgd.join("templates");
    if src_templates.is_dir() {
        if dest_templates.exists() {
            fs::remove_dir_all(&dest_templates)?;
        }
        copy_dir_recursive(&src_templates, &dest_templates)?;
        log("已更新 .bgd/templates");
    }

    let emmyrc = template.join(".emmyrc.json");
    if emmyrc.exists() {
        fs::copy(&emmyrc, project_root.join(".emmyrc.json"))?;
        log("已更新 .emmyrc.json");
    }

    let mut cfg = BgdConfig::load(&dest_bgd)?;
    if let Ok(Some(ver)) = latest_framework_version(repo, proxy) {
        cfg.framework_version = ver.clone();
        cfg.save(&dest_bgd)?;
        Ok(format!("框架已更新到 {ver}"))
    } else {
        cfg.save(&dest_bgd)?;
        Ok("框架已更新（版本未知，已同步 main 分支）".to_string())
    }
}
