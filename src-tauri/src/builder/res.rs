//! res 资源同步：资源文件复制到引擎目录 / 清理
//!
//! 规则驱动（builder/rules.rs 单一来源）：类型集合/磁盘落位前缀全部来自规则，
//! 本文件只负责复制与清理的编排。disk_prefix 与全部路径配置统一相对项目根（0.9.1）。

use super::{code_set_dir, is_excluded, project_rel, rules, LogFn};
use crate::config::BgdConfig;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// 资源类型 -> 引擎目标子路径（相对项目根；规则 disk_prefix 解析）
fn res_target_subdir(res_type: &str, code_set: &str, cfg: &BgdConfig, bgd_root: &Path) -> Option<String> {
    let project = rules::project_name_cached(bgd_root);
    rules::effective_rules(cfg)
        .into_iter()
        .find(|r| r.res_type == res_type)
        .map(|r| rules::resolve_template(&r.disk_prefix, code_set, cfg, &project))
}

/// res 相对路径（"/res/<type>/<path...>"）→ 引擎侧落位绝对路径（未知类型 None）
pub(crate) fn res_disk_path(
    bgd_root: &Path,
    cfg: &BgdConfig,
    code_set: &str,
    rel: &str,
) -> Option<PathBuf> {
    let parts: Vec<&str> = rel.trim_start_matches('/').splitn(3, '/').collect();
    if parts.len() < 3 {
        return None;
    }
    let target_sub = res_target_subdir(parts[1], code_set, cfg, bgd_root)?;
    Some(
        BgdConfig::project_root_of(bgd_root)
            .join(&target_sub)
            .join(parts[2]),
    )
}

/// 同步单个 res 资源文件到引擎目录（二进制原样复制）
pub(super) fn sync_res_file(
    bgd_root: &Path,
    cfg: &BgdConfig,
    code_set: &str,
    rel: &str,
    src_abs: &Path,
    log: &LogFn,
) -> Result<usize> {
    let (_, excludes) = code_set_dir(code_set, cfg);
    if is_excluded(&project_rel(code_set, rel, cfg), excludes) {
        log(&format!("[skip] res excluded: [{code_set}] {rel}"));
        return Ok(0);
    }
    let Some(dest) = res_disk_path(bgd_root, cfg, code_set, rel) else {
        log(&format!("[warn] 未知资源类型: [{code_set}] {rel}"));
        return Ok(0);
    };
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src_abs, &dest)?;
    let project_root = BgdConfig::project_root_of(bgd_root);
    let rel_dest = dest.strip_prefix(&project_root).unwrap_or(&dest);
    log(&format!(
        "[ok] res: [{code_set}] {rel} -> {}",
        rel_dest.to_string_lossy().replace('\\', "/")
    ));
    Ok(1)
}

/// 清除同步到引擎目录的资源文件（规则 disk_prefix 全表 × 双 code set）
pub(super) fn clean_res_files(bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) -> Result<()> {
    let project_root = BgdConfig::project_root_of(bgd_root);
    let project = rules::project_name_cached(bgd_root);
    for code_set in ["libs", "game"] {
        for rule in rules::effective_rules(cfg) {
            let sub = rules::resolve_template(&rule.disk_prefix, code_set, cfg, &project);
            let path = project_root.join(&sub);
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
                log(&format!("  -> 已删除资源 {sub}"));
            }
        }
    }
    Ok(())
}
