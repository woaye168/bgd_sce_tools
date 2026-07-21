//! CLI 子命令入口。
//!
//! 与 Tauri 命令共享同一套核心函数（builder.rs / project.rs），
//! 用于开发期本地功能验证与无 GUI 环境使用。
//! 约定：新增/修改任何构建或项目功能时，必须同步更新本模块（见 AGENTS.md）。

use anyhow::{Context, Result};
use bgd_sce_tools_lib::{builder, config::BgdConfig, project};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const USAGE: &str = "\
bgd_sce_tools CLI

用法: bgd_sce_tools <子命令> [选项]

子命令:
  build                  全量构建
  clean                  清除构建产物（还原入口原文）
  clean-logs             清理 .bgd/log 下的 .log 文件
  init                   初始化项目（下载框架生成 .bgd）
  update-framework       增量更新框架（三路哈希对比）
  check-framework        检查框架是否有新版本
  check-watch            检查项目当前是否处于监听中

选项:
  --project <路径>        项目根目录（缺省为当前目录）
  --proxy <地址>          HTTP 代理，如 http://127.0.0.1:7897
  --repo <owner/repo>     框架仓库（缺省内置默认）
  --force                 init 时强制执行（存在 init.lock 时覆盖，自动备份）
  --log <路径>            将过程日志同时写入指定文件（便于无 GUI 环境查看）

示例:
  bgd_sce_tools build --project D:\\maps\\my_game --log .bgd/log/build.log
  bgd_sce_tools check-watch --project D:\\maps\\my_game
";

struct Cli {
    cmd: String,
    project: PathBuf,
    proxy: String,
    repo: String,
    force: bool,
    log_file: Option<PathBuf>,
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
                if !cli.cmd.is_empty() {
                    return Err(anyhow::anyhow!("多余的参数: {s}\n\n{USAGE}"));
                }
                cli.cmd = s.to_string();
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

const CMDS: [&str; 7] = [
    "build",
    "clean",
    "clean-logs",
    "init",
    "update-framework",
    "check-framework",
    "check-watch",
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
        let logger = Logger::new(cli.log_file.as_ref(), &cli.project)?;
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
                let msg = project::init_project(&cli.project, &cli.repo, &cli.proxy, cli.force, &log)?;
                logger.log(&msg);
            }
            "update-framework" => {
                let (_bgd_root, cfg) = load_project_config(&cli.project)?;
                let report =
                    project::update_framework(&cli.project, &cfg.framework_repo, &cli.proxy, &log)?;
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
                let latest = project::latest_framework_version(&cfg.framework_repo, &cli.proxy)?;
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
