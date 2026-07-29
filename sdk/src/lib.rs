//! bgd_sce_tools 插件 SDK
//!
//! 插件以 cdylib 动态库形式编译，导出 `plugin_create` 等符号由宿主加载。

pub mod config;
pub mod context;
pub mod error;
pub mod traits;

pub use config::BgdConfig;
pub use context::{BuildContext, SidebarItem, UiCommand, UiContext};
pub use error::PluginError;
pub use traits::{BuildHook, Plugin, SettingsHook, UiHook};
