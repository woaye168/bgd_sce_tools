//! 插件错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, PluginError>;
