; NSIS 安装钩子（保留占位；PATH 注册已改为应用启动时自检写入，见 lib.rs ensure_path_registered）
; 原因：Tauri 对 NSIS_HOOK_POSTINSTALL 的调用时机在更新/重装场景下不稳定，
; 改为应用每次启动时检查并写入用户 PATH，确定性更高。

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
