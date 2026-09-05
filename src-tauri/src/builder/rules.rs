//! 路径规则单一来源（0.9.0）：模块名/res 资源路径的源码形态→运行时形态规则
//!
//! 规则归属：bgd_sce_tools 内建默认 + bgd.json overlay（res_rules / rewrite_excludes，
//! 设置界面可编辑）。rewrite.rs（引用路径改写）、res.rs（物理同步/清理）、
//! path_rules.lua 盖戳（dbg_bus eval 直通用）三处消费同一套规则——改规则只改这里。
//!
//! 历史教训：本规则曾散落 4 处硬编码（rewrite.rs 两份表 + res.rs 两份表），
//! 且 rewrite.rs 的 res 前缀写死 bgd_libs_client 与 rewrite_lua 读 cfg 目标名不一致
//! （设置里改构建目标后两者撕裂）——统一由 prefix_for 从 cfg 派生后该 bug 消除。

use crate::config::BgdConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// 资源路径规则（生效形态：默认值经 bgd.json overlay 覆盖后）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResRule {
    /// 资源类型（res 目录名）：image/particle/sound/spine/sprites，可扩
    pub res_type: String,
    /// 期望扩展名（存在性检查用；sprites 可为空——目录无扩展名）
    pub expect_ext: String,
    /// 运行时引用是否去扩展名（sound 去 .ogg / spine 去 .skel）
    pub strip_ext_in_ref: bool,
    /// 物理落位前缀（相对项目根；{prefix} 占位）：ui/image/image/{prefix} 等
    pub disk_prefix: String,
    /// 运行时引用前缀（{prefix}/{project} 占位）：image/image/{prefix}/ 等
    pub runtime_prefix: String,
}

/// 内建默认规则（= 0.9.0 前硬编码行为的逐字节快照）
pub fn default_rules() -> Vec<ResRule> {
    vec![
        ResRule {
            res_type: "image".into(),
            expect_ext: ".png".into(),
            strip_ext_in_ref: false,
            disk_prefix: "ui/image/image/{prefix}".into(),
            runtime_prefix: "image/image/{prefix}/".into(),
        },
        ResRule {
            res_type: "particle".into(),
            expect_ext: ".effect".into(),
            strip_ext_in_ref: false,
            disk_prefix: "res/effect/{prefix}".into(),
            runtime_prefix: "res/effect/{prefix}/".into(),
        },
        ResRule {
            res_type: "sound".into(),
            expect_ext: ".ogg".into(),
            strip_ext_in_ref: true,
            disk_prefix: "res/sound/{prefix}".into(),
            runtime_prefix: "res/sound/{prefix}/".into(),
        },
        ResRule {
            res_type: "spine".into(),
            expect_ext: ".skel".into(),
            strip_ext_in_ref: true,
            disk_prefix: "ui/spine/{prefix}".into(),
            runtime_prefix: "spine/{prefix}/".into(),
        },
        ResRule {
            res_type: "sprites".into(),
            expect_ext: String::new(),
            strip_ext_in_ref: false,
            disk_prefix: "ui/image/sprites/{prefix}".into(),
            runtime_prefix: "@{project}/image/sprites/{prefix}/".into(),
        },
    ]
}

/// 生效规则：内建默认逐条经 cfg.res_rules（按 res_type 稀疏覆盖）合成
pub fn effective_rules(cfg: &BgdConfig) -> Vec<ResRule> {
    let mut rules = default_rules();
    for ov in &cfg.res_rules {
        // 覆盖已有类型；未知类型 = 新增（规则可扩）
        let pos = rules.iter().position(|r| r.res_type == ov.res_type);
        let target = match pos {
            Some(i) => &mut rules[i],
            None => {
                rules.push(ResRule {
                    res_type: ov.res_type.clone(),
                    expect_ext: String::new(),
                    strip_ext_in_ref: false,
                    disk_prefix: String::new(),
                    runtime_prefix: String::new(),
                });
                rules.last_mut().unwrap()
            }
        };
        if let Some(v) = &ov.expect_ext {
            target.expect_ext = v.clone();
        }
        if let Some(v) = ov.strip_ext_in_ref {
            target.strip_ext_in_ref = v;
        }
        if let Some(v) = &ov.disk_prefix {
            target.disk_prefix = v.clone();
        }
        if let Some(v) = &ov.runtime_prefix {
            target.runtime_prefix = v.clone();
        }
    }
    rules
}

/// code set 前缀：从 cfg 构建目标目录名派生（libs→libs_client_target 文件名等）
/// ——rewrite_lua 与 res 改写共用同一来源，设置里改构建目标两处同时跟随
pub fn prefix_for(code_set: &str, cfg: &BgdConfig) -> String {
    let target = if code_set == "libs" {
        &cfg.libs_client_target
    } else {
        &cfg.game_client_target
    };
    Path::new(target)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// 服务端运行时根名（0.9.2：path_rules 盖戳移 common 双端共享后，modules_server 段用）：
/// cfg.libs_server_target/game_server_target 的目录名（bgd_libs_server/bgd_game_server）
pub fn server_prefix_for(code_set: &str, cfg: &BgdConfig) -> String {
    let target = if code_set == "libs" {
        &cfg.libs_server_target
    } else {
        &cfg.game_server_target
    };
    Path::new(target)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// 源码引用前缀（require('libs.x') / 'libs/res/...' 的 libs 段）：cfg.libs_dir 目录名
pub fn libs_prefix(cfg: &BgdConfig) -> String {
    Path::new(&cfg.libs_dir)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// 源码引用前缀（require('src.x') / 'src/res/...' 的 src 段）：cfg.game_dir 目录名
pub fn src_prefix(cfg: &BgdConfig) -> String {
    Path::new(&cfg.game_dir)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// 从 map_settings.json 读取 ProjectName（sprites 引用前缀与地图包名用）
pub fn project_name(bgd_root: &Path) -> Result<String> {
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

/// project_name 的缓存版（F4：每文件现读 map_settings.json 是 0.9.0 性能债）。
/// 地图名在构建期不变，按 bgd_root 缓存进程级。
pub fn project_name_cached(bgd_root: &Path) -> String {
    static CACHE: std::sync::LazyLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, String>>,
    > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut guard = CACHE.lock().unwrap();
    guard
        .entry(bgd_root.to_path_buf())
        .or_insert_with(|| project_name(bgd_root).unwrap_or_else(|_| "unknown".to_string()))
        .clone()
}

/// 解析规则占位符：{prefix} = code set 前缀，{project} = ProjectName
pub fn resolve_template(template: &str, code_set: &str, cfg: &BgdConfig, project: &str) -> String {
    template
        .replace("{prefix}", &prefix_for(code_set, cfg))
        .replace("{project}", project)
}

// ---------------------------------------------------------------------------
// path_rules.lua 盖戳（dbg_bus eval 源码形态直通的运行时对照表）
// ---------------------------------------------------------------------------

/// 盖戳文件相对路径（相对游戏源码目录 src/）。
/// 0.9.2 起从 client/ 移到 **common/**——服务端 eval（dbg_server）同样需要本表，
/// client 目录只进客户端产物，common 双端共享（配合 libs/entrance/{client,server}.lua
/// 均 require 'src.common.path_rules'）。
/// 本文件内容为运行时终值，构建替换会损坏它——默认由 rewrite_excludes 内建默认项
/// `.bgd/src/common/path_rules` 保护（可见、可在设置中删除，删除即失去保护）。
pub const PATH_RULES_REL: &str = "common/path_rules.lua";

/// 0.9.1 及之前的旧盖戳位置（迁移清理：构建时若存在则删除——旧 entrance 已不再加载它，
/// 且排除项已移到 common，旧文件进产物会被替换损坏）
const PATH_RULES_LEGACY_REL: &str = "client/path_rules.lua";

/// 渲染 path_rules.lua 内容（全部为解析后的原样终值，消费端只做前缀替换）
pub fn render_path_rules_lua(bgd_root: &Path, cfg: &BgdConfig) -> Result<String> {
    let project = project_name_cached(bgd_root);
    let libs_root = prefix_for("libs", cfg);
    let game_root = prefix_for("game", cfg);
    let libs_root_s = server_prefix_for("libs", cfg);
    let game_root_s = server_prefix_for("game", cfg);
    let libs_p = libs_prefix(cfg);
    let src_p = src_prefix(cfg);

    let mut res_lines = String::new();
    for rule in effective_rules(cfg) {
        for (code_set, prefix) in [("libs", &libs_p), ("game", &src_p)] {
            let to = resolve_template(&rule.runtime_prefix, code_set, cfg, &project);
            let strip = if rule.strip_ext_in_ref && !rule.expect_ext.is_empty() {
                format!(", strip_ext = '{}'", rule.expect_ext)
            } else {
                String::new()
            };
            res_lines.push_str(&format!(
                "        {{ from = '{prefix}/res/{ty}/', to = '{to}'{strip} }},\n",
                ty = rule.res_type
            ));
        }
    }

    Ok(format!(
        r##"-- ============================================================================
-- AUTO-GENERATED BY bgd_sce_tools —— 路径规则盖戳（请勿手改，每次构建重新生成）
--
-- 这个文件是什么：
--   源码形态路径（require 模块名 / res 资源字面量）→ 运行时路径的对照表。
--   供 dbg_bus lua.eval（客户端 VM）与 dbg_server eval（服务端 VM）逃生舱把
--   「与游戏源码逐字一致」的调试代码翻译成运行时形态（common 目录双端共享）。
--
-- 怎么来的：
--   bgd_sce_tools build 时生成。规则在工具「设置-构建路径配置」查看/编辑
--  （默认值内建于 tools；项目级覆盖写在 .bgd/bgd.json 的 res_rules）。
--   工具改规则 → 下次 build 自动重新盖戳 → 即刻生效，无第二处需要同步。
--   本文件经 rewrite_excludes 内建默认项（.bgd/src/common/path_rules）保护：
--   内容已是运行时终值，构建替换会损坏它（该默认项在设置中可见、可删，删即失去保护）。
--
-- 怎么加载的（时序）：
--   .bgd/libs/entrance/{{client,server}}.lua（框架入口）顶部 pcall require
--  'src.common.path_rules' → bgd_sce_tools 的 entrance 合并机制把它合进
--   ui/src/main.lua 与 script/main.lua 最前段 → 先于 libs/src init 执行
--   → _G.bgd_path_rules 双端从启动起全局可用。加载失败会 log.error 响亮报错（无静默兜底）。
--
-- 缺失怎么办：
--   dbg_bus eval（客户端）/ dbg_server eval（服务端）遇到 libs./src. 引用而
--   _G.bgd_path_rules 为 nil 时直接报错
--  「path_rules 盖戳缺失：请用 bgd_sce_tools 重新构建项目」——说明工具版本
--   过旧或构建被绕过。
-- ============================================================================
_G.bgd_path_rules = {{
    map = '{project}', -- 地图包名（构建时读 project/map_settings.json 的 ProjectName）
    -- require 模块名前缀对照（点形式，key 源码前缀 → value 包内根名；
    -- 消费端组合 '@'..map..'.'..value..其余段——'@' 是引擎跨包标记，eval 环境不可省）
    modules = {{
        ['{libs_p}.'] = '{libs_root}.',
        ['{src_p}.']  = '{game_root}.',
        ['{libs_p}']  = '{libs_root}',
        ['{src_p}']   = '{game_root}',
    }},
    -- 服务端根名对照（0.9.2：dbg_server eval 用；res 段服务端不消费故无 server 版）
    modules_server = {{
        ['{libs_p}.'] = '{libs_root_s}.',
        ['{src_p}.']  = '{game_root_s}.',
        ['{libs_p}']  = '{libs_root_s}',
        ['{src_p}']   = '{game_root_s}',
    }},
    -- res 资源字面量前缀对照（斜杠形式，按序首个前缀命中；strip_ext = 引用去扩展名）
    res = {{
{res_lines}    }},
}}
return _G.bgd_path_rules
"##
    ))
}

/// 盖戳写入 .bgd/src/<PATH_RULES_REL>（0.9.2 起 common/path_rules.lua；
/// 内容一致不重写，防 watch 抖动）。旧位置 client/path_rules.lua 存在即迁移删除。
pub fn write_path_rules(bgd_root: &Path, cfg: &BgdConfig, log: &crate::builder::LogFn) -> Result<()> {
    let content = render_path_rules_lua(bgd_root, cfg)?;
    let game_root = cfg.abs(bgd_root, &cfg.game_dir);
    let out = game_root.join(PATH_RULES_REL);
    // 迁移清理（0.9.1 及之前盖戳在 client/ 下；旧 entrance 不再加载，残留进产物会被替换损坏）
    let legacy = game_root.join(PATH_RULES_LEGACY_REL);
    if legacy.exists() {
        fs::remove_file(&legacy)?;
        log(&format!("[gen] 旧位置盖戳已迁移删除: {}", legacy.display()));
    }
    let old = fs::read_to_string(&out).unwrap_or_default();
    if old == content {
        return Ok(());
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, content)?;
    log(&format!("[gen] path_rules 盖戳: {}", out.display()));
    Ok(())
}
