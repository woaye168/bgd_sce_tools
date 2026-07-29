//! 上下文与 UI 类型定义

use crate::config::BgdConfig;
use std::path::PathBuf;

/// 构建上下文：构建钩子执行期间由宿主注入
pub struct BuildContext {
    /// 项目 `.bgd` 目录的绝对路径
    pub bgd_root: PathBuf,
    /// 当前项目配置（overlay 合并后的结果）
    pub config: BgdConfig,
    /// 日志输出回调（写入宿主构建日志）；CLI 模式下可能为 None（插件自行输出到 stdout）
    pub log: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl BuildContext {
    /// 输出一行构建日志（log 为 None 时输出到 stdout）
    pub fn log(&self, msg: &str) {
        if let Some(ref log) = self.log {
            log(msg);
        } else {
            println!("{msg}");
        }
    }
}

impl std::fmt::Debug for BuildContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildContext")
            .field("bgd_root", &self.bgd_root)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// 侧边栏条目：插件向宿主 GUI 注册的入口
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarItem {
    /// 显示文本
    pub label: String,
    /// 图标标识（由宿主解析）
    pub icon: String,
    /// 点击后跳转的页面 ID（与 [`crate::UiHook::settings_page`] 配合）
    pub page_id: String,
}

/// UI 命令：插件不直接操作 egui，通过命令模式描述界面，由宿主渲染
#[derive(Debug, Clone, PartialEq)]
pub enum UiCommand {
    /// 普通文本
    Label(String),
    /// 标题文本
    Heading(String),
    /// 分隔线
    Separator,
    /// 按钮（action 为点击时回调给插件的动作标识）
    Button { label: String, action: String },
    /// 单行文本输入（key 为配置键）
    TextInput {
        key: String,
        label: String,
        value: String,
    },
    /// 复选框（key 为配置键）
    Checkbox {
        key: String,
        label: String,
        checked: bool,
    },
}

/// 简化 UI 上下文：插件通过 push 命令描述设置页内容
#[derive(Debug, Default)]
pub struct UiContext {
    commands: Vec<UiCommand>,
}

impl UiContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条 UI 命令
    pub fn push(&mut self, cmd: UiCommand) {
        self.commands.push(cmd);
    }

    pub fn label(&mut self, text: impl Into<String>) {
        self.push(UiCommand::Label(text.into()));
    }

    pub fn heading(&mut self, text: impl Into<String>) {
        self.push(UiCommand::Heading(text.into()));
    }

    pub fn separator(&mut self) {
        self.push(UiCommand::Separator);
    }

    pub fn button(&mut self, label: impl Into<String>, action: impl Into<String>) {
        self.push(UiCommand::Button {
            label: label.into(),
            action: action.into(),
        });
    }

    pub fn text_input(
        &mut self,
        key: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
    ) {
        self.push(UiCommand::TextInput {
            key: key.into(),
            label: label.into(),
            value: value.into(),
        });
    }

    pub fn checkbox(&mut self, key: impl Into<String>, label: impl Into<String>, checked: bool) {
        self.push(UiCommand::Checkbox {
            key: key.into(),
            label: label.into(),
            checked,
        });
    }

    /// 查看已排队的命令（宿主渲染用）
    pub fn commands(&self) -> &[UiCommand] {
        &self.commands
    }

    /// 取走全部命令（宿主渲染后清空）
    pub fn take_commands(&mut self) -> Vec<UiCommand> {
        std::mem::take(&mut self.commands)
    }
}
