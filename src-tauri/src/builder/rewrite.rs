//! 模块名改写与 res 资源路径替换（含存在性检查警告）
//!
//! res 规则单一来源 = builder/rules.rs（内建默认 + bgd.json res_rules 覆盖）；
//! 源码前缀（'libs'/'src'）从 cfg.libs_dir/game_dir 派生（0.9.1，不再硬编码）；
//! 本文件只负责「按规则改写 + 存在性告警 + 行级注解跳过」的编排，不含任何规则字面量。

use super::rules;
use super::LogFn;
use crate::config::BgdConfig;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

// ---------------------------------------------------------------- 正则缓存（F4）

/// 正则全局缓存：模式串 -> 编译结果（Regex 内部 Arc，clone 廉价）。
/// 每文件现编译 10+ 条正则是 0.9.0 的性能债，缓存后构建期只编译一次。
fn cached_regex(pattern: &str) -> Regex {
    static CACHE: LazyLock<Mutex<HashMap<String, Regex>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut guard = CACHE.lock().unwrap();
    guard
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).unwrap())
        .clone()
}

// ---------------------------------------------------------------- 行级注解跳过（0.9.1）

/// 计算「注解行的下一行」字节区间（升序）。annotation 为空 = 禁用。
pub(crate) fn skip_ranges(content: &str, annotation: &str) -> Vec<(usize, usize)> {
    if annotation.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut offset = 0usize;
    let mut annotated = false;
    for line in content.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        if annotated {
            ranges.push((start, offset));
            annotated = false;
        }
        if line.contains(annotation) {
            annotated = true;
        }
    }
    ranges
}

/// 判断字节偏移是否落在跳过区间内（区间数极小，线性扫描即可）
pub(crate) fn is_skipped(offset: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|&(s, e)| offset >= s && offset < e)
}

// ---------------------------------------------------------------- 模块名改写

/// 把引号内的源码模块名前缀改写为运行时根名：'libs.xxx' -> 'bgd_libs_server.xxx' 等。
/// 源码前缀从 cfg.libs_dir/game_dir 目录名派生（与 merge.rs 根 init 渲染同源）。
/// skips：行级注解跳过的字节区间，命中的匹配不替换。
pub fn rewrite_lua(content: &str, side: &str, cfg: &BgdConfig, skips: &[(usize, usize)]) -> String {
    let (libs_key, game_key) = if side == "server" {
        (&cfg.libs_server_target, &cfg.game_server_target)
    } else {
        (&cfg.libs_client_target, &cfg.game_client_target)
    };
    let libs_root = Path::new(libs_key)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let game_root = Path::new(game_key)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    // 'libs.xxx' / 'libs' 两种形式都要改写（裸 require('libs') 加载运行时根）
    let re_libs = cached_regex(&format!(
        r#"(['"]){}(['".])"#,
        regex::escape(&rules::libs_prefix(cfg))
    ));
    let re_src = cached_regex(&format!(
        r#"(['"]){}(['".])"#,
        regex::escape(&rules::src_prefix(cfg))
    ));
    let content = re_libs.replace_all(content, |caps: &Captures| {
        if is_skipped(caps.get(0).unwrap().start(), skips) {
            return caps[0].to_string();
        }
        if &caps[2] == "." {
            format!("{}{}.", &caps[1], libs_root)
        } else {
            format!("{}{}{}", &caps[1], libs_root, &caps[2])
        }
    });
    re_src
        .replace_all(&content, |caps: &Captures| {
            if is_skipped(caps.get(0).unwrap().start(), skips) {
                return caps[0].to_string();
            }
            if &caps[2] == "." {
                format!("{}{}.", &caps[1], game_root)
            } else {
                format!("{}{}{}", &caps[1], game_root, &caps[2])
            }
        })
        .into_owned()
}

// ---------------------------------------------------------------- res 资源路径替换

/// res 资源路径替换：把 '<前缀>/res/<类型>/...' 替换为运行时路径。
/// 规则驱动（rules.rs）：sound/spine 去扩展名、sprites 前缀 @{ProjectName} 等均为规则字段；
/// 各类型期望扩展名驱动存在性检查（黄色警告）。skips：行级注解跳过区间。
pub(super) fn rewrite_res_paths(
    content: &str,
    bgd_root: &Path,
    cfg: &BgdConfig,
    skips: &[(usize, usize)],
    log: &LogFn,
) -> String {
    let project_name = rules::project_name_cached(bgd_root);
    let mut result = content.to_string();
    let mut warnings: Vec<String> = Vec::new();

    let rules_list = rules::effective_rules(cfg);
    for rule in &rules_list {
        for (code_set, prefix) in [
            ("libs", rules::libs_prefix(cfg)),
            ("game", rules::src_prefix(cfg)),
        ] {
            // 按规则动态构造匹配：['"]<前缀>/res/<类型>/[^'"]+['"]
            let re = cached_regex(&format!(
                r#"['"]({}/res/{}/[^'"]+)['"]"#,
                regex::escape(&prefix),
                regex::escape(&rule.res_type)
            ));
            result = re
                .replace_all(&result, |caps: &Captures| {
                    if is_skipped(caps.get(0).unwrap().start(), skips) {
                        return caps[0].to_string();
                    }
                    let full_path = &caps[1]; // libs/res/image/armor_dark.png
                    let marker = format!("{prefix}/res/{}/", rule.res_type);
                    let rel_path = full_path.trim_start_matches(&marker);
                    let (new_path, warn) = rewrite_single_res_path(
                        rule, code_set, rel_path, &project_name, bgd_root, cfg,
                    );
                    if let Some(w) = warn {
                        warnings.push(w);
                    }
                    format!("'{new_path}'")
                })
                .into_owned();
        }
    }

    // 输出黄色警告（构建日志流）
    for w in warnings {
        log(&format!("[warn] {w}"));
    }

    result
}

/// 重写单个 res 路径（规则驱动），返回 (新路径, 可选警告)
fn rewrite_single_res_path(
    rule: &rules::ResRule,
    code_set: &str,
    rel_path: &str,
    project_name: &str,
    bgd_root: &Path,
    cfg: &BgdConfig,
) -> (String, Option<String>) {
    // 源文件基目录：cfg 配置的源码目录（0.9.1 起不再写死 .bgd/libs|.bgd/src）
    let (dir, _) = super::code_set_dir(code_set, cfg);
    let base = cfg.abs(bgd_root, dir).join("res").join(&rule.res_type);

    // 引用名：strip_ext_in_ref 时去掉期望扩展名（sound/spine）；sprites 目录无扩展名
    let src_file = base.join(rel_path);
    let name = if rule.strip_ext_in_ref && !rule.expect_ext.is_empty() {
        rel_path.trim_end_matches(&rule.expect_ext).to_string()
    } else {
        rel_path.to_string()
    };

    // 存在性检查（sprites 目录/文件皆可；其余按期望扩展名拼文件）
    let exists = if rule.expect_ext.is_empty() {
        src_file.exists()
    } else {
        base.join(format!(
            "{}{}",
            rel_path.trim_end_matches(&rule.expect_ext),
            rule.expect_ext
        ))
        .exists()
    };
    let warn = if !exists {
        Some(format!(
            "资源不存在: {code_set}/res/{}/{rel_path}（期望 {}）",
            rule.res_type,
            if rule.expect_ext.is_empty() {
                "目录或文件".to_string()
            } else {
                rule.expect_ext.clone()
            }
        ))
    } else {
        None
    };

    let runtime = rules::resolve_template(&rule.runtime_prefix, code_set, cfg, project_name);
    (format!("{runtime}{name}"), warn)
}
