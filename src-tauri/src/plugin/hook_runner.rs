//! 插件钩子执行器

use bgd_sce_tools_sdk::{BuildContext, PluginError};
use std::path::Path;

/// 执行所有插件的 before_build 钩子
pub fn run_before_build(loader: &super::loader::PluginLoader, ctx: &BuildContext) -> Result<(), PluginError> {
    for plugin in &loader.plugins {
        if let Some(hook) = &plugin.build_hook {
            hook.before_build(ctx)?;
        }
    }
    Ok(())
}

/// 执行所有插件的 after_build 钩子
pub fn run_after_build(loader: &super::loader::PluginLoader, ctx: &BuildContext) -> Result<(), PluginError> {
    for plugin in &loader.plugins {
        if let Some(hook) = &plugin.build_hook {
            hook.after_build(ctx)?;
        }
    }
    Ok(())
}

/// 执行所有插件的 transform_file 钩子（链式转换）
pub fn run_transform_file(
    loader: &super::loader::PluginLoader,
    content: String,
    path: &Path,
) -> Result<String, PluginError> {
    let mut content = content;
    for plugin in &loader.plugins {
        if let Some(hook) = &plugin.build_hook {
            content = hook.transform_file(content, path)?;
        }
    }
    Ok(content)
}
