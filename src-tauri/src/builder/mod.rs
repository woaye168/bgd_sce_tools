//! 构建核心：全量构建（白名单）、增量构建、清理（入口还原）、API 聚合生成、
//! 根 init.lua 渲染、入口合并（分界标记）、emmyrc/gitignore 合并、文件监听
//!
//! 模块划分：rewrite=模块名/res 路径改写；res=资源同步与清理；merge=API 聚合/init 渲染/
//! 入口合并/emmyrc/gitignore/AGENTS 同步；watch=文件监听与去重、监听状态；本文件保留
//! 主流程编排（build_all/clean/build_code_set/build_one_file）与跨模块共享的路径/排除辅助。

mod merge;
mod res;
mod rewrite;
pub mod rules;
mod watch;

pub use merge::{
    merge_emmyrc, merge_gitignore, regen_api_aggregations, render_root_init, sync_agents_md,
    update_entrance,
};
pub use rewrite::rewrite_lua;
pub use rules::write_path_rules;
pub use watch::{
    is_watching, remove_watch_state, start_watch, watch_state_path, write_watch_state, WatchState,
    WATCH_STATE_FILE,
};

use crate::config::BgdConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 日志回调：构建/监听过程产生的每一行日志
pub type LogFn<'a> = dyn Fn(&str) + Send + 'a;

const TEXT_EXTS: [&str; 4] = ["lua", "html", "css", "js"];
/// 构建输入白名单：code set 根下只有这些子目录进产物流程
const BUILD_SUBDIRS: [&str; 4] = ["server", "client", "common", "res"];

// ---------------------------------------------------------------- 排除规则

/// rel 形如 "/server/GameServer.lua"；排除项如 "types"、"client/eg"（目录边界匹配）
pub fn is_excluded(rel: &str, excludes: &[String]) -> bool {
    let norm = format!("/{}", rel.trim_start_matches('/').replace('\\', "/"));
    for excl in excludes {
        let e = format!("/{}", excl.trim_matches(['/', '\\']).replace('\\', "/"));
        if norm == e || norm.starts_with(&format!("{e}/")) {
            return true;
        }
    }
    false
}

fn in_whitelist(rel: &str) -> bool {
    BUILD_SUBDIRS.iter().any(|d| rel.starts_with(&format!("/{d}/")))
}

// ---------------------------------------------------------------- 文件复制

fn ext_of(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

/// 单文件复制：lua 改写模块名；html/css/js 包装为 lua 字符串模块；其余二进制原样。
/// skip_rewrite：命中 rewrite_excludes 的文件正常进产物但跳过模块名/res 替换
///（规则见 rules::effective_rewrite_excludes；path_rules.lua 盖戳即靠此保护——
/// 其内容为运行时终值，替换会损坏）
fn transform_and_write(src: &Path, dest: &Path, side: &str, cfg: &BgdConfig, bgd_root: &Path, skip_rewrite: bool, log: &LogFn) -> Result<bool> {
    let ext = ext_of(src);
    let dest = if TEXT_EXTS.contains(&ext.as_str()) && ext != "lua" {
        dest.with_extension("lua")
    } else {
        dest.to_path_buf()
    };

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if TEXT_EXTS.contains(&ext.as_str()) {
        let content = fs::read_to_string(src)
            .with_context(|| format!("无法读取源文件: {}", src.display()))?;
        let content = if ext == "lua" && !skip_rewrite {
            let content = rewrite::rewrite_lua(&content, side, cfg);
            rewrite::rewrite_res_paths(&content, bgd_root, cfg, log)
        } else if ext == "lua" {
            content
        } else {
            format!("return [===[{content}]===]")
        };
        fs::write(&dest, content)?;
    } else {
        fs::copy(src, &dest)?;
    }
    Ok(true)
}

fn dest_for(rel: &str, side: &str, code_set: &str, bgd_root: &Path, cfg: &BgdConfig) -> PathBuf {
    let key = format!("{code_set}_{side}_target");
    let target = match key.as_str() {
        "libs_server_target" => &cfg.libs_server_target,
        "libs_client_target" => &cfg.libs_client_target,
        "game_server_target" => &cfg.game_server_target,
        _ => &cfg.game_client_target,
    };
    cfg.abs(bgd_root, target).join(rel.trim_start_matches('/'))
}

fn code_set_dir<'a>(code_set: &str, cfg: &'a BgdConfig) -> (&'a str, &'a [String]) {
    if code_set == "libs" {
        (&cfg.libs_dir, &cfg.libs_excludes)
    } else {
        (&cfg.game_dir, &cfg.game_excludes)
    }
}

fn sides_for(rel: &str) -> Vec<&'static str> {
    if rel.starts_with("/server/") {
        vec!["server"]
    } else if rel.starts_with("/client/") {
        vec!["client"]
    } else {
        vec!["server", "client"] // common 双端复制
    }
}

fn rel_of(src_abs: &Path, code_set: &str, bgd_root: &Path, cfg: &BgdConfig) -> Result<String> {
    let (dir, _) = code_set_dir(code_set, cfg);
    let src_root = cfg.abs(bgd_root, dir);
    Ok(format!(
        "/{}",
        src_abs
            .strip_prefix(&src_root)
            .with_context(|| format!("文件不在源目录内: {}", src_abs.display()))?
            .to_string_lossy()
            .replace('\\', "/")
    ))
}

/// 构建单个文件（白名单子目录内），返回产出数量
pub fn build_one_file(
    bgd_root: &Path,
    cfg: &BgdConfig,
    code_set: &str,
    src_abs: &Path,
    log: &LogFn,
) -> Result<usize> {
    let rel = rel_of(src_abs, code_set, bgd_root, cfg)?;
    if !in_whitelist(&rel) {
        return Ok(0);
    }
    let (_, excludes) = code_set_dir(code_set, cfg);
    if is_excluded(&rel, excludes) {
        log(&format!("[skip] excluded: [{code_set}] {rel}"));
        return Ok(0);
    }

    // res 资源目录：.lua 不同步（它是代码），其余资源文件同步到引擎目录
    if rel.starts_with("/res/") {
        if ext_of(src_abs) == "lua" {
            return Ok(0);
        }
        return res::sync_res_file(bgd_root, cfg, code_set, &rel, src_abs, log);
    }

    // 替换排除（rewrite_excludes）：正常进产物但跳过模块名/res 替换
    let skip_rewrite = is_excluded(&rel, &rules::effective_rewrite_excludes(cfg));
    if skip_rewrite {
        log(&format!("[ok] rewrite-excluded: [{code_set}] {rel}"));
    }

    let mut count = 0;
    for side in sides_for(&rel) {
        let dest = dest_for(&rel, side, code_set, bgd_root, cfg);
        if transform_and_write(src_abs, &dest, side, cfg, bgd_root, skip_rewrite, log)? {
            log(&format!("[ok] {side}: [{code_set}] {rel}"));
            count += 1;
        }
    }
    Ok(count)
}

// ---------------------------------------------------------------- 全量构建 / 清理

pub fn build_code_set(code_set: &str, bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) -> Result<usize> {
    let (dir, _) = code_set_dir(code_set, cfg);
    let src_root = cfg.abs(bgd_root, dir);
    if !src_root.is_dir() {
        log(&format!("[info] {code_set} 源目录不存在: {}", src_root.display()));
        return Ok(0);
    }
    let mut count = 0;
    for sub in BUILD_SUBDIRS {
        let subdir = src_root.join(sub);
        if !subdir.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&subdir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                count += build_one_file(bgd_root, cfg, code_set, entry.path(), log)?;
            }
        }
    }
    Ok(count)
}

pub fn build_all(bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) -> Result<()> {
    log("===== 开始全量构建 =====");
    regen_api_aggregations(bgd_root, cfg, log)?;
    rules::write_path_rules(bgd_root, cfg, log)?; // 路径规则盖戳（先于构建，当次进产物）
    let libs_count = build_code_set("libs", bgd_root, cfg, log)?;
    let game_count = build_code_set("game", bgd_root, cfg, log)?;
    render_root_init(bgd_root, cfg, log)?;
    update_entrance("server", bgd_root, cfg, log)?;
    update_entrance("client", bgd_root, cfg, log)?;
    merge_emmyrc(bgd_root, cfg, log)?;
    merge_gitignore(bgd_root, cfg, log)?;
    sync_agents_md(bgd_root, cfg, log)?;
    log(&format!(
        "===== 构建完成！框架文件: {libs_count}，游戏文件: {game_count} ====="
    ));
    Ok(())
}

pub fn clean(bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) -> Result<()> {
    log("===== 清除构建产物 =====");
    for target in [
        &cfg.libs_server_target,
        &cfg.libs_client_target,
        &cfg.game_server_target,
        &cfg.game_client_target,
    ] {
        let abs = cfg.abs(bgd_root, target);
        if abs.is_dir() {
            fs::remove_dir_all(&abs)?;
            log(&format!("  -> 已删除 {}", abs.display()));
        } else {
            log(&format!("  -> 目录不存在，跳过 {}", abs.display()));
        }
    }
    merge::restore_entrance("server", bgd_root, cfg, log)?;
    merge::restore_entrance("client", bgd_root, cfg, log)?;
    res::clean_res_files(bgd_root, cfg, log)?;
    log("===== 清理完成 =====");
    Ok(())
}

/// 清理 .bgd/log 下的 .log 文件，返回删除数量
pub fn clean_logs(bgd_root: &Path, log: &LogFn) -> Result<usize> {
    let log_dir = bgd_root.join("log");
    let mut deleted = 0;
    if log_dir.is_dir() {
        for entry in fs::read_dir(&log_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "log") {
                fs::remove_file(&path)?;
                deleted += 1;
            }
        }
    }
    log(&format!("日志清理完成，共删除 {deleted} 个 .log 文件"));
    Ok(deleted)
}
