; NSIS 安装钩子：把安装目录写入用户 PATH（便于 CLI 直接调用）
; 用 StrStr 判断是否已存在（避免 StrFunc 版本兼容问题），仅不存在才追加

!include "StrFunc.nsh"
${StrStr}

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  ${StrStr} $1 "$0" "$INSTDIR"
  StrCmp $1 "" 0 hook_addpath_done
    StrCmp $0 "" 0 hook_addpath_semi
      StrCpy $0 "$INSTDIR"
      Goto hook_addpath_write
    hook_addpath_semi:
      StrCpy $0 "$0;$INSTDIR"
    hook_addpath_write:
      WriteRegStr HKCU "Environment" "Path" "$0"
      SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  hook_addpath_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; 卸载：从用户 PATH 移除安装目录（StrStr 判断存在后，用 StrRep 精确移除）
  ReadRegStr $0 HKCU "Environment" "Path"
  ${StrStr} $1 "$0" "$INSTDIR"
  StrCmp $1 "" hook_rmpath_done
    ; 先移除带尾分号的形式，再移除带头分号的形式，最后移除单独形式
    ${StrRep} $1 "$0" "$INSTDIR;" ""
    ${StrRep} $1 "$1" ";$INSTDIR" ""
    ${StrRep} $1 "$1" "$INSTDIR" ""
    WriteRegStr HKCU "Environment" "Path" "$1"
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  hook_rmpath_done:
!macroend
