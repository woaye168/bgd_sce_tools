//! 模块名改写与 res 资源路径替换（含存在性检查警告）

use super::LogFn;
use crate::config::BgdConfig;
use anyhow::{Context, Result};
use regex::{Captures, Regex};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// res 资源五类（目录名固定）
const RES_TYPES: [&str; 5] = ["image", "particle", "sound", "spine", "sprites"];
/// res 路径替换正则：["libs","src"] × RES_TYPES 展开（与 rewrite_res_paths 循环次序一致）
static RES_REGEXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    ["libs", "src"]
        .iter()
        .flat_map(|cs| {
            RES_TYPES.iter().map(move |rt| {
                Regex::new(&format!(r#"['"]({cs}/res/{rt}/[^'"]+)['"]"#)).unwrap()
            })
        })
        .collect()
});

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

/// res 资源路径替换：把 'libs/res/<类型>/...' 或 'src/res/<类型>/...' 替换为运行时路径
/// 规则：sound 去掉 .ogg 扩展名，sprites 前缀 @<ProjectName>；各类型固定扩展名 + 资源存在性检查（黄色警告）
pub(super) fn rewrite_res_paths(content: &str, bgd_root: &Path, log: &LogFn) -> String {
    let project_name = read_project_name(bgd_root).unwrap_or_else(|_| "unknown".to_string());
    let project_root = bgd_root.parent().unwrap_or(bgd_root);
    let mut result = content.to_string();
    let mut warnings: Vec<String> = Vec::new();

    for (i, re) in RES_REGEXES.iter().enumerate() {
        let code_set = ["libs", "src"][i / RES_TYPES.len()];
        let res_type = RES_TYPES[i % RES_TYPES.len()];
        result = re
            .replace_all(&result, |caps: &Captures| {
                let full_path = &caps[1]; // libs/res/image/armor_dark.png
                let rel_path = full_path
                    .trim_start_matches(&format!("{code_set}/res/{res_type}/"));
                let (new_path, warn) = rewrite_single_res_path(
                    code_set, res_type, rel_path, &project_name, project_root,
                );
                if let Some(w) = warn {
                    warnings.push(w);
                }
                format!("'{new_path}'")
            })
            .into_owned();
    }

    // 输出黄色警告（构建日志流）
    for w in warnings {
        log(&format!("[warn] {w}"));
    }

    result
}

/// 重写单个 res 路径，返回 (新路径, 可选警告)
fn rewrite_single_res_path(
    code_set: &str,
    res_type: &str,
    rel_path: &str,
    project_name: &str,
    project_root: &Path,
) -> (String, Option<String>) {
    let prefix = if code_set == "libs" { "bgd_libs_client" } else { "bgd_game_client" };
    let base = project_root.join(".bgd").join(code_set).join("res").join(res_type);

    match res_type {
        "image" => {
            let name = rel_path.trim_end_matches(".png");
            let file = base.join(format!("{name}.png"));
            let warn = if !file.exists() {
                Some(format!("资源不存在: {code_set}/res/image/{rel_path}（期望 .png）"))
            } else {
                None
            };
            (format!("image/image/{prefix}/{name}.png"), warn)
        }
        "particle" => {
            let name = rel_path.trim_end_matches(".effect");
            let file = base.join(format!("{name}.effect"));
            let warn = if !file.exists() {
                Some(format!("资源不存在: {code_set}/res/particle/{rel_path}（期望 .effect）"))
            } else {
                None
            };
            (format!("res/effect/{prefix}/{name}.effect"), warn)
        }
        "sound" => {
            let name = rel_path.trim_end_matches(".ogg");
            let file = base.join(format!("{name}.ogg"));
            let warn = if !file.exists() {
                Some(format!("资源不存在: {code_set}/res/sound/{rel_path}（期望 .ogg）"))
            } else {
                None
            };
            (format!("res/sound/{prefix}/{name}"), warn)
        }
        "spine" => {
            let name = rel_path.trim_end_matches(".skel");
            let file = base.join(format!("{name}.skel"));
            let warn = if !file.exists() {
                Some(format!("资源不存在: {code_set}/res/spine/{rel_path}（期望 .skel）"))
            } else {
                None
            };
            (format!("spine/{prefix}/{name}"), warn)
        }
        "sprites" => {
            // sprites：目录无扩展名，文件保留扩展名
            let path = base.join(rel_path);
            let (name, ext) = if path.is_dir() {
                (rel_path.to_string(), String::new())
            } else {
                match path.extension() {
                    Some(e) => (
                        rel_path.trim_end_matches(&format!(".{}", e.to_string_lossy())).to_string(),
                        format!(".{}", e.to_string_lossy()),
                    ),
                    None => (rel_path.to_string(), String::new()),
                }
            };
            let warn = if !path.exists() {
                Some(format!("资源不存在: {code_set}/res/sprites/{rel_path}"))
            } else {
                None
            };
            (format!("@{project_name}/image/sprites/{prefix}/{name}{ext}"), warn)
        }
        _ => (rel_path.to_string(), None),
    }
}

/// 从 map_settings.json 读取 ProjectName（sprites 路径前缀用）
fn read_project_name(bgd_root: &Path) -> Result<String> {
    let project_root = bgd_root.parent().unwrap_or(bgd_root);
    let path = project_root.join("project").join("map_settings.json");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("无法读取地图配置: {}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    json["ProjectName"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("map_settings.json 缺少 ProjectName"))
}
