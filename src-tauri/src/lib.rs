//! BGD_SCE_TOOLS 主入口：Tauri 命令注册与应用状态管理

pub mod builder;
pub mod config;
pub mod project;

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

/// 启动监听（内部复用）：当前项目已初始化才启动；已在监听则先停
fn try_start_watch(app: &AppHandle, state: &AppState) -> Result<(), String> {
    // 已初始化才允许监听
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    {
        let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            // 先停旧监听（切换项目或重复启动）
            let _ = guard.take();
        }
        let app_clone = app.clone();
        let log = move |line: &str| emit_log(&app_clone, "watch", line);
        let watcher = builder::start_watch(&bgd_root, &cfg, log).map_err(|e| e.to_string())?;
        *guard = Some(watcher);
    }
    Ok(())
}

#[tauri::command]
fn select_project(app: AppHandle, state: State<AppState>, path: String) -> Result<Vec<String>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("所选路径不是有效目录".to_string());
    }
    // 切换项目：先停掉旧项目的监听
    {
        let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
        if guard.take().is_some() {
            emit_log(&app, "watch", "[watch] 已切换项目，旧监听已停止");
        }
    }
    *state.project.lock().map_err(|e| e.to_string())? = Some(root.clone());
    let app_data = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let recent = project::add_recent(&app_data, &path).map_err(|e| e.to_string())?;

    // 若监听开关为开且新项目已初始化，自动对新项目开启监听
    let watch_enabled = app_data_dir(&app)
        .map(|d| project::load_settings(&d).watch_enabled)
        .unwrap_or(false);
    if watch_enabled && root.join(".bgd").is_dir() {
        if let Err(e) = try_start_watch(&app, &state) {
            emit_log(&app, "watch", &format!("[error] 自动开启监听失败: {e}"));
        } else {
            emit_log(&app, "watch", "[watch] 已对新项目自动开启监听");
            let bgd_root = root.join(".bgd");
            builder::write_watch_state(&bgd_root, &root);
        }
    }
    Ok(recent)
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

/// 确保安装目录在用户 PATH 中（首次启动/更新后自检写入，替代不可靠的 NSIS 钩子）
/// 用 CREATE_NO_WINDOW 隐藏子进程控制台，避免 GUI 应用启动时闪黑窗
fn ensure_path_registered() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let Ok(exe) = std::env::current_exe() else { return };
    let Some(dir) = exe.parent() else { return };
    let dir_str = dir.to_string_lossy().to_string();
    // PowerShell：PATH 不含安装目录才追加（幂等）
    let script = format!(
        "$i='{}'; $p=(Get-ItemProperty -Path 'HKCU:\\Environment' -Name Path -ErrorAction SilentlyContinue).Path; if (($p -split ';') -notcontains $i) {{ $n = if ($p) {{ $p + ';' + $i }} else {{ $i }}; Set-ItemProperty -Path 'HKCU:\\Environment' -Name Path -Value $n }}",
        dir_str.replace('\'', "''")
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn load_proxy(app: &AppHandle) -> String {
    app_data_dir(app)
        .map(|d| project::load_settings(&d).proxy)
        .unwrap_or_default()
}

/// 持久化监听开关状态
fn persist_watch_enabled(app: &AppHandle, enabled: bool) {
    if let Ok(dir) = app_data_dir(app) {
        let mut settings = project::load_settings(&dir);
        settings.watch_enabled = enabled;
        let _ = project::save_settings(&dir, &settings);
    }
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
    // 初始化真实标志：init.lock 存在（删除 lock 即视为未初始化，可重新初始化）
    if !bgd_root.join("init.lock").exists() {
        return Ok(serde_json::json!({ "initialized": false }));
    }
    match BgdConfig::load(&bgd_root) {
        Ok(cfg) => Ok(serde_json::json!({
            "initialized": true,
            "framework_version": cfg.framework_version,
            "framework_repo": cfg.framework_repo,
        })),
        Err(_) => Ok(serde_json::json!({ "initialized": true })),
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
    try_start_watch(&app, &state)?;
    persist_watch_enabled(&app, true);
    // 写入监听状态文件（供 CLI check-watch 跨进程判断）
    if let Ok(bgd_root) = state.bgd_root() {
        if let Some(project_root) = bgd_root.parent() {
            builder::write_watch_state(&bgd_root, project_root);
        }
    }
    Ok(())
}

#[tauri::command]
fn stop_watch(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
    if guard.take().is_some() {
        emit_log(&app, "watch", "[watch] 监听已停止");
        if let Ok(bgd_root) = state.bgd_root() {
            builder::remove_watch_state(&bgd_root);
        }
    }
    persist_watch_enabled(&app, false);
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
fn init_project(app: AppHandle, state: State<AppState>, path: String, repo: String, force: bool) -> Result<String, String> {
    let root = PathBuf::from(&path);
    // force 重新初始化前先停掉当前监听（notify watcher 监视 .bgd/libs 与 .bgd/src，
    // 不停掉会导致重命名/删除 .bgd 时 os error 5 拒绝访问）
    {
        let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
        if guard.take().is_some() {
            emit_log(&app, "build", "[init] 已停止当前监听（重新初始化需要）");
        }
    }
    let log = |line: &str| emit_log(&app, "build", line);
    let proxy = load_proxy(&app);
    let msg = project::init_project(&root, &repo, &proxy, force, &log).map_err(|e| e.to_string())?;
    // 初始化完成后自动设为当前项目
    *state.project.lock().map_err(|e| e.to_string())? = Some(root.clone());
    // 若监听开关为开，自动恢复监听（init 前已停掉，此处按用户选择的状态恢复）
    let watch_enabled = app_data_dir(&app)
        .map(|d| project::load_settings(&d).watch_enabled)
        .unwrap_or(false);
    if watch_enabled && root.join(".bgd").is_dir() {
        if let Err(e) = try_start_watch(&app, &state) {
            emit_log(&app, "watch", &format!("[error] 初始化后自动恢复监听失败: {e}"));
        } else if let Ok(bgd_root) = state.bgd_root() {
            builder::write_watch_state(&bgd_root, &root);
            emit_log(&app, "watch", "[watch] 监听已自动恢复");
        }
    }
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
fn update_framework(app: AppHandle, state: State<AppState>) -> Result<project::UpdateReport, String> {
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
        .setup(|app| {
            // 确保安装目录在用户 PATH（CLI 可用）
            ensure_path_registered();
            // 启动恢复：默认选中最近项目；若上次监听为开则自动开启
            let handle = app.handle().clone();
            let Ok(dir) = handle.path().app_config_dir() else {
                return Ok(());
            };
            let recent = project::load_recent(&dir);
            let Some(first) = recent.projects.first() else {
                return Ok(());
            };
            let root = PathBuf::from(first);
            if !root.is_dir() {
                return Ok(());
            }
            let state = handle.state::<AppState>();
            *state.project.lock().map_err(|e| e.to_string())? = Some(root.clone());
            let settings = project::load_settings(&dir);
            if settings.watch_enabled && root.join(".bgd").is_dir() {
                if try_start_watch(&handle, &state).is_ok() {
                    let bgd_root = root.join(".bgd");
                    builder::write_watch_state(&bgd_root, &root);
                }
            }
            Ok(())
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
