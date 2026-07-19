//! bgd.json 项目配置的读写

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgdConfig {
    pub project_root: String,
    #[serde(default)]
    pub enable_build_log: bool,

    pub asset_target: String,
    pub libs_asset_output_name: String,
    pub game_asset_output_name: String,

    pub server_entrance: String,
    pub client_entrance: String,

    pub libs_dir: String,
    pub libs_server_target: String,
    pub libs_client_target: String,
    #[serde(default)]
    pub libs_excludes: Vec<String>,

    pub game_dir: String,
    pub game_server_target: String,
    pub game_client_target: String,
    #[serde(default)]
    pub game_excludes: Vec<String>,

    pub templates_dir: String,

    #[serde(default)]
    pub framework_version: String,
    #[serde(default)]
    pub framework_repo: String,
}

impl BgdConfig {
    /// 从 .bgd 目录加载 bgd.json
    pub fn load(bgd_root: &Path) -> Result<Self> {
        let path = bgd_root.join("bgd.json");
        let text = fs::read_to_string(&path)
            .with_context(|| format!("无法读取配置文件: {}", path.display()))?;
        let cfg: BgdConfig = serde_json::from_str(&text)
            .with_context(|| format!("配置文件格式错误: {}", path.display()))?;
        Ok(cfg)
    }

    /// 保存到 .bgd/bgd.json（格式化输出）
    pub fn save(&self, bgd_root: &Path) -> Result<()> {
        let path = bgd_root.join("bgd.json");
        let text = serde_json::to_string_pretty(self)?;
        fs::write(&path, text).with_context(|| format!("无法写入配置文件: {}", path.display()))?;
        Ok(())
    }

    /// 配置中的相对路径（相对 .bgd 目录）转绝对路径
    pub fn abs(&self, bgd_root: &Path, rel: &str) -> PathBuf {
        let joined = bgd_root.join(rel);
        // 规范化（处理 ../），不要求路径存在
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
