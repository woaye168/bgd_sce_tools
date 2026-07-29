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

    // ---- 项目状态（仅存在于 bgd.json） ----
    #[serde(default)]
    pub framework_version: String,
    #[serde(default)]
    pub framework_repo: String,
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
            if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&text) {
                merged = obj;
            }
        }

        let override_path = bgd_root.join("bgd.json");
        if override_path.exists() {
            let text = fs::read_to_string(&override_path)
                .with_context(|| format!("无法读取配置文件: {}", override_path.display()))?;
            if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&text) {
                for (k, v) in obj {
                    merged.insert(k, v);
                }
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

impl From<BgdConfig> for bgd_sce_tools_sdk::BgdConfig {
    fn from(cfg: BgdConfig) -> Self {
        serde_json::from_value(serde_json::to_value(cfg).expect("BgdConfig 序列化失败"))
            .expect("BgdConfig 转换失败")
    }
}

impl From<&BgdConfig> for bgd_sce_tools_sdk::BgdConfig {
    fn from(cfg: &BgdConfig) -> Self {
        serde_json::from_value(serde_json::to_value(cfg).expect("BgdConfig 序列化失败"))
            .expect("BgdConfig 转换失败")
    }
}
