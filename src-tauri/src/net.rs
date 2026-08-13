//! 网络：统一 HTTP 客户端（代理 + GitHub Token 认证）
//!
//! 仓库转私有后，api.github.com / codeload / raw.githubusercontent / release asset API
//! 均需要 `Authorization: Bearer <token>`（fine-grained PAT，Contents 只读即可）。
//! token 为空时不加头，兼容公开仓库场景。

use anyhow::{Context, Result};

/// 构造 reqwest 客户端：可选代理 + 可选 GitHub Token
pub fn http_client(proxy: &str, token: &str) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder().user_agent("BGD_SCE_TOOLS");
    let proxy = proxy.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).context("代理地址无效")?);
    }
    let token = token.trim();
    if !token.is_empty() {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            .context("GitHub Token 含非法字符")?;
        // 标记敏感：跨主机重定向（如 release asset 302 到 CDN）时 reqwest 自动剥离，避免泄露
        value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, value);
        builder = builder.default_headers(headers);
    }
    builder.build().context("无法创建 HTTP 客户端")
}
