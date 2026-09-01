//! bgd.json 项目配置的读写（overlay：libs/bgd_default.json 基底 + .bgd/bgd.json 覆盖）

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 项目状态字段（始终写入 bgd.json，不参与 default 回退）
const STATE_KEYS: [&str; 2] = ["framework_version", "framework_repo"];

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

    /// 替换排除（0.9.0）：指定文件名/目录名正常进构建产物但跳过
    /// 模块名/res 路径替换（与 libs_excludes「不构建」语义正交；目录边界匹配）。
    /// 工具自产 artifact（path_rules.lua 盖戳）由构建编排层并入，无需用户配置。
    #[serde(default)]
    pub rewrite_excludes: Vec<String>,

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
    /// overlay 加载：libs/bgd_default.json 为基底，.bgd/bgd.json 逐 key 覆盖
    pub fn load(bgd_root: &Path) -> Result<Self> {
        let mut merged = serde_json::Map::new();

        let default_path = bgd_root.join("libs").join("bgd_default.json");
        if default_path.exists() {
            let text = fs::read_to_string(&default_path)
                .with_context(|| format!("无法读取默认配置: {}", default_path.display()))?;
            match serde_json::from_str(&text)
                .with_context(|| format!("默认配置 JSON 解析失败: {}", default_path.display()))?
            {
                serde_json::Value::Object(obj) => merged = obj,
                _ => anyhow::bail!("默认配置不是 JSON 对象: {}", default_path.display()),
            }
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

    /// 保存为覆盖项：与 libs/bgd_default.json 逐 key 对比，只写不同项 + 状态字段
    pub fn save(&self, bgd_root: &Path) -> Result<()> {
        let default_path = bgd_root.join("libs").join("bgd_default.json");
        let mut defaults = serde_json::Map::new();
        if default_path.exists() {
            if let Ok(text) = fs::read_to_string(&default_path) {
                if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&text) {
                    defaults = obj;
                }
            }
        }

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

    /// 配置中的相对路径（相对 .bgd 目录）转绝对路径
    pub fn abs(&self, bgd_root: &Path, rel: &str) -> PathBuf {
        let joined = bgd_root.join(rel);
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
