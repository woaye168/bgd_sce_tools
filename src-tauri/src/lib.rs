//! BGD_SCE_TOOLS 主入口：Tauri 命令注册与应用状态管理

pub mod apps;
pub mod builder;
pub mod config;
pub mod net;
pub mod project;
pub mod secret;
pub mod updater;

use config::BgdConfig;
use std::fs;
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

/// 写日志文件（按天滚动：.bgd/log/build-YYYY-MM-DD.log）。
/// 句柄按路径缓存复用（每行 open+close 在高频日志下开销明显），跨天/换项目时自动重开。
fn write_log_file(bgd_root: &Path, line: &str) {
    use std::io::Write;
    static LOG_FILE: std::sync::LazyLock<Mutex<Option<(PathBuf, fs::File)>>> =
        std::sync::LazyLock::new(|| Mutex::new(None));
    let log_dir = bgd_root.join("log");
    let _ = fs::create_dir_all(&log_dir);
    let date = format_date(std::time::SystemTime::now());
    let path = log_dir.join(format!("build-{date}.log"));
    let Ok(mut guard) = LOG_FILE.lock() else { return };
    let need_open = !matches!(guard.as_ref(), Some((p, _)) if *p == path);
    if need_open {
        *guard = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()
            .map(|f| (path.clone(), f));
    }
    if let Some((_, f)) = guard.as_mut() {
        let _ = writeln!(f, "{line}");
    }
}

/// 格式化 SystemTime 为 YYYY-MM-DD（UTC+8）
fn format_date(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        + 8 * 3600; // UTC+8
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// days since epoch -> (year, month, day)，公历算法
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
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
        let bgd_root_clone = bgd_root.clone();
        let log = move |line: &str| {
            emit_log(&app_clone, "watch", line);
            // 每次写日志时动态读 save_log（开关改了立即生效）
            let save_log = app_data_dir(&app_clone)
                .map(|d| project::load_settings(&d).save_log)
                .unwrap_or(false);
            if save_log {
                write_log_file(&bgd_root_clone, line);
            }
        };
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

    // 项目切换广播（0.6.10）：notify 所有静默自启应用（异步、失败静默，应用自治处理）
    {
        let ids = project::load_settings(&app_data).auto_start_apps;
        if !ids.is_empty() {
            let pair = format!("project_path={}", root.display());
            for id in ids {
                apps::notify_app(&id, std::slice::from_ref(&pair));
            }
        }
    }

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
/// 用 CREATE_NO_WINDOW 隐藏子进程控制台，避免 GUI 应用启动时闪黑窗。
/// 成功后落标记文件（应用配置目录 .path_registered，内容为安装目录），
/// 后续启动跳过 PowerShell 自检（安装目录变化时标记内容不匹配会自动重检）。
fn ensure_path_registered() {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let Ok(exe) = std::env::current_exe() else { return };
    let Some(dir) = exe.parent() else { return };
    let dir_str = dir.to_string_lossy().to_string();
    let marker = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("com.bgd.sce-tools")
        .join(".path_registered");
    if fs::read_to_string(&marker).map(|s| s.trim() == dir_str).unwrap_or(false) {
        return;
    }
    // PowerShell：PATH 不含安装目录才追加（幂等）
    let script = format!(
        "$i='{}'; $p=(Get-ItemProperty -Path 'HKCU:\\Environment' -Name Path -ErrorAction SilentlyContinue).Path; if (($p -split ';') -notcontains $i) {{ $n = if ($p) {{ $p + ';' + $i }} else {{ $i }}; Set-ItemProperty -Path 'HKCU:\\Environment' -Name Path -Value $n }}",
        dir_str.replace('\'', "''")
    );
    if Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
    {
        let _ = fs::write(&marker, &dir_str);
    }
}

fn load_proxy(app: &AppHandle) -> String {
    app_data_dir(app)
        .map(|d| project::load_settings(&d).proxy)
        .unwrap_or_default()
}

/// 读取 GitHub Token（私有仓库的框架/插件/自我更新认证）
fn load_token(app: &AppHandle) -> String {
    app_data_dir(app)
        .map(|d| project::load_settings(&d).github_token)
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

/// 生效的资源路径规则（内建默认 + bgd.json res_rules 覆盖合成；设置界面展示用）
#[tauri::command]
fn get_effective_res_rules(state: State<AppState>) -> Result<Vec<builder::rules::ResRule>, String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    Ok(builder::rules::effective_rules(&cfg))
}

/// 工具内建默认配置（设置界面「恢复默认」按钮用）
#[tauri::command]
fn get_config_defaults() -> BgdConfig {
    BgdConfig::defaults()
}

// ---------------------------------------------------------------- 构建命令

#[tauri::command]
fn full_build(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let save_log = app_data_dir(&app)
        .map(|d| project::load_settings(&d).save_log)
        .unwrap_or(false);
    let log = |line: &str| {
        emit_log(&app, "build", line);
        if save_log {
            write_log_file(&bgd_root, line);
        }
    };
    builder::build_all(&bgd_root, &cfg, &log).map_err(|e| e.to_string())
}

#[tauri::command]
fn clean_build(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let save_log = app_data_dir(&app)
        .map(|d| project::load_settings(&d).save_log)
        .unwrap_or(false);
    let log = |line: &str| {
        emit_log(&app, "build", line);
        if save_log {
            write_log_file(&bgd_root, line);
        }
    };
    builder::clean(&bgd_root, &cfg, &log).map_err(|e| e.to_string())
}

#[tauri::command]
fn clean_logs(app: AppHandle, state: State<AppState>) -> Result<usize, String> {
    let bgd_root = state.bgd_root()?;
    let log = |line: &str| emit_log(&app, "build", line);
    builder::clean_logs(&bgd_root, &log).map_err(|e| e.to_string())
}

/// 清理编辑器引擎日志（logs / logs_subprocess / logs_temp）
#[tauri::command]
fn clean_engine_logs(state: State<AppState>) -> Result<usize, String> {
    let bgd_root = state.bgd_root()?;
    let root = bgd_root.parent().ok_or("无法推导项目根")?;
    project::clean_engine_logs(root).map_err(|e| e.to_string())
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
    let token = load_token(&app);
    let msg = project::init_project(&root, &repo, &proxy, &token, force, &log).map_err(|e| e.to_string())?;
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
    let token = load_token(&app);
    let latest = project::latest_framework_version(&cfg.framework_repo, &proxy, &token).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "current": cfg.framework_version,
        "latest": latest,
    }))
}

#[tauri::command]
async fn update_framework(app: AppHandle, state: State<'_, AppState>) -> Result<project::UpdateReport, String> {
    let bgd_root = state.bgd_root()?;
    let cfg = load_cfg(&bgd_root)?;
    let proxy = load_proxy(&app);
    let token = load_token(&app);
    let project_root = bgd_root.parent().ok_or("无法确定项目根目录")?.to_path_buf();
    let repo = cfg.framework_repo.clone();

    // 进度回调（线程安全 channel）：writer 移入更新线程，reader 在 async 侧定时转发前端事件
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(u64, Option<u64>)>();
    let on_progress = move |downloaded: u64, total: Option<u64>| {
        let _ = tx.send((downloaded, total));
    };

    // 下载+三路对比是 CPU/IO 混合的同步代码（log 闭包非 Send），放阻塞线程执行
    let app1 = app.clone();
    let handle = tokio::task::spawn_blocking(move || -> Result<project::UpdateReport, String> {
        let log = |line: &str| emit_log(&app1, "build", line);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("创建运行时失败: {e}"))?;
        rt.block_on(project::update_framework_async(
            &project_root, &repo, &proxy, &token, &log, on_progress,
        ))
        .map_err(|e| e.to_string())
    });

    // 转发进度事件到前端（async 侧，不阻塞）
    let app2 = app.clone();
    let forward = tokio::spawn(async move {
        while let Some((downloaded, total)) = rx.recv().await {
            let _ = app2.emit(
                "framework-download-progress",
                serde_json::json!({ "downloaded": downloaded, "total": total }),
            );
        }
    });

    let report = handle.await.map_err(|e| format!("更新任务失败: {e}"))?;
    forward.abort();
    report
}

// ---------------------------------------------------------------- 应用（WeGame 模式）

/// 拉取应用市场清单（0.7.1 起：只拉 registry 骨架并播种自启默认，不做元数据补全——
/// 补全由前端逐应用调 enrich_app 异步完成，保证应用页即时显示）
#[tauri::command]
async fn fetch_app_registry(app: AppHandle, url: String) -> Result<apps::AppRegistry, String> {
    let proxy = load_proxy(&app);
    let token = load_token(&app);
    let registry = apps::fetch_registry_async(&url, &proxy, &token)
        .await
        .map_err(|e| e.to_string())?;

    // 静默自启下发默认：registry 声明 default_auto_start 的应用，
    // 用户未勾选且未显式取消过时播种进本机配置；用户本机记忆（含取消）优先，不再覆盖
    if let Ok(dir) = app.path().app_config_dir() {
        let mut settings = project::load_settings(&dir);
        let mut changed = false;
        for a in &registry.apps {
            if a.default_auto_start
                && !settings.auto_start_apps.iter().any(|id| id == &a.id)
                && !settings.auto_start_disabled.iter().any(|id| id == &a.id)
            {
                settings.auto_start_apps.push(a.id.clone());
                changed = true;
            }
        }
        if changed {
            project::save_settings(&dir, &settings).map_err(|e| e.to_string())?;
        }
    }
    Ok(registry)
}

/// 单应用元数据补全（应用页逐应用异步加载：版本/描述/作者/asset名/版本说明）
#[tauri::command]
async fn enrich_app(app: AppHandle, mut app_info: apps::AppInfo) -> Result<apps::AppInfo, String> {
    let proxy = load_proxy(&app);
    let token = load_token(&app);
    apps::enrich_app_async(&mut app_info, &proxy, &token)
        .await
        .map_err(|e| e.to_string())?;
    Ok(app_info)
}

/// 安装应用（下载 exe 到 <宿主>/apps/{id}/）
#[tauri::command]
async fn install_app(app: AppHandle, state: State<'_, AppState>, app_info: apps::AppInfo) -> Result<(), String> {
    let proxy = load_proxy(&app);
    let token = load_token(&app);

    // 升级前若实例在运行（静默自启/手动打开的），先停止再覆盖——否则 exe 被锁定写入失败。
    // 先 --quit 优雅退出（应用支持时），兜底 taskkill；装完若之前在跑则按自启配置重启。
    // 进程枚举要 spawn PowerShell（~1s），stop_app 含轮询等待，全部移出异步运行时线程
    let id = app_info.id.clone();
    let was_running = tauri::async_runtime::spawn_blocking(move || {
        let running = apps::is_app_running(&apps::app_exe_path(&id)?);
        if running {
            apps::stop_app(&id)?;
        }
        Ok::<bool, anyhow::Error>(running)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("停止运行中的应用失败: {e}"))?;
    // 下载进度回调：向前端发事件（AppPage 显示 已下载/总大小）
    let on_progress = crate::net::progress_emitter(&app, "app-download-progress", Some(app_info.id.clone()));
    apps::install_app_async(&app_info, &proxy, &token, on_progress)
        .await
        .map_err(|e| e.to_string())?;

    if was_running {
        let auto = app
            .path()
            .app_config_dir()
            .map(|dir| project::load_settings(&dir).auto_start_apps.iter().any(|id| id == &app_info.id))
            .unwrap_or(false);
        let exe = apps::app_exe_path(&app_info.id).map_err(|e| e.to_string())?;
        let mut cmd = std::process::Command::new(&exe);
        let project_root = state.project.lock().map_err(|e| e.to_string())?.clone();
        if let Some(root) = &project_root {
            cmd.arg("--project-path").arg(root);
        }
        if auto {
            cmd.arg("--background");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let _ = cmd.spawn();

        // 升级完成后再发一次 notify（带 project_path）：触发应用 refresh——静默驻留实例
        // 也会立即执行 bridge dll 重部署等自同步（否则需打开界面/重启才生效）
        if let Some(root) = &project_root {
            let id = app_info.id.clone();
            let pair = format!("project_path={}", root.display());
            // 等应用完成启动（~1s）后通知，避免应用尚未就绪
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1200));
                apps::notify_app(&id, std::slice::from_ref(&pair));
            });
        }
    }
    Ok(())
}

/// 卸载应用
#[tauri::command]
fn uninstall_app(app_id: String) -> Result<(), String> {
    apps::uninstall_app(&app_id).map_err(|e| e.to_string())
}

/// 列出已安装应用
#[tauri::command]
fn get_installed_apps() -> Result<Vec<apps::InstalledApp>, String> {
    apps::list_installed().map_err(|e| e.to_string())
}

/// 启动应用 EXE（有当前项目则传 --project-path）。
/// 不做单开拦截：bgd_appsdk 应用自带单实例——第二实例向运行中实例发「唤起窗口」信号后
/// 自行退出，宿主一律放行由应用去重唤出（静默自启驻留的应用也借此打开界面）。
#[tauri::command]
fn start_app(state: State<AppState>, app_id: String) -> Result<(), String> {
    let app_exe = apps::app_exe_path(&app_id).map_err(|e| e.to_string())?;
    if !app_exe.is_file() {
        return Err(format!("应用 {app_id} 未安装"));
    }
    let mut cmd = std::process::Command::new(&app_exe);
    // 有当前项目则传 --project-path（应用可选实现）
    if let Some(root) = state.project.lock().map_err(|e| e.to_string())?.as_ref() {
        cmd.arg("--project-path").arg(root);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.spawn().map_err(|e| format!("启动应用失败: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------- 自我更新

/// 检查是否有新版本（私有仓库需 token，从应用设置读取）
#[tauri::command]
fn check_self_update(app: AppHandle, current: String) -> Result<updater::SelfUpdateInfo, String> {
    let proxy = load_proxy(&app);
    let token = load_token(&app);
    updater::check_self_update(&current, &proxy, &token).map_err(|e| e.to_string())
}

/// 下载最新安装包并启动安装器（安装器会自动关闭并替换当前程序）
#[tauri::command]
async fn start_self_update(app: AppHandle) -> Result<(), String> {
    let proxy = load_proxy(&app);
    let token = load_token(&app);
    let on_progress = crate::net::progress_emitter(&app, "self-update-progress", None);
    updater::start_self_update_async(&proxy, &token, on_progress)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- 入口

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            project: Mutex::new(None),
            watcher: Mutex::new(None),
        })
        .setup(|app| {
            // 确保安装目录在用户 PATH（CLI 可用）
            ensure_path_registered();
            let handle = app.handle().clone();
            let Ok(dir) = handle.path().app_config_dir() else {
                return Ok(());
            };
            // 静默自启（0.6.6 引入，0.6.7 异步化）：后台线程执行，不阻塞 GUI 首屏
            {
                let settings = project::load_settings(&dir);
                if !settings.auto_start_apps.is_empty() {
                    let proj = project::load_recent(&dir)
                        .projects
                        .first()
                        .map(PathBuf::from)
                        .filter(|p| p.is_dir());
                    let ids = settings.auto_start_apps.clone();
                    std::thread::spawn(move || {
                        apps::autostart_apps(&ids, proj.as_deref());
                    });
                }
            }
            // 启动恢复：默认选中最近项目；若上次监听为开则自动开启
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
            get_config_defaults,
            get_effective_res_rules,
            full_build,
            clean_build,
            clean_logs,
            clean_engine_logs,
            start_watch,
            stop_watch,
            is_watching,
            init_project,
            check_framework_update,
            update_framework,
            get_app_settings,
            save_app_settings,
            fetch_app_registry,
            enrich_app,
            install_app,
            uninstall_app,
            get_installed_apps,
            start_app,
            check_self_update,
            start_self_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building BGD_SCE_TOOLS")
        .run(|_app, event| {
            // 宿主退出：联动关闭所有正在运行的已安装应用（先 --quit 优雅退出，兜底强杀）
            if let tauri::RunEvent::Exit = event {
                apps::stop_all_running_apps();
            }
        });
}
