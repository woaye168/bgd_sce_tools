//! bgd.json 项目配置的读写（overlay：工具内建默认 + .bgd/bgd.json 覆盖）
//!
//! 默认值唯一来源 = 仓库根 bgd_default.json（include_str! 内嵌进 exe，并随安装
//! 释放到安装目录仅供查看，不作为配置层）。所有路径配置统一相对项目根（0.9.1 起，
//! 旧版相对 .bgd 的 `../` 写法已废弃）。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 工具内建默认配置原文（仓库根 bgd_default.json，随 exe 内嵌/释放）
pub const EMBEDDED_DEFAULTS: &str = include_str!("../../bgd_default.json");

/// 项目状态字段（始终写入 bgd.json，不参与 default 回退）
const STATE_KEYS: [&str; 2] = ["framework_version", "framework_repo"];

/// 内建默认值（解析后的 JSON 对象；save 差异对比基准）
pub fn embedded_defaults() -> serde_json::Map<String, serde_json::Value> {
    static DEFAULTS: std::sync::LazyLock<serde_json::Map<String, serde_json::Value>> =
        std::sync::LazyLock::new(|| {
            serde_json::from_str(EMBEDDED_DEFAULTS).expect("内嵌 bgd_default.json 必须是合法 JSON 对象")
        });
    DEFAULTS.clone()
}

/// 释放内建默认配置到 exe 旁（可见性用途；内容一致跳过，升级后 exe 变化自动覆盖）。
/// 在 exe 入口早期调用（CLI/GUI 两条路径都经过），失败不影响主流程。
pub fn release_embedded_defaults() {
    let Ok(exe) = std::env::current_exe() else { return };
    let Some(dir) = exe.parent() else { return };
    let out = dir.join("bgd_default.json");
    let old = fs::read_to_string(&out).unwrap_or_default();
    if old != EMBEDDED_DEFAULTS {
        let _ = fs::write(&out, EMBEDDED_DEFAULTS);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgdConfig {
    #[serde(default = "default_project_root")]
    pub project_root: String,
    #[serde(default)]
    pub enable_build_log: bool,

    #[serde(default)]
    pub server_entrance: String,
    #[serde(default)]
    pub client_entrance: String,

    #[serde(default)]
    pub libs_dir: String,
    #[serde(default)]
    pub libs_server_target: String,
    #[serde(default)]
    pub libs_client_target: String,
    #[serde(default)]
    pub libs_excludes: Vec<String>,

    #[serde(default)]
    pub game_dir: String,
    #[serde(default)]
    pub game_server_target: String,
    #[serde(default)]
    pub game_client_target: String,
    #[serde(default)]
    pub game_excludes: Vec<String>,

    /// 替换排除：相对项目根的完整路径（不含扩展名），命中文件或目录；
    /// 正常进构建产物但跳过模块名/res 路径替换（与 libs_excludes「不构建」语义正交）。
    /// 单条内可用 `|` 分隔多个路径。默认含工具自产盖戳 path_rules（可见、可删）。
    #[serde(default)]
    pub rewrite_excludes: Vec<String>,

    /// 行级跳过注解（0.9.1）：某行含此注解文本时，其下一行跳过全部替换
    /// （require 改写 + res 路径替换；entrance 合并管线同样生效）。空串 = 禁用。
    #[serde(default)]
    pub rewrite_skip_annotation: String,

    /// 资源路径规则覆盖（0.9.0）：按 res_type 稀疏覆盖内建默认，只列差异字段；
    /// 缺省 = 全部用内建默认（工具升级新规则默认值自动生效）
    #[serde(default)]
    pub res_rules: Vec<ResRuleOverride>,

    // ---- 项目状态（仅存在于 bgd.json） ----
    #[serde(default)]
    pub framework_version: String,
    #[serde(default)]
    pub framework_repo: String,
}

/// 资源路径规则的项目级覆盖（稀疏：只列要改的字段；语义见 builder/rules.rs ResRule）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResRuleOverride {
    pub res_type: String,
    #[serde(default)]
    pub expect_ext: Option<String>,
    #[serde(default)]
    pub strip_ext_in_ref: Option<bool>,
    #[serde(default)]
    pub disk_prefix: Option<String>,
    #[serde(default)]
    pub runtime_prefix: Option<String>,
}

fn default_project_root() -> String {
    ".".to_string()
}

impl BgdConfig {
    /// 内建默认配置（供「恢复默认」/CLI config reset 使用）
    pub fn defaults() -> Self {
        serde_json::from_value(serde_json::Value::Object(embedded_defaults()))
            .expect("内嵌 bgd_default.json 必须能解析为 BgdConfig")
    }

    /// overlay 加载：工具内建默认为基底，.bgd/bgd.json 逐 key 覆盖
    pub fn load(bgd_root: &Path) -> Result<Self> {
        let mut merged = serde_json::Map::new();
        for (k, v) in embedded_defaults() {
            merged.insert(k, v);
        }

        let override_path = bgd_root.join("bgd.json");
        if override_path.exists() {
            let text = fs::read_to_string(&override_path)
                .with_context(|| format!("无法读取配置文件: {}", override_path.display()))?;
            match serde_json::from_str(&text)
                .with_context(|| format!("配置文件 JSON 解析失败: {}", override_path.display()))?
            {
                serde_json::Value::Object(obj) => {
                    for (k, v) in obj {
                        merged.insert(k, v);
                    }
                }
                _ => anyhow::bail!("配置文件不是 JSON 对象: {}", override_path.display()),
            }
        }

        let cfg: BgdConfig = serde_json::from_value(serde_json::Value::Object(merged))
            .context("配置合并结果格式错误")?;
        Ok(cfg)
    }

    /// 保存为覆盖项：与工具内建默认逐 key 对比（深度相等即视为默认，含空数组），
    /// 只写不同项 + 状态字段
    pub fn save(&self, bgd_root: &Path) -> Result<()> {
        let defaults = embedded_defaults();

        let self_value = serde_json::to_value(self)?;
        let mut overrides = serde_json::Map::new();
        if let serde_json::Value::Object(obj) = self_value {
            for (k, v) in obj {
                let is_state = STATE_KEYS.contains(&k.as_str());
                let differs = defaults.get(&k) != Some(&v);
                if is_state || differs {
                    overrides.insert(k, v);
                }
            }
        }

        let path = bgd_root.join("bgd.json");
        let text = serde_json::to_string_pretty(&serde_json::Value::Object(overrides))?;
        fs::write(&path, text).with_context(|| format!("无法写入配置文件: {}", path.display()))?;
        Ok(())
    }

    /// 项目根目录（.bgd 的上一级）
    pub fn project_root_of(bgd_root: &Path) -> PathBuf {
        bgd_root.parent().unwrap_or(bgd_root).to_path_buf()
    }

    /// 配置中的相对路径（相对项目根）转绝对路径；`..` 归一化，绝对路径原样
    pub fn abs(&self, bgd_root: &Path, rel: &str) -> PathBuf {
        let rel_path = Path::new(rel);
        let joined = if rel_path.is_absolute() {
            rel_path.to_path_buf()
        } else {
            Self::project_root_of(bgd_root).join(rel)
        };
        let mut out = PathBuf::new();
        for comp in joined.components() {
            match comp {
                std::path::Component::ParentDir => {
                    out.pop();
                }
                std::path::Component::CurDir => {}
                other => out.push(other.as_os_str()),
            }
        }
        out
    }
}
