// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;

fn main() {
    // 命中 CLI 子命令则以控制台模式执行；否则启动 GUI
    if let Some(code) = cli::run() {
        std::process::exit(code);
    }
    bgd_sce_tools_lib::run()
}
