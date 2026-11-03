; Uninstalling also removes the logon task the app creates (startup.rs); the Run entry is
; removed by Tauri's own uninstaller.
!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'schtasks.exe /Delete /TN "FlowVoice" /F'
  Pop $0
!macroend
