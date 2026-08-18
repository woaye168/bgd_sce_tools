//! CLI 子命令入口。
//!
//! 与 Tauri 命令共享同一套核心函数（builder.rs / project.rs），
//! 用于开发期本地功能验证与无 GUI 环境使用。
//! 约定：新增/修改任何构建或项目功能时，必须同步更新本模块（见 AGENTS.md）。

use anyhow::{Context, Result};
use bgd_sce_tools_lib::{apps, builder, config::BgdConfig, project};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const USAGE: &str = "
bgd_sce_tools CLI

用法: bgd_sce_tools <子命令> [选项]

子命令:
  build                  全量构建
  clean                  清除构建产物（还原入口原文）
  clean-logs             清理 .bgd/log 下的 .log 文件
  init                   初始化项目（下载框架生成 .bgd）
  update-framework       增量更新框架（三路哈希对比）
  check-framework        检查框架是否有新版本
  watch                  监听更新（前台阻塞，Ctrl+C 停止）
  check-watch            检查项目当前是否处于监听中
  config get <键>        读取 bgd 配置（合并后生效值）
  config set <键> <值>   写入 bgd.json 覆盖项（数组用 JSON 数组形式）
  setting get <键>       读取应用设置（proxy / watch_enabled / github_token / editor_exe_name）
  setting set <键> <值>  写入应用设置
  app <应用id>           启动已安装的应用 EXE（透传 --project-path）
  editor start           启动星火编辑器（等待 MCP 桥上线；幂等）
  editor stop            关闭星火编辑器（直接结束进程）
  logs [源] [行数]       获取最新日志文件信息（源: client/server/bridge/all；行数 0=不取内容）
  mcp                    启动 stdio MCP 聚合服务（AI 客户端配置入口，前台阻塞）

选项:
  --project <路径>        项目根目录（缺省为当前目录）
  --proxy <地址>          HTTP 代理，如 http://127.0.0.1:7897
  --repo <owner/repo>     框架仓库（缺省内置默认）
  --force                 init 时强制执行（存在 init.lock 时覆盖，自动备份）
  --log <路径>            将过程日志同时写入指定文件（便于无 GUI 环境查看）

示例:
  bgd_sce_tools build --project D:\\maps\\my_game --log .bgd/log/build.log
  bgd_sce_tools check-watch --project D:\\maps\\my_game
  bgd_sce_tools app visual-injector --project D:\\maps\\my_game
";

struct Cli {
    cmd: String,
    project: PathBuf,
    proxy: String,
    repo: String,
    force: bool,
    log_file: Option<PathBuf>,
    /// config/setting 的子命令与键值（get/set <key> [value]）
    extra: Vec<String>,
}

fn parse_args() -> Result<Cli> {
    let mut args = std::env::args().skip(1);
    let mut cli = Cli {
        cmd: String::new(),
        project: std::env::current_dir()?,
        proxy: String::new(),
        repo: String::new(),
        force: false,
        log_file: None,
        extra: Vec::new(),
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => {
                cli.project = PathBuf::from(args.next().context("--project 缺少路径")?);
            }
            "--proxy" => {
                cli.proxy = args.next().context("--proxy 缺少地址")?;
            }
            "--repo" => {
                cli.repo = args.next().context("--repo 缺少仓库")?;
            }
            "--log" => {
                cli.log_file = Some(PathBuf::from(args.next().context("--log 缺少路径")?));
            }
            "--force" => cli.force = true,
            "--help" | "-h" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            s if s.starts_with('-') => {
                return Err(anyhow::anyhow!("未知选项: {s}\n\n{USAGE}"));
            }
            s => {
                if cli.cmd.is_empty() {
                    cli.cmd = s.to_string();
                } else {
                    // config/setting 的子命令与键值对交给 parse_kv 处理，此处收集
                    cli.extra.push(s.to_string());
                }
            }
        }
    }
    if cli.cmd.is_empty() {
        return Err(anyhow::anyhow!("缺少子命令\n\n{USAGE}"));
    }
    Ok(cli)
}

/// 无 GUI 模式的日志回调：输出到 stdout；若指定 --log 则同时写文件
struct Logger {
    file: Option<Mutex<std::fs::File>>,
}

impl Logger {
    fn new(path: Option<&PathBuf>, project_root: &Path) -> Result<Self> {
        let file = match path {
            Some(p) => {
                // 相对路径按项目根解析
                let abs = if p.is_absolute() { p.clone() } else { project_root.join(p) };
                if let Some(parent) = abs.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let f = std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&abs)
                    .with_context(|| format!("无法打开日志文件: {}", abs.display()))?;
                Some(Mutex::new(f))
            }
            None => None,
        };
        Ok(Self { file })
    }

    fn log(&self, line: &str) {
        println!("{line}");
        if let Some(f) = &self.file {
            use std::io::Write;
            if let Ok(mut f) = f.lock() {
                let _ = writeln!(f, "{line}");
            }
        }
    }
}

fn load_project_config(project_root: &Path) -> Result<(PathBuf, BgdConfig)> {
    let bgd_root = project_root.join(".bgd");
    if !bgd_root.is_dir() {
        return Err(anyhow::anyhow!(
            "项目未初始化（缺少 {}），请先执行 init",
            bgd_root.display()
        ));
    }
    let cfg = BgdConfig::load(&bgd_root)?;
    Ok((bgd_root, cfg))
}

/// 应用配置目录（与 GUI 共用 app_config_dir 约定）
fn app_config_dir() -> Result<PathBuf> {
    let dir = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("com.bgd.sce-tools");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 解析 config/setting 子命令的 (sub, key, value)
fn parse_kv(cli: &Cli, kind: &str) -> Result<(String, String, String)> {
    let sub = cli.extra.first().cloned().unwrap_or_default();
    let key = cli.extra.get(1).cloned().unwrap_or_default();
    let value = cli.extra.get(2).cloned().unwrap_or_default();
    match sub.as_str() {
        "get" => {
            if key.is_empty() {
                return Err(anyhow::anyhow!("{kind} get 缺少键名"));
            }
        }
        "set" => {
            if key.is_empty() || value.is_empty() {
                return Err(anyhow::anyhow!("{kind} set 缺少键名或值"));
            }
        }
        _ => {}
    }
    Ok((sub, key, value))
}

/// 按字段名写入 BgdConfig（覆盖项）。字符串字段直接赋值，数组字段按 JSON 解析
fn set_config_field(cfg: &mut BgdConfig, key: &str, value: &str) -> Result<()> {
    fn parse_list(v: &str) -> Result<Vec<String>> {
        serde_json::from_str(v).with_context(|| format!("数组字段需用 JSON 数组格式: {v}"))
    }
    match key {
        "project_root" => cfg.project_root = value.to_string(),
        "enable_build_log" => cfg.enable_build_log = matches!(value, "true" | "1" | "yes"),
        "server_entrance" => cfg.server_entrance = value.to_string(),
        "client_entrance" => cfg.client_entrance = value.to_string(),
        "libs_dir" => cfg.libs_dir = value.to_string(),
        "libs_server_target" => cfg.libs_server_target = value.to_string(),
        "libs_client_target" => cfg.libs_client_target = value.to_string(),
        "game_dir" => cfg.game_dir = value.to_string(),
        "game_server_target" => cfg.game_server_target = value.to_string(),
        "game_client_target" => cfg.game_client_target = value.to_string(),
        "libs_excludes" => cfg.libs_excludes = parse_list(value)?,
        "game_excludes" => cfg.game_excludes = parse_list(value)?,
        "framework_version" => cfg.framework_version = value.to_string(),
        "framework_repo" => cfg.framework_repo = value.to_string(),
        other => return Err(anyhow::anyhow!("未知配置键: {other}")),
    }
    Ok(())
}

const CMDS: [&str; 14] = [
    "build",
    "watch",
    "clean",
    "clean-logs",
    "init",
    "update-framework",
    "check-framework",
    "check-watch",
    "config",
    "setting",
    "app",
    "editor",
    "logs",
    "mcp",
];

/// 是否命中 CLI 调用（供 main 决定是否 AttachConsole）
pub fn is_cli_invocation() -> bool {
    std::env::args().nth(1).is_some()
}

/// 命中 CLI 子命令时执行并返回进程退出码（调用方保证已判断 is_cli_invocation）
pub fn run() -> i32 {
    let first = std::env::args().nth(1).unwrap_or_default();
    if !CMDS.contains(&first.as_str()) && first != "--help" && first != "-h" {
        eprintln!("未知子命令: {first}\n\n{USAGE}");
        return 2;
    }

    let result = (|| -> Result<()> {
        let cli = parse_args()?;
        let logger = std::sync::Arc::new(Logger::new(cli.log_file.as_ref(), &cli.project)?);
        let log = |line: &str| logger.log(line);
        match cli.cmd.as_str() {
            "build" => {
                let (bgd_root, cfg) = load_project_config(&cli.project)?;
                builder::build_all(&bgd_root, &cfg, &log)?;
            }
            "clean" => {
                let (bgd_root, cfg) = load_project_config(&cli.project)?;
                builder::clean(&bgd_root, &cfg, &log)?;
            }
            "clean-logs" => {
                let bgd_root = cli.project.join(".bgd");
                builder::clean_logs(&bgd_root, &log)?;
            }
            "init" => {
                let token = project::load_settings(&app_config_dir()?).github_token;
                let msg = project::init_project(&cli.project, &cli.repo, &cli.proxy, &token, cli.force, &log)?;
                logger.log(&msg);
            }
            "update-framework" => {
                let (_bgd_root, cfg) = load_project_config(&cli.project)?;
                let token = project::load_settings(&app_config_dir()?).github_token;
                let report =
                    project::update_framework(&cli.project, &cfg.framework_repo, &cli.proxy, &token, &log)?;
                logger.log(&format!(
                    "更新报告: version={} updated={} added={} removed={} kept_local={} conflicts={}",
                    report.version,
                    report.updated,
                    report.added,
                    report.removed,
                    report.kept_local,
                    report.conflicts.len()
                ));
                for c in &report.conflicts {
                    logger.log(&format!("  冲突: {c}"));
                }
                for n in &report.notes {
                    logger.log(&format!("  备注: {n}"));
                }
            }
            "check-framework" => {
                let (_bgd_root, cfg) = load_project_config(&cli.project)?;
                let token = project::load_settings(&app_config_dir()?).github_token;
                let latest = project::latest_framework_version(&cfg.framework_repo, &cli.proxy, &token)?;
                logger.log(&format!("当前: {}", cfg.framework_version));
                logger.log(&format!("最新: {}", latest.as_deref().unwrap_or("(无法获取)")));
            }
            "check-watch" => {
                let bgd_root = cli.project.join(".bgd");
                if builder::is_watching(&bgd_root) {
                    logger.log("监听中（保存即自动增量构建，无需手动构建）");
                } else {
                    logger.log("未监听（修改后需执行「全量构建」或本工具 build 命令）");
                }
            }
            "watch" => {
                let (bgd_root, cfg) = load_project_config(&cli.project)?;
                logger.log("监听已启动（前台阻塞，Ctrl+C 停止）");
                // start_watch 需要 'static 回调，复用共享的 logger
                let logger_arc = std::sync::Arc::clone(&logger);
                let log = move |line: &str| logger_arc.log(line);
                let _watcher = builder::start_watch(&bgd_root, &cfg, log)?;
                // 前台阻塞直到 Ctrl+C
                let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                let r = running.clone();
                ctrlc::set_handler(move || {
                    r.store(false, std::sync::atomic::Ordering::SeqCst);
                })
                .context("无法注册 Ctrl+C 处理")?;
                while running.load(std::sync::atomic::Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                logger.log("监听已停止");
            }
            "config" => {
                let (bgd_root, cfg) = load_project_config(&cli.project)?;
                let (sub, key, value) = parse_kv(&cli, "config")?;
                match sub.as_str() {
                    "get" => {
                        let v = serde_json::to_value(&cfg)?;
                        logger.log(&format!(
                            "{}",
                            v.get(&key).cloned().unwrap_or(serde_json::Value::Null)
                        ));
                    }
                    "set" => {
                        let mut cfg = cfg;
                        set_config_field(&mut cfg, &key, &value)?;
                        cfg.save(&bgd_root)?;
                        logger.log(&format!("已写入 bgd.json: {key} = {value}"));
                    }
                    other => return Err(anyhow::anyhow!("未知 config 子命令: {other}（get/set）")),
                }
            }
            "setting" => {
                let dir = app_config_dir()?;
                let (sub, key, value) = parse_kv(&cli, "setting")?;
                let mut settings = project::load_settings(&dir);
                match sub.as_str() {
                    "get" => {
                        let v = serde_json::to_value(&settings)?;
                        logger.log(&format!(
                            "{}",
                            v.get(&key).cloned().unwrap_or(serde_json::Value::Null)
                        ));
                    }
                    "set" => {
                        match key.as_str() {
                            "proxy" => settings.proxy = value.clone(),
                            "watch_enabled" => {
                                settings.watch_enabled = matches!(value.as_str(), "true" | "1" | "yes")
                            }
                            "github_token" => settings.github_token = value.clone(),
                            "editor_exe_name" => settings.editor_exe_name = value.clone(),
                            other => {
                                return Err(anyhow::anyhow!(
                                    "未知设置键: {other}（可用: proxy / watch_enabled / github_token / editor_exe_name）"
                                ))
                            }
                        }
                        project::save_settings(&dir, &settings)?;
                        logger.log(&format!("已写入应用设置: {key} = {value}"));
                    }
                    other => return Err(anyhow::anyhow!("未知 setting 子命令: {other}（get/set）")),
                }
            }
            "app" => {
                let app_id = cli.extra.first().cloned().unwrap_or_default();
                if app_id.is_empty() {
                    return Err(anyhow::anyhow!("app 缺少应用 id，用法: bgd_sce_tools app <应用id> [--project <路径>]"));
                }
                let app_exe = apps::app_exe_path(&app_id)?;
                if !app_exe.is_file() {
                    return Err(anyhow::anyhow!("应用 {app_id} 未安装（{} 不存在）", app_exe.display()));
                }
                let mut cmd = std::process::Command::new(&app_exe);
                // 透传 --project-path（应用可选实现；CLI 用 --project 指定，缺省当前目录）
                cmd.arg("--project-path").arg(&cli.project);
                cmd.spawn().with_context(|| format!("启动应用失败: {}", app_exe.display()))?;
                logger.log(&format!("已启动应用: {app_id}（--project-path {}）", cli.project.display()));
            }
            "editor" => {
                let sub = cli.extra.first().cloned().unwrap_or_default();
                let exe_name = {
                    let s = project::load_settings(&app_config_dir()?);
                    if s.editor_exe_name.trim().is_empty() {
                        bgd_sce_tools_lib::editor::DEFAULT_EDITOR_EXE.to_string()
                    } else {
                        s.editor_exe_name
                    }
                };
                match sub.as_str() {
                    "start" => {
                        let r = bgd_sce_tools_lib::editor::editor_start(
                            &cli.project,
                            &exe_name,
                            true,
                            120_000,
                        )?;
                        logger.log(&serde_json::to_string_pretty(&r)?);
                    }
                    "stop" => {
                        let r = bgd_sce_tools_lib::editor::editor_stop(&cli.project, &exe_name)?;
                        logger.log(&serde_json::to_string_pretty(&r)?);
                    }
                    other => {
                        return Err(anyhow::anyhow!("未知 editor 子命令: {other}（start/stop）"))
                    }
                }
            }
            "logs" => {
                let source = cli.extra.first().cloned().unwrap_or_else(|| "all".to_string());
                let tail: usize = cli
                    .extra
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let r = bgd_sce_tools_lib::editor::get_logs(&cli.project, &source, tail)?;
                logger.log(&serde_json::to_string_pretty(&r)?);
            }
            "mcp" => {
                // stdio MCP 主循环（阻塞直到客户端断开/stdin EOF），直接以其返回码退出
                std::process::exit(bgd_sce_tools_lib::mcp::run_stdio());
            }
            _ => unreachable!(),
        }
        Ok(())
    })();

    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("错误: {e:#}");
            1
        }
    }
}
