//! res 资源同步：五类资源文件复制到引擎目录 / 清理

use super::{code_set_dir, is_excluded, LogFn};
use crate::config::BgdConfig;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// res 资源类型 -> 引擎目标子路径（相对项目根）
/// image/particle/sound/spine/sprites 五类，前缀按 code_set 区分
fn res_target_subdir(res_type: &str, code_set: &str) -> Option<String> {
    let prefix = if code_set == "libs" { "bgd_libs_client" } else { "bgd_game_client" };
    match res_type {
        "image" => Some(format!("ui/image/image/{prefix}")),
        "particle" => Some(format!("res/effect/{prefix}")),
        "sound" => Some(format!("res/sound/{prefix}")),
        "spine" => Some(format!("ui/spine/{prefix}")),
        "sprites" => Some(format!("ui/image/sprites/{prefix}")),
        _ => None,
    }
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
    if is_excluded(rel, excludes) {
        log(&format!("[skip] res excluded: [{code_set}] {rel}"));
        return Ok(0);
    }
    // rel 形如 /res/<type>/<path...>
    let parts: Vec<&str> = rel.trim_start_matches('/').splitn(3, '/').collect();
    if parts.len() < 3 {
        return Ok(0);
    }
    let res_type = parts[1];
    let sub_path = parts[2];
    let Some(target_sub) = res_target_subdir(res_type, code_set) else {
        log(&format!("[warn] 未知资源类型: [{code_set}] {rel}"));
        return Ok(0);
    };
    let project_root = bgd_root.parent().unwrap_or(bgd_root);
    let dest = project_root.join(&target_sub).join(sub_path);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src_abs, &dest)?;
    log(&format!("[ok] res: [{code_set}] {rel} -> {target_sub}/{sub_path}"));
    Ok(1)
}

/// 清除同步到引擎目录的资源文件（ui/image/、res/effect/ 等下的 bgd_* 目录）
pub(super) fn clean_res_files(bgd_root: &Path, log: &LogFn) -> Result<()> {
    let project_root = bgd_root.parent().unwrap_or(bgd_root);
    for code_set in ["libs", "game"] {
        let prefix = if code_set == "libs" { "bgd_libs_client" } else { "bgd_game_client" };
        for sub in [
            format!("ui/image/image/{prefix}"),
            format!("res/effect/{prefix}"),
            format!("res/sound/{prefix}"),
            format!("ui/spine/{prefix}"),
            format!("ui/image/sprites/{prefix}"),
        ] {
            let path = project_root.join(&sub);
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
                log(&format!("  -> 已删除资源 {sub}"));
            }
        }
    }
    Ok(())
}
