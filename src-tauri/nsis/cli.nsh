; 功能：随应用安装 CLI，提供用户 PATH 选项，卸载只移除本安装加入的路径。
!include "LogicLib.nsh"
!define PINPAGE_CLI_SOURCE "${__FILEDIR__}\..\binaries\pinpage-cli.exe"

LangString PinPagePathQuestion ${LANG_SIMPCHINESE} "是否将 PinPage CLI 加入当前用户的 PATH？新终端可直接使用 pinpage-cli。"
LangString PinPagePathQuestion ${LANG_ENGLISH} "Add PinPage CLI to your user PATH? New terminals can run pinpage-cli directly."

; 输入为 $INSTDIR；只精确移除一个目录分量，保留用户的其他 PATH 项。
!macro PinPageRemovePath PREFIX
Function ${PREFIX}PinPageRemovePath
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  ClearErrors
  ReadRegStr $0 HKCU "Environment" "Path"
  IfErrors restore_${PREFIX}
  StrCpy $1 ""
  StrCpy $2 ""
  StrCpy $3 0
  loop_${PREFIX}:
    StrCpy $4 $0 1 $3
    StrCmp $4 "" flush_${PREFIX}
    StrCmp $4 ";" flush_${PREFIX}
    StrCpy $2 "$2$4"
    IntOp $3 $3 + 1
    Goto loop_${PREFIX}
  flush_${PREFIX}:
    StrCmp $2 "$INSTDIR" next_${PREFIX}
    StrCmp $2 "" next_${PREFIX}
    StrCmp $1 "" 0 +3
      StrCpy $1 "$2"
      Goto next_${PREFIX}
    StrCpy $1 "$1;$2"
  next_${PREFIX}:
    StrCpy $2 ""
    StrCmp $4 "" done_${PREFIX}
    IntOp $3 $3 + 1
    Goto loop_${PREFIX}
  done_${PREFIX}:
    WriteRegExpandStr HKCU "Environment" "Path" "$1"
    System::Call 'user32::SendMessageTimeoutW(p 0xffff, i 0x1a, p 0, w "Environment", i 2, i 1000, *p .r4)'
  restore_${PREFIX}:
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd
!macroend
!insertmacro PinPageRemovePath ""
!insertmacro PinPageRemovePath "un."

!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR"
  File /oname=pinpage-cli.exe "${PINPAGE_CLI_SOURCE}"
  ReadRegStr $0 HKCU "Software\PinPage\CLI" "PathAdded"
  StrCmp $0 "$INSTDIR" pinpage_path_done
  ${IfNot} ${Silent}
    MessageBox MB_YESNO|MB_DEFBUTTON2 "$(PinPagePathQuestion)" IDNO pinpage_path_done
    ; 保留已存在的目录，不把用户原有 PATH 项标成由安装器创建。
    ClearErrors
    ReadRegStr $0 HKCU "Environment" "Path"
    IfErrors pinpage_path_done
    StrLen $1 $0
    StrLen $2 "$INSTDIR"
    IntOp $1 $1 + $2
    IntOp $1 $1 + 2
    IntCmp $1 ${NSIS_MAX_STRLEN} pinpage_path_done 0 pinpage_path_done
    ; 精确检查已有目录分量。
    StrCpy $1 ";$0;"
    StrCpy $2 ";$INSTDIR;"
    StrLen $3 $2
    StrCpy $4 0
    pinpage_find_path:
      StrCpy $5 $1 $3 $4
      StrCmp $5 $2 pinpage_path_done
      StrCmp $5 "" pinpage_append_path
      IntOp $4 $4 + 1
      Goto pinpage_find_path
    pinpage_append_path:
      StrCmp $0 "" 0 +3
        StrCpy $0 "$INSTDIR"
        Goto pinpage_write_path
      StrCpy $0 "$0;$INSTDIR"
    pinpage_write_path:
      WriteRegExpandStr HKCU "Environment" "Path" "$0"
      WriteRegStr HKCU "Software\PinPage\CLI" "PathAdded" "$INSTDIR"
      System::Call 'user32::SendMessageTimeoutW(p 0xffff, i 0x1a, p 0, w "Environment", i 2, i 1000, *p .r5)'
  ${EndIf}
  pinpage_path_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\pinpage-cli.exe"
  ReadRegStr $0 HKCU "Software\PinPage\CLI" "PathAdded"
  StrCmp $0 "$INSTDIR" 0 pinpage_uninstall_done
    Call un.PinPageRemovePath
    DeleteRegKey HKCU "Software\PinPage\CLI"
  pinpage_uninstall_done:
!macroend
