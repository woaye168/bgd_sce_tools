//! BGD_SCE_TOOLS 主入口：Tauri 命令注册与应用状态管理

mod builder;
mod config;
mod project;

use config::BgdConfig;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

/// 应用状态：当前项目路径 + 监听句柄
struct AppState {
    project: Mutex<Option<PathBuf>>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

impl AppState {
    fn bgd_root(&self) -> Result<PathBuf, String> {
        let guard = self.project.lock().map_err(|e| e.to_string())?;
        let root = guard.as_ref().ok_or("尚未选择项目")?;
        let bgd = root.join(".bgd");
        if !bgd.is_dir() {
            return Err("项目缺少 .bgd 目录，请先初始化项目".to_string());
        }
        Ok(bgd)
    }
}

/// 向前端推送日志
fn emit_log(app: &AppHandle, source: &str, line: &str) {
    let _ = app.emit(
        "bgd-log",
        serde_json::json!({ "source": source, "line": line }),
    );
}

fn load_cfg(bgd_root: &Path) -> Result<BgdConfig, String> {
    BgdConfig::load(bgd_root).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- 项目命令

#[tauri::command]
fn select_project(app: AppHandle, state: State<AppState>, path: String) -> Result<Vec<String>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("所选路径不是有效目录".to_string());
    }
    *state.project.lock().map_err(|e| e.to_string())? = Some(root);
    let app_data = app.path().app_config_dir().map_err(|e| e.to_string())?;
    project::add_recent(&app_data, &path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_current_project(state: State<AppState>) -> Option<String> {
    state
        .project
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
fn get_recent_projects(app: AppHandle) -> Vec<String> {
    app.path()
        .app_config_dir()
        .map(|d| project::load_recent(&d).projects)
        .unwrap_or_default()
}

// ---------------------------------------------------------------- 应用设置

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

fn load_proxy(app: &AppHandle) -> String {
    app_data_dir(app)
        .map(|d| project::load_settings(&d).proxy)
        .unwrap_or_default()
}

#[tauri::command]
fn get_app_settings(app: AppHandle) -> Result<project::AppSettings, String> {
    Ok(project::load_settings(&app_data_dir(&app)?))
}

#[tauri::command]
fn save_app_settings(app: AppHandle, settings: project::AppSettings) -> Result<(), String> {
    project::save_settings(&app_data_dir(&app)?, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_project_info(state: State<AppState>) -> Result<serde_json::Value, String> {
    let bgd_root = state.bgd_root()?;
    match BgdConfig::load(&bgd_root) {
        Ok(cfg) => Ok(serde_json::json!({
            "initialized": true,
            "framework_version": cfg.framework_version,
            "framework_repo": cfg.framework_repo,
        })),
        Err(_) => Ok(serde_json::json!({ "initialized": false })),
    }
}

// ---------------------------------------------------------------- 配置命令

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<BgdConfig, String> {
    let bgd_root = state.bgd_root()?;
    load_cfg(&bgd_root)
}

#[tauri::command]
fn save_config(state: State<AppState>, config: BgdConfig) -> Result<(), String> {
    let bgd_root = state.bgd_root()?;
    config.save(&bgd_root).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- 构建命令

#[tauri::command]
fn full_build(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let log = |line: &str| emit_log(&app, "build", line);
    builder::build_all(&bgd_root, &cfg, &log).map_err(|e| e.to_string())
}

#[tauri::command]
fn clean_build(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let log = |line: &str| emit_log(&app, "build", line);
    builder::clean(&bgd_root, &cfg, &log).map_err(|e| e.to_string())
}

#[tauri::command]
fn clean_logs(app: AppHandle, state: State<AppState>) -> Result<usize, String> {
    let bgd_root = state.bgd_root()?;
    let log = |line: &str| emit_log(&app, "build", line);
    builder::clean_logs(&bgd_root, &log).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- 监听命令

#[tauri::command]
fn start_watch(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    {
        let guard = state.watcher.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("监听已在运行中".to_string());
        }
    }
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let app_clone = app.clone();
    let log = move |line: &str| emit_log(&app_clone, "watch", line);
    let watcher = builder::start_watch(&bgd_root, &cfg, log).map_err(|e| e.to_string())?;
    *state.watcher.lock().map_err(|e| e.to_string())? = Some(watcher);
    Ok(())
}

#[tauri::command]
fn stop_watch(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
    if guard.take().is_some() {
        emit_log(&app, "watch", "[watch] 监听已停止");
    }
    Ok(())
}

#[tauri::command]
fn is_watching(state: State<AppState>) -> bool {
    state
        .watcher
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
}

// ---------------------------------------------------------------- 初始化 / 框架更新

#[tauri::command]
fn init_project(app: AppHandle, state: State<AppState>, path: String, repo: String) -> Result<String, String> {
    let root = PathBuf::from(&path);
    let log = |line: &str| emit_log(&app, "build", line);
    let proxy = load_proxy(&app);
    let msg = project::init_project(&root, &repo, &proxy, &log).map_err(|e| e.to_string())?;
    // 初始化完成后自动设为当前项目
    *state.project.lock().map_err(|e| e.to_string())? = Some(root);
    Ok(msg)
}

#[tauri::command]
fn check_framework_update(app: AppHandle, state: State<AppState>) -> Result<serde_json::Value, String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let proxy = load_proxy(&app);
    let latest = project::latest_framework_version(&cfg.framework_repo, &proxy).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "current": cfg.framework_version,
        "latest": latest,
    }))
}

#[tauri::command]
fn update_framework(app: AppHandle, state: State<AppState>) -> Result<String, String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let log = |line: &str| emit_log(&app, "build", line);
    let proxy = load_proxy(&app);
    let project_root = bgd_root.parent().ok_or("无法确定项目根目录")?.to_path_buf();
    project::update_framework(&project_root, &cfg.framework_repo, &proxy, &log).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- 入口

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            project: Mutex::new(None),
            watcher: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            select_project,
            get_current_project,
            get_recent_projects,
            get_project_info,
            get_config,
            save_config,
            full_build,
            clean_build,
            clean_logs,
            start_watch,
            stop_watch,
            is_watching,
            init_project,
            check_framework_update,
            update_framework,
            get_app_settings,
            save_app_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BGD_SCE_TOOLS");
}
