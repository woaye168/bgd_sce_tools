; NSIS 安装钩子：把安装目录写入用户 PATH（便于 CLI 直接调用）
; 注意：Tauri 在 Section 上下文之外调用本钩子，不能使用 StrFunc 等
; 只能在 Section/Function 内使用的指令，统一用 nsExec::Exec 调 PowerShell 处理

!macro NSIS_HOOK_POSTINSTALL
  ; 追加安装目录到用户 PATH（PowerShell 内部判断重复，幂等）
  nsExec::Exec 'powershell -NoProfile -WindowStyle Hidden -Command "$i=''$INSTDIR''; $p=(Get-ItemProperty -Path ''HKCU:\Environment'' -Name Path -ErrorAction SilentlyContinue).Path; if (($p -split '';'' ) -notcontains $i) { $n = if ($p) { $p + '';'' + $i } else { $i }; Set-ItemProperty -Path ''HKCU:\Environment'' -Name Path -Value $n }"'
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; 从用户 PATH 移除安装目录
  nsExec::Exec 'powershell -NoProfile -WindowStyle Hidden -Command "$i=''$INSTDIR''; $p=(Get-ItemProperty -Path ''HKCU:\Environment'' -Name Path -ErrorAction SilentlyContinue).Path; $n=($p -split '';'' | Where-Object { $_ -and ($_ -ne $i) }) -join '';''; Set-ItemProperty -Path ''HKCU:\Environment'' -Name Path -Value $n"'
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend
