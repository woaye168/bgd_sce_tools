//! 插件 trait 定义

use crate::{BuildContext, PluginError, SidebarItem, UiContext};
use std::path::Path;

/// 插件元信息（所有插件必须实现）
pub trait Plugin: Send + Sync {
    /// 插件名称（唯一标识）
    fn name(&self) -> &str;
    /// 插件版本
    fn version(&self) -> &str;
    /// 插件描述
    fn description(&self) -> &str;
    /// 插件作者
    fn author(&self) -> &str;
}

/// 构建流程钩子（所有方法均有默认空实现，按需覆盖）
pub trait BuildHook: Plugin {
    /// 构建开始前调用；返回 Err 将中止本次构建
    fn before_build(&self, _ctx: &BuildContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// 构建完成后调用
    fn after_build(&self, _ctx: &BuildContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// 对单个源文件内容进行转换；默认原样返回
    fn transform_file(&self, content: String, _path: &Path) -> Result<String, PluginError> {
        Ok(content)
    }
}

/// 界面钩子（所有方法均有默认实现，按需覆盖）
pub trait UiHook: Plugin {
    /// 插件设置页：返回一个向 [`UiContext`] push 命令的闭包；None 表示无设置页
    fn settings_page(&self) -> Option<Box<dyn Fn(&mut UiContext)>> {
        None
    }

    /// 侧边栏条目；None 表示不注册
    fn sidebar_item(&self) -> Option<SidebarItem> {
        None
    }
}

/// 插件自身配置的读写
pub trait SettingsHook: Plugin {
    /// 加载插件配置（不存在时返回 `serde_json::Value::Null` 或默认结构）
    fn load_settings(&self) -> serde_json::Value;

    /// 保存插件配置
    fn save_settings(&self, value: serde_json::Value);
}
