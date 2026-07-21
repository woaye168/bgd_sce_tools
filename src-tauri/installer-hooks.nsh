; NSIS 安装钩子：把安装目录写入用户 PATH（便于 CLI 直接调用）
; 仅当 PATH 中不存在该目录时才追加，避免重复与超长截断

!include "StrFunc.nsh"
${StrRep}

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  ${StrRep} $1 "$0" "$INSTDIR" ""
  StrCmp $1 $0 0 hook_addpath_done
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
  ReadRegStr $0 HKCU "Environment" "Path"
  ${StrRep} $1 "$0" ";$INSTDIR" ""
  ${StrRep} $1 "$1" "$INSTDIR;" ""
  ${StrRep} $1 "$1" "$INSTDIR" ""
  WriteRegStr HKCU "Environment" "Path" "$1"
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend
