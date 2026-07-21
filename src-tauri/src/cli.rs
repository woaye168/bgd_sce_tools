//! CLI 子命令入口。
//!
//! 与 Tauri 命令共享同一套核心函数（builder.rs / project.rs），
//! 用于开发期本地功能验证与无 GUI 环境使用。
//! 约定：新增/修改任何构建或项目功能时，必须同步更新本模块（见 AGENTS.md）。

use anyhow::{Context, Result};
use bgd_sce_tools_lib::{builder, config::BgdConfig, project};
use std::path::{Path, PathBuf};

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

选项:
  --project <路径>        项目根目录（缺省为当前目录）
  --proxy <地址>          HTTP 代理，如 http://127.0.0.1:7897
  --repo <owner/repo>     框架仓库（缺省内置默认）
  --force                 init 时强制执行（存在 init.lock 时覆盖，自动备份）

示例:
  bgd_sce_tools build --project D:\\maps\\my_game
  bgd_sce_tools update-framework --proxy http://127.0.0.1:7897
";

struct Cli {
    cmd: String,
    project: PathBuf,
    proxy: String,
    repo: String,
    force: bool,
}

fn parse_args() -> Result<Cli> {
    let mut args = std::env::args().skip(1);
    let mut cli = Cli {
        cmd: String::new(),
        project: std::env::current_dir()?,
        proxy: String::new(),
        repo: String::new(),
        force: false,
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

/// 无 GUI 模式的日志回调：直接输出到 stdout
fn stdout_log(line: &str) {
    println!("{line}");
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

/// 命中 CLI 子命令时执行并返回进程退出码；否则返回 None（进入 GUI）
pub fn run() -> Option<i32> {
    // 无参数 => GUI；有参数但第一个是未知子命令 => 报 CLI 用法错误
    let Some(first) = std::env::args().nth(1) else {
        return None;
    };
    const CMDS: [&str; 6] = [
        "build",
        "clean",
        "clean-logs",
        "init",
        "update-framework",
        "check-framework",
    ];
    if !CMDS.contains(&first.as_str()) && first != "--help" && first != "-h" {
        eprintln!("未知子命令: {first}\n\n{USAGE}");
        return Some(2);
    }

    let result = (|| -> Result<()> {
        let cli = parse_args()?;
        let log = stdout_log;
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
                println!("{msg}");
            }
            "update-framework" => {
                let (_bgd_root, cfg) = load_project_config(&cli.project)?;
                let report =
                    project::update_framework(&cli.project, &cfg.framework_repo, &cli.proxy, &log)?;
                println!(
                    "更新报告: version={} updated={} added={} removed={} kept_local={} conflicts={}",
                    report.version,
                    report.updated,
                    report.added,
                    report.removed,
                    report.kept_local,
                    report.conflicts.len()
                );
                for c in &report.conflicts {
                    println!("  冲突: {c}");
                }
                for n in &report.notes {
                    println!("  备注: {n}");
                }
            }
            "check-framework" => {
                let (_bgd_root, cfg) = load_project_config(&cli.project)?;
                let latest = project::latest_framework_version(&cfg.framework_repo, &cli.proxy)?;
                println!("当前: {}", cfg.framework_version);
                println!("最新: {}", latest.as_deref().unwrap_or("(无法获取)"));
            }
            _ => unreachable!(),
        }
        Ok(())
    })();

    match result {
        Ok(()) => Some(0),
        Err(e) => {
            eprintln!("错误: {e:#}");
            Some(1)
        }
    }
}
