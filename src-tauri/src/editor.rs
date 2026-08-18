//! 编辑器生命周期与日志能力（0.5.3 场景一：编辑器外的本地实现）。
//!
//! - 轻量 locate 链：map_settings.json api_version + tsconfig.json typeRoots → 编辑器根 → 运行根
//!   （与 sce_app_editor-patch 的 locate.rs 同源，两仓库独立发布，各维护一份）
//! - editor_start / editor_stop：星火编辑器进程启停（启动命令形态已实证，见 0.5.3 需求文档）
//! - get_logs：读 <运行根>/logs/ 下游戏客户端/服务端/bgd_csharp 最新日志文件信息（离线可用）
//! - 在线检测与桥接调用：<运行根>/logs/bgd_csharp/port 文件 → 127.0.0.1:<port>（bgd_mcp_bridge）

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 默认编辑器 exe 名（应用设置 editor_exe_name 可覆盖，防用户改名）
pub const DEFAULT_EDITOR_EXE: &str = "星火编辑器.exe";

/// 一次定位的结果
pub struct EditorTarget {
    /// 编辑器 api 版本（map_settings.json，如 "13"）
    pub api_version: String,
    /// 编辑器更新根目录（<运行根>/Update/editor-pd.spark.xd.com）
    pub editor_root: PathBuf,
    /// 引擎运行根（editor_root 上两级，如 D:/sce_online）
    pub engine_root: PathBuf,
}

impl EditorTarget {
    /// bgd_mcp_bridge 端口文件：<运行根>/logs/bgd_csharp/port
    pub fn port_file(&self) -> PathBuf {
        self.engine_root.join("logs").join("bgd_csharp").join("port")
    }

    /// 日志根目录：<运行根>/logs
    pub fn logs_root(&self) -> PathBuf {
        self.engine_root.join("logs")
    }
}

/// 从项目路径定位编辑器（项目需含 project/map_settings.json 与 script/tsconfig.json）
pub fn locate(project_root: &Path) -> Result<EditorTarget> {
    let api_version = read_api_version(project_root)?;
    let editor_root = find_editor_root(project_root)?;
    let engine_root = editor_root
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| editor_root.clone());
    Ok(EditorTarget {
        api_version,
        editor_root,
        engine_root,
    })
}

/// map_settings.json → api_version（兼容对象/数字/字符串形态）
fn read_api_version(project_root: &Path) -> Result<String> {
    let path = project_root.join("project").join("map_settings.json");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("读取 {} 失败", path.display()))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("解析 {} 失败", path.display()))?;
    match json.get("api_version") {
        Some(Value::Object(o)) => o
            .get("api_version")
            .map(|v| match v {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("map_settings.json 的 api_version 对象中缺少 api_version 字段")),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(anyhow!("map_settings.json 缺少 api_version 字段")),
    }
}

/// tsconfig.json typeRoots 任意一条含 /Res/_m/ 的路径，前缀即编辑器根
fn find_editor_root(project_root: &Path) -> Result<PathBuf> {
    let path = project_root.join("script").join("tsconfig.json");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("读取 {} 失败", path.display()))?;
    // 容忍注释/尾逗号：直接在文本里找 "…/Res/_m/" 字符串值
    let normalized = text.replace('\\', "/");
    for seg in normalized.split('"') {
        let lower = seg.to_lowercase();
        if let Some(idx) = lower.find("/res/_m/") {
            let prefix = seg[..idx].trim_end_matches('/');
            if !prefix.is_empty() && prefix.contains(':') {
                return Ok(PathBuf::from(prefix));
            }
        }
    }
    Err(anyhow!(
        "tsconfig.json 的 typeRoots 中没有包含 Res/_m 的路径，无法定位编辑器目录"
    ))
}

// ---------------------------------------------------------------- 在线检测与桥接

/// 读端口文件（不存在/非法返回 None）
pub fn read_port(target: &EditorTarget) -> Option<u16> {
    std::fs::read_to_string(target.port_file())
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
}

/// 探测桥是否在线（POST /mcp initialize 握手，短超时）
pub fn bridge_online(port: u16) -> bool {
    bridge_rpc(port, "initialize", json!({}), 5_000).is_ok()
}

/// 调桥（HTTP JSON-RPC，POST /rpc 或 /mcp 均可；initialize 走 /mcp）。
/// timeout_ms 为客户端总超时：长操作（start_debug 桥内 120s 轮询）必须放大，
/// 否则会先于桥返回而超时。
pub fn bridge_rpc(port: u16, method: &str, params: Value, timeout_ms: u64) -> Result<Value> {
    let (path, body) = if method == "initialize" {
        (
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":params}),
        )
    } else {
        ("/rpc", json!({"id":1,"method":method,"params":params}))
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()?;
    let resp = client
        .post(format!("http://127.0.0.1:{port}{path}"))
        .json(&body)
        .send()
        .with_context(|| format!("连接编辑器桥失败（127.0.0.1:{port}）"))?;
    let text = resp.text()?;
    let doc: Value = serde_json::from_str(&text).context("桥响应不是合法 JSON")?;
    if let Some(err) = doc.get("error") {
        return Err(anyhow!("桥返回错误: {err}"));
    }
    Ok(doc.get("result").cloned().unwrap_or(Value::Null))
}

/// 调桥能力目录（invoke_capability 封装），timeout_ms 透传
pub fn bridge_invoke(port: u16, id: &str, args: Value, timeout_ms: u64) -> Result<Value> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(timeout_ms + 5000))
        .build()?;
    let body = json!({
        "id": 1,
        "method": "invoke_capability",
        "params": { "id": id, "args": args, "timeout_ms": timeout_ms }
    });
    let resp = client
        .post(format!("http://127.0.0.1:{port}/rpc"))
        .json(&body)
        .send()
        .with_context(|| format!("连接编辑器桥失败（127.0.0.1:{port}）"))?;
    let doc: Value = serde_json::from_str(&resp.text()?)?;
    if let Some(err) = doc.get("error") {
        return Err(anyhow!("桥返回错误: {err}"));
    }
    Ok(doc.get("result").cloned().unwrap_or(Value::Null))
}

/// 在线则返回端口（port 文件 + 握手双重确认）
pub fn online_port(target: &EditorTarget) -> Option<u16> {
    let port = read_port(target)?;
    if bridge_online(port) {
        Some(port)
    } else {
        None
    }
}

// ---------------------------------------------------------------- editor_start / editor_stop

/// 启动星火编辑器并等待 MCP 桥上线。幂等：已在线直接返回现状。
pub fn editor_start(
    project_root: &Path,
    exe_name: &str,
    wait_online: bool,
    timeout_ms: u64,
) -> Result<Value> {
    let target = locate(project_root)?;

    // 幂等：已在线
    if let Some(port) = online_port(&target) {
        return Ok(json!({
            "already_running": true,
            "port": port,
            "mcp_url": format!("http://127.0.0.1:{port}/mcp"),
        }));
    }

    let exe = target.engine_root.join(exe_name);
    if !exe.is_file() {
        return Err(anyhow!(
            "编辑器 exe 不存在: {}（可用 setting set editor_exe_name <名字> 修改默认 exe 名）",
            exe.display()
        ));
    }

    let sce = project_root.join("project.sce");
    let started = Instant::now();
    let child = std::process::Command::new(&exe)
        .arg("-inner")
        .arg("-winui_material_editor")
        .arg("-winui_resource_store")
        .arg(format!("-editor_api_version={}", target.api_version))
        .arg(format!("-file_path={}", sce.display()))
        .spawn()
        .with_context(|| format!("启动编辑器失败: {}", exe.display()))?;
    let pid = child.id();

    if !wait_online {
        return Ok(json!({ "started": true, "pid": pid, "wait_online": false }));
    }

    // 等待桥上线：port 文件出现 + 握手成功
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Some(port) = online_port(&target) {
            return Ok(json!({
                "started": true,
                "pid": pid,
                "port": port,
                "mcp_url": format!("http://127.0.0.1:{port}/mcp"),
                "elapsed_ms": started.elapsed().as_millis() as u64,
            }));
        }
        if Instant::now() >= deadline {
            return Ok(json!({
                "started": true,
                "pid": pid,
                "mcp_online": false,
                "warning": "编辑器进程已拉起但 MCP 桥超时未上线。排查：bgd_mcp_bridge 补丁模块是否启用；编辑器升级是否覆盖了补丁（打开「编辑器补丁」应用检查）；日志见 <运行根>/logs/bgd_csharp/",
                "elapsed_ms": started.elapsed().as_millis() as u64,
            }));
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

/// 关闭星火编辑器：直接结束进程（定稿不做优雅退出，避免保存确认弹窗挂住）。
pub fn editor_stop(project_root: &Path, exe_name: &str) -> Result<Value> {
    let target = locate(project_root)?;

    // 取 pid：在线走 server_info，离线按 exe 路径匹配进程
    let pid = online_port(&target)
        .and_then(|port| {
            bridge_rpc(port, "server_info", json!({}), 10_000).ok()?["pid"]
                .as_u64()
                .map(|p| p as u32)
        })
        .or_else(|| find_editor_pid(&exe_path_str(&target, exe_name)));

    let Some(pid) = pid else {
        return Ok(json!({ "stopped": false, "message": "编辑器未在运行" }));
    };

    // /F 强杀（不带 /F 的 WM_CLOSE 会触发保存确认弹窗）；/T 连带子进程
    let out = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .context("执行 taskkill 失败")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(anyhow!("taskkill 失败: {stderr}{stdout}"));
    }

    // 等进程消失（最多 15s）
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let check = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        if let Ok(o) = check {
            let s = String::from_utf8_lossy(&o.stdout);
            if !s.contains(&pid.to_string()) {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Ok(json!({ "stopped": true, "pid": pid }))
}

fn exe_path_str(target: &EditorTarget, exe_name: &str) -> String {
    target
        .engine_root
        .join(exe_name)
        .display()
        .to_string()
}

/// 按 exe 全路径找进程 pid（Win32_Process ExecutablePath 匹配，兼容大小写与斜杠）
fn find_editor_pid(exe_path: &str) -> Option<u32> {
    let want = exe_path.replace('/', "\\").to_lowercase();
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let doc: Value = serde_json::from_str(&text).ok()?;
    let list = match &doc {
        Value::Array(a) => a.clone(),
        Value::Object(_) => vec![doc],
        _ => return None,
    };
    for p in list {
        let exe = p["ExecutablePath"].as_str().unwrap_or("").to_lowercase();
        if exe == want {
            return p["ProcessId"].as_u64().map(|v| v as u32);
        }
    }
    None
}

// ---------------------------------------------------------------- get_logs

/// 日志源定义：(源名, 子目录, 文件前缀) —— 每类取最新一个文件
const LOG_SOURCES: &[(&str, &str, &str)] = &[
    ("client", "lua", "lua-game-"),
    ("server_core", "server", "core-game-server-"),
    ("server_lua", "server", "lua-game-server-"),
    ("bridge_main", "bgd_csharp", "bgd_csharp-"),
    ("bridge_audit", "bgd_csharp", "audit-"),
];

/// 单个日志文件信息
fn file_info(path: &Path, tail_lines: usize) -> Value {
    let meta = std::fs::metadata(path).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let to_local = |t: Option<std::time::SystemTime>| -> Value {
        t.map(|t| {
            let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64
                + 8 * 3600;
            let days = secs / 86400;
            let rem = secs % 86400;
            let (y, m, d) = civil_from_days(days);
            Value::String(format!(
                "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
                rem / 3600,
                (rem % 3600) / 60,
                rem % 60
            ))
        })
        .unwrap_or(Value::Null)
    };
    let created = meta.as_ref().and_then(|m| m.created().ok());
    let modified = meta.as_ref().and_then(|m| m.modified().ok());

    // 流式计行（不整读）
    let lines = count_lines(path).unwrap_or(0);

    let mut info = json!({
        "path": path.display().to_string(),
        "size": size,
        "created": to_local(created),
        "modified": to_local(modified),
        "lines": lines,
    });
    if tail_lines > 0 {
        const TAIL_BYTE_CAP: usize = 64 * 1024;
        let (tail, truncated) = read_tail(path, tail_lines, TAIL_BYTE_CAP);
        info["tail"] = Value::String(tail);
        info["truncated"] = Value::Bool(truncated);
    }
    info
}

/// days-from-unix-epoch → 年月日（Howard Hinnant 算法）
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn count_lines(path: &Path) -> Option<usize> {
    use std::io::{BufRead, BufReader};
    let f = std::fs::File::open(path).ok()?;
    let mut reader = BufReader::with_capacity(256 * 1024, f);
    let mut n = 0;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) => break,
            Ok(_) => n += 1,
            Err(_) => break,
        }
    }
    Some(n)
}

/// 读文件末尾 N 行（带字节上限；从尾部窗口读避免整文件载入）
fn read_tail(path: &Path, tail_lines: usize, byte_cap: usize) -> (String, bool) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else {
        return (String::new(), false);
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(byte_cap as u64);
    let truncated = start > 0;
    if f.seek(SeekFrom::Start(start)).is_err() {
        return (String::new(), false);
    }
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return (String::new(), false);
    }
    let text = String::from_utf8_lossy(&buf);
    let lines: Vec<&str> = text.lines().collect();
    // 截断读入时第一行可能是半行，丢弃
    let lines = if truncated && !lines.is_empty() {
        &lines[1..]
    } else {
        &lines[..]
    };
    let tail: Vec<&str> = lines.iter().rev().take(tail_lines).rev().cloned().collect();
    (tail.join("\n"), truncated)
}

/// 目录下按前缀找最新文件（mtime 优先）
fn latest_file(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "log").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(prefix))
                    .unwrap_or(false)
        })
        .max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH)
        })
}

/// 获取日志（离线可用；默认只返回文件路径与信息，tail_lines>0 才带内容）
pub fn get_logs(project_root: &Path, source: &str, tail_lines: usize) -> Result<Value> {
    let target = locate(project_root)?;
    let logs_root = target.logs_root();

    let mut out = serde_json::Map::new();
    for (name, sub, prefix) in LOG_SOURCES {
        // source 过滤：client/server/bridge/all
        let group = name.split('_').next().unwrap_or("");
        if source != "all" && source != group {
            continue;
        }
        let dir = logs_root.join(sub);
        match latest_file(&dir, prefix) {
            Some(p) => {
                out.insert(name.to_string(), file_info(&p, tail_lines));
            }
            None => {
                out.insert(
                    name.to_string(),
                    json!({ "path": Value::Null, "note": format!("{} 下无 {prefix}*.log（未产生过该类日志）", dir.display()) }),
                );
            }
        }
    }
    Ok(json!({ "logs_root": logs_root.display().to_string(), "logs": out }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_civil_from_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20270), (2025, 7, 1));
    }

    #[test]
    fn test_tail_and_count() {
        let dir = std::env::temp_dir().join(format!("bgd_logs_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("lua-game-20260101.log");
        let content: String = (1..=100).map(|i| format!("line{i}\n")).collect();
        std::fs::write(&f, &content).unwrap();
        assert_eq!(count_lines(&f), Some(100));
        let (tail, truncated) = read_tail(&f, 3, 64 * 1024);
        assert!(!truncated);
        assert_eq!(tail, "line98\nline99\nline100");
        let (tail2, truncated2) = read_tail(&f, 3, 64);
        assert!(truncated2);
        assert!(tail2.contains("line100"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_latest_file() {
        let dir = std::env::temp_dir().join(format!("bgd_logs_latest_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lua-game-1.log"), "a").unwrap();
        std::fs::write(dir.join("core-game-server-1.log"), "b").unwrap();
        std::fs::write(dir.join("other.txt"), "c").unwrap();
        let got = latest_file(&dir, "lua-game-").unwrap();
        assert!(got.ends_with("lua-game-1.log"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
