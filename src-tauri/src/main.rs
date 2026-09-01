// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;

/// release 模式是 windows 子系统（无控制台），CLI 子命令的 stdout 会被吞。
/// 命中 CLI 时附加到父进程控制台，使 println 输出到调用方终端。
#[cfg(windows)]
fn attach_parent_console() {
    unsafe {
        // ATTACH_PARENT_PROCESS = -1 (0xFFFFFFFF)
        windows_sys::Win32::System::Console::AttachConsole(0xFFFFFFFF);
    }
}

fn main() {
    // 释放内建默认配置到 exe 旁（可见性用途；CLI/GUI 两条路径都经过这里）
    bgd_sce_tools_lib::config::release_embedded_defaults();
    // 命中 CLI 子命令则以控制台模式执行；否则启动 GUI
    if cli::is_cli_invocation() {
        #[cfg(windows)]
        attach_parent_console();
        std::process::exit(cli::run());
    }
    bgd_sce_tools_lib::run()
}
