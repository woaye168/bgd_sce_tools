//! 文件监听：notify 事件驱动 + 同文件事件去重；监听状态文件（跨进程共享）

use super::merge::{
    merge_emmyrc, merge_gitignore, regen_api_aggregations, render_root_init, sync_agents_md,
    update_entrance,
};
use super::{
    build_one_file, code_set_dir, dest_for, ext_of, in_whitelist, is_excluded, project_rel, rel_of,
    res, rules, sides_for, LogFn,
};
use crate::config::BgdConfig;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------- 监听状态（跨进程共享）

/// 监听状态文件：监听开启时写入，关闭时删除。供 CLI check-watch 跨进程判断。
pub const WATCH_STATE_FILE: &str = ".watch_state.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WatchState {
    pub pid: u32,
    pub started_at: u64,
    pub project: String,
}

pub fn watch_state_path(bgd_root: &Path) -> PathBuf {
    bgd_root.join(WATCH_STATE_FILE)
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// 写入监听状态（监听开启时调用）
pub fn write_watch_state(bgd_root: &Path, project_root: &Path) {
    let state = WatchState {
        pid: std::process::id(),
        started_at: now_ts(),
        project: project_root.to_string_lossy().into_owned(),
    };
    if let Ok(text) = serde_json::to_string_pretty(&state) {
        let _ = fs::write(watch_state_path(bgd_root), text);
    }
}

/// 删除监听状态（监听停止时调用）
pub fn remove_watch_state(bgd_root: &Path) {
    let _ = fs::remove_file(watch_state_path(bgd_root));
}

/// 判断指定 PID 的进程是否为「本工具的监听进程」：存活且映像名与当前 exe 一致
/// （只校 PID 会被 PID 复用误判——状态文件里的旧 PID 可能已被无关进程占用）
fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let want = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
            .unwrap_or_default();
        if let Ok(out) = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // CSV 首字段为带引号的映像名（如 "bgd_sce_tools.exe"）
            let image = stdout
                .lines()
                .next()
                .and_then(|l| l.split(',').next())
                .unwrap_or("")
                .trim_matches('"')
                .to_lowercase();
            let image = image.trim_end_matches(".exe");
            return !want.is_empty() && image == want;
        }
        false
    }
    #[cfg(not(windows))]
    {
        Path::new(&format!("/proc/{pid}")).exists()
    }
}

/// 查询项目当前是否处于监听中（供 CLI check-watch）。
/// 状态文件存在且 PID 存活且映像名为本工具 => true；否则清理状态文件后 false。
pub fn is_watching(bgd_root: &Path) -> bool {
    let path = watch_state_path(bgd_root);
    let Ok(text) = fs::read_to_string(&path) else {
        return false;
    };
    let Ok(state) = serde_json::from_str::<WatchState>(&text) else {
        return false;
    };
    if pid_alive(state.pid) {
        true
    } else {
        // 上次异常退出未清理，顺手清掉
        let _ = fs::remove_file(&path);
        false
    }
}

// ---------------------------------------------------------------- 监听

fn identify_code_set(path: &Path, bgd_root: &Path, cfg: &BgdConfig) -> Option<&'static str> {
    for code_set in ["libs", "game"] {
        let (dir, _) = code_set_dir(code_set, cfg);
        let src_root = cfg.abs(bgd_root, dir);
        if path.starts_with(&src_root) {
            return Some(code_set);
        }
    }
    None
}

fn delete_outputs(path: &Path, bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) -> Result<()> {
    let Some(code_set) = identify_code_set(path, bgd_root, cfg) else {
        return Ok(());
    };
    let rel = rel_of(path, code_set, bgd_root, cfg)?;
    if !in_whitelist(&rel) {
        return Ok(());
    }
    let (_, excludes) = code_set_dir(code_set, cfg);
    if is_excluded(&project_rel(code_set, &rel, cfg), excludes) {
        return Ok(());
    }
    // res 资源：落位在规则 disk_prefix 目录（不在产物 target 目录），按规则定位删除
    if rel.starts_with("/res/") {
        if let Some(disk) = res::res_disk_path(bgd_root, cfg, code_set, &rel) {
            if disk.exists() {
                fs::remove_file(&disk)?;
                log(&format!("[deleted] res: [{code_set}] {rel}"));
            }
        }
        return Ok(());
    }
    for side in sides_for(&rel) {
        let mut dest = dest_for(&rel, side, code_set, bgd_root, cfg);
        let ext = ext_of(&dest);
        if ["html", "css", "js"].contains(&ext.as_str()) {
            dest = dest.with_extension("lua");
        }
        if dest.exists() {
            fs::remove_file(&dest)?;
            log(&format!("[deleted] {side}: [{code_set}] {rel}"));
        }
    }
    Ok(())
}

/// 配置热更新（0.9.1）：轮询 bgd.json mtime，变化即重读替换监听持有的配置快照。
/// 返回是否发生了重载。配置变更后补调 write_path_rules（盖戳与 res_rules 同步，内容一致防抖）。
fn maybe_reload_cfg(
    cfg: &mut BgdConfig,
    mtime: &mut Option<std::time::SystemTime>,
    bgd_root: &Path,
    log: &LogFn,
) -> bool {
    let path = bgd_root.join("bgd.json");
    let cur = fs::metadata(&path).and_then(|m| m.modified()).ok();
    if cur == *mtime {
        return false;
    }
    *mtime = cur;
    match BgdConfig::load(bgd_root) {
        Ok(new_cfg) => {
            *cfg = new_cfg;
            log("[watch] 检测到 bgd.json 变更，已热更新配置（历史产物不追溯，全量构建后完全生效）");
            if let Err(e) = rules::write_path_rules(bgd_root, cfg, log) {
                log(&format!("[warn] path_rules 盖戳刷新失败: {e}"));
            }
            true
        }
        Err(e) => {
            log(&format!("[warn] bgd.json 变更但解析失败，沿用旧配置: {e}"));
            false
        }
    }
}

/// 监听单文件路由：白名单文件增量构建；根级元文件走专门流程
fn handle_file(path: &Path, deleted: bool, bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) {
    let Some(code_set) = identify_code_set(path, bgd_root, cfg) else {
        return;
    };
    let Ok(rel) = rel_of(path, code_set, bgd_root, cfg) else {
        return;
    };

    // 根级 init.lua：模板重渲染
    if rel == "/init.lua" {
        let _ = render_root_init(bgd_root, cfg, log);
        return;
    }
    // 根级 emmyrc/gitignore 片段：重新合并
    if rel == "/.emmyrc.json" {
        let _ = merge_emmyrc(bgd_root, cfg, log);
        return;
    }
    if rel == "/.gitignore" {
        let _ = merge_gitignore(bgd_root, cfg, log);
        return;
    }
    // src/AGENTS.md：同步到项目根
    if rel == "/AGENTS.md" && code_set == "game" {
        let _ = sync_agents_md(bgd_root, cfg, log);
        return;
    }
    // entrance 文件：重新合并入口
    if rel.starts_with("/entrance/") {
        let _ = update_entrance("server", bgd_root, cfg, log);
        let _ = update_entrance("client", bgd_root, cfg, log);
        return;
    }
    // api 目录变动：重新生成聚合并立即构建
    if rel.contains("/api/") {
        if let Ok(gen_files) = regen_api_aggregations(bgd_root, cfg, log) {
            for gen in gen_files {
                if let Some(gen_set) = identify_code_set(&gen, bgd_root, cfg) {
                    let _ = build_one_file(bgd_root, cfg, gen_set, &gen, log);
                }
            }
        }
    }
    // 普通白名单文件
    if deleted {
        let _ = delete_outputs(path, bgd_root, cfg, log);
    } else {
        let _ = build_one_file(bgd_root, cfg, code_set, path, log);
    }
}

/// 启动监听（notify 事件驱动，带事件去重）。返回 watcher 句柄，由调用方持有；drop 即停止。
///
/// Windows 编辑器保存文件（尤其原子保存）会对一次保存派发多个事件，直接逐事件处理
/// 会产生重复构建/重复日志。这里把同一文件在短时间窗口内的事件聚合为一次：
/// 只要窗口结束时文件仍存在就构建一次，不存在则删除一次。
pub fn start_watch<L>(bgd_root: &Path, cfg: &BgdConfig, log: L) -> Result<RecommendedWatcher>
where
    L: Fn(&str) + Send + Sync + 'static,
{
    use std::collections::HashMap;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const DEBOUNCE_MS: u64 = 300;

    let bgd_root = bgd_root.to_path_buf();
    let cfg = cfg.clone();
    let log = Arc::new(log);

    let (tx, rx) = mpsc::channel::<Event>();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })?;

    for code_set in ["libs", "game"] {
        let (dir, _) = code_set_dir(code_set, &cfg);
        let src_root = cfg.abs(&bgd_root, dir);
        if src_root.is_dir() {
            watcher.watch(&src_root, RecursiveMode::Recursive)?;
            (log)(&format!("[watch] 正在监听: {}", src_root.display()));
        }
    }
    (log)("[watch] 监听已启动");

    // 后台线程：收集事件并按文件去重后批量处理
    let bgd_cb = bgd_root.clone();
    let cfg_cb = cfg.clone();
    let log_cb = log.clone();
    std::thread::spawn(move || {
        // path -> (deleted, last_seen)
        let mut pending: HashMap<PathBuf, (bool, Instant)> = HashMap::new();
        // 配置快照（bgd.json 变更时热更新；0.9.1 前监听全程持有启动时快照导致配置改动不生效）
        let mut cfg_live = cfg_cb;
        let mut cfg_mtime = fs::metadata(bgd_cb.join("bgd.json"))
            .and_then(|m| m.modified())
            .ok();
        // 处理已静默超过窗口的文件（每次收事件也顺带调用，防事件持续流入时永不超时、到期项积压）
        fn process_ready(
            pending: &mut HashMap<PathBuf, (bool, Instant)>,
            bgd_root: &Path,
            cfg: &BgdConfig,
            log: &LogFn,
        ) {
            let now = Instant::now();
            let ready: Vec<PathBuf> = pending
                .iter()
                .filter(|(_, (_, t))| now.duration_since(*t) >= Duration::from_millis(DEBOUNCE_MS))
                .map(|(p, _)| p.clone())
                .collect();
            for path in ready {
                let (deleted, _) = pending.remove(&path).unwrap_or((false, now));
                // 最终状态以文件实际存在性为准（避免误删）
                let actually_deleted = deleted && !path.exists();
                handle_file(&path, actually_deleted, bgd_root, cfg, log);
            }
        }
        let log_line = |s: &str| (log_cb)(s);
        loop {
            // 每轮检查配置变更（事件驱动与超时轮询都经过这里，最迟 300ms 生效）
            maybe_reload_cfg(&mut cfg_live, &mut cfg_mtime, &bgd_cb, &log_line);
            match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                Ok(event) => {
                    for path in event.paths {
                        if path.is_dir() {
                            continue;
                        }
                        let deleted = matches!(event.kind, EventKind::Remove(_));
                        pending
                            .entry(path)
                            .and_modify(|e| {
                                e.0 = deleted;
                                e.1 = Instant::now();
                            })
                            .or_insert((deleted, Instant::now()));
                    }
                    process_ready(&mut pending, &bgd_cb, &cfg_live, &log_line);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    process_ready(&mut pending, &bgd_cb, &cfg_live, &log_line)
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(watcher)
}
