//! 插件加载器：libloading 扫描动态库

use bgd_sce_tools_sdk::{BuildHook, Plugin, SettingsHook, UiHook};
use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginLoadError {
    #[error("加载动态库失败 {path}: {source}")]
    LoadLibrary {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("缺少入口符号 plugin_create: {0}")]
    MissingEntry(String),
    #[error("plugin_create 返回空指针: {0}")]
    NullPlugin(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// 已加载的插件（trait 对象 + 动态库句柄）
pub struct LoadedPlugin {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub plugin: Box<dyn Plugin>,
    pub build_hook: Option<Box<dyn BuildHook>>,
    pub ui_hook: Option<Box<dyn UiHook>>,
    pub settings_hook: Option<Box<dyn SettingsHook>>,
    /// 动态库句柄：保持库在插件对象存活期间不被卸载
    #[allow(dead_code)]
    lib: Library,
}

impl std::fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("path", &self.path)
            .field("has_build_hook", &self.build_hook.is_some())
            .field("has_ui_hook", &self.ui_hook.is_some())
            .field("has_settings_hook", &self.settings_hook.is_some())
            .finish_non_exhaustive()
    }
}

/// 从动态库中按可选符号创建钩子实例；符号缺失或返回空指针时为 None
unsafe fn optional_hook<T: ?Sized>(lib: &Library, sym: &[u8]) -> Option<Box<T>> {
    let create: Symbol<unsafe extern "C" fn() -> *mut T> = lib.get(sym).ok()?;
    let raw = create();
    if raw.is_null() {
        None
    } else {
        Some(Box::from_raw(raw))
    }
}

/// 插件加载器：持有一组已加载插件
#[derive(Default)]
pub struct PluginLoader {
    pub plugins: Vec<LoadedPlugin>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// 扫描目录下所有动态库并加载为插件
    pub fn load_dir(&mut self, dir: &Path) -> Result<(), PluginLoadError> {
        if !dir.is_dir() {
            return Ok(());
        }
        let mut first_err = None;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext, "dll" | "so" | "dylib") {
                continue;
            }
            if let Err(e) = self.load_file(&path) {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// 加载单个动态库文件为插件
    pub fn load_file(&mut self, path: &Path) -> Result<(), PluginLoadError> {
        let display = path.display().to_string();
        let lib = unsafe { Library::new(path) }.map_err(|e| PluginLoadError::LoadLibrary {
            path: display.clone(),
            source: Box::new(e),
        })?;

        let plugin: Box<dyn Plugin> = unsafe {
            let create: Symbol<unsafe extern "C" fn() -> *mut dyn Plugin> = lib
                .get(b"plugin_create")
                .map_err(|_| PluginLoadError::MissingEntry(display.clone()))?;
            let raw = create();
            if raw.is_null() {
                return Err(PluginLoadError::NullPlugin(display.clone()));
            }
            Box::from_raw(raw)
        };

        let build_hook = unsafe { optional_hook::<dyn BuildHook>(&lib, b"plugin_create_build_hook") };
        let ui_hook = unsafe { optional_hook::<dyn UiHook>(&lib, b"plugin_create_ui_hook") };
        let settings_hook =
            unsafe { optional_hook::<dyn SettingsHook>(&lib, b"plugin_create_settings_hook") };

        let loaded = LoadedPlugin {
            name: plugin.name().to_string(),
            version: plugin.version().to_string(),
            path: path.to_path_buf(),
            plugin,
            build_hook,
            ui_hook,
            settings_hook,
            lib,
        };
        self.plugins.push(loaded);
        Ok(())
    }

    /// 卸载所有插件（先释放插件对象，再卸载动态库）
    pub fn unload_all(&mut self) {
        self.plugins.clear();
    }
}
