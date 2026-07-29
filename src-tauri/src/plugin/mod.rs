//! 插件系统：加载、注册表、钩子执行

pub mod hook_runner;
pub mod loader;
pub mod registry;

pub use loader::{LoadedPlugin, PluginLoader};
pub use registry::{fetch_registry, Registry, RegistryEntry};

/// 从主 crate BgdConfig 构造 SDK BuildContext
pub fn make_context(
    bgd_root: &std::path::Path,
    cfg: &crate::config::BgdConfig,
    log: Option<Box<dyn Fn(&str) + Send + Sync>>,
) -> bgd_sce_tools_sdk::BuildContext {
    bgd_sce_tools_sdk::BuildContext {
        bgd_root: bgd_root.to_path_buf(),
        config: bgd_sce_tools_sdk::BgdConfig::from(cfg),
        log,
    }
}
