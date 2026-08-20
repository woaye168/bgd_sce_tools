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

/// 流式下载 URL 到内存（async；`on_progress(downloaded, total)` 回调进度，total 未知为 None）。
/// 统一供：应用安装、自我更新、框架下载——进度显示与下载逻辑只维护这一份。
/// 一律带 `Accept: application/octet-stream`：GitHub release asset API 无此头会返回
/// asset 元数据 JSON 而非二进制（曾导致把 1.6KB JSON 写成 exe 的事故）。
pub async fn download_bytes_async(
    url: &str,
    proxy: &str,
    token: &str,
    on_progress: impl Fn(u64, Option<u64>) + Send,
) -> Result<Vec<u8>> {
    let resp = async_http_client(proxy, token)?
        .get(url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .with_context(|| format!("请求下载失败: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("下载失败: HTTP {}（{url}）", resp.status()));
    }
    let total = resp.content_length();
    let mut bytes = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut downloaded = 0u64;
    on_progress(0, total);
    use futures_util::StreamExt;
    let mut s = resp.bytes_stream();
    while let Some(chunk) = s.next().await {
        let chunk = chunk.context("读取下载流失败")?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// 校验下载产物的文件魔数（防把错误响应体写盘：exe 期望 PE `MZ`，zip 期望 `PK`）。
/// 失败时附带产物开头文本片段（通常是 JSON 错误体），便于定位真实原因。
pub fn check_magic(bytes: &[u8], magic: &[u8], what: &str) -> Result<()> {
    if bytes.starts_with(magic) {
        return Ok(());
    }
    let head: String = bytes.iter().take(120).map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' }).collect();
    Err(anyhow::anyhow!(
        "{what}下载产物无效（期望魔数 {:?}，实际 {} 字节），开头内容: {head}",
        magic,
        bytes.len()
    ))
}

/// 向 Tauri 前端发送下载进度事件的便捷构造（事件名固定三处：
/// `app-download-progress` / `self-update-progress` / `framework-download-progress`）
pub fn progress_emitter(
    app: &tauri::AppHandle,
    event: &'static str,
    extra_id: Option<String>,
) -> impl Fn(u64, Option<u64>) + Send + use<> {
    use tauri::Emitter;
    let app = app.clone();
    move |downloaded: u64, total: Option<u64>| {
        let mut payload = serde_json::json!({ "downloaded": downloaded, "total": total });
        if let Some(id) = &extra_id {
            payload["id"] = serde_json::Value::String(id.clone());
        }
        let _ = app.emit(event, payload);
    }
}

/// 构造 reqwest 异步客户端（0.7.2 起：网络等待让出线程，消除 GUI 假死感）
pub fn async_http_client(proxy: &str, token: &str) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().user_agent("BGD_SCE_TOOLS");
    let proxy = proxy.trim();
    if !proxy.is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(proxy).context("代理地址无效")?);
    }
    let token = token.trim();
    if !token.is_empty() {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            .context("GitHub Token 含非法字符")?;
        value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, value);
        builder = builder.default_headers(headers);
    }
    builder.build().context("无法创建 HTTP 客户端")
}
