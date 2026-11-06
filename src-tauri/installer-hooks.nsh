; 卸载时清理应用数据和管理员 WebView 缓存。
; 作者：lhy
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = ${BST_CHECKED}
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    Delete "$LOCALAPPDATA\SplitRoute\preferences.json"
    Delete "$LOCALAPPDATA\SplitRoute\config.json"
    Delete "$LOCALAPPDATA\SplitRoute\recovery.json"
    Delete "$LOCALAPPDATA\SplitRoute\preferences.tmp"
    Delete "$LOCALAPPDATA\SplitRoute\config.tmp"
    Delete "$LOCALAPPDATA\SplitRoute\recovery.tmp"
    RmDir "$LOCALAPPDATA\SplitRoute"
    RmDir /r "$LOCALAPPDATA\${BUNDLEID}.admin"
  ${EndIf}
!macroend
