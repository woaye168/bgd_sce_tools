//! 模块名改写与 res 资源路径替换（含存在性检查警告）
//!
//! res 规则单一来源 = builder/rules.rs（内建默认 + bgd.json res_rules 覆盖）；
//! 本文件只负责「按规则改写 + 存在性告警」的编排，不含任何规则字面量。

use super::rules;
use super::LogFn;
use crate::config::BgdConfig;
use regex::{Captures, Regex};
use std::sync::LazyLock;
use std::path::Path;

/// 把引号内的源码模块名前缀改写为运行时根名：'libs.xxx' -> 'bgd_libs_server.xxx' 等
pub fn rewrite_lua(content: &str, side: &str, cfg: &BgdConfig) -> String {
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
    static RE_LIBS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(['"])libs(['".])"#).unwrap());
    static RE_SRC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(['"])src(['".])"#).unwrap());
    let content = RE_LIBS.replace_all(content, |caps: &Captures| {
        if &caps[2] == "." {
            format!("{}{}.", &caps[1], libs_root)
        } else {
            format!("{}{}{}", &caps[1], libs_root, &caps[2])
        }
    });
    RE_SRC
        .replace_all(&content, |caps: &Captures| {
            if &caps[2] == "." {
                format!("{}{}.", &caps[1], game_root)
            } else {
                format!("{}{}{}", &caps[1], game_root, &caps[2])
            }
        })
        .into_owned()
}

/// res 资源路径替换：把 'libs/res/<类型>/...' 或 'src/res/<类型>/...' 替换为运行时路径。
/// 规则驱动（rules.rs）：sound/spine 去扩展名、sprites 前缀 @{ProjectName} 等均为规则字段；
/// 各类型期望扩展名驱动存在性检查（黄色警告）。
pub(super) fn rewrite_res_paths(content: &str, bgd_root: &Path, cfg: &BgdConfig, log: &LogFn) -> String {
    let project_name = rules::project_name(bgd_root).unwrap_or_else(|_| "unknown".to_string());
    let project_root = bgd_root.parent().unwrap_or(bgd_root);
    let mut result = content.to_string();
    let mut warnings: Vec<String> = Vec::new();

    let rules_list = rules::effective_rules(cfg);
    for rule in &rules_list {
        for code_set in ["libs", "src"] {
            // 按规则动态构造匹配：['"](libs|src)/res/<类型>/[^'"]+['"]
            let re = Regex::new(&format!(
                r#"['"]({code_set}/res/{}/[^'"]+)['"]"#,
                regex::escape(&rule.res_type)
            ))
            .unwrap();
            result = re
                .replace_all(&result, |caps: &Captures| {
                    let full_path = &caps[1]; // libs/res/image/armor_dark.png
                    let marker = format!("{code_set}/res/{}/", rule.res_type);
                    let rel_path = full_path.trim_start_matches(&marker);
                    let (new_path, warn) = rewrite_single_res_path(
                        rule, code_set, rel_path, &project_name, project_root, cfg,
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
    project_root: &Path,
    cfg: &BgdConfig,
) -> (String, Option<String>) {
    let base = project_root
        .join(".bgd")
        .join(code_set)
        .join("res")
        .join(&rule.res_type);

    // 引用名：strip_ext_in_ref 时去掉期望扩展名（sound/spine）；sprites 目录无扩展名
    let src_file = base.join(rel_path);
    let name = if rule.strip_ext_in_ref && !rule.expect_ext.is_empty() {
        rel_path.trim_end_matches(&rule.expect_ext).to_string()
    } else if rule.expect_ext.is_empty() && src_file.is_dir() {
        rel_path.to_string() // sprites 目录：原样
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
