//! bgd.json 项目配置的简化定义（字段与主 crate `src-tauri/src/config.rs` 保持一致）

use serde::{Deserialize, Serialize};

/// bgd.json 项目配置（简化版）
///
/// 字段与主 crate 的 `BgdConfig` 一一对应；SDK 不实现 overlay 加载/保存逻辑，
/// 配置由宿主构建完成后注入 [`crate::BuildContext`]。
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

impl Default for BgdConfig {
    fn default() -> Self {
        Self {
            project_root: default_project_root(),
            enable_build_log: false,
            server_entrance: String::new(),
            client_entrance: String::new(),
            libs_dir: String::new(),
            libs_server_target: String::new(),
            libs_client_target: String::new(),
            libs_excludes: Vec::new(),
            game_dir: String::new(),
            game_server_target: String::new(),
            game_client_target: String::new(),
            game_excludes: Vec::new(),
            framework_version: String::new(),
            framework_repo: String::new(),
        }
    }
}
